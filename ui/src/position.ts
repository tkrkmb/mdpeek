import { codeLineRect, codeLinesOf } from "./codelines";
import { bottomInset } from "./find";
import { topInset } from "./pathbar";
import { revealRect } from "./scroll";
import { findBlock } from "./sourcepos";
import { view } from "./view";

/** 画面の上端付近にあるブロックのソース行と、画面内でのオフセット */
export type Anchor = {
  line: number;
  offset: number;
};

/** いま読んでいる位置を記録する */
export function capture(): Anchor | null {
  const inset = topInset();
  for (const block of view.table) {
    const rect = block.element.getBoundingClientRect();
    if (rect.bottom > inset) {
      return { line: block.startLine, offset: rect.top };
    }
  }
  return null;
}

/** 記録した位置へ戻す */
export function restore(anchor: Anchor): void {
  const target = findBlock(view.table, anchor.line) ?? view.table[0];
  if (target === undefined) {
    return;
  }
  const top = window.scrollY + target.element.getBoundingClientRect().top - anchor.offset;
  window.scrollTo({ top, behavior: "instant" });
}

/** その行のブロックが画面に無ければ、上から1/3の位置に来るようにスクロールする */
export function followCursor(line: number): void {
  const block = findBlock(view.table, line);
  if (block === null) {
    return;
  }
  // コードブロックの中の行なら、ブロックではなくその行を対象にする
  const lines = line >= block.startLine && line <= block.endLine ? codeLinesOf(block) : null;
  const rect = (lines !== null ? codeLineRect(lines, line) : null) ?? block.element.getBoundingClientRect();
  revealRect(rect, topInset(), bottomInset());
}
