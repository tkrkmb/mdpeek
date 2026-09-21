import { beforeEach, describe, expect, test } from "vitest";

import { buildTable, findBlock, parseSourcepos } from "./sourcepos";

function root(html: string): HTMLElement {
  const element = document.createElement("article");
  element.className = "markdown-body";
  element.innerHTML = html;
  return element;
}

beforeEach(() => {
  document.body.innerHTML = "";
});

describe("parseSourcepos", () => {
  test("reads the start and the end line", () => {
    expect(parseSourcepos("3:1-5:12")).toEqual({ start: 3, end: 5 });
  });

  test("returns null for anything else", () => {
    expect(parseSourcepos(null)).toBeNull();
    expect(parseSourcepos("")).toBeNull();
    expect(parseSourcepos("3:1")).toBeNull();
  });
});

describe("buildTable", () => {
  test("keeps the direct children in order of their start line", () => {
    const table = buildTable(
      root(
        `<h1 data-sourcepos="1:1-1:7">t</h1>` +
          `<p data-sourcepos="5:1-6:3">b</p>` +
          `<p data-sourcepos="3:1-3:4">a</p>`,
      ),
    );
    expect(table.map((block) => block.startLine)).toEqual([1, 3, 5]);
    expect(table[0].endLine).toBe(1);
  });

  test("keeps list items but not other nested elements", () => {
    const table = buildTable(
      root(
        `<ul data-sourcepos="1:1-2:6">` +
          `<li data-sourcepos="1:1-1:6"><p data-sourcepos="1:3-1:6">x</p></li>` +
          `<li data-sourcepos="2:1-2:6">y</li>` +
          `</ul>`,
      ),
    );
    expect(table.map((block) => block.element.tagName)).toEqual(["UL", "LI", "LI"]);
  });

  test("ignores elements without a source position", () => {
    expect(buildTable(root(`<p>no position</p>`))).toEqual([]);
  });

  test("returns an empty table for an empty document", () => {
    expect(buildTable(root(""))).toEqual([]);
  });
});

describe("findBlock", () => {
  const table = buildTable(
    root(
      `<h1 data-sourcepos="1:1-1:7">t</h1>` +
        `<p data-sourcepos="3:1-5:9">a</p>` +
        `<p data-sourcepos="9:1-9:4">b</p>`,
    ),
  );

  test("finds the block that contains the line", () => {
    expect(findBlock(table, 4)?.startLine).toBe(3);
    expect(findBlock(table, 1)?.startLine).toBe(1);
  });

  test("falls back to the block before the line", () => {
    expect(findBlock(table, 7)?.startLine).toBe(3);
    expect(findBlock(table, 100)?.startLine).toBe(9);
  });

  test("returns null when there is nothing before the line", () => {
    expect(findBlock(table, 0)).toBeNull();
    expect(findBlock([], 3)).toBeNull();
  });
});
