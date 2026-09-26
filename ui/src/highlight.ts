import hljs from "highlight.js/lib/common";
import lightCss from "highlight.js/styles/github.css?inline";
import darkCss from "highlight.js/styles/github-dark.css?inline";

import type { Appearance } from "./theme";

/**
 * 配色のルールを、`:root[data-theme]` に合うときだけ効くようにする。
 * `.hljs-meta .hljs-keyword` のような子孫セレクタもあるので、各セレクタの先頭にだけ付ける
 */
function scoped(css: string, shown: Appearance): string {
  const prefix = `:where(:root[data-theme="${shown}"]) `;
  return css
    .replace(/\/\*[\s\S]*?\*\//g, "")
    .replace(/([^{}]+)\{/g, (_, selectors: string) => {
      const list = selectors
        .split(",")
        .map((selector) => prefix + selector.trim())
        .join(",");
      return `${list}{`;
    });
}

function stylesheet(): void {
  const element = document.createElement("style");
  element.textContent = `${scoped(lightCss, "light")}\n${scoped(darkCss, "dark")}`;
  document.head.appendChild(element);
}

stylesheet();

// コードと言語名の組をキーにして、結果をキャッシュする
const cache = new Map<string, string>();

/** 言語名の付いたコードブロック（mermaid を除く）を色付けする */
export function highlightCode(root: HTMLElement): void {
  for (const code of Array.from(root.querySelectorAll<HTMLElement>("pre > code[class^='language-']"))) {
    const language = code.className.slice("language-".length);
    // 言語名がないものと、highlight.js が知らない言語は色付けしない
    if (language === "mermaid" || hljs.getLanguage(language) === undefined) {
      continue;
    }
    const source = code.textContent ?? "";
    const key = `${language}\n${source}`;
    let highlighted = cache.get(key);
    if (highlighted === undefined) {
      highlighted = hljs.highlight(source, { language, ignoreIllegals: true }).value;
      cache.set(key, highlighted);
    }
    code.innerHTML = highlighted;
  }
}
