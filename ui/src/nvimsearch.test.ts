import { describe, expect, test } from "vitest";

import { applyNvimSearch, groupByBlock, type NvimMatch } from "./nvimsearch";
import { locateInOrder } from "./search";
import { buildTable } from "./sourcepos";

function body(html: string): HTMLElement {
  const root = document.createElement("article");
  root.innerHTML = html;
  return root;
}

const match = (line: number, text: string, current = false): NvimMatch => ({ line, text, current });

describe("locateInOrder", () => {
  test("finds each text after the previous one", () => {
    expect(locateInOrder("page and page", ["page", "page"])).toEqual([
      { start: 0, end: 4 },
      { start: 9, end: 13 },
    ]);
  });

  test("skips a text that is not there without moving on", () => {
    expect(locateInOrder("a page", ["**b**", "page"])).toEqual([null, { start: 2, end: 6 }]);
  });
});

describe("groupByBlock", () => {
  test("puts matches into the block that holds their line, in order", () => {
    const root = body('<p data-sourcepos="1:1-2:5">x</p><ul data-sourcepos="4:1-5:3"><li data-sourcepos="4:1-4:3">a</li><li data-sourcepos="5:1-5:3">b</li></ul>');
    const table = buildTable(root);
    const groups = [...groupByBlock(table, [match(1, "a"), match(2, "b"), match(4, "c"), match(5, "d"), match(9, "e")])];
    expect(groups.map(([block, list]) => [block.element.tagName, list.map((m) => m.text)])).toEqual([
      ["P", ["a", "b"]],
      ["UL", ["c", "d"]],
    ]);
  });
});

describe("applyNvimSearch", () => {
  test("marks the matches of each block in order, and the current one", () => {
    const root = body('<p data-sourcepos="1:1-1:20">Page one <em>page</em> two</p><p data-sourcepos="3:1-3:10">a page</p>');
    const table = buildTable(root);
    applyNvimSearch(root, table, {
      gen: 1,
      version: 1,
      matches: [match(1, "Page"), match(1, "page", true), match(3, "page")],
    });
    const marks = [...root.querySelectorAll("mark.mdsight-nvim-hit")];
    expect(marks.map((mark) => mark.textContent)).toEqual(["Page", "page", "page"]);
    expect(marks.map((mark) => mark.classList.contains("mdsight-nvim-current"))).toEqual([false, true, false]);
    expect(marks[1].parentElement?.tagName).toBe("EM");
  });

  test("replaces the previous marks and ignores text it cannot find", () => {
    const root = body('<p data-sourcepos="1:1-1:20">bold text here</p>');
    const table = buildTable(root);
    applyNvimSearch(root, table, { gen: 1, version: 1, matches: [match(1, "text")] });
    applyNvimSearch(root, table, { gen: 1, version: 2, matches: [match(1, "**bold**"), match(1, "here")] });
    expect([...root.querySelectorAll("mark")].map((mark) => mark.textContent)).toEqual(["here"]);
    applyNvimSearch(root, table, { gen: 1, version: 3, matches: [] });
    expect(root.querySelectorAll("mark")).toHaveLength(0);
    expect(root.innerHTML).toBe('<p data-sourcepos="1:1-1:20">bold text here</p>');
  });
});
