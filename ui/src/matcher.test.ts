import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";

import type { MatchRequest, MatchResponse } from "./matchworker";
import { vimMatches } from "./vimregex";

/**
 * テスト用の Worker。本物と同じく、ブロックを1つ探し終えるたびに結果を返す。
 * 文字列に `HANG` を含むブロックでは、探し終わらない（止まらない正規表現のかわり）
 */
class FakeWorker {
  static created = 0;
  static terminated = 0;
  onmessage: ((event: MessageEvent<MatchResponse>) => void) | null = null;
  private alive = true;

  constructor() {
    FakeWorker.created += 1;
  }

  postMessage(request: MatchRequest): void {
    const regex = new RegExp(request.source, request.flags);
    void (async () => {
      for (let index = request.start; index < request.texts.length; index += 1) {
        const text = request.texts[index];
        if (text.includes("HANG")) {
          return;
        }
        await Promise.resolve();
        if (!this.alive) {
          return;
        }
        this.onmessage?.({ data: { id: request.id, index, ranges: vimMatches(text, regex) } } as unknown as MessageEvent<MatchResponse>);
      }
    })();
  }

  terminate(): void {
    this.alive = false;
    FakeWorker.terminated += 1;
  }
}

beforeEach(() => {
  vi.resetModules();
  vi.useFakeTimers();
  vi.stubGlobal("Worker", FakeWorker);
  FakeWorker.created = 0;
  FakeWorker.terminated = 0;
});

afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllGlobals();
});

describe("findMatches", () => {
  test("returns the ranges for each block", async () => {
    const { findMatches } = await import("./matcher");
    const result = await findMatches(/b/dgu, ["abc", "xyz", "bb"]);
    expect(result).toEqual({
      kind: "done",
      ranges: [[{ start: 1, end: 2 }], [], [{ start: 0, end: 1 }, { start: 1, end: 2 }]],
    });
    expect(FakeWorker.terminated).toBe(0);
  });

  test("skips only the block that takes too long and keeps the others", async () => {
    const { findMatches, BLOCK_TIMEOUT_MS } = await import("./matcher");
    const pending = findMatches(/b/dgu, ["ab", "HANG b", "b"]);
    await vi.advanceTimersByTimeAsync(BLOCK_TIMEOUT_MS);
    expect(await pending).toEqual({
      kind: "done",
      ranges: [[{ start: 1, end: 2 }], null, [{ start: 0, end: 1 }]],
    });
    // 固まったスレッドは捨て、続きは新しいスレッドで探す
    expect(FakeWorker.terminated).toBe(1);
    expect(FakeWorker.created).toBe(2);
  });

  test("gives up the rest once the whole search runs out of time", async () => {
    const { findMatches, BLOCK_TIMEOUT_MS, SEARCH_TIMEOUT_MS } = await import("./matcher");
    const texts = [...Array.from({ length: 10 }, () => "HANG"), "b"];
    let settled = false;
    const pending = findMatches(/b/dgu, texts).then((result) => {
      settled = true;
      return result;
    });
    await vi.advanceTimersByTimeAsync(SEARCH_TIMEOUT_MS - 1);
    expect(settled).toBe(false);
    await vi.advanceTimersByTimeAsync(1);
    const result = await pending;
    // 時間内に試せたブロックも、試せなかったブロックも、飛ばしたものとして返す
    expect(result).toEqual({ kind: "done", ranges: texts.map(() => null) });
    expect(FakeWorker.terminated).toBe(Math.ceil(SEARCH_TIMEOUT_MS / BLOCK_TIMEOUT_MS));
  });

  test("stops a running search when the next one starts", async () => {
    const { findMatches } = await import("./matcher");
    const first = findMatches(/b/dgu, ["HANG"]);
    const second = findMatches(/a/dgu, ["a"]);
    expect(await first).toEqual({ kind: "superseded" });
    expect((await second).kind).toBe("done");
    expect(FakeWorker.terminated).toBe(1);
  });

  test("cancels a running search", async () => {
    const { cancelMatches, findMatches } = await import("./matcher");
    const pending = findMatches(/b/dgu, ["HANG"]);
    cancelMatches();
    expect(await pending).toEqual({ kind: "superseded" });
  });

  test("keeps the worker when nothing is running", async () => {
    const { cancelMatches, findMatches } = await import("./matcher");
    await findMatches(/a/dgu, ["a"]);
    cancelMatches();
    await findMatches(/a/dgu, ["a"]);
    expect(FakeWorker.created).toBe(1);
    expect(FakeWorker.terminated).toBe(0);
  });

  test("finishes at once when there is nothing to search", async () => {
    const { findMatches } = await import("./matcher");
    expect(await findMatches(/a/dgu, [])).toEqual({ kind: "done", ranges: [] });
  });
});
