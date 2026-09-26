import type { Block } from "./sourcepos";

/**
 * コードブロックの中を、行単位でソース行に対応させる。
 * コードブロックは折り返さないので、行の位置は「内側の上端 + 行の高さ × 何行目」で決まる
 */

/** 位置表のコードブロックと、そのコードの最初の行のソース行、コードの行数 */
export type CodeLines = {
  pre: HTMLElement;
  first: number;
  count: number;
};

/** コードの行数（末尾の改行の後ろは数えない） */
export function countCodeLines(text: string): number {
  if (text === "") {
    return 0;
  }
  const lines = text.split("\n");
  return lines[lines.length - 1] === "" ? lines.length - 1 : lines.length;
}

/**
 * コードの最初の行に当たるソース行。ブロックの行範囲がコードの行数より2行多ければ前後に囲い、
 * 1行多ければ開始の囲いだけ、同じなら字下げのコードブロック。それ以外なら null
 */
export function firstCodeLine(startLine: number, endLine: number, count: number): number | null {
  const extra = endLine - startLine + 1 - count;
  if (extra === 2 || extra === 1) {
    return startLine + 1;
  }
  return extra === 0 ? startLine : null;
}

/** 内側の上端からの距離が offset の位置は、何行目か（0から）。コードの行の外なら null */
export function lineIndexAt(offset: number, lineHeight: number, count: number): number | null {
  if (!(lineHeight > 0)) {
    return null;
  }
  const index = Math.floor(offset / lineHeight);
  return index >= 0 && index < count ? index : null;
}

/** ブロックが位置表のコードブロック（コピーのボタンの枠。Mermaid は含まれない）なら、その行の対応を返す */
export function codeLinesOf(block: Block): CodeLines | null {
  if (!block.element.classList.contains("mdsight-code")) {
    return null;
  }
  const pre = block.element.querySelector<HTMLElement>(":scope > pre");
  const code = pre?.querySelector<HTMLElement>(":scope > code");
  if (pre === null || pre === undefined || code === null || code === undefined) {
    return null;
  }
  const count = countCodeLines(code.textContent ?? "");
  const first = firstCodeLine(block.startLine, block.endLine, count);
  return first === null ? null : { pre, first, count };
}

/** 内側の上端（画面上の位置）と行の高さ */
function metrics(pre: HTMLElement): { top: number; lineHeight: number } {
  const style = getComputedStyle(pre);
  const top = pre.getBoundingClientRect().top + Number.parseFloat(style.borderTopWidth) + Number.parseFloat(style.paddingTop);
  return { top, lineHeight: Number.parseFloat(style.lineHeight) };
}

/** ソース行がコードの行なら、その行の画面上の上端と下端。そうでなければ null */
export function codeLineRect(lines: CodeLines, line: number): { top: number; bottom: number } | null {
  const index = line - lines.first;
  if (index < 0 || index >= lines.count) {
    return null;
  }
  const { top, lineHeight } = metrics(lines.pre);
  if (!(lineHeight > 0)) {
    return null;
  }
  return { top: top + index * lineHeight, bottom: top + (index + 1) * lineHeight };
}

/** 画面上の高さ clientY にあるコードの行のソース行。囲いや余白の上なら null */
export function codeLineAt(lines: CodeLines, clientY: number): number | null {
  const { top, lineHeight } = metrics(lines.pre);
  const index = lineIndexAt(clientY - top, lineHeight, lines.count);
  return index === null ? null : lines.first + index;
}
