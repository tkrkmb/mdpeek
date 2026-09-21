# 引き継ぎ資料

## 現在の段階

段階1〜5はすべて完了。CI（Release用のビルド）も追加し、`v0.1.0` を公開済み。
`git@github.com:tkrkmb/mdpeek.git` の `main` にプッシュ済み。

## 次にやること

- 次の版を出すときは `git tag vX.Y.Z && git push origin vX.Y.Z`。5〜6分でReleaseに両OSの実行ファイルが付く
- 以降は利用者の依頼に応じる。仕様を変える場合は、先に `AGENTS.md` を更新してから実装する
- コミットしたら `git push` まで行う

## 未解決の問題と、確認できていないこと

- macOSでの実機動作は未確認（CIでユニバーサルバイナリ（x86_64 + arm64）ができることまでは確認済み）
- 画面の見た目とスクロール同期の手触りは、利用者の実機確認に依存する
- 署名・公証をしないため、macOSではダウンロード後に `xattr -dr com.apple.quarantine` が要る
- 起動直後、登録時に送るカーソル行は、画面の準備が間に合わないと取りこぼす（実害なし）
- `beforeBuildCommand` は `ui/` で実行される。`npm run build` と書く（`--prefix ui` は誤り）
- 親プロセスが消えたあとの標準エラー出力はEPIPEでパニックするため、`main.rs` の `report()` 経由で書く

## 動作確認に使うコマンド

- `cd src-tauri && cargo build && cargo test`（18件）、`cd ui && npm run build && npm test`（9件）
- リリース：ルートで `cargo tauri build --no-bundle` → `src-tauri/target/release/mdpeek`
- CI：`gh workflow run release.yml --ref main` で手動実行し、`gh run view <id> --json status,jobs` で結果を見る。
  成果物は `gh run download <id> -n mdpeek-x86_64-linux`
- Lua：`nvim --headless -n --listen <sock> -c "luafile <setup>"` で起動し、`--remote-send '5G'` などで操作、
  `--remote-expr "json_encode(luaeval('...'))"` で状態を読む。`-l` ではTextChangedもCursorMovedも発火しない
- 画面の中身は、`main.ts` の末尾に一時的な自己診断を足し、`invoke("resolve_image", { path: "SELFTEST " .. , version })`
  とRust側の一時ログで読む。確認後は必ず消す

## 次の作業で最初に読むファイル

- `AGENTS.md`（仕様）と `README.md`（利用者向けの説明）
- `.github/workflows/release.yml`（CI）
- `lua/mdpeek/init.lua`、`lua/mdpeek/rpc.lua`
- `src-tauri/src/main.rs`、`ui/src/main.ts`
