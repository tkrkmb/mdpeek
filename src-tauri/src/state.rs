use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::nvim;

/// 最新の本文をフロントエンドへ届けるイベント名
const DOCUMENT_EVENT: &str = "mdsight://document";
/// カーソル行をフロントエンドへ届けるイベント名
const CURSOR_EVENT: &str = "mdsight://cursor";
/// Neovimの検索の一致をフロントエンドへ届けるイベント名
const SEARCH_EVENT: &str = "mdsight://search";
/// 読み直しの失敗など、利用者に知らせる問題を届けるイベント名（`null` で取り消し）
const PROBLEM_EVENT: &str = "mdsight://problem";

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

/// Neovimの検索の一致1つ（行番号、一致した文字列、現在の一致か）
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SearchMatch {
    pub line: u64,
    pub text: String,
    pub current: bool,
}

/// Neovimの検索の一致。`matches` が空なら強調を消す
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Search {
    #[serde(rename = "gen")]
    pub generation: u64,
    pub version: u64,
    pub matches: Vec<SearchMatch>,
}

/// ジャンプ要求のために、Neovimとの接続を預かる
#[derive(Default)]
pub struct Session(Mutex<Option<nvim::Nvim>>);

impl Session {
    pub fn open(&self, nvim: nvim::Nvim) {
        *self.0.lock().expect("session lock") = Some(nvim);
    }

    pub fn get(&self) -> Option<nvim::Nvim> {
        self.0.lock().expect("session lock").clone()
    }
}

/// 世代と版が最新の本文だけを保持する
#[derive(Default)]
pub struct Documents(Mutex<Option<Document>>);

impl Documents {
    pub fn accept(&self, document: Document) -> bool {
        let mut slot = self.0.lock().expect("documents lock");
        if let Some(current) = slot.as_ref() {
            if (document.generation, document.version) <= (current.generation, current.version) {
                return false;
            }
        }
        *slot = Some(document);
        true
    }

    pub fn current(&self) -> Option<Document> {
        self.0.lock().expect("documents lock").clone()
    }

    /// いまの文書と、その文書のあるディレクトリ。`version` が表示中の版と違えば失敗する。
    /// 相対パスの画像やリンクを、文書のディレクトリを基準に解決するときに使う
    pub fn current_with_base(&self, version: u64) -> Result<(Document, PathBuf), String> {
        let document = self.current().ok_or_else(|| "no document".to_string())?;
        if document.version != version {
            return Err("the document has moved on".to_string());
        }
        let base = Path::new(&document.path)
            .parent()
            .ok_or_else(|| "the document has no directory".to_string())?
            .to_path_buf();
        Ok((document, base))
    }
}

/// 起動直後の取りこぼしに備え、最新のカーソル行を保持する
#[derive(Default)]
pub struct Cursors(Mutex<Option<Cursor>>);

impl Cursors {
    pub fn accept(&self, cursor: Cursor) {
        *self.0.lock().expect("cursors lock") = Some(cursor);
    }

    pub fn current(&self) -> Option<Cursor> {
        *self.0.lock().expect("cursors lock")
    }
}

/// 起動直後の取りこぼしに備え、最新の検索の一致を保持する
#[derive(Default)]
pub struct Searches(Mutex<Option<Search>>);

impl Searches {
    pub fn accept(&self, search: Search) {
        *self.0.lock().expect("searches lock") = Some(search);
    }

    pub fn current(&self) -> Option<Search> {
        self.0.lock().expect("searches lock").clone()
    }
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

/// 受け取った検索の一致を、そのままフロントエンドへ送る（描画は待たない）
pub fn publish_search(app: &AppHandle, search: Search) {
    app.state::<Searches>().accept(search.clone());
    let _ = app.emit(SEARCH_EVENT, search);
}

/// いま利用者に知らせている問題。切り離して動くときは標準エラー出力が見えないため、
/// ウィンドウに出す。
#[derive(Default)]
pub struct Problem(Mutex<Option<String>>);

/// 問題を知らせる（`None` で取り消す）。変わらなければイベントは送らない。
pub fn set_problem(app: &AppHandle, problem: Option<String>) {
    {
        let state = app.state::<Problem>();
        let mut slot = state.0.lock().expect("problem lock");
        if *slot == problem {
            return;
        }
        *slot = problem.clone();
    }
    let _ = app.emit(PROBLEM_EVENT, problem);
}

/// 起動直後に取りこぼした問題を、フロントエンドが拾い直す。
#[tauri::command]
pub fn current_problem(problem: tauri::State<'_, Problem>) -> Option<String> {
    problem.0.lock().expect("problem lock").clone()
}

#[tauri::command]
pub fn current_document(documents: tauri::State<'_, Documents>) -> Option<Document> {
    documents.current()
}

/// 起動直後に取りこぼしたカーソル行を、フロントエンドが拾い直す。
#[tauri::command]
pub fn current_cursor(cursors: tauri::State<'_, Cursors>) -> Option<Cursor> {
    cursors.current()
}

/// 起動直後に取りこぼした検索の一致を、フロントエンドが拾い直す。
#[tauri::command]
pub fn current_search(searches: tauri::State<'_, Searches>) -> Option<Search> {
    searches.current()
}

#[cfg(test)]
mod tests {
    use super::{Cursor, Cursors, Document, Documents};

    fn document(generation: u64, version: u64) -> Document {
        Document {
            generation,
            version,
            path: "/tmp/note.md".to_string(),
            html: String::new(),
        }
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

    #[test]
    fn finds_the_directory_of_the_current_document() {
        let documents = Documents::default();
        assert_eq!(documents.current_with_base(1).unwrap_err(), "no document");
        assert!(documents.accept(document(1, 3)));
        let (current, base) = documents.current_with_base(3).expect("a document");
        assert_eq!(current.version, 3);
        assert_eq!(base, std::path::PathBuf::from("/tmp"));
        assert_eq!(documents.current_with_base(2).unwrap_err(), "the document has moved on");
    }
}
