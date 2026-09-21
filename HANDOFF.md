# 引き継ぎ資料

## 現在の段階

段階2「自動更新」：実装完了、承認待ち（未コミット）。段階1はコミット済み。

## 次にやること

- 利用者がプレビューの自動更新とスクロール位置の保持を目視確認する
- 承認後に段階2をコミットする
- そのあと段階3「描画機能」（Mermaid、KaTeX、キャッシュ、画像、リンク、テーマ、CSP）へ進む

## 未解決の問題と、確認できていないこと

- 画面の見た目（更新されるか、読んでいた位置が保たれるか）の目視確認が未実施。GNOMEがプログラムからのスクリーンショットを拒否するため自動確認できない
- 段階4で必要になる `rpc.jump`、`mdpeek_cursor`、位置表、vitestはまだ無い
- 注意：親プロセスが消えたあとの標準エラー出力はEPIPEでパニックするため、`main.rs` の `report()` 経由で書く

## 動作確認に使うコマンド

- `cd src-tauri && cargo build && cargo test`（9件）、`cd ui && npm run build`
- Lua：`nvim --headless -l` ではTextChangedが発火しないので、`nvim --headless -n --listen <sock> -c "luafile <setup>"` で起動し、
  `nvim --server <sock> --remote-send 'ox<Esc>'` でキー入力、`--remote-expr "json_encode(luaeval('...'))"` で状態を読む
- 送信内容を見たいときは `vim.rpcnotify` を差し替えて記録する。スタブの実行ファイルは長時間眠るものにする（短いと終了してstateが解放される）
- 手動：`:set runtimepath+=<repo>` → `require("mdpeek").setup({ bin = "<repo>/src-tauri/target/debug/mdpeek" })` → markdownファイルで `:MdPeek`

## 次の作業で最初に読むファイル

- `lua/mdpeek/init.lua`（`send_content`、デバウンス、対象の設定）
- `src-tauri/src/nvim.rs`（通知の受け口、キュー、`document_from`）
- `ui/src/main.ts`（版の比較、`capture`／`restore` によるスクロール位置の保持）
