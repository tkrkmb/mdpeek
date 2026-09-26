import { bottomInset, closeFind, isTypingInFind, openFind, stepFind } from "./find";
import { navigateHistory } from "./history";
import { topInset } from "./pathbar";
import { cycle } from "./theme";

export const isMac = navigator.userAgent.includes("Macintosh");

/** 修飾クリック（macOSは `metaKey`、Linuxは `ctrlKey`）か */
export function isModifiedClick(event: MouseEvent): boolean {
  return isMac ? event.metaKey : event.ctrlKey;
}

function isBack(event: KeyboardEvent): boolean {
  return isMac ? event.metaKey && event.key === "[" : event.altKey && event.key === "ArrowLeft";
}

function isForward(event: KeyboardEvent): boolean {
  return isMac ? event.metaKey && event.key === "]" : event.altKey && event.key === "ArrowRight";
}

function isFind(event: KeyboardEvent): boolean {
  return (isMac ? event.metaKey : event.ctrlKey) && event.key.toLowerCase() === "f";
}

/** j／k で動かす量（本文の1行の高さ 24px の3行分） */
const LINE_STEP = 72;
/** gg と数える、2回目の g までの時間 */
const DOUBLE_G_MS = 1000;
/** 直前に g を押した時刻（gg の1回目） */
let lastG = 0;

function scrollByInstant(top: number): void {
  window.scrollBy({ top, behavior: "instant" });
}

/** パスの帯と検索の帯を除いた、見えている高さの半分 */
function halfPage(): number {
  return (window.innerHeight - topInset() - bottomInset()) / 2;
}

/**
 * Vim風のキー操作。プレビューのスクロールと検索だけを動かし、Neovimのカーソルは動かさない。
 * 扱ったら true を返す
 */
function handleVimKey(event: KeyboardEvent): boolean {
  const onlyCtrl = event.ctrlKey && !event.metaKey && !event.altKey;
  if (onlyCtrl && (event.key === "d" || event.key === "u")) {
    scrollByInstant(event.key === "d" ? halfPage() : -halfPage());
    return true;
  }
  if (event.ctrlKey || event.metaKey || event.altKey) {
    return false;
  }
  const previousG = lastG;
  lastG = 0;
  switch (event.key) {
    case "j":
      scrollByInstant(LINE_STEP);
      return true;
    case "k":
      scrollByInstant(-LINE_STEP);
      return true;
    case "g":
      if (event.timeStamp - previousG <= DOUBLE_G_MS && previousG !== 0) {
        window.scrollTo({ top: 0, behavior: "instant" });
      } else {
        lastG = event.timeStamp;
      }
      return true;
    case "G":
      window.scrollTo({ top: document.documentElement.scrollHeight, behavior: "instant" });
      return true;
    case "/":
      openFind();
      return true;
    case "n":
      void stepFind(1);
      return true;
    case "N":
      void stepFind(-1);
      return true;
    default:
      return false;
  }
}

/** キー入力を、検索・戻る／進む・Vim風の操作・テーマの切り替えに振り分ける */
export function initKeys(): void {
  document.addEventListener("keydown", (event) => {
    if (isFind(event)) {
      event.preventDefault();
      openFind();
      return;
    }
    if (event.key === "Escape") {
      closeFind();
      return;
    }
    if (isBack(event)) {
      event.preventDefault();
      void navigateHistory("go_back");
      return;
    }
    if (isForward(event)) {
      event.preventDefault();
      void navigateHistory("go_forward");
      return;
    }
    // 検索窓に入力している間は、どのキーも文字として入力する
    if (isTypingInFind(event)) {
      return;
    }
    if (handleVimKey(event)) {
      event.preventDefault();
      return;
    }
    if (event.ctrlKey || event.metaKey || event.altKey) {
      return;
    }
    if (event.key.toLowerCase() === "t") {
      cycle();
    }
  });
}
