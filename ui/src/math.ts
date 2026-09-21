import katex from "katex";
import "katex/dist/katex.min.css";

// 数式の元のコードをキーにして、結果をキャッシュする
const cache = new Map<string, string>();

/** `[data-math-style]` の要素だけを描画する */
export function renderMath(root: HTMLElement): void {
  for (const element of Array.from(root.querySelectorAll<HTMLElement>("[data-math-style]"))) {
    const code = element.textContent ?? "";
    const displayMode = element.getAttribute("data-math-style") === "display";
    const key = `${displayMode ? "display" : "inline"}\n${code}`;
    let rendered = cache.get(key);
    if (rendered === undefined) {
      rendered = katex.renderToString(code, {
        displayMode,
        trust: false,
        throwOnError: false,
      });
      cache.set(key, rendered);
    }
    element.innerHTML = rendered;
  }
}
