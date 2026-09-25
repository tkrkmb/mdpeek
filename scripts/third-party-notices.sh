#!/bin/sh
# 実行ファイルに含まれるRustの依存と、フロントエンドのバンドルに含まれたnpmパッケージの
# ライセンスを、THIRD_PARTY_NOTICES.md にまとめる。
# 先に `npm run build`（ui）を済ませておくこと（npm側の一覧は、そのとき Vite が書き出す）。
# 使い方: scripts/third-party-notices.sh [出力先]（既定はリポジトリのルートの THIRD_PARTY_NOTICES.md）
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
out=${1:-"$root/THIRD_PARTY_NOTICES.md"}
npm_licenses="$root/ui/dist/.vite/license.md"

if [ ! -f "$npm_licenses" ]; then
  echo "missing $npm_licenses; run npm run build in ui first" >&2
  exit 1
fi

rust_licenses=$(cd "$root/src-tauri" && cargo about generate about.hbs)

{
  echo "# Third-party notices"
  echo
  echo "MdSight includes the following third-party software."
  echo
  echo "# Rust crates"
  echo "$rust_licenses"
  echo
  echo "# npm packages"
  # Vite の一覧の先頭の見出し（# Licenses）は、上の見出しに置き換える
  sed '1{/^# /d;}' "$npm_licenses"
} > "$out"
