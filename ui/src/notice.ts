/**
 * 本文の左下に出す知らせの帯。
 * Rustから知らされた問題は取り消されるまで出し続け、一時的な知らせ（リンクを開けなかった等）は
 * 数秒だけ問題の上に重ねて出す。消えたあとは、残っている問題の表示に戻る。
 */
const FLASH_MS = 4000;

let problem: string | null = null;
let flashText: string | null = null;
let flashTimer: ReturnType<typeof setTimeout> | undefined;

function element(): HTMLElement | null {
  return document.getElementById("mdsight-notice");
}

function update(): void {
  const target = element();
  if (target === null) {
    return;
  }
  const text = flashText ?? problem;
  target.textContent = text ?? "";
  target.hidden = text === null;
}

/** 取り消されるまで出し続ける問題を設定する（`null` で取り消す） */
export function setProblem(text: string | null): void {
  problem = text;
  update();
}

/** 数秒だけ知らせる */
export function flash(text: string): void {
  flashText = text;
  update();
  clearTimeout(flashTimer);
  flashTimer = setTimeout(() => {
    flashText = null;
    update();
  }, FLASH_MS);
}
