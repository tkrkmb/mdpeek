import "./style.css";

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

import { applyPendingCursor, receiveCursor } from "./cursor";
import { addCopyButtons } from "./copy";
import { renderDiagrams } from "./diagrams";
import { initFind, refreshFind } from "./find";
import { highlightCode } from "./highlight";
import { arrive, initHistory, matchPendingNavigation, refreshHistoryAvailability, resetSettling, restoreSettled } from "./history";
import { resolveImages } from "./images";
import { initClicks } from "./jump";
import { initKeys } from "./keys";
import { renderMath } from "./math";
import { applyNvimSearch, type NvimSearch } from "./nvimsearch";
import { setProblem } from "./notice";
import { initPathBar, showPath } from "./pathbar";
import { capture, restore } from "./position";
import { onThemeChange, start as startTheme } from "./theme";
import { body, isCurrent, rebuildTable, view } from "./view";

type Document = {
  gen: number;
  version: number;
  path: string;
  html: string;
};

type CursorEvent = {
  gen: number;
  line: number;
};

/** 最後に届いたNeovimの検索の一致。表示中の世代・版と一致し、描画が終わっているときだけ適用する */
let latestSearch: NvimSearch | null = null;

function applySearchIfCurrent(): void {
  if (latestSearch === null || view.rendering) {
    return;
  }
  if (latestSearch.gen !== view.gen || latestSearch.version !== view.version) {
    return;
  }
  applyNvimSearch(body, view.table, latestSearch);
}

/** 画像や図が入ると高さが変わるので、そのたびに位置表を作り直す */
function watchImages(version: number): void {
  for (const image of Array.from(body.querySelectorAll("img"))) {
    const update = () => {
      if (version === view.version) {
        rebuildTable();
      }
    };
    image.addEventListener("load", update, { once: true });
    image.addEventListener("error", update, { once: true });
  }
}

function render(doc: Document | null): void {
  if (doc === null) {
    return;
  }
  // 表示中の版より古ければ破棄する
  if (doc.version < view.version) {
    return;
  }
  // リンクや履歴での移動先なら、読んでいた位置ではなく先頭／見出しへ動く
  const navigation = matchPendingNavigation(doc.gen, doc.version);
  const switched = doc.gen !== view.gen;
  if (switched) {
    // 対象世代が変わった(自分の操作でも、:MdSightによる切り替えでも)ので、
    // 戻る／進むボタンの有効/無効を最新の状態に合わせ直す
    void refreshHistoryAvailability();
  }
  view.gen = doc.gen;
  view.version = doc.version;
  showPath(doc.path);
  const current = isCurrent(doc.version);
  view.rendering = true;

  // 読んでいた位置を、差し替えの前に記録して、後で戻す。
  // 別の文書に替わったときは、前の文書の位置は意味を持たないので記録しない
  const anchor = navigation === null && !switched ? capture() : null;
  // 開いていた details を、出現順を手がかりに、差し替えた後も開いたままにする
  const opened = navigation === null && !switched ? Array.from(body.querySelectorAll("details"), (details) => details.open) : [];
  body.innerHTML = doc.html;
  Array.from(body.querySelectorAll("details")).forEach((details, index) => {
    if (opened[index]) {
      details.open = true;
    }
  });
  renderMath(body);
  highlightCode(body);
  // 位置表を作る前に、コードブロックを枠で包む（data-sourcepos が枠へ移る）
  addCopyButtons(body);
  // 検索窓が開いていれば、同じ文字列で探し直す（スクロールはしない）
  refreshFind();
  rebuildTable();

  const diagrams = renderDiagrams(body, current).then((drawn) => {
    if (drawn && current()) {
      rebuildTable();
    }
  });
  const images = resolveImages(body, doc.version, current).then(() => {
    if (current()) {
      watchImages(doc.version);
      rebuildTable();
    }
  });

  resetSettling();
  if (navigation !== null) {
    arrive(navigation.fragment, navigation.anchor);
  } else if (switched) {
    window.scrollTo({ top: 0 });
  } else if (anchor !== null) {
    restore(anchor);
  }

  void Promise.all([diagrams, images]).then(() => {
    if (!current()) {
      return;
    }
    rebuildTable();
    view.rendering = false;
    restoreSettled();
    applyPendingCursor();
    // 描画を待っている間に届いた検索の一致も、ここで適用する
    applySearchIfCurrent();
  });
}

initClicks();
initPathBar();
initFind(body);
initHistory();

startTheme();
onThemeChange(() => {
  void renderDiagrams(body, isCurrent(view.version)).then((drawn) => {
    if (drawn) {
      rebuildTable();
    }
  });
});

initKeys();

window.addEventListener("resize", rebuildTable);

void listen<Document>("mdsight://document", (event) => {
  render(event.payload);
});

void listen<CursorEvent>("mdsight://cursor", (event) => {
  receiveCursor(event.payload.gen, event.payload.line);
});

void listen<NvimSearch>("mdsight://search", (event) => {
  latestSearch = event.payload;
  applySearchIfCurrent();
});

// 読み直しの失敗などは、切り離して動いていると端末に出ないので、ウィンドウに出す
void listen<string | null>("mdsight://problem", (event) => {
  setProblem(event.payload);
});

// 起動直後に取りこぼした本文とカーソル行、問題を拾う
void invoke<string | null>("current_problem").then(setProblem);
void invoke<Document | null>("current_document").then(render);
void invoke<NvimSearch | null>("current_search").then((search) => {
  // イベントで新しいものが先に届いていれば、そちらを使う
  if (search !== null && latestSearch === null) {
    latestSearch = search;
    applySearchIfCurrent();
  }
});
void invoke<CursorEvent | null>("current_cursor").then((cursor) => {
  if (cursor !== null) {
    receiveCursor(cursor.gen, cursor.line);
  }
});
