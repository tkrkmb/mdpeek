import { topInset } from "./pathbar";
import { clearMarks, collectText, markRanges } from "./search";
import { compileVimPattern, vimMatches } from "./vimregex";

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
  const inset = topInset();
  const bottom = window.innerHeight - bottomInset();
  const rect = first.getBoundingClientRect();
  if (rect.bottom > inset && rect.top < bottom) {
    return;
  }
  const top = window.scrollY + rect.top - (inset + (bottom - inset) / 3);
  window.scrollTo({ top, behavior: "instant" });
}

function setCurrent(index: number, scroll: boolean): void {
  for (const mark of matches[current] ?? []) {
    mark.classList.remove(CURRENT);
  }
  current = index;
  for (const mark of matches[current] ?? []) {
    mark.classList.add(CURRENT);
  }
  showCount();
  if (scroll) {
    reveal(matches[current] ?? []);
  }
}

/**
 * 入力欄の文字列で探し直す。`keep` のときは現在の一致の順番を保ち（件数を超えたら最後）、
 * スクロールしない。そうでなければ、最初の一致を現在の一致にする
 */
function search(keep: boolean): void {
  if (root === null || input === null) {
    return;
  }
  const previous = current;
  clearMarks(root, HIT);
  matches = [];
  current = -1;
  invalid = false;
  if (isOpen() && input.value !== "") {
    const regex = compileVimPattern(input.value);
    if (regex === null) {
      invalid = true;
    } else {
      // ブロックごとに探し、ブロックをまたいでは一致させない
      for (const block of Array.from(root.children)) {
        const index = collectText(block);
        matches.push(...markRanges(index, vimMatches(index.text, regex), HIT));
      }
    }
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
    search(false);
  }
  input.focus();
  input.select();
}

export function closeFind(): void {
  if (box === null || input === null || !isOpen()) {
    return;
  }
  toggleBox(false);
  input.blur();
  search(false);
}

/** 本文を差し替えた後に呼ぶ。検索窓が開いていれば、同じ文字列で探し直す */
export function refreshFind(): void {
  if (isOpen()) {
    search(true);
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
  input.addEventListener("input", () => search(false));
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
