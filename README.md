# MdPeek

Neovimで編集中のMarkdownを、GitHub風のスタイルで別ウィンドウに表示します。ファイルを保存しなくても、変更がすぐに反映されます。

## 機能

- 本文の編集に応じた自動更新
- Mermaidによる図、KaTeXによる数式、画像の表示
- ライトテーマとダークテーマの切り替え
- Neovimのカーソル位置に連動したスクロール
- 修飾キーを押しながらのクリックによる、Neovimの該当行への移動

## 必要なもの

- macOS（Intel / Apple Silicon）、またはLinux（x86_64）
- Neovim 0.10以上
- Linuxのみ：WebKitGTK 4.1（Fedoraでは`sudo dnf install webkit2gtk4.1`）

## インストール

次の2つを配置し、Neovimの設定に1行追加します。

- **実行ファイル**：プレビューを表示するアプリ。`~/.local/bin/mdpeek`に置きます。
- **Luaプラグイン**：`:MdPeek`などのコマンドを追加します。`~/.config/nvim/lua/mdpeek/`に置きます。

以下はv0.3.0の例です。別の版を使う場合は、[Releases](https://github.com/tkrkmb/mdpeek/releases)で版を選び、手順1と2の`VERSION`を同じ値にしてください。

### 1. 実行ファイルを置く

macOS：

```sh
VERSION=v0.3.0
curl -fLO "https://github.com/tkrkmb/mdpeek/releases/download/${VERSION}/mdpeek-${VERSION}-universal-macos.tar.gz"
tar -xzf "mdpeek-${VERSION}-universal-macos.tar.gz"
mkdir -p ~/.local/bin
mv "mdpeek-${VERSION}-universal-macos/mdpeek" ~/.local/bin/
```

Linux：

```sh
VERSION=v0.3.0
curl -fLO "https://github.com/tkrkmb/mdpeek/releases/download/${VERSION}/mdpeek-${VERSION}-x86_64-linux.tar.gz"
tar -xzf "mdpeek-${VERSION}-x86_64-linux.tar.gz"
mkdir -p ~/.local/bin
mv "mdpeek-${VERSION}-x86_64-linux/mdpeek" ~/.local/bin/
```

### 2. Luaプラグインを置く

実行ファイルのアーカイブにはLuaプラグインが入っていません。同じ版のソースアーカイブから、`lua`ディレクトリだけを取り出します。

```sh
VERSION=v0.3.0
curl -fL "https://github.com/tkrkmb/mdpeek/archive/refs/tags/${VERSION}.tar.gz" -o mdpeek-plugin.tar.gz
mkdir -p ~/.config/nvim
tar -xzf mdpeek-plugin.tar.gz --strip-components=1 -C ~/.config/nvim "mdpeek-${VERSION#v}/lua"
```

### 3. Neovimの設定に追記する

`~/.config/nvim/init.lua`に、次の1行を追記します。

```lua
require("mdpeek").setup({ bin = vim.fn.expand("~/.local/bin/mdpeek") })
```

### 4. 確認する

Markdownファイルを開いて`:MdPeek`を実行します。別ウィンドウに本文が表示されれば完了です。

### 補足

**設定ディレクトリを変更している場合**：`:lua print(vim.fn.stdpath("config"))`で表示されるディレクトリを、手順2と3の`~/.config/nvim`の代わりに使ってください。

**init.vimを使っている場合**：手順3では、`init.vim`に次の1行を追記します。

```vim
lua require("mdpeek").setup({ bin = vim.fn.expand("~/.local/bin/mdpeek") })
```

**macOSでブラウザからダウンロードした場合**：署名と公証をしていないため、隔離属性が付いて起動できないことがあります。配布元を確認したうえで、次を実行してください（`curl`で取得した場合は不要です）。

```sh
xattr -dr com.apple.quarantine ~/.local/bin/mdpeek
```

## 使い方

### Neovimのコマンド

| コマンド | 動作 |
| --- | --- |
| `:MdPeek` | 現在のバッファをプレビューします。プレビューが開いていれば、表示するバッファを切り替えます。 |
| `:MdPeekClose` | プレビューを閉じます。 |

`:MdPeek`の対象にできるのは、`filetype`が`markdown`で、ファイル名が付いている通常のバッファだけです。

### プレビューでの操作

| 操作 | 動作 |
| --- | --- |
| `T` | テーマを system → light → dark の順に切り替えます。 |
| Cmd+クリック（macOS）<br>Ctrl+クリック（Linux） | Neovimのカーソルを、クリックしたブロックの開始行へ移動します。 |
| リンクのクリック | `#`で始まるリンクはページ内へ移動し、`http:` / `https:`のリンクは既定のブラウザで開きます。 |

### 自動で行われること

- 入力が約200ms止まると、表示を更新します（保存は不要です）。
- Neovimのカーソルに追従してスクロールします。カーソルのあるブロックが画面外にあるときだけ、そのブロックを画面の上から1/3の位置に表示します。

## 既知の制限

- ウィンドウは1枚だけで、同時に表示できるバッファも1つだけです。
- 生HTMLは描画しません。
- 画像は、相対パスと`https:`のものだけを表示します。拡張子は png / jpg / jpeg / gif / webp / svg に限ります。
- スクロール同期はブロック単位です。長い段落や表の中では、位置がおおよそになります。
- Neovimから起動して使う専用のアプリです。単体では起動できません。

## ソースコードからのビルド

### 準備

- macOS：Xcodeコマンドラインツール
- Linux：Node.jsと、WebKitGTKなどの開発パッケージ。Fedoraの例：

  ```sh
  sudo dnf install nodejs24 nodejs24-npm
  sudo dnf install webkit2gtk4.1-devel gtk3-devel libsoup3-devel \
    librsvg2-devel openssl-devel gcc gcc-c++ make file
  ```

- 共通：RustとTauri CLI

  ```sh
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
  source "$HOME/.cargo/env"
  cargo install tauri-cli --locked --version "^2.0"
  ```

その他の環境については、[Tauriの前提条件](https://v2.tauri.app/start/prerequisites/)を参照してください。

### ビルド

```sh
git clone https://github.com/tkrkmb/mdpeek.git
cd mdpeek
(cd ui && npm install)
cargo tauri build --no-bundle
mkdir -p ~/.local/bin
cp src-tauri/target/release/mdpeek ~/.local/bin/mdpeek
```

フロントエンドは、`cargo tauri build`の中で自動的にビルドされます。Luaプラグインは、クローンしたリポジトリの`lua/mdpeek/`を`~/.config/nvim/lua/`にコピーしてください。

## 開発

テスト：

```sh
(cd src-tauri && cargo test)   # Rust
(cd ui && npm test)            # フロントエンド（vitest）
```

リリース：`v`で始まるタグをpushすると、GitHub ActionsがLinuxとmacOSの実行ファイルをビルドし、GitHubのReleaseに添付します。
