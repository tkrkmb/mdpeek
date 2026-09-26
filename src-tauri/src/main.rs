mod icon;
mod image;
mod nvim;
mod raise;
mod render;
mod standalone;

use std::sync::Mutex;

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};

/// 最新の本文をフロントエンドへ届けるイベント名
const DOCUMENT_EVENT: &str = "mdsight://document";
/// カーソル行をフロントエンドへ届けるイベント名
const CURSOR_EVENT: &str = "mdsight://cursor";
/// Neovimの検索の一致をフロントエンドへ届けるイベント名
const SEARCH_EVENT: &str = "mdsight://search";
/// 読み直しの失敗など、利用者に知らせる問題を届けるイベント名（`null` で取り消し）
const PROBLEM_EVENT: &str = "mdsight://problem";

pub struct Args {
    pub socket: String,
    pub token: String,
}

/// 起動引数から決まる、動く相手
enum Mode {
    Nvim(Args),
    File { path: PathBuf, foreground: bool },
    /// `--version`：版を表示して終わる
    Version,
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
            RuntimeMode::Nvim => "MdSight — Linked to Neovim",
            RuntimeMode::File => "MdSight — Read Only",
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

/// 文書で読んでいた位置。フロントエンドの「画面の上端付近にあるブロックのソース行と、画面内でのオフセット」
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct Anchor {
    pub line: u64,
    pub offset: f64,
}

/// リンクや履歴で開いた文書の、行き先を示す最小限の情報。
/// 戻る／進むで開いたときは、その文書で読んでいた位置も返す
#[derive(Clone, Copy, Debug, Serialize)]
pub struct NavigationTarget {
    #[serde(rename = "gen")]
    pub generation: u64,
    pub version: u64,
    pub anchor: Option<Anchor>,
}

impl From<&Document> for NavigationTarget {
    fn from(document: &Document) -> Self {
        NavigationTarget {
            generation: document.generation,
            version: document.version,
            anchor: None,
        }
    }
}

/// 履歴の1項目。文書のパスと、そこで読んでいた位置
#[derive(Clone, Debug, PartialEq)]
struct HistoryEntry {
    path: PathBuf,
    anchor: Option<Anchor>,
}

/// リンクで開いた文書の履歴。開いてよいパスを、Rust側だけが決める。
#[derive(Default)]
pub struct History(Mutex<HistoryState>);

#[derive(Default)]
struct HistoryState {
    back: Vec<HistoryEntry>,
    forward: Vec<HistoryEntry>,
    /// Neovim連携モードで、直前に自分（open_link/go_back/go_forward）が
    /// 切り替えた先の世代。nvim.rsが、Neovimから届いた世代と比べるのに使う。
    expected_generation: Option<u64>,
}

impl HistoryState {
    /// リンクで別の文書へ移った。離れる文書を戻る履歴に積み、進む履歴は空にする
    fn follow_link(&mut self, left: HistoryEntry) {
        self.back.push(left);
        self.forward.clear();
    }

    /// 戻る（`back`）／進むで開く項目を取り出す
    fn take(&mut self, back: bool) -> Option<HistoryEntry> {
        if back {
            self.back.pop()
        } else {
            self.forward.pop()
        }
    }

    /// 開けたら、離れる文書を反対側の履歴に積む
    fn arrive(&mut self, back: bool, left: HistoryEntry) {
        if back {
            self.forward.push(left);
        } else {
            self.back.push(left);
        }
    }

    /// 開けなかったら、取り出した項目を元の履歴に戻す
    fn put_back(&mut self, back: bool, entry: HistoryEntry) {
        if back {
            self.back.push(entry);
        } else {
            self.forward.push(entry);
        }
    }
}

impl History {
    /// これから自分の操作で対象世代が `generation` になることを、Neovimに頼む前に覚えておく。
    /// 本文はNeovimの返事より先に届くことがあるので、返事を待ってからでは間に合わない。
    fn expect(&self, generation: u64) {
        self.0.lock().expect("history lock").expected_generation = Some(generation);
    }

    /// 頼みが失敗したら、`expect` で覚えたものを取り消す（本文が届いて使われていたら、何もしない）
    fn cancel(&self, generation: u64) {
        let mut state = self.0.lock().expect("history lock");
        if state.expected_generation == Some(generation) {
            state.expected_generation = None;
        }
    }

