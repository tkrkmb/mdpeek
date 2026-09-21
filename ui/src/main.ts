import "./style.css";

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

import { renderDiagrams } from "./diagrams";
import { resolveImages } from "./images";
import { handleLink } from "./links";
import { renderMath } from "./math";
import { buildTable, findBlock, type Block } from "./sourcepos";
import { cycle, onThemeChange, start as startTheme } from "./theme";

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

/** 画面の上端付近にあるブロックのソース行と、画面内でのオフセット */
type Anchor = {
  line: number;
  offset: number;
};

const body = document.querySelector<HTMLElement>(".markdown-body")!;

let shownGen = -1;
let shownVersion = -1;
let table: Block[] = [];
/** 描画（画像と図を含む）が終わるまで true */
let rendering = false;
/** 描画を待っている間に届いた、いちばん新しいカーソル行 */
let pendingLine: number | null = null;

function isCurrent(version: number): () => boolean {
  return () => version === shownVersion;
}

function rebuildTable(): void {
  table = buildTable(body);
}

function capture(): Anchor | null {
  for (const block of table) {
    const rect = block.element.getBoundingClientRect();
    if (rect.bottom > 0) {
      return { line: block.startLine, offset: rect.top };
    }
  }
  return null;
}

function restore(anchor: Anchor): void {
  const target = findBlock(table, anchor.line) ?? table[0];
  if (target === undefined) {
    return;
  }
  const top = window.scrollY + target.element.getBoundingClientRect().top - anchor.offset;
  window.scrollTo({ top, behavior: "instant" });
}

/** その行のブロックが画面に無ければ、上から1/3の位置に来るようにスクロールする */
function followCursor(line: number): void {
  const block = findBlock(table, line);
  if (block === null) {
    return;
  }
  const rect = block.element.getBoundingClientRect();
  const visible = rect.bottom > 0 && rect.top < window.innerHeight;
  if (visible) {
    return;
  }
  const top = window.scrollY + rect.top - window.innerHeight / 3;
  window.scrollTo({ top, behavior: "instant" });
}

function applyPendingCursor(): void {
  if (pendingLine === null) {
    return;
  }
  const line = pendingLine;
  pendingLine = null;
  followCursor(line);
}

/** 画像や図が入ると高さが変わるので、そのたびに位置表を作り直す */
function watchImages(version: number): void {
  for (const image of Array.from(body.querySelectorAll("img"))) {
    const update = () => {
      if (version === shownVersion) {
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
  if (doc.version < shownVersion) {
    return;
  }
  shownGen = doc.gen;
  shownVersion = doc.version;
  const current = isCurrent(doc.version);
  rendering = true;

  // 読んでいた位置を、差し替えの前に記録して、後で戻す
  const anchor = capture();
  body.innerHTML = doc.html;
  renderMath(body);
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

  if (anchor !== null) {
    restore(anchor);
  }

  void Promise.all([diagrams, images]).then(() => {
    if (!current()) {
      return;
    }
    rebuildTable();
    rendering = false;
    applyPendingCursor();
  });
}

const isMac = navigator.userAgent.includes("Macintosh");

function blockAt(target: Element): Block | null {
  // クリックされた要素に最も近い祖先で、位置表に含まれるもの
  for (let element: Element | null = target; element !== null; element = element.parentElement) {
    const found = table.find((block) => block.element === element);
    if (found !== undefined) {
      return found;
    }
  }
  return null;
}

/** 見つからないときは、縦方向で最も近いブロックを使う */
function nearestBlock(clientY: number): Block | null {
  let best: Block | null = null;
  let bestDistance = Number.POSITIVE_INFINITY;
  for (const block of table) {
    const rect = block.element.getBoundingClientRect();
    const distance =
      clientY < rect.top ? rect.top - clientY : clientY > rect.bottom ? clientY - rect.bottom : 0;
    if (distance < bestDistance) {
      best = block;
      bestDistance = distance;
    }
  }
  return best;
}

body.addEventListener("click", (event) => {
  const modified = isMac ? event.metaKey : event.ctrlKey;
  if (!modified) {
    handleLink(event);
    return;
  }
  // 修飾クリックのときは、リンクの通常動作を止める
  event.preventDefault();
  // 描画を待っている間は無視する
  if (rendering) {
    return;
  }
  const target = event.target instanceof Element ? event.target : null;
  const block = (target !== null ? blockAt(target) : null) ?? nearestBlock(event.clientY);
  if (block === null) {
    return;
  }
  void invoke("jump", { gen: shownGen, version: shownVersion, line: block.startLine });
});

startTheme();
onThemeChange(() => {
  void renderDiagrams(body, isCurrent(shownVersion)).then((drawn) => {
    if (drawn) {
      rebuildTable();
    }
  });
});

document.addEventListener("keydown", (event) => {
  if (event.ctrlKey || event.metaKey || event.altKey) {
    return;
  }
  if (event.key.toLowerCase() === "t") {
    cycle();
  }
});

window.addEventListener("resize", rebuildTable);

void listen<Document>("mdpeek://document", (event) => {
  render(event.payload);
});

void listen<CursorEvent>("mdpeek://cursor", (event) => {
  if (event.payload.gen !== shownGen) {
    return;
  }
  if (rendering) {
    // 最新のものだけを保持し、描画が完了してから適用する
    pendingLine = event.payload.line;
    return;
  }
  followCursor(event.payload.line);
});

// 起動直後に取りこぼした本文を拾う
void invoke<Document | null>("current_document").then(render);
