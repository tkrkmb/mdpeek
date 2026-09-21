# 引き継ぎ資料

## 現在の段階

段階5「仕上げ」：README作成まで完了、承認待ち（未コミット）。段階1〜4はコミット済み。
これで計画の5段階はすべて実装済み。

## 次にやること

- 利用者がREADMEを読んで確認する
- 承認後に段階5をコミットする
- 以降は、利用者から新しい依頼が出たときに着手する（`AGENTS.md` の仕様を先に更新してから実装する）

## 未解決の問題と、確認できていないこと

- 画面の見た目とスクロール同期の手触りは、利用者の実機確認に依存している（GNOMEがプログラムからのスクリーンショットを拒否する）
- `beforeBuildCommand` は `ui/` で実行される。`npm run build` と書く（`--prefix ui` は誤り。段階5で判明して修正済み）
- 起動直後、登録時に送るカーソル行は、画面の準備が間に合わないと取りこぼす（実害なし）
- mermaid は 11.17.2。`npm audit` は0件
- 注意：親プロセスが消えたあとの標準エラー出力はEPIPEでパニックするため、`main.rs` の `report()` 経由で書く

## 動作確認に使うコマンド

- `cd src-tauri && cargo build && cargo test`（18件）、`cd ui && npm run build && npm test`（9件）
- リリース：リポジトリのルートで `cargo tauri build --no-bundle` → `src-tauri/target/release/mdpeek`
- Lua：`nvim --headless -n --listen <sock> -c "luafile <setup>"` で起動し、`--remote-send '5G'` などで操作、
  `--remote-expr "json_encode(luaeval('...'))"` で状態を読む。`-l` ではTextChangedもCursorMovedも発火しない
- 画面の中身を見たいときは、`main.ts` の末尾に一時的な自己診断を足し、
  `invoke("resolve_image", { path: "SELFTEST " + JSON.stringify(x), version })` とRust側の一時ログで読む。確認後は必ず消す

## 次の作業で最初に読むファイル

- `AGENTS.md`（仕様）と `README.md`（利用者向けの説明）
- `lua/mdpeek/init.lua`、`lua/mdpeek/rpc.lua`
- `src-tauri/src/main.rs`（コマンドと状態）、`nvim.rs`（RPC）
- `ui/src/main.ts`（描画と同期の配線）
