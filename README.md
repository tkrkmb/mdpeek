# MdPeek

Neovimで編集中のMarkdownを、GitHub風の見た目で別ウィンドウにプレビューするツール。保存しなくても、入力が止まると表示が追いつく。

- 本文はNeovimのバッファから直接受け取る（ファイルは読まない）
- Mermaidの図、KaTeXの数式、画像、ダークテーマに対応する
- Neovimのカーソルにプレビューが追従し、プレビューの修飾クリックでNeovimがその行へ飛ぶ

## 必要なもの

- macOS または Linux
- Neovim 0.10以上
- Node.js と npm
- Rust
- Tauri CLI

Fedoraの場合、Node.jsとWebKitGTKなどの開発パッケージをDNFで導入する。

```sh
sudo dnf install nodejs24 nodejs24-npm
sudo dnf install webkit2gtk4.1-devel gtk3-devel libsoup3-devel \
  librsvg2-devel openssl-devel gcc gcc-c++ make file
```

ほかのディストリビューションでは、[Tauriの前提条件](https://v2.tauri.app/start/prerequisites/)を参照する。

Rustは[rustup](https://rustup.rs)でユーザー領域に導入する。

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"
```

Tauri CLIはCargoで導入する。

```sh
cargo install tauri-cli --locked --version "^2.0"
```

## ビルド

```sh
git clone <このリポジトリ> mdpeek
cd mdpeek/ui && npm install
cd .. && cargo tauri build --no-bundle
```

実行ファイルは `src-tauri/target/release/mdpeek` にできる。フロントエンドのビルドは `cargo tauri build` が自動で実行する。

配布用のパッケージは作らない。この実行ファイルをそのまま使う。

## 導入

リポジトリのルートをプラグインとして読み込み、`setup` に実行ファイルのパスを渡す。設定項目は `bin` だけ。

lazy.nvim の場合:

```lua
{
  dir = "/path/to/mdpeek",
  config = function()
    require("mdpeek").setup({
      bin = "/path/to/mdpeek/src-tauri/target/release/mdpeek",
    })
  end,
}
```

プラグインマネージャを使わない場合:

```lua
vim.opt.runtimepath:append("/path/to/mdpeek")
require("mdpeek").setup({
  bin = "/path/to/mdpeek/src-tauri/target/release/mdpeek",
})
```

## 使い方

| コマンド | 動き |
| --- | --- |
| `:MdPeek` | 現在のバッファをプレビューする。開いていれば、表示対象をこのバッファに切り替える |
| `:MdPeekClose` | プレビューを閉じる |

対象にできるのは、`buftype` が空で、ファイル名があり、`filetype` が `markdown` のバッファだけ。それ以外は理由を通知して何もしない。

プレビュー側の操作:

| 操作 | 動き |
| --- | --- |
| `T` | テーマを system → light → dark の順に切り替える（選択は保存される） |
| Cmd+クリック（macOS）／ Ctrl+クリック（Linux） | Neovimのカーソルをその行へ移動する |
| クリック | `#` のリンクはページ内を移動し、`http:` / `https:` は既定のブラウザで開く |

Neovimのカーソルを動かすと、その行を含むブロックが画面になければ、上から1/3の位置に来るようにスクロールする。すでに見えていれば動かさない。

Neovimを終了するか、プレビューのウィンドウを閉じると、もう一方も終了する。

## 既知の制限

- ウィンドウは1枚だけで、同時に見られるバッファも1つだけ
- Markdown内の生HTMLは描画しない（そのまま文字として表示される）
- 画像は、文書からの相対パスと `https:` のものだけ表示する。ほかは代替テキストになる。拡張子は png / jpg / jpeg / gif / webp / svg に限る
- スクロール同期はブロック単位。長い段落や表の内部では、位置が概算になる
- 図や数式に構文エラーがあると、そのブロックだけにエラーを表示する
- 配布用パッケージ、署名、自動更新は用意しない
- Windowsには対応しない
- 実行ファイルは単体では起動できない。Neovimが `--nvim <ソケット> --token <トークン>` を付けて起動する前提なので、`cargo tauri dev` は使わない

## 開発

```sh
cd src-tauri && cargo test   # Rust
cd ui && npm test            # フロントエンド（vitest）
```
