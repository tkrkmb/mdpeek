# 引き継ぎ資料

## 現在の段階

段階1「接続と基本表示」：実装完了。`cargo build` / `cargo test` / `npm run build` と、実機での接続・終了確認まで済み。

## 次にやること

- 利用者がプレビュー画面の表示を目視確認する
- 承認後に段階1をコミットする（`Cargo.lock` と `ui/package-lock.json` も一緒に）
- そのあと段階2「自動更新」へ進む

## 未解決の問題と、確認できていないこと

- ウィンドウに本文が描画されているかの目視確認が未実施（GNOMEがプログラムからのスクリーンショットを拒否するため自動確認不可）
- `:MdPeekClose` の「応答がなければ終了」の待ち時間は仕様に無いため1000msとした
- 段階1の範囲外のため未実装：`rpc.jump`、`mdpeek_content` / `mdpeek_cursor` の送受信、対象切り替え時の本文送信、CSP、テーマ切り替え、Mermaid・KaTeX・画像・リンク
- 注意：親プロセスが消えたあとの標準エラー出力はEPIPEでパニックするため、`main.rs` の `report()` 経由で書く（直接 `eprintln!` を使うと終了処理に到達しない）

## 動作確認に使うコマンド

- フロントエンド：`cd ui && npm install && npm run build`
- Rust：`cd src-tauri && cargo build && cargo test`
- Lua（headless）：`FAKE_BIN=<引数を無視して30秒眠るスクリプト> nvim --headless -l <確認用スクリプト>`
  （確認用スクリプトはリポジトリ外の作業ディレクトリに置いた。必要なら作り直す）
- 手動：`require("mdpeek").setup({ bin = "<repo>/src-tauri/target/debug/mdpeek" })` の後、markdownファイルで `:MdPeek`

## 次の作業で最初に読むファイル

- `lua/mdpeek/init.lua`（対象の設定、起動、終了）
- `lua/mdpeek/rpc.lua`（`register`。`jump` は未実装）
- `src-tauri/src/nvim.rs`（接続、登録、通知の受け口）
- `ui/src/main.ts`（版の比較と `.markdown-body` の差し替え）
