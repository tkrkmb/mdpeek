import { invoke } from "@tauri-apps/api/core";

import { codeLineAt, codeLinesOf } from "./codelines";
import { followLink } from "./history";
import { isModifiedClick } from "./keys";
import { handleLink } from "./links";
import type { Block } from "./sourcepos";
import { body, view } from "./view";

function blockAt(target: Element): Block | null {
  // クリックされた要素に最も近い祖先で、位置表に含まれるもの
  for (let element: Element | null = target; element !== null; element = element.parentElement) {
    const found = view.table.find((block) => block.element === element);
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
  for (const block of view.table) {
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

/** 本文のクリックを、リンクの処理と、修飾クリックによるNeovimへのジャンプに振り分ける */
export function initClicks(): void {
  body.addEventListener("click", (event) => {
    if (!isModifiedClick(event)) {
      handleLink(event, (href) => void followLink(href));
      return;
    }
    // 修飾クリックのときは、リンクの通常動作を止める
    event.preventDefault();
    // 描画を待っている間は無視する
    if (view.rendering) {
      return;
    }
    const target = event.target instanceof Element ? event.target : null;
    const block = (target !== null ? blockAt(target) : null) ?? nearestBlock(event.clientY);
    if (block === null) {
      return;
    }
    // コードブロックの中なら、クリックした位置のコードの行へ（囲いや余白の上なら、ブロックの開始行へ）
    const lines = codeLinesOf(block);
    const codeLine = lines !== null && target !== null && lines.pre.contains(target) ? codeLineAt(lines, event.clientY) : null;
    void invoke("jump", { gen: view.gen, version: view.version, line: codeLine ?? block.startLine });
  });
}
