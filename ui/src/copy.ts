import { flash } from "./notice";

/** コピーの印（重なった2枚の紙）とチェックの印。自分で描いた単純な図形 */
const ICONS =
  '<svg class="mdsight-copy-icon" viewBox="0 0 16 16" aria-hidden="true"><rect x="5.5" y="5.5" width="8" height="8" rx="1.5" /><path d="M10.5 3.5v-.5a1.5 1.5 0 0 0-1.5-1.5H4A1.5 1.5 0 0 0 2.5 3v5A1.5 1.5 0 0 0 4 9.5h.5" /></svg>' +
  '<svg class="mdsight-copied-icon" viewBox="0 0 16 16" aria-hidden="true"><path d="M3 8.5l3.25 3.25L13 5" /></svg>';

/** チェックの印を出しておく時間 */
const COPIED_MS = 2000;

function copyButton(code: HTMLElement): HTMLButtonElement {
  const button = document.createElement("button");
  button.type = "button";
  button.className = "mdsight-copy";
  button.setAttribute("aria-label", "Copy code");
  button.title = "Copy";
  button.innerHTML = ICONS;
  button.addEventListener("click", (event) => {
    // 修飾クリックでも、本文のクリック（ジャンプ）として扱わない
    event.preventDefault();
    event.stopPropagation();
    navigator.clipboard
      .writeText(code.textContent ?? "")
      .then(() => {
        button.classList.add("mdsight-copied");
        setTimeout(() => button.classList.remove("mdsight-copied"), COPIED_MS);
      })
      .catch((error: unknown) => {
        flash(`Cannot copy the code (${String(error)})`);
      });
  });
  return button;
}

/**
 * コードブロック（Mermaid を除く）を枠で包み、コピーのボタンを置く。
 * ボタンを pre の外に置くのは、横スクロールで流れず、検索の対象の文字にも混ざらないようにするため。
 * 位置表は枠をブロックとして扱うので、`data-sourcepos` を枠へ移す
 */
export function addCopyButtons(root: HTMLElement): void {
  for (const code of Array.from(root.querySelectorAll<HTMLElement>("pre > code"))) {
    const pre = code.parentElement!;
    if (code.classList.contains("language-mermaid") || pre.parentElement?.classList.contains("mdsight-code")) {
      continue;
    }
    const frame = document.createElement("div");
    frame.className = "mdsight-code";
    const position = pre.getAttribute("data-sourcepos");
    if (position !== null) {
      frame.setAttribute("data-sourcepos", position);
      pre.removeAttribute("data-sourcepos");
    }
    pre.replaceWith(frame);
    frame.append(pre, copyButton(code));
  }
}
