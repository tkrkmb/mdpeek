/** 画面上の範囲 */
export type VerticalRect = {
  top: number;
  bottom: number;
};

/**
 * 範囲が、上下の帯に隠れていない部分に見えていなければ、範囲の上端が見える部分の上から1/3の位置に
 * 来るためのスクロール位置を返す。見えていれば null
 */
export function revealTop(
  rect: VerticalRect,
  scrollY: number,
  viewportHeight: number,
  topInset: number,
  bottomInset: number,
): number | null {
  const bottom = viewportHeight - bottomInset;
  if (rect.bottom > topInset && rect.top < bottom) {
    return null;
  }
  return scrollY + rect.top - (topInset + (bottom - topInset) / 3);
}

/**
 * 範囲が画面内に見えていなければ、上から1/3の位置に来るようにスクロールする。
 * パスの帯（`topInset`）と下端の検索の帯（`bottomInset`）に隠れた部分は、見えていないものとして扱う
 */
export function revealRect(rect: VerticalRect, topInset: number, bottomInset: number): void {
  const top = revealTop(rect, window.scrollY, window.innerHeight, topInset, bottomInset);
  if (top !== null) {
    window.scrollTo({ top, behavior: "instant" });
  }
}
