# 引き継ぎ資料

## 現在の段階

段階3「描画機能」：完了・コミット済み。段階1・2も済み。

## 次にやること

- 段階4「スクロール同期」：位置表、カーソル追従、修飾クリックでのジャンプ、往復防止、vitest
- `lua/mdpeek/rpc.lua` の `jump` と、`mdpeek_cursor` の送受信がまだ無い

## 未解決の問題と、確認できていないこと

- 見た目そのものの目視確認は利用者に依頼している（GNOMEがプログラムからのスクリーンショットを拒否する）
- mermaid は 11.17.2 を使う（12系はlodash-esの勧告があるため利用者の判断で下げた）。`npm audit` は0件
- `T` キーは大文字小文字どちらでも効く（修飾キー付きは無視）。切り替え時の画面表示は「不要」と判断され削除済み
- `AGENTS.md` の直近コミット（d3f14ee）に、利用者自身の変更と私の変更が混ざっている
- 注意：親プロセスが消えたあとの標準エラー出力はEPIPEでパニックするため、`main.rs` の `report()` 経由で書く

## 動作確認に使うコマンド

- `cd src-tauri && cargo build && cargo test`（16件）、`cd ui && npm run build`
- Lua：`nvim --headless -n --listen <sock> -c "luafile <setup>"` で起動し、`--remote-send` でキー入力、
  `--remote-expr "json_encode(luaeval('...'))"` で状態を読む（`-l` ではTextChangedが発火しない）
- 画面の中身を確認したいときは、`main.ts` の末尾に一時的な自己診断を足し、
  `invoke("resolve_image", { path: "SELFTEST " + JSON.stringify(summary), version })` でRust側のエラーメッセージに載せて読む。
  確認後は必ず消す（フロントエンドを変えたら `npm run build` → `cargo build` の順で焼き直す）

## 次の作業で最初に読むファイル

- `ui/src/main.ts`（版の管理、差し替え、スクロール位置、キーとクリックの配線）
- `ui/src/diagrams.ts` / `math.ts` / `images.ts` / `links.ts` / `theme.ts`
- `src-tauri/src/image.rs`（画像パスの解決とテスト）、`src-tauri/tauri.conf.json`（CSPとassetプロトコル）
