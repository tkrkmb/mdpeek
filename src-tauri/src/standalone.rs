use std::path::{Path, PathBuf};
use std::sync::mpsc::channel;
use std::time::Duration;

use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use tauri::{AppHandle, Manager};

use crate::{render, report, Document, Documents};

/// 指定されたファイルを読み、初期表示用の文書を作る。
/// ファイルが存在しないか、拡張子が md／markdown でなければ失敗する。
pub fn load(path: &Path) -> Result<Document, String> {
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
    Ok(Document {
        generation: 1,
        version: 1,
        path: canonical.to_string_lossy().into_owned(),
        html: render::to_html(&markdown),
    })
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

    std::thread::spawn(move || {
        // ウォッチャーを保持し続ける（drop すると監視が止まる）
        let _watcher = watcher;
        loop {
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
            reload(&app, &path);
        }
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
        }
        Err(err) => report(&format!("cannot reload {}: {err}", path.display())),
    }
}

#[cfg(test)]
mod tests {
    use super::{is_markdown, load};
    use std::fs;
    use std::path::{Path, PathBuf};

    fn workspace(name: &str) -> PathBuf {
        let directory = std::env::temp_dir().join(format!("mdpeek-standalone-{name}"));
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
}
