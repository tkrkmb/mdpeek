import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";

import { flash, setProblem } from "./notice";

function notice(): HTMLElement {
  return document.getElementById("mdsight-notice")!;
}

beforeEach(() => {
  vi.useFakeTimers();
  document.body.innerHTML = '<div id="mdsight-notice" hidden></div>';
  setProblem(null);
});

afterEach(() => {
  vi.runAllTimers();
  vi.useRealTimers();
});

describe("notice", () => {
  test("stays hidden when there is nothing to say", () => {
    expect(notice().hidden).toBe(true);
  });

  test("keeps a problem until it is cleared", () => {
    setProblem("cannot reload");
    vi.advanceTimersByTime(60_000);
    expect(notice().hidden).toBe(false);
    expect(notice().textContent).toBe("cannot reload");

    setProblem(null);
    expect(notice().hidden).toBe(true);
  });

  test("shows a flash for a few seconds only", () => {
    flash("cannot open the link");
    expect(notice().textContent).toBe("cannot open the link");
    vi.advanceTimersByTime(5_000);
    expect(notice().hidden).toBe(true);
  });

  test("returns to the standing problem after a flash", () => {
    setProblem("cannot reload");
    flash("cannot open the link");
    expect(notice().textContent).toBe("cannot open the link");
    vi.advanceTimersByTime(5_000);
    expect(notice().textContent).toBe("cannot reload");
  });
});
