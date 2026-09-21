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

const body = document.querySelector<HTMLElement>(".markdown-body")!;

let shownVersion = -1;

function render(doc: Document | null): void {
  if (doc === null) {
    return;
  }
  // 表示中の版より古ければ破棄する
  if (doc.version < shownVersion) {
    return;
  }
  shownVersion = doc.version;
  body.innerHTML = doc.html;
}

void listen<Document>("mdpeek://document", (event) => {
  render(event.payload);
});

// 起動直後に取りこぼした本文を拾う
void invoke<Document | null>("current_document").then(render);
