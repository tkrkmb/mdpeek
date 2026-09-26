import { clearMarks, collectText, locateInOrder, markRanges, type Range } from "./search";
import { findBlock, type Block } from "./sourcepos";

/** Neovimの検索の一致（`mdsight_search`） */
export type NvimMatch = {
  line: number;
  text: string;
  current: boolean;
};

export type NvimSearch = {
  gen: number;
  version: number;
  matches: NvimMatch[];
};

/** Neovimの検索の一致と、現在の一致の `<mark>` のクラス */
const HIT = "mdsight-nvim-hit";
const CURRENT = "mdsight-nvim-current";

/**
 * 一致を、その行を含むブロックごとに振り分ける。ブロックの中では、届いた順（バッファでの出現順）に並べる。
 * どのブロックにも含まれない行の一致は捨てる
 */
export function groupByBlock(table: Block[], matches: NvimMatch[]): Map<Block, NvimMatch[]> {
  const groups = new Map<Block, NvimMatch[]>();
  for (const match of matches) {
    const block = findBlock(table, match.line);
    if (block === null || match.line < block.startLine || match.line > block.endLine) {
      continue;
    }
    const group = groups.get(block);
    if (group === undefined) {
      groups.set(block, [match]);
    } else {
      group.push(match);
    }
  }
  return groups;
}

/**
 * Neovimの検索の一致を強調し直す。ブロックの文字の中から、一致した文字列を出現順に探して包む。
 * 見つからない一致（Markdownの記号を含むものなど）は強調しない
 */
export function applyNvimSearch(root: HTMLElement, table: Block[], search: NvimSearch): void {
  clearMarks(root, HIT);
  for (const [block, group] of groupByBlock(table, search.matches)) {
    const index = collectText(block.element);
    const located = locateInOrder(
      index.text,
      group.map((match) => match.text),
    );
    const ranges: Range[] = [];
    const current: boolean[] = [];
    located.forEach((range, position) => {
      if (range !== null) {
        ranges.push(range);
        current.push(group[position].current);
      }
    });
    markRanges(index, ranges, HIT).forEach((marks, position) => {
      if (current[position]) {
        for (const mark of marks) {
          mark.classList.add(CURRENT);
        }
      }
    });
  }
}
