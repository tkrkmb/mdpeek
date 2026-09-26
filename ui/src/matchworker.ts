/// <reference lib="webworker" />
/**
 * ページ内検索の一致を、画面とは別のスレッドで探す。
 * 検索語によっては探すのに長い時間がかかるので、画面を止めず、呼び出し側が打ち切れるようにする
 */
import { vimMatches } from "./vimregex";

export type MatchRequest = {
  id: number;
  source: string;
  flags: string;
  texts: string[];
};

export type MatchResponse = {
  id: number;
  ranges: { start: number; end: number }[][];
};

self.onmessage = (event: MessageEvent<MatchRequest>) => {
  const { id, source, flags, texts } = event.data;
  const regex = new RegExp(source, flags);
  const ranges = texts.map((text) => vimMatches(text, regex));
  self.postMessage({ id, ranges } satisfies MatchResponse);
};