    /// `:MdSight` など、自分の操作以外で対象世代が変わったら、履歴を空にする。
    /// 自分の操作以外だったら true を返す。
    pub fn reset_if_unexpected(&self, generation: u64) -> bool {
        let mut state = self.0.lock().expect("history lock");
        if state.expected_generation == Some(generation) {
            state.expected_generation = None;
            false
        } else {
            state.back.clear();
            state.forward.clear();
            true
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

/// 起動直後の取りこぼしに備え、最新の検索の一致を保持する
#[derive(Default)]
pub struct Searches(Mutex<Option<Search>>);

/// Neovimが先に終了して標準エラー出力のパイプが閉じていても、
/// 書き込みの失敗でパニックしないようにする。
fn report(message: &str) {
    use std::io::Write;
    let _ = writeln!(std::io::stderr(), "mdsight: {message}");
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
    *app.state::<Searches>().0.lock().expect("searches lock") = Some(search.clone());
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

/// 起動直後に取りこぼした検索の一致を、フロントエンドが拾い直す。
#[tauri::command]
fn current_search(searches: tauri::State<'_, Searches>) -> Option<Search> {
    searches.0.lock().expect("searches lock").clone()
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

/// フロントエンドが適用したテーマ（ライト／ダーク）に、アプリのアイコンを合わせる。
#[tauri::command]
fn set_app_icon(app: AppHandle, appearance: icon::Appearance) {
    icon::apply(&app, appearance);
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
/// パスと読んでいた位置（`anchor`）を戻る履歴に積み、進む履歴は空にする。
#[tauri::command]
async fn open_link(
    app: AppHandle,
    documents: tauri::State<'_, Documents>,
    history: tauri::State<'_, History>,
    mode: tauri::State<'_, RuntimeMode>,
    session: tauri::State<'_, Session>,
    href: String,
    version: u64,
    anchor: Option<Anchor>,
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

    history.0.lock().expect("history lock").follow_link(HistoryEntry {
        path: PathBuf::from(current.path),
        anchor,
    });
    Ok(navigation)
}

/// 戻る／進むで、履歴にあるパスを開き直す。開けなかったら履歴は動かさない。
/// `anchor` は、いま読んでいる位置。開けたら、移動先で読んでいた位置を返す
#[tauri::command]
async fn go_back(
    app: AppHandle,
    documents: tauri::State<'_, Documents>,
    history: tauri::State<'_, History>,
    mode: tauri::State<'_, RuntimeMode>,
    session: tauri::State<'_, Session>,
    anchor: Option<Anchor>,
) -> Result<NavigationTarget, String> {
    navigate_history(&app, &documents, &history, *mode, &session, true, anchor).await
}

#[tauri::command]
async fn go_forward(
    app: AppHandle,
    documents: tauri::State<'_, Documents>,
    history: tauri::State<'_, History>,
    mode: tauri::State<'_, RuntimeMode>,
    session: tauri::State<'_, Session>,
    anchor: Option<Anchor>,
) -> Result<NavigationTarget, String> {
    navigate_history(&app, &documents, &history, *mode, &session, false, anchor).await
}

async fn navigate_history(
    app: &AppHandle,
    documents: &Documents,
    history: &History,
    mode: RuntimeMode,
    session: &Session,
    back: bool,
    anchor: Option<Anchor>,
) -> Result<NavigationTarget, String> {
    let current = documents.current().ok_or_else(|| "no document".to_string())?;
    let target = history
        .0
        .lock()
        .expect("history lock")
        .take(back)
        .ok_or_else(|| "no more history".to_string())?;

    match open_path(app, documents, history, mode, session, &target.path).await {
        Ok(navigation) => {
            history.0.lock().expect("history lock").arrive(
                back,
                HistoryEntry {
                    path: PathBuf::from(current.path),
                    anchor,
                },
            );
            Ok(NavigationTarget {
                anchor: target.anchor,
                ..navigation
            })
        }
        Err(message) => {
            // 開けなかったら、取り出したものを元の履歴に戻す
            history.0.lock().expect("history lock").put_back(back, target);
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
            raise::listen(app, Path::new(&document.path));
            Ok(NavigationTarget::from(&document))
        }
        RuntimeMode::Nvim => {
            let nvim = session.get().ok_or_else(|| "not connected".to_string())?;
            let path = target.to_string_lossy().into_owned();
            // 開けたら、対象世代は必ず1つ進む。これから届く本文は自分のこの操作によるものなので、
            // nvim.rsが受け取ったとき、外部からの切り替えと区別できるように、頼む前に覚えておく
            // （本文は、Neovimの返事より先に届くことがある）
            let expected = current.generation + 1;
            history.expect(expected);
            match nvim::open(nvim, current.generation, current.version, &path).await {
                Ok((generation, version)) => Ok(NavigationTarget {
                    generation,
                    version,
                    anchor: None,
                }),
                Err(message) => {
                    history.cancel(expected);
                    Err(message)
                }
            }
        }
    }
}

const USAGE: &str =
    "usage: mdsight --nvim <socket> --token <token> | mdsight [--foreground] <file> | mdsight --version";

/// `--nvim <socket> --token <token>` ならNeovim連携モード、
/// 単一の位置引数（ファイルパス、`--foreground` を付けてもよい）ならスタンドアローンモードにする。
/// `--version` だけなら、版を表示して終わる。
fn parse_args(argv: impl Iterator<Item = String>) -> Result<Mode, String> {
    let mut socket = None;
    let mut token = None;
    let mut foreground = false;
    let mut version = false;
    let mut positional = Vec::new();
    let mut argv = argv;
    while let Some(arg) = argv.next() {
        match arg.as_str() {
            "--nvim" => socket = argv.next(),
            "--token" => token = argv.next(),
            "--foreground" => foreground = true,
            "--version" => version = true,
            other if other.starts_with("--") => return Err(format!("unknown argument: {other}")),
            other => positional.push(other.to_string()),
        }
    }
    if version {
        // `--version` は、他の引数と組み合わせない
        return if socket.is_none() && token.is_none() && !foreground && positional.is_empty() {
            Ok(Mode::Version)
        } else {
            Err(USAGE.to_string())
        };
    }
    match (socket, token, positional.as_slice()) {
        (Some(socket), Some(token), []) if !foreground => Ok(Mode::Nvim(Args { socket, token })),
        (None, None, [path]) => Ok(Mode::File {
            path: PathBuf::from(path),
            foreground,
        }),
        _ => Err(USAGE.to_string()),
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
        Mode::Version => {
            println!("mdsight {}", env!("CARGO_PKG_VERSION"));
            std::process::exit(0);
        }
        Mode::Nvim(args) => Launch::Nvim(args),
        Mode::File { path, foreground } => match standalone::load(&path) {
            // 同じファイルを開いている窓があれば、そちらを前面に出して終わる
            Ok(document) if raise::request(Path::new(&document.path)) => {
                let name = standalone::file_name(Path::new(&document.path));
                println!("MdSight: brought the window already showing {name} to the front");
                std::process::exit(0);
            }
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
        .manage(Searches::default())
        .manage(Session::default())
        .manage(History::default())
        .manage(Problem::default())
        .manage(standalone::Watching::default())
        .manage(raise::Listening::default())
        .manage(runtime_mode)
        .invoke_handler(tauri::generate_handler![
            current_document,
            current_cursor,
            current_search,
            resolve_image,
            jump,
            app_mode,
            history_state,
            current_problem,
            open_link,
            go_back,
            go_forward,
            set_app_icon
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
                    standalone::start_watching(&handle, watched.clone());
                    raise::listen(&handle, &watched);
                }
            }
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("failed to build mdsight")
        .run(|app, event| match event {
            // Tauriが開発ビルドで既定のアイコンを設定した後に届くので、ここで上書きする
            tauri::RunEvent::Ready => icon::follow_os(app),
            tauri::RunEvent::Exit => raise::stop(app),
            _ => {}
        });
}

#[cfg(test)]
mod tests {
    use super::{parse_args, Anchor, Cursor, Cursors, Document, Documents, History, HistoryEntry, HistoryState, Mode};

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
            _ => panic!("expected nvim mode"),
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
            _ => panic!("expected file mode"),
        }
    }

    #[test]
    fn parses_the_foreground_flag() {
        let mode = parse_args(["--foreground", "note.md"].into_iter().map(String::from))
            .expect("a mode");
        match mode {
            Mode::File { foreground, .. } => assert!(foreground),
            _ => panic!("expected file mode"),
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
    fn parses_the_version_flag() {
        let mode = parse_args(["--version"].into_iter().map(String::from)).expect("a mode");
        assert!(matches!(mode, Mode::Version));
    }

    #[test]
    fn rejects_the_version_flag_with_other_arguments() {
        assert!(parse_args(["--version", "note.md"].into_iter().map(String::from)).is_err());
        assert!(parse_args(["--foreground", "--version"].into_iter().map(String::from)).is_err());
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

    fn entry(path: &str, anchor: Option<Anchor>) -> HistoryEntry {
        HistoryEntry {
            path: std::path::PathBuf::from(path),
            anchor,
        }
    }

    fn at(line: u64, offset: f64) -> Option<Anchor> {
        Some(Anchor { line, offset })
    }

    #[test]
    fn remembers_where_each_document_was_read() {
        let mut state = HistoryState::default();
        // a.md を 10 行目まで読んでから、リンクで b.md へ
        state.follow_link(entry("/tmp/a.md", at(10, 40.0)));
        // b.md を 30 行目まで読んでから戻る
        let back = state.take(true).expect("a.md");
        assert_eq!(back, entry("/tmp/a.md", at(10, 40.0)));
        state.arrive(true, entry("/tmp/b.md", at(30, 12.5)));
        // a.md で 12 行目まで読み進めてから進む
        let forward = state.take(false).expect("b.md");
        assert_eq!(forward, entry("/tmp/b.md", at(30, 12.5)));
        state.arrive(false, entry("/tmp/a.md", at(12, 0.0)));
        assert_eq!(state.back, vec![entry("/tmp/a.md", at(12, 0.0))]);
        assert!(state.forward.is_empty());
    }

    #[test]
    fn keeps_the_entry_when_it_cannot_be_opened() {
        let mut state = HistoryState::default();
        state.follow_link(entry("/tmp/a.md", at(10, 40.0)));
        let back = state.take(true).expect("a.md");
        state.put_back(true, back);
        assert_eq!(state.back, vec![entry("/tmp/a.md", at(10, 40.0))]);
        assert!(state.forward.is_empty());
    }

    #[test]
    fn a_new_link_drops_the_forward_history() {
        let mut state = HistoryState::default();
        state.follow_link(entry("/tmp/a.md", None));
        let back = state.take(true).expect("a.md");
        state.arrive(true, entry("/tmp/b.md", at(3, 0.0)));
        assert_eq!(back.path, std::path::PathBuf::from("/tmp/a.md"));
        state.follow_link(entry("/tmp/a.md", at(5, 0.0)));
        assert!(state.forward.is_empty());
        assert_eq!(state.back.len(), 1);
    }

    #[test]
    fn keeps_history_for_its_own_expected_switch() {
        let history = History::default();
        {
            let mut state = history.0.lock().expect("history lock");
            state.back.push(entry("/tmp/a.md", None));
        }
        history.0.lock().expect("history lock").expected_generation = Some(2);

        history.reset_if_unexpected(2);

        let state = history.0.lock().expect("history lock");
        assert_eq!(state.back.len(), 1, "own navigation should not clear history");
        assert_eq!(state.expected_generation, None, "the marker should be consumed");
    }

    #[test]
    fn keeps_history_when_the_content_arrives_before_the_reply() {
        let history = History::default();
        history.0.lock().expect("history lock").back.push(entry("/tmp/a.md", None));

        // 頼む前に覚えておけば、Neovimの返事より先に本文が届いても、自分の操作と見分けられる
        history.expect(2);
        assert!(!history.reset_if_unexpected(2));

        assert_eq!(history.0.lock().expect("history lock").back.len(), 1);
    }

    #[test]
    fn forgets_the_expectation_when_the_request_fails() {
        let history = History::default();
        history.expect(2);
        history.cancel(2);
        // 失敗した頼みの世代が、あとから別の切り替えで届いても、外部の操作として扱う
        assert!(history.reset_if_unexpected(2));
    }

    #[test]
    fn keeps_a_used_expectation_gone_when_cancelling() {
        let history = History::default();
        history.expect(2);
        assert!(!history.reset_if_unexpected(2));
        // 本文がすでに使い終えていたら、新しい目印を残さない
        history.cancel(2);
        assert_eq!(history.0.lock().expect("history lock").expected_generation, None);
    }

    #[test]
    fn clears_history_on_an_unexpected_switch() {
        let history = History::default();
        {
            let mut state = history.0.lock().expect("history lock");
            state.back.push(entry("/tmp/a.md", None));
            state.forward.push(entry("/tmp/b.md", None));
        }

        // :MdSight による切り替えなど、自分の操作以外で世代が変わった
        history.reset_if_unexpected(5);

        let state = history.0.lock().expect("history lock");
        assert!(state.back.is_empty());
        assert!(state.forward.is_empty());
    }
}
