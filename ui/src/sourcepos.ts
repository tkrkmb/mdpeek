/** ソース行と、それを表す要素の対応 */
export type Block = {
  startLine: number;
  endLine: number;
  element: HTMLElement;
};

/** `data-sourcepos` の `3:1-5:12` から開始行と終了行を読む */
export function parseSourcepos(value: string | null): { start: number; end: number } | null {
  if (value === null) {
    return null;
  }
  const match = /^(\d+):\d+-(\d+):\d+$/.exec(value.trim());
  if (match === null) {
    return null;
  }
  const start = Number.parseInt(match[1], 10);
  const end = Number.parseInt(match[2], 10);
  if (!Number.isFinite(start) || !Number.isFinite(end)) {
    return null;
  }
  return { start, end };
}

/**
 * 位置表を作る。
 * `.markdown-body` の直下にある要素と、リストの `li` だけを対象にし、開始行の順に並べる。
 */
export function buildTable(root: HTMLElement): Block[] {
  const found: Block[] = [];
  const add = (element: Element) => {
    const position = parseSourcepos(element.getAttribute("data-sourcepos"));
    if (position !== null) {
      found.push({ startLine: position.start, endLine: position.end, element: element as HTMLElement });
    }
  };

  for (const child of Array.from(root.children)) {
    add(child);
  }
  for (const item of Array.from(root.querySelectorAll("li"))) {
    add(item);
  }

  found.sort((left, right) => left.startLine - right.startLine);
  return found;
}

/** その行を含むブロックを返す。無ければ直前のブロックを返す。 */
export function findBlock(table: Block[], line: number): Block | null {
  let previous: Block | null = null;
  for (const block of table) {
    if (block.startLine <= line && line <= block.endLine) {
      return block;
    }
    if (block.startLine <= line) {
      previous = block;
    }
  }
  return previous;
}
