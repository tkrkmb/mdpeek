import { describe, expect, test } from "vitest";

import { countCodeLines, firstCodeLine, lineIndexAt } from "./codelines";

describe("countCodeLines", () => {
  test("does not count the part after the last newline", () => {
    expect(countCodeLines("a\nb\n")).toBe(2);
    expect(countCodeLines("a\n\nb\n")).toBe(3);
    expect(countCodeLines("a")).toBe(1);
    expect(countCodeLines("")).toBe(0);
  });
});

describe("firstCodeLine", () => {
  test("skips the opening fence of a fenced block", () => {
    // 3行目から7行目：```、コード3行、```
    expect(firstCodeLine(3, 7, 3)).toBe(4);
  });

  test("skips the opening fence of a block left open until the end", () => {
    // 3行目から6行目：```、コード3行（閉じていない）
    expect(firstCodeLine(3, 6, 3)).toBe(4);
  });

  test("starts at the first line of an indented block", () => {
    expect(firstCodeLine(3, 5, 3)).toBe(3);
  });

  test("gives up when the lines do not add up", () => {
    expect(firstCodeLine(3, 10, 3)).toBeNull();
  });

  test("handles an empty fenced block", () => {
    expect(firstCodeLine(3, 4, 0)).toBe(4);
  });
});

describe("lineIndexAt", () => {
  test("maps a position to a line, and outside the code to null", () => {
    expect(lineIndexAt(0, 20, 3)).toBe(0);
    expect(lineIndexAt(19.9, 20, 3)).toBe(0);
    expect(lineIndexAt(20, 20, 3)).toBe(1);
    expect(lineIndexAt(59, 20, 3)).toBe(2);
    expect(lineIndexAt(60, 20, 3)).toBeNull();
    expect(lineIndexAt(-1, 20, 3)).toBeNull();
    expect(lineIndexAt(10, Number.NaN, 3)).toBeNull();
  });
});
