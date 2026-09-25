mod image;
mod nvim;
mod render;
mod standalone;

use std::sync::Mutex;

use std::path::{Path, PathBuf};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

/// 最新の本文をフロントエンドへ届けるイベント名
const DOCUMENT_EVENT: &str = "mdpeek://document";
/// カーソル行をフロントエンドへ届けるイベント名
const CURSOR_EVENT: &str = "mdpeek://cursor";
/// 読み直しの失敗など、利用者に知らせる問題を届けるイベント名（`null` で取り消し）
const PROBLEM_EVENT: &str = "mdpeek://problem";

pub struct Args {
    pub socket: String,
    pub token: String,
}

/// 起動引数から決まる、動く相手
enum Mode {
    Nvim(Args),
    File { path: PathBuf, foreground: bool },
}

/// 起動時にどちらの相手で始めるか（ファイルモードは、起動前に読み込みまで済ませておく）
enum Launch {
    Nvim(Args),
    File(Document),
}

/// フロントエンドがリンクの扱いを切り替えるための、いまの動作モード
#[derive(Clone, Copy)]
pub enum RuntimeMode {
    Nvim,
    File,
}

impl RuntimeMode {
    fn as_str(self) -> &'static str {
        match self {
            RuntimeMode::Nvim => "nvim",
            RuntimeMode::File => "file",
        }
    }

    /// 窓の種類を、タイトルで見分けられるようにする
    fn title(self) -> &'static str {
        match self {
            RuntimeMode::Nvim => "MdPeek — Linked to Neovim",
            RuntimeMode::File => "MdPeek — Read Only",
        }
    }
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

/// リンクや履歴で開いた文書の、行き先を示す最小限の情報
#[derive(Clone, Copy, Debug, Serialize)]
pub struct NavigationTarget {
    #[serde(rename = "gen")]
    pub generation: u64,
    pub version: u64,
}

impl From<&Document> for NavigationTarget {
    fn from(document: &Document) -> Self {
        NavigationTarget {
            generation: document.generation,
            version: document.version,
        }
    }
}

/// リンクで開いた文書の履歴。開いてよいパスを、Rust側だけが決める。
#[derive(Default)]
pub struct History(Mutex<HistoryState>);

#[derive(Default)]
struct HistoryState {
    back: Vec<PathBuf>,
    forward: Vec<PathBuf>,
    /// Neovim連携モードで、直前に自分（open_link/go_back/go_forward）が
    /// 切り替えた先の世代。nvim.rsが、Neovimから届いた世代と比べるのに使う。
    expected_generation: Option<u64>,
}

impl History {
    /// `:MdPeek` など、自分の操作以外で対象世代が変わったら、履歴を空にする。
    pub fn reset_if_unexpected(&self, generation: u64) {
        let mut state = self.0.lock().expect("history lock");
        if state.expected_generation == Some(generation) {
            state.expected_generation = None;
        } else {
            state.back.clear();
            state.forward.clear();
        }
    }
}

