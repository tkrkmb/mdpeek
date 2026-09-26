import { expect, test, vi } from "vitest";

import indexHtml from "../index.html?raw";

/**
 * main.ts が各モジュールを正しくつなげているかを確かめる。
 * Tauri は模擬にして、本文の描画から、カーソル追従、クリック、キー、検索までの流れを通す
 */

const calls: { command: string; args: unknown }[] = [];
const handlers = new Map<string, (event: { payload: unknown }) => void>();

vi.mock("@tauri-apps/api/core", () => ({
  convertFileSrc: (path: string) => path,
  invoke: async (command: string, args: unknown) => {
    calls.push({ command, args });
    if (command === "history_state") {
      return { can_back: false, can_forward: false };
    }
    if (command === "open_link") {
      return { gen: 1, version: 2, anchor: null };
    }
    return null;
  },
}));
vi.mock("@tauri-apps/api/event", () => ({
  listen: async (name: string, handler: (event: { payload: unknown }) => void) => {
    handlers.set(name, handler);
    return () => {};
  },
}));
vi.mock("@tauri-apps/api/path", () => ({ homeDir: async () => "/Users/me" }));
vi.mock("@tauri-apps/plugin-opener", () => ({ openUrl: vi.fn() }));
vi.mock("mermaid", () => ({ default: { initialize: vi.fn(), render: vi.fn() } }));
// jsdom には Worker が無いので、ページ内検索の一致はその場で探す
vi.mock("./matcher", async () => {
  const { vimMatches } = await import("./vimregex");
  return {
    cancelMatches: () => {},
    findMatches: async (regex: RegExp, texts: string[]) => ({
      kind: "done",
      ranges: texts.map((text) => vimMatches(text, regex)),
    }),
  };
});

function document_(gen: number, version: number, html: string) {
  return { gen, version, path: "/tmp/a.md", html };
}

function send(name: string, payload: unknown): void {
  const handler = handlers.get(name);
  if (handler === undefined) {
    throw new Error(`no listener for ${name}`);
  }
  handler({ payload });
}

test("main.ts wires rendering, following, clicks, keys and search together", async () => {
  // index.html の本文（スクリプトの手前まで）を、そのまま使う
  document.body.innerHTML = indexHtml.slice(indexHtml.indexOf("<body>") + "<body>".length, indexHtml.indexOf("<script")).trim();
  window.matchMedia = ((query: string) => ({
    matches: false,
    media: query,
    addEventListener() {},
    removeEventListener() {},
  })) as never;
  const scrollTo = vi.fn();
  const scrollBy = vi.fn();
  window.scrollTo = scrollTo as never;
  window.scrollBy = scrollBy as never;

  await import("./main");
  const { body, view } = await import("./view");
  await vi.waitFor(() => expect(handlers.has("mdsight://document") && handlers.has("mdsight://cursor")).toBe(true));
  const settled = () => vi.waitFor(() => expect(view.rendering).toBe(false));

  // 描画：本文、コードブロックの枠、位置表
  send(
    "mdsight://document",
    document_(
      1,
      1,
      '<h1 data-sourcepos="1:1-1:5">T</h1>\n<p data-sourcepos="3:1-3:20"><a href="b.md#x">link</a></p>\n<pre data-sourcepos="5:1-8:3"><code class="language-js">a\nb\n</code></pre>',
    ),
  );
  await settled();
  expect(body.querySelector("h1")?.textContent).toBe("T");
  expect(body.querySelector(".mdsight-code > pre")).not.toBeNull();
  expect(view.table.map((block) => block.startLine)).toEqual([1, 3, 5]);

  // 古い版は捨てる
  send("mdsight://document", document_(1, 0, "<p>stale</p>"));
  expect(body.textContent).not.toContain("stale");

  // 同じ世代の新しい版で、本文が更新される
  send(
    "mdsight://document",
    document_(
      1,
      2,
      '<h1 data-sourcepos="1:1-1:9">Edited</h1>\n<p data-sourcepos="3:1-3:20"><a href="b.md#x">link</a></p>\n<pre data-sourcepos="5:1-8:3"><code>a\nb\n</code></pre>',
    ),
  );
  await settled();
  expect(body.querySelector("h1")?.textContent).toBe("Edited");
  expect(view.version).toBe(2);

  // カーソルの行が画面に見えていなければ、スクロールする
  scrollTo.mockClear();
  send("mdsight://cursor", { gen: 1, line: 3 });
  expect(scrollTo).toHaveBeenCalled();

  // 修飾クリック（jsdom は Linux 扱いなので Ctrl）で、クリックした要素の行へジャンプする
  body.querySelector("h1")!.dispatchEvent(new MouseEvent("click", { bubbles: true, ctrlKey: true }));
  expect(calls.filter((call) => call.command === "jump").at(-1)?.args).toEqual({ gen: 1, version: 2, line: 1 });

  // 通常のクリックでは、相対パスの .md リンクを開く
  body.querySelector("a")!.dispatchEvent(new MouseEvent("click", { bubbles: true, cancelable: true }));
  await vi.waitFor(() => expect(calls.some((call) => call.command === "open_link")).toBe(true));
  expect(calls.find((call) => call.command === "open_link")?.args).toMatchObject({ href: "b.md", version: 2 });

  // j でスクロールする。Ctrl+F で検索の帯が開き、入力している間は j でスクロールしない
  document.dispatchEvent(new KeyboardEvent("keydown", { key: "j", bubbles: true }));
  expect(scrollBy).toHaveBeenCalledWith({ top: 72, behavior: "instant" });
  document.dispatchEvent(new KeyboardEvent("keydown", { key: "f", ctrlKey: true, bubbles: true }));
  expect((document.getElementById("mdsight-find") as HTMLElement).hidden).toBe(false);
  scrollBy.mockClear();
  const input = document.querySelector<HTMLInputElement>("#mdsight-find input")!;
  input.dispatchEvent(new KeyboardEvent("keydown", { key: "j", bubbles: true }));
  expect(scrollBy).not.toHaveBeenCalled();

  // ページ内検索で強調され、Esc で消える
  input.value = "link";
  input.dispatchEvent(new Event("input", { bubbles: true }));
  await vi.waitFor(() => expect(body.querySelectorAll("mark.mdsight-hit")).toHaveLength(1));
  expect(document.querySelector(".mdsight-find-count")?.textContent).toBe("1/1");
  document.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
  expect(body.querySelectorAll("mark.mdsight-hit")).toHaveLength(0);

  // Neovimの検索の一致が、現在の一致として強調される
  send("mdsight://search", { gen: 1, version: 2, matches: [{ line: 1, text: "Edited", current: true }] });
  expect(body.querySelectorAll("mark.mdsight-nvim-current")).toHaveLength(1);

  // 別の文書に切り替わったら、本文と世代が替わる
  send("mdsight://document", document_(2, 3, '<p data-sourcepos="1:1-1:3">other</p>'));
  await settled();
  expect(view.gen).toBe(2);
  expect(body.textContent).toContain("other");
});
