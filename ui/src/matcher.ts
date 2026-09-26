import type { MatchRequest, MatchResponse } from "./matchworker";
import type { Range } from "./search";

/** 1つのブロックを探すのにこれより長くかかったら、そのブロックは飛ばす */
export const BLOCK_TIMEOUT_MS = 300;
/** 全体でこれより長くかかったら、まだ探していないブロックも飛ばす（飛ばすブロックが多くても、待たせすぎないように） */
export const SEARCH_TIMEOUT_MS = 2000;

/**
 * - `done`：ブロックごとの一致の範囲。時間がかかって飛ばしたブロックは `null`
 * - `superseded`：終わる前に、次の検索が始まった（または止められた）
 */
export type MatchResult = { kind: "done"; ranges: (Range[] | null)[] } | { kind: "superseded" };

let worker: Worker | null = null;
let nextId = 0;
/** 探している最中の検索。次の検索が始まったら、結果を待たずに打ち切る */
let running: {
  id: number;
  receive: (index: number, ranges: Range[]) => void;
  finish: (result: MatchResult) => void;
} | null = null;

function spawn(): Worker {
  const created = new Worker(new URL("./matchworker.ts", import.meta.url), { type: "module" });
  created.onmessage = (event: MessageEvent<MatchResponse>) => {
    // 捨てたスレッドから、捨てる前に届いていた結果は使わない
    if (created === worker && running !== null && running.id === event.data.id) {
      running.receive(event.data.index, event.data.ranges);
    }
  };
  return created;
}

/** 止まらない正規表現もあるので、スレッドごと捨てる */
function discardWorker(): void {
  worker?.terminate();
  worker = null;
}

/** 探している最中の検索があれば、結果を待たずに止める（検索の帯を閉じたときなど） */
export function cancelMatches(): void {
  if (running !== null) {
    running.finish({ kind: "superseded" });
    discardWorker();
  }
}

/**
 * それぞれの文字列（ブロック）から、正規表現の一致の範囲を、画面を止めずに探す。
 * 時間のかかったブロックは飛ばし、ほかのブロックの一致は返す
 */
export function findMatches(regex: RegExp, texts: string[]): Promise<MatchResult> {
  cancelMatches();
  const id = ++nextId;
  const ranges: (Range[] | null)[] = texts.map(() => null);
  const deadline = Date.now() + SEARCH_TIMEOUT_MS;
  return new Promise((resolve) => {
    let timer: ReturnType<typeof setTimeout> | undefined;
    /** いま探しているブロックの番号 */
    let next = 0;

    const finish = (result: MatchResult) => {
      clearTimeout(timer);
      running = null;
      resolve(result);
    };
    const done = () => finish({ kind: "done", ranges });

    /** いまのブロックに使える時間を計り直す。全体の残り時間を超えない */
    const arm = () => {
      clearTimeout(timer);
      timer = setTimeout(giveUp, Math.max(0, Math.min(BLOCK_TIMEOUT_MS, deadline - Date.now())));
    };

    /** 次に探すブロックから、新しいスレッドで探す */
    const startFrom = (index: number) => {
      next = index;
      if (next >= texts.length) {
        done();
        return;
      }
      worker ??= spawn();
      worker.postMessage({ id, source: regex.source, flags: regex.flags, texts, start: next } satisfies MatchRequest);
      arm();
    };

    /** いまのブロックに時間がかかりすぎた。そのブロックを飛ばして続ける。全体の時間を使い切ったら、残りも飛ばす */
    const giveUp = () => {
      discardWorker();
      if (Date.now() >= deadline) {
        done();
      } else {
        startFrom(next + 1);
      }
    };

    running = {
      id,
      receive: (index, found) => {
        if (index !== next) {
          return;
        }
        ranges[index] = found;
        next = index + 1;
        if (next >= texts.length) {
          done();
        } else {
          arm();
        }
      },
      finish,
    };
    startFrom(0);
  });
}
