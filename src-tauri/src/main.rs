mod image;
mod nvim;
mod render;

use std::sync::Mutex;

use std::path::Path;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

/// 最新の本文をフロントエンドへ届けるイベント名
const DOCUMENT_EVENT: &str = "mdpeek://document";
/// カーソル行をフロントエンドへ届けるイベント名
const CURSOR_EVENT: &str = "mdpeek://cursor";

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

#[derive(Clone, Copy, Debug, Serialize)]
pub struct Cursor {
    #[serde(rename = "gen")]
    pub generation: u64,
    pub line: u64,
}

/// ジャンプ要求のために、Neovimとの接続を預かる
#[derive(Default)]
pub struct Session(Mutex<Option<nvim::Nvim>>);

impl Session {
    pub fn open(&self, nvim: nvim::Nvim) {
        *self.0.lock().expect("session lock") = Some(nvim);
    }

    fn get(&self) -> Option<nvim::Nvim> {
        self.0.lock().expect("session lock").clone()
    }
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

/// 起動直後の取りこぼしに備え、最新のカーソル行を保持する
#[derive(Default)]
pub struct Cursors(Mutex<Option<Cursor>>);

impl Cursors {
    fn accept(&self, cursor: Cursor) {
        *self.0.lock().expect("cursors lock") = Some(cursor);
    }

    fn current(&self) -> Option<Cursor> {
        *self.0.lock().expect("cursors lock")
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

pub fn publish_cursor(app: &AppHandle, cursor: Cursor) {
    app.state::<Cursors>().accept(cursor);
    let _ = app.emit(CURSOR_EVENT, cursor);
}

#[tauri::command]
fn current_document(documents: tauri::State<'_, Documents>) -> Option<Document> {
    documents.current()
}

/// 起動直後に取りこぼしたカーソル行を、フロントエンドが拾い直す。
#[tauri::command]
fn current_cursor(cursors: tauri::State<'_, Cursors>) -> Option<Cursor> {
    cursors.current()
}

/// プレビュー側の修飾クリックを、Neovimのカーソル移動に変える。
#[tauri::command]
async fn jump(
    session: tauri::State<'_, Session>,
    gen: u64,
    version: u64,
    line: u64,
) -> Result<(), String> {
    let nvim = session.get().ok_or_else(|| "not connected".to_string())?;
    nvim::jump(nvim, gen, version, line).await
}

/// 相対パスの画像を、文書のディレクトリを基準に解決する。
/// 解決できたファイルだけを、1つずつassetプロトコルのスコープに加える。
#[tauri::command]
fn resolve_image(
    app: AppHandle,
    documents: tauri::State<'_, Documents>,
    path: String,
    version: u64,
) -> Result<String, String> {
    let document = documents.current().ok_or_else(|| "no document".to_string())?;
    if document.version != version {
        return Err("the document has moved on".to_string());
    }
    let base = Path::new(&document.path)
        .parent()
        .ok_or_else(|| "the document has no directory".to_string())?;

    let resolved = image::resolve(base, &path)?;
    app.asset_protocol_scope()
        .allow_file(&resolved)
        .map_err(|err| format!("cannot allow {}: {err}", resolved.display()))?;
    Ok(resolved.to_string_lossy().into_owned())
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
        .plugin(tauri_plugin_opener::init())
        .manage(Documents::default())
        .manage(Cursors::default())
        .manage(Session::default())
        .invoke_handler(tauri::generate_handler![
            current_document,
            current_cursor,
            resolve_image,
            jump
        ])
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
    use super::{parse_args, Cursors, Cursor, Document, Documents};

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

    #[test]
    fn keeps_the_latest_cursor() {
        let cursors = Cursors::default();
        assert!(cursors.current().is_none());
        cursors.accept(Cursor {
            generation: 1,
            line: 10,
        });
        cursors.accept(Cursor {
            generation: 1,
            line: 42,
        });
        let cursor = cursors.current().expect("a cursor");
        assert_eq!(cursor.generation, 1);
        assert_eq!(cursor.line, 42);
    }
}
