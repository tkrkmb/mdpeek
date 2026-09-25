use std::path::{Path, PathBuf};
use std::sync::mpsc::channel;
use std::sync::Mutex;
use std::time::Duration;

use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use tauri::{AppHandle, Manager};

use crate::{render, report, set_problem, Document, Documents};

/// いま監視しているファイルのウォッチャー。差し替えると、古い方の監視スレッドは自然に終わる。
#[derive(Default)]
pub struct Watching(Mutex<Option<RecommendedWatcher>>);

fn read(path: &Path) -> Result<(PathBuf, String), String> {
    let canonical = std::fs::canonicalize(path)
        .map_err(|err| format!("cannot open {}: {err}", path.display()))?;
    if !canonical.is_file() {
        return Err(format!("{} is not a file", canonical.display()));
    }
    if !is_markdown(&canonical) {
        return Err(format!("{} is not a markdown file", canonical.display()));
    }
    let markdown = std::fs::read_to_string(&canonical)
        .map_err(|err| format!("cannot read {}: {err}", canonical.display()))?;
    Ok((canonical, markdown))
}

/// 指定されたファイルを読み、初期表示用の文書を作る。
/// ファイルが存在しないか、拡張子が md／markdown でなければ失敗する。
pub fn load(path: &Path) -> Result<Document, String> {
    let (canonical, markdown) = read(path)?;
    Ok(Document {
        generation: 1,
        version: 1,
        path: canonical.to_string_lossy().into_owned(),
        html: render::to_html(&markdown),
    })
}

/// リンクや履歴で開いた別の文書を、指定した世代・版で読み込む。
pub fn open(path: &Path, generation: u64, version: u64) -> Result<Document, String> {
    let (canonical, markdown) = read(path)?;
    Ok(Document {
        generation,
        version,
        path: canonical.to_string_lossy().into_owned(),
        html: render::to_html(&markdown),
    })
}

/// 自分自身を `--foreground` 付きで起動し直し、端末から切り離して動かす。
/// 別のプロセスグループにするので、端末でCtrl-Cを押しても子プロセスには届かない。
/// 標準入出力は捨てる（端末を閉じても書き込みで止まらないように）。
pub fn detach(path: &Path) -> Result<(), String> {
    use std::os::unix::process::CommandExt;
    use std::process::{Command, Stdio};

    let exe = std::env::current_exe()
        .map_err(|err| format!("cannot find the mdsight executable: {err}"))?;
    Command::new(exe)
        .arg("--foreground")
        .arg(path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .process_group(0)
        .spawn()
        .map(|_| ())
        .map_err(|err| format!("cannot start mdsight in the background: {err}"))
}

fn is_markdown(path: &Path) -> bool {
    let extension = path
        .extension()
        .map(|found| found.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    extension == "md" || extension == "markdown"
}

/// 開いているファイルを監視し、変更されたら読み直して表示を更新する。
/// エディタが別名で保存してから置き換えることがあるので、親ディレクトリを
/// 非再帰で監視し、対象のパスへのイベントだけを拾う。
/// リンクや履歴で別の文書を開いたときは、この関数をもう一度呼んで監視を差し替える。
pub fn watch(app: AppHandle, path: PathBuf) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "the file has no directory".to_string())?
        .to_path_buf();

    let (tx, rx) = channel::<notify::Result<notify::Event>>();
    let mut watcher: RecommendedWatcher = notify::recommended_watcher(move |event| {
        let _ = tx.send(event);
    })
    .map_err(|err| format!("cannot watch {}: {err}", parent.display()))?;
    watcher
        .watch(&parent, RecursiveMode::NonRecursive)
        .map_err(|err| format!("cannot watch {}: {err}", parent.display()))?;

    // 古いウォッチャーをここで差し替える。drop されると、そのイベントを待っている
    // 古い監視スレッドは channel が閉じて自然に終わる。
    *app.state::<Watching>().0.lock().expect("watching lock") = Some(watcher);

    let handle = app.clone();
    std::thread::spawn(move || loop {
        let event = match rx.recv() {
            Ok(Ok(event)) => event,
            Ok(Err(_)) => continue,
            Err(_) => break,
        };
        if !touches(&event, &path) {
            continue;
        }
        // 別名保存からの置き換えに備え、少し待って最後の状態だけ拾う
        while rx.recv_timeout(Duration::from_millis(100)).is_ok() {}
        reload(&handle, &path);
    });
    Ok(())
}

