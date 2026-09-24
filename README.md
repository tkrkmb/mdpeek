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

実行ファイルとLuaプラグインは、1つのアーカイブにまとめて配布しています。これをNeovimのパッケージ用ディレクトリ（`pack/*/start/`）に展開すると、プラグインは起動時に自動で読み込まれます。プラグインマネージャーは不要です。

以下はv0.4.0の例です。別の版を使う場合は、[Releases](https://github.com/tkrkmb/mdpeek/releases)で版を選び、`VERSION`をその値にしてください。

### 1. アーカイブを展開する

macOS：

```sh
VERSION=v0.4.0
DIR=~/.local/share/nvim/site/pack/mdpeek/start/mdpeek
mkdir -p "$DIR"
curl -fL "https://github.com/tkrkmb/mdpeek/releases/download/${VERSION}/mdpeek-${VERSION}-universal-macos.tar.gz" \
  | tar -xz --strip-components=1 -C "$DIR"
```

Linux：

```sh
VERSION=v0.4.0
DIR=~/.local/share/nvim/site/pack/mdpeek/start/mdpeek
mkdir -p "$DIR"
curl -fL "https://github.com/tkrkmb/mdpeek/releases/download/${VERSION}/mdpeek-${VERSION}-x86_64-linux.tar.gz" \
  | tar -xz --strip-components=1 -C "$DIR"
```

### 2. Neovimの設定に追記する

`~/.config/nvim/init.lua`に、次の1行を追記します。

```lua
require("mdpeek").setup({ bin = vim.fn.stdpath("data") .. "/site/pack/mdpeek/start/mdpeek/mdpeek" })
```

### 3. 確認する

Markdownファイルを開いて`:MdPeek`を実行します。別ウィンドウに本文が表示されれば完了です。

### 更新と削除

- 更新：`VERSION`を変えて、手順1のコマンドをもう一度実行します。
- 削除：`~/.local/share/nvim/site/pack/mdpeek`を削除し、手順2の1行を消します。

### 補足

**データディレクトリを変更している場合**：`:lua print(vim.fn.stdpath("data"))`で表示されるディレクトリを、手順1の`~/.local/share/nvim`の代わりに使ってください。

**init.vimを使っている場合**：手順2では、`init.vim`に次の1行を追記します。

```vim
lua require("mdpeek").setup({ bin = vim.fn.stdpath("data") .. "/site/pack/mdpeek/start/mdpeek/mdpeek" })
```

**macOSでブラウザからダウンロードした場合**：署名と公証をしていないため、隔離属性が付いて起動できないことがあります。配布元を確認したうえで、次を実行してください（`curl`で取得した場合は不要です）。

```sh
xattr -dr com.apple.quarantine ~/.local/share/nvim/site/pack/mdpeek/start/mdpeek/mdpeek
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
DIR=~/.local/share/nvim/site/pack/mdpeek/start/mdpeek
mkdir -p "$DIR"
cp -R lua src-tauri/target/release/mdpeek "$DIR/"
```

フロントエンドは、`cargo tauri build`の中で自動的にビルドされます。配置先は配布版と同じなので、`setup`の書き方も同じです。

## 開発

テスト：

```sh
(cd src-tauri && cargo test)   # Rust
(cd ui && npm test)            # フロントエンド（vitest）
```

リリース：`v`で始まるタグをpushすると、GitHub ActionsがLinuxとmacOSの実行ファイルをビルドし、GitHubのReleaseに添付します。
