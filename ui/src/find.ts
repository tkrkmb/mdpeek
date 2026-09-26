import { topInset } from "./pathbar";
import { revealRect } from "./scroll";
import { cancelMatches, findMatches } from "./matcher";
import { clearMarks, collectText, markRanges } from "./search";
import { compileVimPattern } from "./vimregex";

/** ページ内検索の一致と、現在の一致の `<mark>` のクラス（検索窓の `.mdsight-find` と重ならない名前にする） */
const HIT = "mdsight-hit";
const CURRENT = "mdsight-hit-current";

let root: HTMLElement | null = null;
let box: HTMLElement | null = null;
let input: HTMLInputElement | null = null;
let count: HTMLElement | null = null;
let previousButton: HTMLButtonElement | null = null;
let nextButton: HTMLButtonElement | null = null;
/** 一致ごとの `<mark>`（要素をまたぐ一致は複数） */
let matches: HTMLElement[][] = [];
let current = -1;
/** 検索語が正しくないか、対応しない書き方を含む */
let invalid = false;
/** 探すのに時間がかかりすぎたので、打ち切った */
let timedOut = false;
/** いちばん新しい検索の番号。結果が届いたときに、これより古い検索のものなら捨てる */
let latestSearch = 0;
/** 閉じる前に現在だった一致の順番。閉じた後の n／N で、ここから進める */
let resumeAt = -1;

function isOpen(): boolean {
  return box !== null && !box.hidden;
}

function showCount(): void {
  if (previousButton !== null && nextButton !== null) {
    previousButton.disabled = matches.length === 0;
    nextButton.disabled = matches.length === 0;
  }
  if (count === null || input === null) {
    return;
  }
  if (input.value === "") {
    count.textContent = "";
  } else if (invalid) {
    count.textContent = "Invalid pattern";
  } else if (timedOut) {
    count.textContent = "Timed out";
  } else if (matches.length === 0) {
    count.textContent = "No results";
  } else {
    count.textContent = `${current + 1}/${matches.length}`;
  }
}

/** 窓の下端の検索の帯の高さ。帯に隠れた部分は、画面に見えていないものとして扱う */
export function bottomInset(): number {
  return box !== null && !box.hidden ? box.offsetHeight : 0;
}

/** 現在の一致が画面内に見えていなければ、上から1/3の位置に来るようにスクロールする */
function reveal(marks: HTMLElement[]): void {
  const first = marks[0];
  if (first === undefined) {
    return;
  }
  revealRect(first.getBoundingClientRect(), topInset(), bottomInset());
}

function setCurrent(index: number, scroll: boolean): void {
  for (const mark of matches[current] ?? []) {
    mark.classList.remove(CURRENT);
  }
  current = index;
  for (const mark of matches[current] ?? []) {
    mark.classList.add(CURRENT);
  }
  // 閉じた details の中の一致なら、見えるように開く
  for (let details = matches[current]?.[0]?.closest("details") ?? null; details !== null; details = details.parentElement?.closest("details") ?? null) {
    details.open = true;
  }
  showCount();
  if (scroll) {
    reveal(matches[current] ?? []);
  }
}

/** 強調を消し、一致が無い状態にする */
function clearMatches(): void {
  if (root !== null) {
    clearMarks(root, HIT);
  }
  matches = [];
  current = -1;
}

/**
 * 入力欄の文字列で探し直す。`keep` のときは現在の一致の順番を保ち（件数を超えたら最後）、
 * スクロールしない。そうでなければ、最初の一致を現在の一致にする。
 * 一致は別のスレッドで探すので、画面は止まらない。結果が届くまでは、前の強調を残す（ちらつかないように）
 */