/// 戻る／進むそれぞれを辿れるかどうか
#[derive(Clone, Copy, Debug, Serialize)]
pub struct HistoryAvailability {
    can_back: bool,
    can_forward: bool,
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
fn current_problem(problem: tauri::State<'_, Problem>) -> Option<String> {
    problem.0.lock().expect("problem lock").clone()
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

/// フロントエンドが、起動時のモードを取得する。
#[tauri::command]
fn app_mode(mode: tauri::State<'_, RuntimeMode>) -> &'static str {
    mode.as_str()
}

/// フロントエンドが、戻る／進むボタンの有効/無効を決めるために使う。
#[tauri::command]
fn history_state(history: tauri::State<'_, History>) -> HistoryAvailability {
    let state = history.0.lock().expect("history lock");
    HistoryAvailability {
        can_back: !state.back.is_empty(),
        can_forward: !state.forward.is_empty(),
    }
}

/// 相対パスの `.md`／`.markdown` リンクを開く要求を受けたら、いまの文書の
/// ディレクトリを基準に解決してから、新しい文書として開く。開けたら、いまの
/// パスを戻る履歴に積み、進む履歴は空にする。
#[tauri::command]
async fn open_link(
    app: AppHandle,
    documents: tauri::State<'_, Documents>,
    history: tauri::State<'_, History>,
    mode: tauri::State<'_, RuntimeMode>,
    session: tauri::State<'_, Session>,
    href: String,
    version: u64,
) -> Result<NavigationTarget, String> {
    let current = documents.current().ok_or_else(|| "no document".to_string())?;
    if current.version != version {
        return Err("the document has moved on".to_string());
    }
    let base = Path::new(&current.path)
        .parent()
        .ok_or_else(|| "the document has no directory".to_string())?;
    let target = image::resolve_markdown_link(base, &href)?;

    let navigation = open_path(&app, &documents, &history, *mode, &session, &target).await?;

    let mut history = history.0.lock().expect("history lock");
    history.back.push(PathBuf::from(current.path));
    history.forward.clear();
    Ok(navigation)
}

/// 戻る／進むで、履歴にあるパスを開き直す。開けなかったら履歴は動かさない。
#[tauri::command]
async fn go_back(
    app: AppHandle,
    documents: tauri::State<'_, Documents>,
    history: tauri::State<'_, History>,
    mode: tauri::State<'_, RuntimeMode>,
    session: tauri::State<'_, Session>,
) -> Result<NavigationTarget, String> {
    navigate_history(&app, &documents, &history, *mode, &session, true).await
}

#[tauri::command]
async fn go_forward(
    app: AppHandle,
    documents: tauri::State<'_, Documents>,
    history: tauri::State<'_, History>,
    mode: tauri::State<'_, RuntimeMode>,
    session: tauri::State<'_, Session>,
) -> Result<NavigationTarget, String> {
    navigate_history(&app, &documents, &history, *mode, &session, false).await
}

async fn navigate_history(
    app: &AppHandle,
    documents: &Documents,
    history: &History,
    mode: RuntimeMode,
    session: &Session,
    back: bool,
) -> Result<NavigationTarget, String> {
    let current = documents.current().ok_or_else(|| "no document".to_string())?;
    let target = {
        let mut history = history.0.lock().expect("history lock");
        let stack = if back {
            &mut history.back
        } else {
            &mut history.forward
        };
        stack.pop().ok_or_else(|| "no more history".to_string())?
    };

    match open_path(app, documents, history, mode, session, &target).await {
        Ok(navigation) => {
            let mut history = history.0.lock().expect("history lock");
            let push_to = if back {
                &mut history.forward
            } else {
                &mut history.back
            };
            push_to.push(PathBuf::from(current.path));
            Ok(navigation)
        }
        Err(message) => {
            // 開けなかったら、ポップしたものを元の履歴に戻す
            let mut history = history.0.lock().expect("history lock");
            let stack = if back {
                &mut history.back
            } else {
                &mut history.forward
            };
            stack.push(target);
            Err(message)
        }
    }
}

/// 絶対パスを、いまのモードに応じて新しい文書として開く。
/// スタンドアローンモードではファイルを読み、監視の対象も差し替える。
/// Neovim連携モードでは、rpc.open を呼んで対象ウィンドウにも同じファイルを開かせる
/// (本文とカーソル行は、いつもどおり別の通知で届く)。
async fn open_path(
    app: &AppHandle,
    documents: &Documents,
    history: &History,
    mode: RuntimeMode,
    session: &Session,
    target: &Path,
) -> Result<NavigationTarget, String> {
    let current = documents.current().ok_or_else(|| "no document".to_string())?;
    match mode {
        RuntimeMode::File => {
            let document = standalone::open(target, current.generation + 1, current.version + 1)?;
            crate::publish(app, document.clone());
            standalone::start_watching(app, target.to_path_buf());
            Ok(NavigationTarget::from(&document))
        }
        RuntimeMode::Nvim => {
            let nvim = session.get().ok_or_else(|| "not connected".to_string())?;
            let path = target.to_string_lossy().into_owned();
            let (generation, version) =
                nvim::open(nvim, current.generation, current.version, &path).await?;
            // これから届く本文の世代は、自分のこの操作によるもの。
            // nvim.rsが受け取ったとき、外部からの切り替えと区別するために覚えておく
            history.0.lock().expect("history lock").expected_generation = Some(generation);
            Ok(NavigationTarget { generation, version })
        }
    }
}

/// `--nvim <socket> --token <token>` ならNeovim連携モード、
/// 単一の位置引数（ファイルパス、`--foreground` を付けてもよい）ならスタンドアローンモードにする。
fn parse_args(argv: impl Iterator<Item = String>) -> Result<Mode, String> {
    let mut socket = None;
    let mut token = None;
    let mut foreground = false;
    let mut positional = Vec::new();
    let mut argv = argv;
    while let Some(arg) = argv.next() {
        match arg.as_str() {
            "--nvim" => socket = argv.next(),
            "--token" => token = argv.next(),
            "--foreground" => foreground = true,
            other if other.starts_with("--") => return Err(format!("unknown argument: {other}")),
            other => positional.push(other.to_string()),
        }
    }
    match (socket, token, positional.as_slice()) {
        (Some(socket), Some(token), []) if !foreground => Ok(Mode::Nvim(Args { socket, token })),
        (None, None, [path]) => Ok(Mode::File {
            path: PathBuf::from(path),
            foreground,
        }),
        _ => Err(
            "usage: mdpeek --nvim <socket> --token <token> | mdpeek [--foreground] <file>"
                .to_string(),
        ),
    }
}

fn main() {
    let mode = match parse_args(std::env::args().skip(1)) {
        Ok(mode) => mode,
        Err(message) => {
            report(&message);
            std::process::exit(2);
        }
    };
    let launch = match mode {
        Mode::Nvim(args) => Launch::Nvim(args),
        Mode::File { path, foreground } => match standalone::load(&path) {
            Ok(document) if foreground => Launch::File(document),
            Ok(document) => {
                // 検証できたので、端末から切り離した子プロセスに任せて、すぐに戻る
                match standalone::detach(Path::new(&document.path)) {
                    Ok(()) => std::process::exit(0),
                    Err(message) => {
                        report(&message);
                        std::process::exit(2);
                    }
                }
            }
            Err(message) => {
                report(&message);
                std::process::exit(2);
            }
        },
    };
    let runtime_mode = match &launch {
        Launch::Nvim(_) => RuntimeMode::Nvim,
        Launch::File(_) => RuntimeMode::File,
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(Documents::default())
        .manage(Cursors::default())
        .manage(Session::default())
        .manage(History::default())
        .manage(Problem::default())
        .manage(standalone::Watching::default())
        .manage(runtime_mode)
        .invoke_handler(tauri::generate_handler![
            current_document,
            current_cursor,
            resolve_image,
            jump,
            app_mode,
            history_state,
            current_problem,
            open_link,
            go_back,
            go_forward
        ])
        .setup(move |app| {
            if let Some(window) = app.get_webview_window("main") {
                window.set_title(runtime_mode.title())?;
            }
            let handle = app.handle().clone();
            match launch {
                Launch::Nvim(args) => {
                    tauri::async_runtime::spawn(async move {
                        if let Err(message) = nvim::run(handle.clone(), args).await {
                            report(&message);
                        }
                        // RPCが切断されたら、アプリを終了する
                        handle.exit(0);
                    });
                }
                Launch::File(document) => {
                    // 監視には、引数そのものではなく正規化済みの絶対パスを使う
                    // （相対パスで起動すると、親ディレクトリが空になり監視できないため）
                    let watched = PathBuf::from(&document.path);
                    crate::publish(&handle, document);
                    standalone::start_watching(&handle, watched);
                }
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("failed to run mdpeek");
}

#[cfg(test)]
mod tests {
    use super::{parse_args, Cursor, Cursors, Document, Documents, History, Mode};

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
        let mode = parse_args(
            ["--nvim", "/tmp/nvim.sock", "--token", "abc"]
                .into_iter()
                .map(String::from),
        )
        .expect("a mode");
        match mode {
            Mode::Nvim(args) => {
                assert_eq!(args.socket, "/tmp/nvim.sock");
                assert_eq!(args.token, "abc");
            }
            Mode::File { .. } => panic!("expected nvim mode"),
        }
    }

    #[test]
    fn rejects_missing_arguments() {
        assert!(parse_args(["--nvim", "/tmp/nvim.sock"].into_iter().map(String::from)).is_err());
    }

    #[test]
    fn parses_a_single_file_argument() {
        let mode = parse_args(["note.md"].into_iter().map(String::from)).expect("a mode");
        match mode {
            Mode::File { path, foreground } => {
                assert_eq!(path, std::path::PathBuf::from("note.md"));
                assert!(!foreground, "standalone mode runs in the background by default");
            }
            Mode::Nvim(_) => panic!("expected file mode"),
        }
    }

    #[test]
    fn parses_the_foreground_flag() {
        let mode = parse_args(["--foreground", "note.md"].into_iter().map(String::from))
            .expect("a mode");
        match mode {
            Mode::File { foreground, .. } => assert!(foreground),
            Mode::Nvim(_) => panic!("expected file mode"),
        }
    }

    #[test]
    fn rejects_the_foreground_flag_for_neovim() {
        assert!(parse_args(
            ["--nvim", "/tmp/nvim.sock", "--token", "abc", "--foreground"]
                .into_iter()
                .map(String::from)
        )
        .is_err());
    }

    #[test]
    fn rejects_the_foreground_flag_alone() {
        assert!(parse_args(["--foreground"].into_iter().map(String::from)).is_err());
    }

    #[test]
    fn rejects_mixed_arguments() {
        assert!(parse_args(
            ["--nvim", "/tmp/nvim.sock", "--token", "abc", "note.md"]
                .into_iter()
                .map(String::from)
        )
        .is_err());
    }

    #[test]
    fn rejects_two_file_arguments() {
        assert!(parse_args(["a.md", "b.md"].into_iter().map(String::from)).is_err());
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
    fn keeps_history_for_its_own_expected_switch() {
        let history = History::default();
        {
            let mut state = history.0.lock().expect("history lock");
            state.back.push(std::path::PathBuf::from("/tmp/a.md"));
        }
        history.0.lock().expect("history lock").expected_generation = Some(2);

        history.reset_if_unexpected(2);

        let state = history.0.lock().expect("history lock");
        assert_eq!(state.back.len(), 1, "own navigation should not clear history");
        assert_eq!(state.expected_generation, None, "the marker should be consumed");
    }

    #[test]
    fn clears_history_on_an_unexpected_switch() {
        let history = History::default();
        {
            let mut state = history.0.lock().expect("history lock");
            state.back.push(std::path::PathBuf::from("/tmp/a.md"));
            state.forward.push(std::path::PathBuf::from("/tmp/b.md"));
        }

        // :MdPeek による切り替えなど、自分の操作以外で世代が変わった
        history.reset_if_unexpected(5);

        let state = history.0.lock().expect("history lock");
        assert!(state.back.is_empty());
        assert!(state.forward.is_empty());
    }
}
