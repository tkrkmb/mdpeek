/// <reference lib="webworker" />
/**
 * ページ内検索の一致を、画面とは別のスレッドで探す。
 * 検索語によっては探すのに長い時間がかかるので、画面を止めず、呼び出し側が打ち切れるようにする。
 * ブロックを1つ探し終えるたびに結果を返すので、呼び出し側は、時間のかかったブロックだけを飛ばせる
 */
import { vimMatches } from "./vimregex";

export type MatchRequest = {
  id: number;
  source: string;
  flags: string;
  texts: string[];
  /** この番号のブロックから探す（前のスレッドで時間のかかったブロックを飛ばして、続きから探すため） */
  start: number;
};

export type MatchResponse = {
  id: number;
  index: number;
  ranges: { start: number; end: number }[];
};

self.onmessage = (event: MessageEvent<MatchRequest>) => {
  const { id, source, flags, texts, start } = event.data;
  const regex = new RegExp(source, flags);
  for (let index = start; index < texts.length; index += 1) {
    self.postMessage({ id, index, ranges: vimMatches(texts[index], regex) } satisfies MatchResponse);
  }
};
