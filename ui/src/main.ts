import "github-markdown-css/github-markdown-light.css";
import "./style.css";

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

type Document = {
  gen: number;
  version: number;
  path: string;
  html: string;
};

/** 画面の上端付近にあるブロックのソース行と、画面内でのオフセット */
type Anchor = {
  line: number;
  offset: number;
};

const body = document.querySelector<HTMLElement>(".markdown-body")!;

let shownVersion = -1;

function startLine(element: Element): number | null {
  const sourcepos = element.getAttribute("data-sourcepos");
  if (sourcepos === null) {
    return null;
  }
  const line = Number.parseInt(sourcepos.split(":")[0], 10);
  return Number.isFinite(line) ? line : null;
}

/** `.markdown-body` の直下にある、ソース行の分かるブロック */
function blocks(): { line: number; element: HTMLElement }[] {
  const found: { line: number; element: HTMLElement }[] = [];
  for (const child of Array.from(body.children)) {
    const line = startLine(child);
    if (line !== null) {
      found.push({ line, element: child as HTMLElement });
    }
  }
  return found;
}

function capture(): Anchor | null {
  for (const block of blocks()) {
    const rect = block.element.getBoundingClientRect();
    if (rect.bottom > 0) {
      return { line: block.line, offset: rect.top };
    }
  }
  return null;
}

function restore(anchor: Anchor): void {
  const found = blocks();
  let target: HTMLElement | null = null;
  for (const block of found) {
    if (block.line > anchor.line) {
      break;
    }
    target = block.element;
  }
  if (target === null) {
    target = found.length > 0 ? found[0].element : null;
  }
  if (target === null) {
    return;
  }
  const top = window.scrollY + target.getBoundingClientRect().top - anchor.offset;
  window.scrollTo({ top, behavior: "instant" });
}

function render(doc: Document | null): void {
  if (doc === null) {
    return;
  }
  // 表示中の版より古ければ破棄する
  if (doc.version < shownVersion) {
    return;
  }
  shownVersion = doc.version;

  // 読んでいた位置を、差し替えの前に記録して、後で戻す
  const anchor = capture();
  body.innerHTML = doc.html;
  if (anchor !== null) {
    restore(anchor);
  }
}

void listen<Document>("mdpeek://document", (event) => {
  render(event.payload);
});

// 起動直後に取りこぼした本文を拾う
void invoke<Document | null>("current_document").then(render);
