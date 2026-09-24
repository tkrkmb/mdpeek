import { describe, expect, test } from "vitest";

import { accumulateSwipe, isAtScrollEdge, isHorizontalSwipe } from "./navigation";

describe("isHorizontalSwipe", () => {
  test("accepts a horizontally-dominant move", () => {
    expect(isHorizontalSwipe(20, 3)).toBe(true);
  });

  test("rejects a vertically-dominant move", () => {
    expect(isHorizontalSwipe(3, 20)).toBe(false);
  });

  test("rejects an exact tie", () => {
    expect(isHorizontalSwipe(5, 5)).toBe(false);
  });
});

describe("isAtScrollEdge", () => {
  test("is at the left edge when scrollLeft is 0", () => {
    expect(isAtScrollEdge("left", 0, 500, 200)).toBe(true);
  });

  test("is not at the left edge when scrolled right", () => {
    expect(isAtScrollEdge("left", 50, 500, 200)).toBe(false);
  });

  test("is at the right edge when scrolled to the end", () => {
    expect(isAtScrollEdge("right", 300, 500, 200)).toBe(true);
  });

  test("is not at the right edge when there is more to scroll", () => {
    expect(isAtScrollEdge("right", 100, 500, 200)).toBe(false);
  });
});

describe("accumulateSwipe", () => {
  test("keeps accumulating below the threshold", () => {
    const result = accumulateSwipe(0, -30, 80);
    expect(result).toEqual({ accumulated: -30, direction: null });
  });

  test("fires back once the negative threshold is crossed", () => {
    const result = accumulateSwipe(-60, -30, 80);
    expect(result).toEqual({ accumulated: 0, direction: "back" });
  });

  test("fires forward once the positive threshold is crossed", () => {
    const result = accumulateSwipe(60, 30, 80);
    expect(result).toEqual({ accumulated: 0, direction: "forward" });
  });

  test("resets after firing so the next swipe starts fresh", () => {
    const fired = accumulateSwipe(-60, -30, 80);
    const next = accumulateSwipe(fired.accumulated, -10, 80);
    expect(next).toEqual({ accumulated: -10, direction: null });
  });
});
