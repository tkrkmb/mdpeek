mod nvim;
mod render;

use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

/// 最新の本文をフロントエンドへ届けるイベント名
const DOCUMENT_EVENT: &str = "mdpeek://document";

pub struct Args {
    pub socket: String,
    pub token: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct Document {
    #[serde(rename = "gen")]
    pub generation: u64,
    pub version: u64,
    pub path: String,
    pub html: String,
}

/// 世代と版が最新の本文だけを保持する
#[derive(Default)]
pub struct Documents(Mutex<Option<Document>>);

impl Documents {
    fn accept(&self, document: Document) -> bool {
        let mut slot = self.0.lock().expect("documents lock");
        if let Some(current) = slot.as_ref() {
            if (document.generation, document.version) <= (current.generation, current.version) {
                return false;
            }
        }
        *slot = Some(document);
        true
    }

    fn current(&self) -> Option<Document> {
        self.0.lock().expect("documents lock").clone()
    }
}

/// Neovimが先に終了して標準エラー出力のパイプが閉じていても、
/// 書き込みの失敗でパニックしないようにする。
fn report(message: &str) {
    use std::io::Write;
    let _ = writeln!(std::io::stderr(), "mdpeek: {message}");
}

pub fn publish(app: &AppHandle, document: Document) {
    if app.state::<Documents>().accept(document.clone()) {
        let _ = app.emit(DOCUMENT_EVENT, document);
    }
}

#[tauri::command]
fn current_document(documents: tauri::State<'_, Documents>) -> Option<Document> {
    documents.current()
}

fn parse_args(argv: impl Iterator<Item = String>) -> Result<Args, String> {
    let mut socket = None;
    let mut token = None;
    let mut argv = argv;
    while let Some(arg) = argv.next() {
        match arg.as_str() {
            "--nvim" => socket = argv.next(),
            "--token" => token = argv.next(),
            other => return Err(format!("unknown argument: {other}")),
        }
    }
    match (socket, token) {
        (Some(socket), Some(token)) => Ok(Args { socket, token }),
        _ => Err("usage: mdpeek --nvim <socket> --token <token>".to_string()),
    }
}

fn main() {
    let args = match parse_args(std::env::args().skip(1)) {
        Ok(args) => args,
        Err(message) => {
            report(&message);
            std::process::exit(2);
        }
    };

    tauri::Builder::default()
        .manage(Documents::default())
        .invoke_handler(tauri::generate_handler![current_document])
        .setup(move |app| {
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                if let Err(message) = nvim::run(handle.clone(), args).await {
                    report(&message);
                }
                // RPCが切断されたら、アプリを終了する
                handle.exit(0);
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("failed to run mdpeek");
}

#[cfg(test)]
mod tests {
    use super::{parse_args, Document, Documents};

    fn document(generation: u64, version: u64) -> Document {
        Document {
            generation,
            version,
            path: "/tmp/note.md".to_string(),
            html: String::new(),
        }
    }

    #[test]
    fn parses_the_expected_arguments() {
        let args = parse_args(
            ["--nvim", "/tmp/nvim.sock", "--token", "abc"]
                .into_iter()
                .map(String::from),
        )
        .expect("args");
        assert_eq!(args.socket, "/tmp/nvim.sock");
        assert_eq!(args.token, "abc");
    }

    #[test]
    fn rejects_missing_arguments() {
        assert!(parse_args(["--nvim", "/tmp/nvim.sock"].into_iter().map(String::from)).is_err());
    }

    #[test]
    fn keeps_only_the_newest_document() {
        let documents = Documents::default();
        assert!(documents.accept(document(1, 1)));
        assert!(!documents.accept(document(1, 1)));
        assert!(!documents.accept(document(1, 0)));
        assert!(documents.accept(document(1, 2)));
        assert_eq!(documents.current().expect("a document").version, 2);
    }
}
