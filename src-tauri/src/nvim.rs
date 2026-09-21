use async_trait::async_trait;
use nvim_rs::{compat::tokio::Compat, create::tokio::new_path, Handler, Neovim, Value};
use tauri::AppHandle;
use tokio::{io::WriteHalf, net::UnixStream};

use crate::{render, Args, Document};

type Writer = Compat<WriteHalf<UnixStream>>;

#[derive(Clone)]
struct NvimHandler {
    app: AppHandle,
}

#[async_trait]
impl Handler for NvimHandler {
    type Writer = Writer;

    async fn handle_notify(&self, name: String, _args: Vec<Value>, _nvim: Neovim<Writer>) {
        // 受け取ったらすぐ返す。描画の完了は待たない。
        if name == "mdpeek_close" {
            self.app.exit(0);
        }
    }
}

/// Neovimのソケットに接続し、登録してから、切断されるまで待つ。
pub async fn run(app: AppHandle, args: Args) -> Result<(), String> {
    let handler = NvimHandler { app: app.clone() };
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
    crate::publish(&app, document);

    // RPCが切断されたら、待ち受けを終える
    io.await
        .map_err(|err| format!("rpc loop stopped: {err}"))?
        .map_err(|err| format!("rpc loop stopped: {err}"))?;
    Ok(())
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

#[cfg(test)]
mod tests {
    use super::document_from;
    use nvim_rs::Value;

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
    fn rejects_a_nil_result() {
        assert!(document_from(&Value::Nil).is_none());
    }
}
