import mermaid from "mermaid";

import { appearance, type Appearance } from "./theme";

// 図の元のコードとテーマの組をキーにして、SVGをキャッシュする
const cache = new Map<string, string>();
let counter = 0;

function configure(theme: Appearance): void {
  mermaid.initialize({
    startOnLoad: false,
    securityLevel: "strict",
    theme: theme === "dark" ? "dark" : "default",
  });
}

/** まだ描いていないブロックと、すでに描いたブロックの両方を集める */
function diagrams(root: HTMLElement): { block: HTMLElement; source: string }[] {
  const found: { block: HTMLElement; source: string }[] = [];
  for (const code of Array.from(root.querySelectorAll<HTMLElement>("code.language-mermaid"))) {
    const block = code.parentElement;
    if (block !== null) {
      block.dataset.mermaidSource = code.textContent ?? "";
      found.push({ block, source: block.dataset.mermaidSource });
    }
  }
  for (const block of Array.from(root.querySelectorAll<HTMLElement>("[data-mermaid-source]"))) {
    if (block.querySelector("code.language-mermaid") === null) {
      found.push({ block, source: block.dataset.mermaidSource ?? "" });
    }
  }
  return found;
}

function show(block: HTMLElement, svg: string): void {
  block.innerHTML = svg;
  block.classList.add("mermaid-diagram");
}

function showError(block: HTMLElement, message: string): void {
  // 構文エラーはそのブロック内にだけ表示し、他の部分の描画は続ける
  const notice = document.createElement("pre");
  notice.className = "mermaid-error";
  notice.textContent = message;
  block.replaceChildren(notice);
  block.classList.remove("mermaid-diagram");
}

/**
 * ```mermaid のブロックを図に差し替える。
 * テーマを変えたあとの描き直しにも、同じ関数を使う。
 */
export async function renderDiagrams(
  root: HTMLElement,
  isCurrent: () => boolean,
): Promise<boolean> {
  const blocks = diagrams(root);
  if (blocks.length === 0) {
    return false;
  }

  const theme = appearance();
  configure(theme);

  for (const { block, source } of blocks) {
    const key = `${theme}\n${source}`;
    const cached = cache.get(key);
    if (cached !== undefined) {
      show(block, cached);
      continue;
    }

    try {
      counter += 1;
      const { svg } = await mermaid.render(`mdpeek-diagram-${counter}`, source);
      // 描き終えた時点で版を確認する
      if (!isCurrent()) {
        return false;
      }
      cache.set(key, svg);
      show(block, svg);
    } catch (error) {
      if (!isCurrent()) {
        return false;
      }
      showError(block, error instanceof Error ? error.message : String(error));
    }
  }
  return true;
}
