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
            "mdpeek_content" => {
                if let Some(payload) = args.into_iter().next() {
                    let _ = self.queue.send(payload);
                }
            }
            "mdpeek_cursor" => {
                if let Some(cursor) = args.first().and_then(cursor_from) {
                    crate::publish_cursor(&self.app, cursor);
                }
            }
            "mdpeek_close" => self.app.exit(0),
            _ => {}
        }
    }
}

/// キューに溜まっているもののうち、最後の本文を返す。
fn newest(first: Value, queue: &mut UnboundedReceiver<Value>) -> Value {
    let mut latest = first;
    while let Ok(newer) = queue.try_recv() {
        latest = newer;
    }
    latest
}

/// キューに届いた本文のうち、最新のものだけをHTMLに変換して送る。
async fn render_queue(app: AppHandle, mut queue: UnboundedReceiver<Value>) {
    while let Some(payload) = queue.recv().await {
        let payload = newest(payload, &mut queue);
        if let Some(document) = document_from(&payload) {
            // 対象世代が変わっていたら、自分(open_link/go_back/go_forward)による
            // ものかどうかを確かめ、そうでなければ(:MdPeekによる切り替えなど)
            // リンクの履歴を空にして、隠れていれば窓を前に出す
            let retargeted = app
                .state::<Documents>()
                .current()
                .is_some_and(|current| current.generation != document.generation);
            if retargeted && app.state::<History>().reset_if_unexpected(document.generation) {
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
            r#"return require("mdpeek.rpc").register(...)"#,
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

/// `mdpeek_cursor` の `{gen, line}` を取り出す。
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
        r#"return require("mdpeek.rpc").jump(...)"#,
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
            r#"return require("mdpeek.rpc").open(...)"#,
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

    #[test]
    fn keeps_only_the_last_queued_content() {
        let (queue, mut incoming) = unbounded_channel();
        for version in 1u64..=3 {
            queue.send(Value::from(version)).expect("the queue is open");
        }
        let first = incoming.try_recv().expect("a queued payload");
        assert_eq!(newest(first, &mut incoming), Value::from(3u64));
    }
}
