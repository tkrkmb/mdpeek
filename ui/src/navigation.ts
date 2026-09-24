/** 横方向が優勢なホイールイベントか(トラックパッドの横スワイプ) */
export function isHorizontalSwipe(deltaX: number, deltaY: number): boolean {
  return Math.abs(deltaX) > Math.abs(deltaY);
}

/** 要素が、指定した向きへこれ以上スクロールできない(端に達している)か */
export function isAtScrollEdge(
  direction: "left" | "right",
  scrollLeft: number,
  scrollWidth: number,
  clientWidth: number,
): boolean {
  if (direction === "left") {
    return scrollLeft <= 0;
  }
  return scrollLeft >= scrollWidth - clientWidth - 1;
}

export type SwipeDirection = "back" | "forward" | null;

/**
 * 累積した横方向の移動量に deltaX を足し、しきい値を超えたら戻る／進むを返す。
 * 指を右へ動かす(自然なスクロールでは deltaX が負になることが多い)と「戻る」、
 * 左へ動かすと「進む」を想定している。実機で逆に感じたら、下の2つの比較の
 * 不等号(`<=` と `>=`)を入れ替える。
 */
export function accumulateSwipe(
  accumulated: number,
  deltaX: number,
  threshold: number,
): { accumulated: number; direction: SwipeDirection } {
  const next = accumulated + deltaX;
  if (next <= -threshold) {
    return { accumulated: 0, direction: "back" };
  }
  if (next >= threshold) {
    return { accumulated: 0, direction: "forward" };
  }
  return { accumulated: next, direction: null };
}

/** クリックされた位置の祖先から、横スクロールできる最初の要素を探す */
function scrollableAncestor(target: Element): Element | null {
  for (let element: Element | null = target; element !== null; element = element.parentElement) {
    if (element.scrollWidth > element.clientWidth) {
      return element;
    }
  }
  return null;
}

const SWIPE_THRESHOLD = 80;
const SWIPE_COOLDOWN_MS = 600;

/** 戻る／進むボタンの配線。返り値で、有効/無効をあとから更新できる */
export function initHistoryButtons(
  onBack: () => void,
  onForward: () => void,
): (state: { can_back: boolean; can_forward: boolean }) => void {
  const back = document.getElementById("mdpeek-back");
  const forward = document.getElementById("mdpeek-forward");
  back?.addEventListener("click", onBack);
  forward?.addEventListener("click", onForward);

  return (state) => {
    if (back instanceof HTMLButtonElement) {
      back.disabled = !state.can_back;
    }
    if (forward instanceof HTMLButtonElement) {
      forward.disabled = !state.can_forward;
    }
  };
}

/**
 * トラックパッドの2本指横スワイプで戻る／進む。横スクロールできる要素の上では、
 * その要素が端に達しているときだけジェスチャーとして扱う(表・コードブロック・
 * Mermaid図などの、通常の横スクロールを妨げないため)。
 */
export function initSwipeGestures(onBack: () => void, onForward: () => void): void {
  let accumulated = 0;
  let cooldownUntil = 0;

  window.addEventListener(
    "wheel",
    (event) => {
      const now = performance.now();
      if (now < cooldownUntil) {
        return;
      }
      if (!isHorizontalSwipe(event.deltaX, event.deltaY)) {
        accumulated = 0;
        return;
      }

      const target = event.target instanceof Element ? event.target : null;
      const scrollable = target !== null ? scrollableAncestor(target) : null;
      if (scrollable !== null) {
        const direction = event.deltaX < 0 ? "left" : "right";
        const atEdge = isAtScrollEdge(direction, scrollable.scrollLeft, scrollable.scrollWidth, scrollable.clientWidth);
        if (!atEdge) {
          // 端に達していなければ、要素自体の横スクロールを優先する
          accumulated = 0;
          return;
        }
      }

      const result = accumulateSwipe(accumulated, event.deltaX, SWIPE_THRESHOLD);
      accumulated = result.accumulated;
      if (result.direction === null) {
        return;
      }
      cooldownUntil = now + SWIPE_COOLDOWN_MS;
      if (result.direction === "back") {
        onBack();
      } else {
        onForward();
      }
    },
    { passive: true },
  );
}
