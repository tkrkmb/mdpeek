import { convertFileSrc, invoke } from "@tauri-apps/api/core";

import { isRelativePath } from "./paths";

/** 表示できない画像は、代替テキストに置き換える */
function showAlt(image: HTMLImageElement): void {
  const alt = document.createTextNode(image.getAttribute("alt") ?? "");
  image.replaceWith(alt);
}

/**
 * 画像の1つのパスを、表示できるURLにする。`https:` はそのまま、相対パスはRustで解決してから変換する。
 * それ以外のスキームや、解決できないパスは null
 */
async function resolveSource(source: string, version: number): Promise<string | null> {
  if (/^https:/i.test(source)) {
    return source;
  }
  if (!isRelativePath(source)) {
    return null;
  }
  try {
    return convertFileSrc(await invoke<string>("resolve_image", { path: source, version }));
  } catch {
    return null;
  }
}

/**
 * 生HTMLの `<source srcset>` の各候補（「パス 記述子」をカンマで区切ったもの）を、画像と同じ規則で解決する。
 * 表示できない候補は除き、1つも残らなければ `source` を取り除く（`picture` の中の `img` が使われる）
 */
async function resolveSources(root: HTMLElement, version: number, isCurrent: () => boolean): Promise<void> {
  for (const source of Array.from(root.querySelectorAll("source[srcset]"))) {
    const candidates = (source.getAttribute("srcset") ?? "")
      .split(",")
      .map((candidate) => candidate.trim())
      .filter((candidate) => candidate !== "");
    const resolved: string[] = [];
    for (const candidate of candidates) {
      const [path, ...descriptor] = candidate.split(/\s+/);
      const url = await resolveSource(path, version);
      if (!isCurrent()) {
        return;
      }
      if (url !== null) {
        resolved.push([url, ...descriptor].join(" "));
      }
    }
    if (resolved.length === 0) {
      source.remove();
    } else {
      source.setAttribute("srcset", resolved.join(", "));
    }
  }
}

/**
 * 相対パスの画像はRustで解決してからURLに変換する。
 * `https:` はそのまま、それ以外のスキームは表示しない。生HTMLの `img` と `source` も同じ。
 */
export async function resolveImages(
  root: HTMLElement,
  version: number,
  isCurrent: () => boolean,
): Promise<void> {
  await resolveSources(root, version, isCurrent);
  if (!isCurrent()) {
    return;
  }
  for (const image of Array.from(root.querySelectorAll("img"))) {
    const source = image.getAttribute("src") ?? "";
    const url = await resolveSource(source, version);
    // 解決できた（できなかった）時点で版を確認する
    if (!isCurrent()) {
      return;
    }
    if (url === null) {
      showAlt(image);
    } else if (url !== source) {
      image.src = url;
    }
  }
}
