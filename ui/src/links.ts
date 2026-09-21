import { openUrl } from "@tauri-apps/plugin-opener";

/** どの場合もWebView自体は遷移させない */
export function handleLink(event: MouseEvent): void {
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
  if (/^https?:/i.test(href)) {
    void openUrl(href);
  }
}
