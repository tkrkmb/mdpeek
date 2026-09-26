import { describe, expect, test } from "vitest";

import { clearMarks, collectText, markRanges } from "./search";

function fragment(html: string): HTMLElement {
  const root = document.createElement("div");
  root.innerHTML = html;
  return root;
}

describe("collectText", () => {
  test("joins text across elements", () => {
    const index = collectText(fragment("<p>foo <strong>bar</strong> baz</p>"));
    expect(index.text).toBe("foo bar baz");
    expect(index.nodes.map((entry) => entry.start)).toEqual([0, 4, 7]);
  });

  test("skips diagrams and math", () => {
    const root = fragment(
      '<p>keep</p><pre data-mermaid-source="graph"><svg><text>skip</text></svg></pre>' +
        '<pre><code class="language-mermaid">skip</code></pre><span data-math-style="inline">skip</span>',
    );
    expect(collectText(root).text).toBe("keep");
  });

  test("returns nothing when the block itself is a diagram or math", () => {
    const root = fragment('<pre data-mermaid-source="graph"><svg><text>skip</text></svg></pre>');
    expect(collectText(root.firstElementChild!).text).toBe("");
  });

  test("includes code blocks and tables", () => {
    const root = fragment(
      '<pre><code class="language-rust"><span class="hljs-keyword">fn</span> main</code></pre><table><tr><td>cell</td></tr></table>',
    );
    expect(collectText(root).text).toBe("fn maincell");
  });
});

describe("markRanges and clearMarks", () => {
  test("wraps a match that spans elements, in order", () => {
    const root = fragment("<p>foo <strong>bar</strong> baz</p>");
    const index = collectText(root);
    const marks = markRanges(index, [{ start: 2, end: 9 }], "hit");
    expect(marks).toHaveLength(1);
    expect(marks[0].map((mark) => mark.textContent)).toEqual(["o ", "bar", " b"]);
    expect(root.innerHTML).toBe(
      '<p>fo<mark class="hit">o </mark><strong><mark class="hit">bar</mark></strong><mark class="hit"> b</mark>az</p>',
    );
  });

  test("wraps several matches in one text node", () => {
    const root = fragment("<p>ab ab ab</p>");
    const index = collectText(root);
    const marks = markRanges(index, [{ start: 0, end: 2 }, { start: 3, end: 5 }, { start: 6, end: 8 }], "hit");
    expect(marks.map((group) => group.length)).toEqual([1, 1, 1]);
    expect(root.querySelectorAll("mark.hit")).toHaveLength(3);
    expect(root.textContent).toBe("ab ab ab");
  });

  test("restores the original text nodes", () => {
    const root = fragment("<p>foo <strong>bar</strong> baz</p>");
    const before = root.innerHTML;
    const index = collectText(root);
    markRanges(index, [{ start: 5, end: 6 }, { start: 9, end: 10 }], "hit");
    clearMarks(root, "hit");
    expect(root.innerHTML).toBe(before);
    expect(root.querySelector("p")?.childNodes).toHaveLength(3);
  });

  test("leaves marks of another kind alone", () => {
    const root = fragment("<p>foo bar</p>");
    let index = collectText(root);
    markRanges(index, [{ start: 0, end: 3 }], "other");
    index = collectText(root);
    markRanges(index, [{ start: 4, end: 7 }], "hit");
    clearMarks(root, "hit");
    expect(root.innerHTML).toBe('<p><mark class="other">foo</mark> bar</p>');
  });
});
