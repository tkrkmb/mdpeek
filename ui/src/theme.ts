import lightHref from "github-markdown-css/github-markdown-light.css?url";
import darkHref from "github-markdown-css/github-markdown-dark.css?url";

export type Theme = "system" | "light" | "dark";
export type Appearance = "light" | "dark";

const STORAGE_KEY = "mdpeek.theme";
const ORDER: Theme[] = ["system", "light", "dark"];

function stylesheet(href: string): HTMLLinkElement {
  const element = document.createElement("link");
  element.rel = "stylesheet";
  element.href = href;
  document.head.appendChild(element);
  return element;
}

const light = stylesheet(lightHref);
const dark = stylesheet(darkHref);
const prefersDark = window.matchMedia("(prefers-color-scheme: dark)");

const listeners: (() => void)[] = [];
let current: Theme = load();

function load(): Theme {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored === "system" || stored === "light" || stored === "dark") {
      return stored;
    }
  } catch {
    // localStorageが使えない環境では既定のまま
  }
  return "system";
}

function save(theme: Theme): void {
  try {
    localStorage.setItem(STORAGE_KEY, theme);
  } catch {
    // 保存できなくても動作は続ける
  }
}

export function appearance(): Appearance {
  if (current === "system") {
    return prefersDark.matches ? "dark" : "light";
  }
  return current;
}

function apply(): void {
  const shown = appearance();
  light.disabled = shown === "dark";
  dark.disabled = shown === "light";
  document.documentElement.dataset.theme = shown;
  for (const listener of listeners) {
    listener();
  }
}

/** テーマが変わったときに呼ばれる（Mermaidの描き直しに使う） */
export function onThemeChange(listener: () => void): void {
  listeners.push(listener);
}

/** system → light → dark の順に切り替える */
export function cycle(): void {
  current = ORDER[(ORDER.indexOf(current) + 1) % ORDER.length];
  save(current);
  apply();
}

export function start(): void {
  // systemのときだけ、OSの設定変更に追従する
  prefersDark.addEventListener("change", () => {
    if (current === "system") {
      apply();
    }
  });
  apply();
}
