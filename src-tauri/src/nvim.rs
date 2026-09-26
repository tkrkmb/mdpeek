use async_trait::async_trait;
use nvim_rs::{compat::tokio::Compat, create::tokio::new_path, Handler, Neovim, Value};
use tauri::{AppHandle, Manager};
use tokio::{
    io::WriteHalf,
    net::UnixStream,
    sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
};

use crate::{render, Args, Cursor, Document, Documents, History, Session};

type Writer = Compat<WriteHalf<UnixStream>>;
pub type Nvim = Neovim<Writer>;

#[derive(Clone)]
struct NvimHandler {
    app: AppHandle,
    queue: UnboundedSender<Value>,
}

#[async_trait]
impl Handler for NvimHandler {
    type Writer = Writer;

    async fn handle_notify(&self, name: String, args: Vec<Value>, _nvim: Neovim<Writer>) {
        // 受け取ったらキューに渡してすぐ返す。描画の完了は待たない。
        match name.as_str() {
            "mdsight_content" => {
                if let Some(payload) = args.into_iter().next() {
                    let _ = self.queue.send(payload);
                }
            }
            "mdsight_cursor" => {
                if let Some(cursor) = args.first().and_then(cursor_from) {
                    crate::publish_cursor(&self.app, cursor);
                }
            }
            "mdsight_close" => self.app.exit(0),
            _ => {}
        }
    }
}

/// キューに溜まっているもののうち、最後の本文を返す。
/// あわせて、表示中の世代（`shown`）から先に変わった世代のうち、最初の送信の
/// `follow` が `false` のもの（`:MdSight` による切り替えなど）があったかを返す。
/// 途中の本文を捨てても、窓を前面に出すかどうかの判断が変わらないようにするため。
fn newest(first: Value, queue: &mut UnboundedReceiver<Value>, shown: Option<u64>) -> (Value, bool) {
    let mut previous = shown;
    let mut raise = false;
    let mut latest = first;
    loop {
        let generation = field(&latest, "gen").and_then(Value::as_u64);
        if generation != previous {
            raise |= !follows(&latest);
            previous = generation;
        }
        match queue.try_recv() {
            Ok(newer) => latest = newer,
            Err(_) => return (latest, raise),
        }
    }
}

/// `mdsight_content` の `follow`（対象ウィンドウ内の移動への追従か）
fn follows(value: &Value) -> bool {
    field(value, "follow").and_then(Value::as_bool).unwrap_or(false)
}

/// キューに届いた本文のうち、最新のものだけをHTMLに変換して送る。
async fn render_queue(app: AppHandle, mut queue: UnboundedReceiver<Value>) {
    while let Some(payload) = queue.recv().await {
        let shown = app.state::<Documents>().current().map(|current| current.generation);
        let (payload, raise) = newest(payload, &mut queue, shown);
        if let Some(document) = document_from(&payload) {
            // 対象世代が変わっていたら、自分(open_link/go_back/go_forward)による
            // ものかどうかを確かめ、そうでなければリンクの履歴を空にする。
            // そのうち:MdSightによる切り替えなどでは、隠れていれば窓を前に出す
            // (対象ウィンドウ内の移動への追従では出さない)
            let retargeted = shown.is_some_and(|generation| generation != document.generation);
            if retargeted && app.state::<History>().reset_if_unexpected(document.generation) && raise {
                crate::raise::show_without_focus(&app);
            }
            crate::publish(&app, document);
        }
    }
}

/// Neovimのソケットに接続し、登録してから、切断されるまで待つ。
pub async fn run(app: AppHandle, args: Args) -> Result<(), String> {
    let (queue, incoming) = unbounded_channel();
    tauri::async_runtime::spawn(render_queue(app.clone(), incoming));

    let handler = NvimHandler {
        app: app.clone(),
        queue,
    };
    let (nvim, io) = new_path(&args.socket, handler)
        .await
        .map_err(|err| format!("cannot connect to {}: {err}", args.socket))?;

    let info = nvim
        .get_api_info()
        .await
        .map_err(|err| format!("nvim_get_api_info() failed: {err}"))?;
    let chan = info
        .first()
        .and_then(Value::as_u64)
        .ok_or_else(|| "nvim_get_api_info() returned no channel id".to_string())?;

    let registered = nvim
        .exec_lua(
            r#"return require("mdsight.rpc").register(...)"#,
            vec![Value::from(args.token.as_str()), Value::from(chan)],
        )
        .await
        .map_err(|err| format!("register failed: {err}"))?;

    let document =
        document_from(&registered).ok_or_else(|| "register was rejected".to_string())?;
    // ジャンプ要求に答えられるように、接続を預けておく
    app.state::<Session>().open(nvim.clone());
    crate::publish(&app, document);
    if let Some(line) = field(&registered, "line").and_then(Value::as_u64) {
        crate::publish_cursor(
            &app,
            Cursor {
                generation: document_generation(&registered),
                line,
            },
        );
    }

    // RPCが切断されたら、待ち受けを終える
    io.await
        .map_err(|err| format!("rpc loop stopped: {err}"))?
        .map_err(|err| format!("rpc loop stopped: {err}"))?;
    Ok(())
}

fn document_generation(value: &Value) -> u64 {
    field(value, "gen").and_then(Value::as_u64).unwrap_or_default()
}

/// `mdsight_cursor` の `{gen, line}` を取り出す。
fn cursor_from(value: &Value) -> Option<Cursor> {
    Some(Cursor {
        generation: field(value, "gen")?.as_u64()?,
        line: field(value, "line")?.as_u64()?,
    })
}

fn field<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    value
        .as_map()?
        .iter()
        .find(|(name, _)| name.as_str() == Some(key))
        .map(|(_, found)| found)
}