async function search(keep: boolean): Promise<void> {
  if (root === null || input === null) {
    return;
  }
  const searching = ++latestSearch;
  const previous = current;
  const regex = isOpen() && input.value !== "" ? compileVimPattern(input.value) : null;
  invalid = isOpen() && input.value !== "" && regex === null;
  if (regex === null) {
    cancelMatches();
    timedOut = false;
    clearMatches();
    showCount();
    return;
  }
  // 本文が差し替わっていたら、前の強調はもう画面に無いので、移動の対象にしない
  if (matches.some((marks) => marks[0]?.isConnected !== true)) {
    matches = [];
  }
  // ブロックごとに探し、ブロックをまたいでは一致させない
  const blocks = Array.from(root.children);
  const texts = blocks.map((block) => collectText(block).text);
  const result = await findMatches(regex, texts);
  if (searching !== latestSearch || result.kind === "superseded") {
    return;
  }
  clearMatches();
  timedOut = result.kind === "timeout";
  if (result.kind === "done") {
    blocks.forEach((block, position) => {
      // 探している間に本文が変わったブロックは、範囲がずれるので強調しない
      const index = collectText(block);
      if (block.parentElement === root && index.text === texts[position]) {
        matches.push(...markRanges(index, result.ranges[position], HIT));
      }
    });
  }
  if (matches.length === 0) {
    showCount();
    return;
  }
  if (keep) {
    setCurrent(Math.min(Math.max(previous, 0), matches.length - 1), false);
  } else {
    setCurrent(0, true);
  }
}

/** 次（`step` = 1）／前（`step` = -1）の一致へ移る。端では反対の端に戻る */
function move(step: number): void {
  if (matches.length === 0) {
    return;
  }
  setCurrent((current + step + matches.length) % matches.length, true);
}

/** 検索の帯を開閉する。開いている間は、本文の末尾を帯の分だけ延ばす（CSSの `mdsight-finding`） */
function toggleBox(open: boolean): void {
  if (box === null) {
    return;
  }
  box.hidden = !open;
  document.documentElement.classList.toggle("mdsight-finding", open);
}

export function openFind(): void {
  if (box === null || input === null) {
    return;
  }
  if (!isOpen()) {
    toggleBox(true);
    void search(false);
  }
  input.focus();
  input.select();
}

export function closeFind(): void {
  if (box === null || input === null || !isOpen()) {
    return;
  }
  resumeAt = current;
  toggleBox(false);
  input.blur();
  void search(false);
}

/**
 * n／N：次（`step` = 1）／前（`step` = -1）の一致へ移る。検索の帯を閉じていても、最後の検索語があれば、
 * 帯を開き直して（入力欄にはフォーカスを移さない）、閉じる前の一致から進める
 */
export async function stepFind(step: number): Promise<void> {
  if (input === null) {
    return;
  }
  if (!isOpen()) {
    if (input.value === "") {
      return;
    }
    toggleBox(true);
    current = resumeAt;
    await search(true);
  }
  move(step);
}

/** 本文を差し替えた後に呼ぶ。検索窓が開いていれば、同じ文字列で探し直す */
export function refreshFind(): void {
  if (isOpen()) {
    void search(true);
  }
}

/** 検索窓の入力欄に入力しているか（`T` キーでテーマを切り替えないため） */
export function isTypingInFind(event: KeyboardEvent): boolean {
  return input !== null && event.target === input;
}

export function initFind(body: HTMLElement): void {
  root = body;
  box = document.getElementById("mdsight-find");
  input = box?.querySelector<HTMLInputElement>("input") ?? null;
  count = box?.querySelector<HTMLElement>(".mdsight-find-count") ?? null;
  previousButton = box?.querySelector<HTMLButtonElement>(".mdsight-find-prev") ?? null;
  nextButton = box?.querySelector<HTMLButtonElement>(".mdsight-find-next") ?? null;
  if (input === null) {
    return;
  }
  const field = input;
  // ボタンで移った後も、続けて入力や Enter ができるように、入力欄にフォーカスを戻す
  previousButton?.addEventListener("click", () => {
    move(-1);
    field.focus();
  });
  nextButton?.addEventListener("click", () => {
    move(1);
    field.focus();
  });
  box?.querySelector(".mdsight-find-close")?.addEventListener("click", closeFind);
  input.addEventListener("input", () => void search(false));
  input.addEventListener("keydown", (event) => {
    // 日本語入力の変換を確定する Enter では移らない
    if (event.isComposing) {
      return;
    }
    if (event.key === "Enter") {
      event.preventDefault();
      move(event.shiftKey ? -1 : 1);
    } else if (event.key === "Escape") {
      event.preventDefault();
      closeFind();
    }
  });
}
