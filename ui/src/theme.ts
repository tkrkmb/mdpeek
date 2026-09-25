import { invoke } from "@tauri-apps/api/core";
import lightCss from "github-markdown-css/github-markdown-light.css?inline";
import darkCss from "github-markdown-css/github-markdown-dark.css?inline";

export type Theme = "system" | "light" | "dark";
export type Appearance = "light" | "dark";

const STORAGE_KEY = "mdsight.theme";
const ORDER: Theme[] = ["system", "light", "dark"];

/**
 * 2つのテーマを1枚のスタイルに入れ、`:root[data-theme]` で選ぶ。
 * スタイルシートの有効・無効を切り替えると、WebKitでは切り替えが1回分遅れて
 * 見た目が崩れるため、切り替えは属性の書き換えだけで済ませる。
 * `:where()` で包むのは、詳細度を `.markdown-body` のままに保つため。
 */
function scoped(css: string, shown: Appearance): string {
  return css.replaceAll(".markdown-body", `:where(:root[data-theme="${shown}"]) .markdown-body`);
}

function stylesheet(): void {
  const element = document.createElement("style");
  element.textContent = `${scoped(lightCss, "light")}\n${scoped(darkCss, "dark")}`;
  document.head.appendChild(element);
}

stylesheet();
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
  document.documentElement.dataset.theme = appearance();
  // アプリのアイコンも、実際に使っている色に合わせる
  invoke("set_app_icon", { appearance: appearance() }).catch(() => {});
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