/// registerの戻り値（対象世代、版、本文の行配列、文書の絶対パス）を取り出す。
fn document_from(value: &Value) -> Option<Document> {
    let lines: Vec<&str> = field(value, "lines")?
        .as_array()?
        .iter()
        .map(|line| line.as_str().unwrap_or_default())
        .collect();
    Some(Document {
        generation: field(value, "gen")?.as_u64()?,
        version: field(value, "version")?.as_u64()?,
        path: field(value, "path")?.as_str()?.to_string(),
        html: render::to_html(&lines.join("\n")),
    })
}

/// アプリからNeovimへジャンプを要求する。
pub async fn jump(nvim: Nvim, gen: u64, version: u64, line: u64) -> Result<(), String> {
    nvim.exec_lua(
        r#"return require("mdsight.rpc").jump(...)"#,
        vec![Value::from(gen), Value::from(version), Value::from(line)],
    )
    .await
    .map(|_| ())
    .map_err(|err| format!("jump failed: {err}"))
}

/// アプリからNeovimへ、リンクで解決した絶対パスを開くよう要求する。
/// 成功したら、Neovim側で新しくなった対象世代・版を返す
/// (本文とカーソル行は、いつもどおり別の通知で届く)。
pub async fn open(nvim: Nvim, gen: u64, version: u64, path: &str) -> Result<(u64, u64), String> {
    let result = nvim
        .exec_lua(
            r#"return require("mdsight.rpc").open(...)"#,
            vec![Value::from(gen), Value::from(version), Value::from(path)],
        )
        .await
        .map_err(|err| format!("open failed: {err}"))?;
    open_result(&result).ok_or_else(|| "nvim declined to open the file".to_string())
}

/// `rpc.open` の戻り値から `{gen, version}` を取り出す。失敗（`false`）なら `None`。
fn open_result(value: &Value) -> Option<(u64, u64)> {
    Some((
        field(value, "gen")?.as_u64()?,
        field(value, "version")?.as_u64()?,
    ))
}

#[cfg(test)]
mod tests {
    use super::{document_from, newest, open_result};
    use nvim_rs::Value;
    use tokio::sync::mpsc::unbounded_channel;

    fn entry(key: &str, value: Value) -> (Value, Value) {
        (Value::from(key), value)
    }

    #[test]
    fn reads_the_register_result() {
        let value = Value::Map(vec![
            entry("gen", Value::from(2u64)),
            entry("version", Value::from(7u64)),
            entry("path", Value::from("/tmp/note.md")),
            entry(
                "lines",
                Value::Array(vec![Value::from("# title"), Value::from("")]),
            ),
            entry("line", Value::from(1u64)),
        ]);
        let document = document_from(&value).expect("a document");
        assert_eq!(document.generation, 2);
        assert_eq!(document.version, 7);
        assert_eq!(document.path, "/tmp/note.md");
        assert!(document.html.contains("title"), "{}", document.html);
    }

    #[test]
    fn reads_a_cursor_notification() {
        let value = Value::Map(vec![
            entry("gen", Value::from(3u64)),
            entry("line", Value::from(42u64)),
        ]);
        let cursor = super::cursor_from(&value).expect("a cursor");
        assert_eq!(cursor.generation, 3);
        assert_eq!(cursor.line, 42);
    }

    #[test]
    fn rejects_a_cursor_without_a_line() {
        let value = Value::Map(vec![entry("gen", Value::from(3u64))]);
        assert!(super::cursor_from(&value).is_none());
    }

    #[test]
    fn rejects_a_nil_result() {
        assert!(document_from(&Value::Nil).is_none());
    }

    #[test]
    fn reads_the_open_result() {
        let value = Value::Map(vec![entry("gen", Value::from(4u64)), entry("version", Value::from(9u64))]);
        assert_eq!(open_result(&value), Some((4, 9)));
    }

    #[test]
    fn treats_a_declined_open_as_none() {
        assert_eq!(open_result(&Value::from(false)), None);
    }

    fn content(gen: u64, version: u64, follow: bool) -> Value {
        Value::Map(vec![
            entry("gen", Value::from(gen)),
            entry("version", Value::from(version)),
            entry("follow", Value::from(follow)),
        ])
    }

    /// キューに入れた本文を newest に渡し、(最後の版, 前面に出すか) を返す
    fn drain(payloads: Vec<Value>, shown: u64) -> (u64, bool) {
        let (queue, mut incoming) = unbounded_channel();
        for payload in payloads {
            queue.send(payload).expect("the queue is open");
        }
        let first = incoming.try_recv().expect("a queued payload");
        let (latest, raise) = newest(first, &mut incoming, Some(shown));
        (super::field(&latest, "version").and_then(Value::as_u64).expect("a version"), raise)
    }

    #[test]
    fn keeps_only_the_last_queued_content() {
        let payloads = (1u64..=3).map(|version| content(1, version, false)).collect();
        assert_eq!(drain(payloads, 1), (3, false));
    }

    #[test]
    fn raises_when_the_target_is_switched_by_the_command() {
        assert_eq!(drain(vec![content(2, 5, false)], 1), (5, true));
    }

    #[test]
    fn does_not_raise_when_following_the_window() {
        assert_eq!(drain(vec![content(2, 5, true)], 1), (5, false));
    }

    #[test]
    fn does_not_raise_when_an_edit_follows_a_window_move_in_the_same_batch() {
        let payloads = vec![content(2, 5, true), content(2, 6, false)];
        assert_eq!(drain(payloads, 1), (6, false));
    }

    #[test]
    fn raises_when_a_command_switch_is_hidden_behind_a_window_move() {
        let payloads = vec![content(2, 5, false), content(3, 6, true)];
        assert_eq!(drain(payloads, 1), (6, true));
    }
}
