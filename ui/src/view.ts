import { buildTable, type Block } from "./sourcepos";

/** 本文を差し替える先 */
export const body = document.querySelector<HTMLElement>(".markdown-body")!;

/** 表示中の文書の状態。他のモジュールは、使うときに読む（値を控えておかない） */
export const view = {
  gen: -1,
  version: -1,
  table: [] as Block[],
  /** 描画（画像と図を含む）が終わるまで true */
  rendering: false,
};

export function isCurrent(version: number): () => boolean {
  return () => version === view.version;
}

export function rebuildTable(): void {
  view.table = buildTable(body);
}
