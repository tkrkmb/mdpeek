# 引き継ぎ資料

## 現在の段階

段階4「スクロール同期」：実装完了、承認待ち（未コミット）。段階1〜3はコミット済み。

## 次にやること

- 利用者が実機で確認する（カーソル追従、Ctrl+クリックでのジャンプ）
- 承認後に段階4をコミットする
- そのあと段階5「仕上げ」：README（導入、`setup`、コマンド、`cargo tauri build --no-bundle`、既知の制限）

## 未解決の問題と、確認できていないこと

- 実機での手触り（追従の速さ、1/3の位置が読みやすいか）は利用者の確認待ち
- `mdpeek_cursor` は版を持たないため、フロントエンドは世代だけで照合している（仕様どおり）
- 起動直後、アプリの登録時に送るカーソル行は、画面の準備が間に合わないと取りこぼす（実害なし）
- mermaid は 11.17.2。`npm audit` は0件
- 注意：親プロセスが消えたあとの標準エラー出力はEPIPEでパニックするため、`main.rs` の `report()` 経由で書く

## 動作確認に使うコマンド

- `cd src-tauri && cargo build && cargo test`（18件）、`cd ui && npm run build && npm test`（9件）
- Lua：`nvim --headless -n --listen <sock> -c "luafile <setup>"` で起動し、`--remote-send '5G'` などで操作、
  `--remote-expr "json_encode(luaeval('...'))"` で状態を読む。`-l` ではTextChangedもCursorMovedも発火しない
- `nvim_win_set_cursor` ではCursorMovedがすぐに出ないので、スロットルの測定は `doautocmd CursorMoved` を使う
- 画面の中身を見たいときは、`main.ts` の末尾に一時的な自己診断を足し、
  `invoke("resolve_image", { path: "SELFTEST " + JSON.stringify(x), version })` とRust側の一時ログで読む。確認後は必ず消す

## 次の作業で最初に読むファイル

- `ui/src/main.ts`（位置表の作り直し、カーソル追従、修飾クリック）と `ui/src/sourcepos.ts`（+ そのテスト）
- `lua/mdpeek/init.lua`（カーソルのスロットル、WinClosed）と `lua/mdpeek/rpc.lua`（`jump` の検証）
- `src-tauri/src/nvim.rs`（`mdpeek_cursor` の転送、`jump`）と `main.rs`（`Session`、コマンド）
