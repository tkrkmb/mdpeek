import { homeDir } from "@tauri-apps/api/path";

/** ホームディレクトリの部分を ~ に縮める */
export function abbreviateHome(path: string, home: string | null): string {
  if (home === null || home === "") {
    return path;
  }
  const base = home.endsWith("/") ? home.slice(0, -1) : home;
  if (path === base) {
    return "~";
  }
  if (path.startsWith(`${base}/`)) {
    return `~${path.slice(base.length)}`;
  }
  return path;
}

/** 流す速さ（px/秒） */
const SPEED = 60;

let bar: HTMLElement | null = null;
let home: Promise<string | null> = Promise.resolve(null);
/** 最後に表示を頼まれた絶対パス */
let latest: string | null = null;

/** 帯の高さ。帯に隠れた部分は、画面に見えていないものとして扱う */
export function topInset(): number {
  return bar?.offsetHeight ?? 0;
}

/** 表示中の文書のパスを帯に出す */
export function showPath(path: string): void {
  if (bar === null || path === latest) {
    return;
  }
  latest = path;
  const text = bar.querySelector<HTMLElement>(".mdsight-path-text")!;
  void home.then((dir) => {
    // 待っている間に別の文書へ移っていたら、そちらに任せる
    if (latest === path) {
      text.textContent = abbreviateHome(path, dir);
    }
  });
}

/** 帯を用意する。収まらないパスは、パスの上にホバーしている間だけ横に流す */
export function initPathBar(): void {
  const element = document.querySelector<HTMLElement>(".mdsight-path")!;
  const track = element.querySelector<HTMLElement>(".mdsight-path-track")!;
  const text = element.querySelector<HTMLElement>(".mdsight-path-text")!;
  const reducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)");
  bar = element;
  home = homeDir().catch(() => null);

  track.addEventListener("mouseenter", () => {
    const overflow = text.getBoundingClientRect().width - track.clientWidth;
    if (overflow <= 0) {
      track.removeAttribute("title");
      return;
    }
    if (reducedMotion.matches) {
      // 動かさず、ツールチップで全体を見せる
      track.title = text.textContent ?? "";
      return;
    }
    track.removeAttribute("title");
    track.style.setProperty("--mdsight-path-distance", `${-overflow}px`);
    // 両端で止まる時間（全体の2割）を含めた長さにする
    track.style.setProperty("--mdsight-path-duration", `${(overflow / SPEED / 0.8).toFixed(2)}s`);
    track.classList.add("mdsight-path-track--scrolling");
  });

  track.addEventListener("mouseleave", () => {
    track.classList.remove("mdsight-path-track--scrolling");
  });
}
