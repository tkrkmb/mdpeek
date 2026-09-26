import { describe, expect, test } from "vitest";

import { revealTop } from "./scroll";

describe("revealTop", () => {
  // 高さ 900 の窓に、上に 30、下に 60 の帯がある。見える部分は 30〜840（高さ 810）
  const reveal = (rect: { top: number; bottom: number }) => revealTop(rect, 1000, 900, 30, 60);

  test("does not scroll when the range is visible", () => {
    expect(reveal({ top: 100, bottom: 140 })).toBeNull();
  });

  test("does not scroll when the range is partly visible", () => {
    expect(reveal({ top: 820, bottom: 900 })).toBeNull();
  });

  test("puts the range one third down the visible part when it is below", () => {
    // 30 + 810 / 3 = 300 の位置に、上端が来る
    expect(reveal({ top: 1200, bottom: 1240 })).toBe(1000 + 1200 - 300);
  });

  test("puts the range one third down the visible part when it is above", () => {
    expect(reveal({ top: -500, bottom: -460 })).toBe(1000 - 500 - 300);
  });

  test("treats a range hidden behind the top bar as not visible", () => {
    expect(reveal({ top: 0, bottom: 30 })).not.toBeNull();
  });

  test("treats a range hidden behind the bottom bar as not visible", () => {
    expect(reveal({ top: 840, bottom: 880 })).not.toBeNull();
  });
});
