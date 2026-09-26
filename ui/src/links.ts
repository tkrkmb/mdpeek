import { openUrl } from "@tauri-apps/plugin-opener";

import { isRelativePath } from "./paths";

/** 相対パスで、拡張子が md／markdown（末尾に #見出し が付いていてもよい）のリンクか */
export function isRelativeMarkdownLink(href: string): boolean {
  if (!isRelativePath(href)) {
    return false;
  }
  const path = href.split("#", 1)[0];
  return /\.(md|markdown)$/i.test(path);
}

/**
 * どの場合もWebView自体は遷移させない。
 * 相対パスの .md／.markdown リンクは、onNavigate に渡して呼び出し側に任せる。
 */
export function handleLink(event: MouseEvent, onNavigate: (href: string) => void): void {
  const target = event.target;
  if (!(target instanceof Element)) {
    return;
  }
  const anchor = target.closest("a");
  if (anchor === null) {
    return;
  }

  event.preventDefault();
  const href = anchor.getAttribute("href") ?? "";
  if (href.startsWith("#")) {
    const id = decodeURIComponent(href.slice(1));
    document.getElementById(id)?.scrollIntoView();
    return;
  }
  if (isRelativeMarkdownLink(href)) {
    onNavigate(href);
    return;
  }
  if (/^https?:/i.test(href)) {
    void openUrl(href);
  }
}
