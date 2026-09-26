import type { MatchRequest, MatchResponse } from "./matchworker";
import type { Range } from "./search";

/** 探すのにこれより長くかかったら、打ち切る */
export const MATCH_TIMEOUT_MS = 1000;

/**
 * - `done`：ブロックごとの一致の範囲
 * - `timeout`：時間がかかりすぎたので打ち切った
 * - `superseded`：終わる前に、次の検索が始まった
 */
export type MatchResult = { kind: "done"; ranges: Range[][] } | { kind: "timeout" } | { kind: "superseded" };

let worker: Worker | null = null;
let nextId = 0;
/** 探している最中の検索。次の検索が始まったら、結果を待たずに打ち切る */
let running: { id: number; finish: (result: MatchResult) => void } | null = null;

function spawn(): Worker {
  const created = new Worker(new URL("./matchworker.ts", import.meta.url), { type: "module" });
  created.onmessage = (event: MessageEvent<MatchResponse>) => {
    if (running !== null && running.id === event.data.id) {
      running.finish({ kind: "done", ranges: event.data.ranges });
    }
  };
  return created;
}

/** 探している最中の検索を止める。止まらない正規表現もあるので、スレッドごと捨てる */
function abort(result: MatchResult): void {
  if (running === null) {
    return;
  }
  running.finish(result);
  worker?.terminate();
  worker = null;
}

/** 探している最中の検索があれば、結果を待たずに止める（検索の帯を閉じたときなど） */
export function cancelMatches(): void {
  abort({ kind: "superseded" });
}

/** それぞれの文字列から、正規表現の一致の範囲を、画面を止めずに探す */
export function findMatches(regex: RegExp, texts: string[]): Promise<MatchResult> {
  abort({ kind: "superseded" });
  worker ??= spawn();
  const id = ++nextId;
  return new Promise((resolve) => {
    const timer = setTimeout(() => abort({ kind: "timeout" }), MATCH_TIMEOUT_MS);
    running = {
      id,
      finish: (result) => {
        clearTimeout(timer);
        running = null;
        resolve(result);
      },
    };
    worker!.postMessage({ id, source: regex.source, flags: regex.flags, texts } satisfies MatchRequest);
  });
}
