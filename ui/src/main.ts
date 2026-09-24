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

type NavigationTarget = {
  gen: number;
  version: number;
};

/** リンクや履歴で移動した先。届いた文書と世代・版が一致したら、そちらを優先する */
type PendingNavigation = {
  gen: number;
  version: number;
  fragment: string | null;
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
/** 描画を待っている間に届いた、いちばん新しいカーソル（世代付き） */
let pending: { gen: number; line: number } | null = null;
/** リンクや履歴での移動先。まだ文書が届いていない間だけ保持する */
let pendingNavigation: PendingNavigation | null = null;

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

function receiveCursor(gen: number, line: number): void {
  // 古い対象のものは捨てる
  if (gen < shownGen) {
    return;
  }
  if (rendering || gen !== shownGen) {
    // 最新のものだけを保持し、その世代の描画が終わってから適用する
    pending = { gen, line };
    return;
  }
  followCursor(line);
}

function applyPendingCursor(): void {
  if (pending === null || pending.gen !== shownGen) {
    pending = null;
    return;
  }
  const line = pending.line;
  pending = null;
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

/** 届いた文書が、いま待っているリンク先や履歴の行き先と一致するか */
function matchPendingNavigation(gen: number, version: number): PendingNavigation | null {
  if (pendingNavigation === null || pendingNavigation.gen !== gen || pendingNavigation.version !== version) {
    return null;
  }
  const navigation = pendingNavigation;
  pendingNavigation = null;
  return navigation;
}

/** #見出し があればその要素へ、なければ先頭へ移動する */
function goToFragmentOrTop(fragment: string | null): void {
  const target = fragment !== null ? document.getElementById(fragment) : null;
  if (target !== null) {
    target.scrollIntoView();
  } else {
    window.scrollTo({ top: 0 });
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
  // リンクや履歴での移動先なら、読んでいた位置ではなく先頭／見出しへ動く
  const navigation = matchPendingNavigation(doc.gen, doc.version);
  shownGen = doc.gen;
  shownVersion = doc.version;
  const current = isCurrent(doc.version);
  rendering = true;

  // 読んでいた位置を、差し替えの前に記録して、後で戻す
  const anchor = navigation === null ? capture() : null;
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

  if (navigation !== null) {
    goToFragmentOrTop(navigation.fragment);
  } else if (anchor !== null) {
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

/** 相対パスの .md／.markdown リンクを開く。#見出し があれば、開いた後にそこへ動く */
async function followLink(href: string): Promise<void> {
  const hashIndex = href.indexOf("#");
  const fragment = hashIndex === -1 ? null : decodeURIComponent(href.slice(hashIndex + 1));
  const path = hashIndex === -1 ? href : href.slice(0, hashIndex);
  try {
    const target = await invoke<NavigationTarget>("open_link", { href: path, version: shownVersion });
    pendingNavigation = { gen: target.gen, version: target.version, fragment };
  } catch {
    // 解決できなかった、または開けなかった。何もしない
  }
}

/** 戻る／進むの履歴を辿る */
async function navigateHistory(command: "go_back" | "go_forward"): Promise<void> {
  try {
    const target = await invoke<NavigationTarget>(command);
    pendingNavigation = { gen: target.gen, version: target.version, fragment: null };
  } catch {
    // 履歴がない、または開けなかった。何もしない
  }
}

function isBack(event: KeyboardEvent): boolean {
  return isMac ? event.metaKey && event.key === "[" : event.altKey && event.key === "ArrowLeft";
}

function isForward(event: KeyboardEvent): boolean {
  return isMac ? event.metaKey && event.key === "]" : event.altKey && event.key === "ArrowRight";
}

body.addEventListener("click", (event) => {
  const modified = isMac ? event.metaKey : event.ctrlKey;
  if (!modified) {
    handleLink(event, (href) => void followLink(href));
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
  if (isBack(event)) {
    event.preventDefault();
    void navigateHistory("go_back");
    return;
  }
  if (isForward(event)) {
    event.preventDefault();
    void navigateHistory("go_forward");
    return;
  }
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
  receiveCursor(event.payload.gen, event.payload.line);
});

// 起動直後に取りこぼした本文とカーソル行を拾う
void invoke<Document | null>("current_document").then(render);
void invoke<CursorEvent | null>("current_cursor").then((cursor) => {
  if (cursor !== null) {
    receiveCursor(cursor.gen, cursor.line);
  }
});
