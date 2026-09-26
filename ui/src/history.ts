import { invoke } from "@tauri-apps/api/core";

import { skipFirstCursor } from "./cursor";
import { initHistoryButtons, initSwipeGestures } from "./navigation";
import { flash } from "./notice";
import { capture, restore, type Anchor } from "./position";
import { view } from "./view";

/** 戻る／進むで開いたときは、その文書で読んでいた位置（`anchor`）も返ってくる */
type NavigationTarget = {
  gen: number;
  version: number;
  anchor: Anchor | null;
};

type HistoryAvailability = {
  can_back: boolean;
  can_forward: boolean;
};

/** リンクや履歴で移動した先。届いた文書と世代・版が一致したら、そちらを優先する */
type PendingNavigation = {
  gen: number;
  version: number;
  fragment: string | null;
  anchor: Anchor | null;
};

/** リンクや履歴での移動先。まだ文書が届いていない間だけ保持する */
let pendingNavigation: PendingNavigation | null = null;
/** 戻る／進むで戻した位置。画像と図の描画が終わった時点で、もう一度合わせる */
let settling: Anchor | null = null;

let updateHistoryButtons: ((state: HistoryAvailability) => void) | null = null;
/** 直前に取得した、戻る／進むを辿れるかどうか */
let availability: HistoryAvailability = { can_back: false, can_forward: false };

/** 届いた文書が、いま待っているリンク先や履歴の行き先と一致するか */
export function matchPendingNavigation(gen: number, version: number): PendingNavigation | null {
  if (pendingNavigation === null || pendingNavigation.gen !== gen || pendingNavigation.version !== version) {
    return null;
  }
  const navigation = pendingNavigation;
  pendingNavigation = null;
  return navigation;
}

/**
 * 移動先の文書は、コマンドの結果より先に届いていることがある。
 * 届いていればすぐに動き、まだなら届いたときに動けるように覚えておく
 */
function navigateTo(target: NavigationTarget, fragment: string | null): void {
  if (target.anchor !== null) {
    skipFirstCursor(target.gen);
  }
  if (target.gen === view.gen) {
    arrive(fragment, target.anchor);
  } else if (target.gen > view.gen) {
    pendingNavigation = { gen: target.gen, version: target.version, fragment, anchor: target.anchor };
  }
}

/** 移動先の文書で、読んでいた位置があればそこへ、なければ #見出し か先頭へ動く */
export function arrive(fragment: string | null, anchor: Anchor | null): void {
  if (anchor === null) {
    goToFragmentOrTop(fragment);
    return;
  }
  restore(anchor);
  // 描画を待っている間なら、画像と図で高さが変わった後に、もう一度合わせる
  settling = view.rendering ? anchor : null;
}

/** 描画を始めるときに呼ぶ。前の文書の「合わせ直す位置」を捨てる */
export function resetSettling(): void {
  settling = null;
}

/** 描画が終わったときに呼ぶ。合わせ直す位置があれば、そこへ戻す */
export function restoreSettled(): void {
  if (settling !== null) {
    restore(settling);
    settling = null;
  }
}

/** #見出し があればその要素へ、なければ先頭へ移動する */
function goToFragmentOrTop(fragment: string | null): void {
  const target = fragment !== null ? document.getElementById(fragment) : null;
  if (target !== null) {
    target.scrollIntoView();
  } else {
    window.scrollTo({ top: 0 });
  }
}

/** 戻る／進むボタンの有効/無効を、いまの履歴に合わせ直す */
export async function refreshHistoryAvailability(): Promise<void> {
  try {
    availability = await invoke<HistoryAvailability>("history_state");
    updateHistoryButtons?.(availability);
  } catch {
    // 取得できなくても、表示は変えない
  }
}

/** 相対パスの .md／.markdown リンクを開く。#見出し があれば、開いた後にそこへ動く */
export async function followLink(href: string): Promise<void> {
  const hashIndex = href.indexOf("#");
  const fragment = hashIndex === -1 ? null : decodeURIComponent(href.slice(hashIndex + 1));
  const path = hashIndex === -1 ? href : href.slice(0, hashIndex);
  try {
    // いま読んでいる位置を渡し、戻ったときにそこへ戻れるようにする
    const target = await invoke<NavigationTarget>("open_link", { href: path, version: view.version, anchor: capture() });
    navigateTo(target, fragment);
  } catch (error) {
    flash(`Cannot open the link: ${path} (${String(error)})`);
  } finally {
    void refreshHistoryAvailability();
  }
}

/** 戻る／進むの履歴を辿る */
export async function navigateHistory(command: "go_back" | "go_forward"): Promise<void> {
  const back = command === "go_back";
  // 辿れる履歴が無いときは、失敗として知らせず何もしない
  if (back ? !availability.can_back : !availability.can_forward) {
    return;
  }
  try {
    const target = await invoke<NavigationTarget>(command, { anchor: capture() });
    navigateTo(target, null);
  } catch (error) {
    flash(`${back ? "Cannot go back" : "Cannot go forward"} (${String(error)})`);
  } finally {
    void refreshHistoryAvailability();
  }
}

/** 戻る／進むのボタンとスワイプを配線し、ボタンの状態を取得する */
export function initHistory(): void {
  updateHistoryButtons = initHistoryButtons(
    () => void navigateHistory("go_back"),
    () => void navigateHistory("go_forward"),
  );
  initSwipeGestures(
    () => void navigateHistory("go_back"),
    () => void navigateHistory("go_forward"),
  );
  void refreshHistoryAvailability();
}
