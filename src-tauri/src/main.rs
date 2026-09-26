mod args;
mod history;
mod icon;
mod nvim;
mod raise;
mod render;
mod resolve;
mod standalone;
mod state;

use std::path::{Path, PathBuf};

use tauri::{AppHandle, Manager};

use args::{parse_args, Args, Mode};
use history::History;
use state::{Cursors, Document, Documents, Problem, Searches, Session};

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

/// Neovimが先に終了して標準エラー出力のパイプが閉じていても、
/// 書き込みの失敗でパニックしないようにする。
fn report(message: &str) {
    use std::io::Write;
    let _ = writeln!(std::io::stderr(), "mdsight: {message}");
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
    let (_, base) = documents.current_with_base(version)?;
    let resolved = resolve::resolve(&base, &path)?;
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
            state::current_document,
            state::current_cursor,
            state::current_search,
            resolve_image,
            jump,
            app_mode,
            history::history_state,
            state::current_problem,
            history::open_link,
            history::go_back,
            history::go_forward,
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
                    state::publish(&handle, document);
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
