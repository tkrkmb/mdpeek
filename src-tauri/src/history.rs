use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use crate::state::{Document, Documents, Session};
use crate::{nvim, raise, resolve, standalone, RuntimeMode};

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

/// フロントエンドが、戻る／進むボタンの有効/無効を決めるために使う。
#[tauri::command]
pub fn history_state(history: tauri::State<'_, History>) -> HistoryAvailability {
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
pub async fn open_link(
    app: AppHandle,
    documents: tauri::State<'_, Documents>,
    history: tauri::State<'_, History>,
    mode: tauri::State<'_, RuntimeMode>,
    session: tauri::State<'_, Session>,
    href: String,
    version: u64,
    anchor: Option<Anchor>,
) -> Result<NavigationTarget, String> {
    let (current, base) = documents.current_with_base(version)?;
    let target = resolve::resolve_markdown_link(&base, &href)?;

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
pub async fn go_back(
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
pub async fn go_forward(
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
            crate::state::publish(app, document.clone());
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

#[cfg(test)]
mod tests {
    use super::{Anchor, History, HistoryEntry, HistoryState};

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
