/** 検索の対象にしない要素（Mermaidの図と数式は、描画後の文字が元のコードと違うため） */
const EXCLUDED = "[data-math-style], [data-mermaid-source], code.language-mermaid";

/** 要素の中の文字を1本の文字列につなげたもの。どの位置がどのテキストノードかを覚えておく */
export type TextIndex = {
  text: string;
  nodes: { node: Text; start: number }[];
};

/** 文字列の中の位置 [start, end) */
export type Range = {
  start: number;
  end: number;
};

export function collectText(root: Element): TextIndex {
  const nodes: { node: Text; start: number }[] = [];
  let text = "";
  // ブロックそのものが図や数式のときも、中の文字を拾わない
  if (root.matches(EXCLUDED)) {
    return { text, nodes };
  }
  const walker = root.ownerDocument.createTreeWalker(root, NodeFilter.SHOW_ELEMENT | NodeFilter.SHOW_TEXT, {
    acceptNode(node) {
      if (node instanceof Element) {
        return node.matches(EXCLUDED) ? NodeFilter.FILTER_REJECT : NodeFilter.FILTER_SKIP;
      }
      return NodeFilter.FILTER_ACCEPT;
    },
  });
  for (let node = walker.nextNode(); node !== null; node = walker.nextNode()) {
    const content = (node as Text).data;
    nodes.push({ node: node as Text, start: text.length });
    text += content;
  }
  return { text, nodes };
}

/**
 * 一致した文字を `<mark class="...">` で包む。
 * 要素をまたぐ一致は、テキストノードごとに分けて包む。一致ごとの `<mark>` の配列を返す
 */
export function markRanges(index: TextIndex, ranges: Range[], className: string): HTMLElement[][] {
  const marks: HTMLElement[][] = ranges.map(() => []);
  for (const { node, start } of index.nodes) {
    const end = start + node.data.length;
    // このテキストノードにかかる部分を集め、後ろから包む（前の位置がずれないように）
    const pieces: { from: number; to: number; match: number }[] = [];
    ranges.forEach((range, match) => {
      const from = Math.max(range.start, start);
      const to = Math.min(range.end, end);
      if (from < to) {
        pieces.push({ from: from - start, to: to - start, match });
      }
    });
    for (const piece of pieces.reverse()) {
      if (piece.to < node.data.length) {
        node.splitText(piece.to);
      }
      const target = piece.from > 0 ? node.splitText(piece.from) : node;
      const mark = node.ownerDocument.createElement("mark");
      mark.className = className;
      target.replaceWith(mark);
      mark.appendChild(target);
      // 1つのテキストノードにかかる部分は一致ごとに1つだけなので、ノードの順に足せば出現順になる
      marks[piece.match].push(mark);
    }
  }
  return marks;
}

/** `markRanges` で包んだ `<mark>` を外し、元のテキストに戻す */
export function clearMarks(root: Element, className: string): void {
  const parents = new Set<Node>();
  for (const mark of Array.from(root.querySelectorAll(`mark.${className}`))) {
    const parent = mark.parentNode;
    if (parent === null) {
      continue;
    }
    mark.replaceWith(...Array.from(mark.childNodes));
    parents.add(parent);
  }
  for (const parent of parents) {
    parent.normalize();
  }
}

/**
 * 文字列を、並んだ順に探す。それぞれ前の一致の後ろから探し、見つからなければ `null` にする
 * （見つからなかったときは、次を探す位置を進めない）
 */
export function locateInOrder(text: string, needles: string[]): (Range | null)[] {
  let from = 0;
  return needles.map((needle) => {
    const start = needle === "" ? -1 : text.indexOf(needle, from);
    if (start === -1) {
      return null;
    }
    from = start + needle.length;
    return { start, end: from };
  });
}
