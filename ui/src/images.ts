import { convertFileSrc, invoke } from "@tauri-apps/api/core";

function hasScheme(source: string): boolean {
  return /^[a-z][a-z0-9+.-]*:/i.test(source) || source.startsWith("//");
}

/** 表示できない画像は、代替テキストに置き換える */
function showAlt(image: HTMLImageElement): void {
  const alt = document.createTextNode(image.getAttribute("alt") ?? "");
  image.replaceWith(alt);
}

/**
 * 相対パスの画像はRustで解決してからURLに変換する。
 * `https:` はそのまま、それ以外のスキームは表示しない。
 */
export async function resolveImages(
  root: HTMLElement,
  version: number,
  isCurrent: () => boolean,
): Promise<void> {
  for (const image of Array.from(root.querySelectorAll("img"))) {
    const source = image.getAttribute("src") ?? "";
    if (/^https:/i.test(source)) {
      continue;
    }
    if (hasScheme(source) || source === "") {
      showAlt(image);
      continue;
    }
    try {
      const resolved = await invoke<string>("resolve_image", { path: source, version });
      // 解決できた時点で版を確認する
      if (!isCurrent()) {
        return;
      }
      image.src = convertFileSrc(resolved);
    } catch {
      if (!isCurrent()) {
        return;
      }
      showAlt(image);
    }
  }
}