fn touches(event: &notify::Event, path: &Path) -> bool {
    event.paths.iter().any(|candidate| candidate == path)
}

/// 現在の世代のまま、版だけを1増やして読み直す。読み直しに失敗したら表示は変えない。
fn reload(app: &AppHandle, path: &Path) {
    let documents = app.state::<Documents>();
    let Some(current) = documents.current() else {
        return;
    };
    // リンクや履歴で別の文書に移っていたら、この監視の対象はもう表示されていない
    // （差し替えの途中の一瞬だけ、古い監視が残っていることがある）
    if Path::new(&current.path) != path {
        return;
    }
    match std::fs::read_to_string(path) {
        Ok(markdown) => {
            crate::publish(
                app,
                Document {
                    generation: current.generation,
                    version: current.version + 1,
                    path: current.path,
                    html: render::to_html(&markdown),
                },
            );
            // 読み直せたので、前の失敗の知らせは取り消す
            set_problem(app, None);
        }
        Err(err) => {
            report(&format!("cannot reload {}: {err}", path.display()));
            set_problem(
                app,
                Some(format!(
                    "Cannot reload {}. Showing the last content that could be read ({err})",
                    file_name(path)
                )),
            );
        }
    }
}

/// 監視を始める（差し替える）。始められなければ、ウィンドウにも知らせる。
pub fn start_watching(app: &AppHandle, path: PathBuf) {
    let name = file_name(&path);
    match watch(app.clone(), path) {
        Ok(()) => set_problem(app, None),
        Err(message) => {
            report(&message);
            set_problem(
                app,
                Some(format!(
                    "Cannot watch {name}. Saving it will not update the view ({message})"
                )),
            );
        }
    }
}

/// 知らせに出すための、パスの最後の部分
fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::{is_markdown, load, open};
    use std::fs;
    use std::path::{Path, PathBuf};

    fn workspace(name: &str) -> PathBuf {
        let directory = std::env::temp_dir().join(format!("mdsight-standalone-{name}"));
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).expect("a test directory");
        directory
    }

    #[test]
    fn loads_a_markdown_file() {
        let base = workspace("load");
        let file = base.join("note.md");
        fs::write(&file, "# title\n").expect("a test file");
        let document = load(&file).expect("a document");
        assert_eq!(document.generation, 1);
        assert_eq!(document.version, 1);
        assert!(document.html.contains("title"), "{}", document.html);
    }

    #[test]
    fn rejects_a_missing_file() {
        let base = workspace("missing");
        assert!(load(&base.join("gone.md")).is_err());
    }

    #[test]
    fn rejects_a_non_markdown_extension() {
        let base = workspace("extension");
        let file = base.join("note.txt");
        fs::write(&file, "text").expect("a test file");
        assert!(load(&file).is_err());
    }

    #[test]
    fn accepts_the_markdown_extension_too() {
        let base = workspace("markdown-extension");
        let file = base.join("note.markdown");
        fs::write(&file, "# title\n").expect("a test file");
        assert!(load(&file).is_ok());
    }

    #[test]
    fn accepts_an_uppercase_extension() {
        assert!(is_markdown(Path::new("/tmp/note.MD")));
    }

    #[test]
    fn rejects_a_directory() {
        let base = workspace("directory");
        assert!(load(&base).is_err());
    }

    #[test]
    fn opens_a_document_with_the_given_generation_and_version() {
        let base = workspace("open");
        let file = base.join("other.md");
        fs::write(&file, "# other\n").expect("a test file");
        let document = open(&file, 3, 8).expect("a document");
        assert_eq!(document.generation, 3);
        assert_eq!(document.version, 8);
        assert!(document.html.contains("other"), "{}", document.html);
    }
}
