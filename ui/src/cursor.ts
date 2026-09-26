import { followCursor } from "./position";
import { view } from "./view";

/** 描画を待っている間に届いた、いちばん新しいカーソル（世代付き） */
let pending: { gen: number; line: number } | null = null;
/** 戻る／進むで位置を戻した世代。その世代で最初に届くカーソル行には追従しない */
let skipCursorGen: number | null = null;
/** 最後にカーソル行を受け取った世代 */
let lastCursorGen = -1;

export function receiveCursor(gen: number, line: number): void {
  // 古い対象のものは捨てる
  if (gen < view.gen) {
    return;
  }
  const first = gen !== lastCursorGen;
  lastCursorGen = gen;
  // 戻した位置を、Neovimのカーソル位置で上書きしない
  if (first && gen === skipCursorGen) {
    skipCursorGen = null;
    return;
  }
  if (view.rendering || gen !== view.gen) {
    // 最新のものだけを保持し、その世代の描画が終わってから適用する
    pending = { gen, line };
    return;
  }
  followCursor(line);
}

/** 描画が終わったら呼ぶ。待っていたカーソル行があれば、追従する */
export function applyPendingCursor(): void {
  if (pending === null || pending.gen !== view.gen) {
    pending = null;
    return;
  }
  const line = pending.line;
  pending = null;
  followCursor(line);
}

/**
 * 位置を戻す世代で、最初に届くカーソル行に追従しないようにする。
 * カーソル行はコマンドの結果より先に届いていることがあるので、そのときは待っているものを捨てる
 */
export function skipFirstCursor(gen: number): void {
  if (lastCursorGen !== gen) {
    skipCursorGen = gen;
  } else if (pending !== null && pending.gen === gen) {
    pending = null;
  }
}
