import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";

import type { MatchRequest, MatchResponse } from "./matchworker";
import { vimMatches } from "./vimregex";

/** テスト用の Worker。`respond` が true なら本物と同じ計算をして返し、false なら返さない（固まった相手） */
class FakeWorker {
  static respond = true;
  static created = 0;
  static terminated = 0;
  onmessage: ((event: MessageEvent<MatchResponse>) => void) | null = null;

  constructor() {
    FakeWorker.created += 1;
  }

  postMessage(request: MatchRequest): void {
    if (!FakeWorker.respond) {
      return;
    }
    const regex = new RegExp(request.source, request.flags);
    const ranges = request.texts.map((text) => vimMatches(text, regex));
    queueMicrotask(() => this.onmessage?.({ data: { id: request.id, ranges } } as MessageEvent<MatchResponse>));
  }

  terminate(): void {
    FakeWorker.terminated += 1;
  }
}

beforeEach(() => {
  vi.resetModules();
  vi.useFakeTimers();
  vi.stubGlobal("Worker", FakeWorker);
  FakeWorker.respond = true;
  FakeWorker.created = 0;
  FakeWorker.terminated = 0;
});

afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllGlobals();
});

describe("findMatches", () => {
  test("returns the ranges for each text", async () => {
    const { findMatches } = await import("./matcher");
    const result = await findMatches(/b/dgu, ["abc", "xyz", "bb"]);
    expect(result).toEqual({
      kind: "done",
      ranges: [[{ start: 1, end: 2 }], [], [{ start: 0, end: 1 }, { start: 1, end: 2 }]],
    });
  });

  test("gives up and throws the worker away when it takes too long", async () => {
    const { findMatches, MATCH_TIMEOUT_MS } = await import("./matcher");
    FakeWorker.respond = false;
    const pending = findMatches(/.*.*.*x/dgu, ["a".repeat(800)]);
    await vi.advanceTimersByTimeAsync(MATCH_TIMEOUT_MS);
    expect(await pending).toEqual({ kind: "timeout" });
    expect(FakeWorker.terminated).toBe(1);

    // 次の検索は、新しい Worker で動く
    FakeWorker.respond = true;
    expect((await findMatches(/a/dgu, ["a"])).kind).toBe("done");
    expect(FakeWorker.created).toBe(2);
  });

  test("stops a running search when the next one starts", async () => {
    const { findMatches } = await import("./matcher");
    FakeWorker.respond = false;
    const first = findMatches(/.*.*.*x/dgu, ["a".repeat(800)]);
    FakeWorker.respond = true;
    const second = findMatches(/a/dgu, ["a"]);
    expect(await first).toEqual({ kind: "superseded" });
    expect((await second).kind).toBe("done");
    expect(FakeWorker.terminated).toBe(1);
  });

  test("cancels a running search", async () => {
    const { cancelMatches, findMatches } = await import("./matcher");
    FakeWorker.respond = false;
    const pending = findMatches(/.*.*.*x/dgu, ["a".repeat(800)]);
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
});
