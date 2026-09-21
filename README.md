# MdPeek

Neovimで編集中のMarkdownを、GitHub風のスタイルで別ウィンドウに表示します。ファイルを保存せずに変更を確認できます。
配布バイナリを`~/bin`に置き、`setup({bin=...})`して`:MdPeek`するだけです。
対応OSはmacOSとLinuxです。Windowsには対応しません。Neovim専用で、単体起動や配布パッケージ・署名・自動更新には対応しません。

- 本文の編集に応じた自動更新
- Mermaidによる図、KaTeXによる数式、画像の表示
- ライトテーマとダークテーマの切り替え
- Neovimのカーソル位置に連動したスクロール
- 修飾キーを押しながらのクリックによる、Neovimの該当行への移動

Neovim 0.10以上が必要です。

---

## インストール

1. 実行ファイルを`~/bin`に置きます。
2. `setup`で`bin`を指定します。
3. `:MdPeek`します。

配布バイナリを使う場合、ビルドは不要です。ソースから建てる場合は[ソースコードからのビルド](#ソースコードからのビルド)を見てください。

### 1. 実行ファイルを置く

| 環境 | 補足 |
| --- | --- |
| macOS | ユニバーサルバイナリ（Intel/Apple Silicon対応）。追加ライブラリは不要です。 |
| Linux（x86_64） | WebKitGTK 4.1が必要です。 |

#### macOS

```sh
VERSION=v0.2.0
curl -LO "https://github.com/tkrkmb/mdpeek/releases/download/${VERSION}/mdpeek-${VERSION}-universal-macos.tar.gz"
tar -xzf "mdpeek-${VERSION}-universal-macos.tar.gz"
mkdir -p ~/bin
mv "mdpeek-${VERSION}-universal-macos/mdpeek" ~/bin/
```

> **ブラウザでダウンロードした場合**
>
> 署名・公証をしていないため、macOSが隔離属性を付けます。`curl`で取得した場合は不要です。
>
> ```sh
> xattr -dr com.apple.quarantine ~/bin/mdpeek
> ```

#### Linux

Fedoraでは以下を実行してください。他のディストリビューションでは対応するWebKitGTK 4.1を入れてください。

```sh
sudo dnf install webkit2gtk4.1
```

```sh
VERSION=v0.2.0
curl -LO "https://github.com/tkrkmb/mdpeek/releases/download/${VERSION}/mdpeek-${VERSION}-x86_64-linux.tar.gz"
tar -xzf "mdpeek-${VERSION}-x86_64-linux.tar.gz"
mkdir -p ~/bin
mv "mdpeek-${VERSION}-x86_64-linux/mdpeek" ~/bin/
```

最新版は[Releases](https://github.com/tkrkmb/mdpeek/releases)で確認してください。

### 2. Neovimプラグインの設定

設定項目は`bin`のみです。

| 導入法 | 設定 |
| --- | --- |
| lazy.nvim | プラグイン一覧に`setup({bin=...})`を追加します。 |
| 手動 | リポジトリを`runtimepath`に追加して`setup({bin=...})`します。 |

**lazy.nvimの場合**

```lua
{
  "tkrkmb/mdpeek",
  config = function()
    require("mdpeek").setup({ bin = vim.fn.expand("~/bin/mdpeek") })
  end,
}
```

**手動の場合（例：`~/src/mdpeek`にクローン）**

```lua
vim.opt.runtimepath:append(vim.fn.expand("~/src/mdpeek"))
require("mdpeek").setup({ bin = vim.fn.expand("~/bin/mdpeek") })
```

### 3. 動作確認

Markdownファイルを開いて`:MdPeek`してください。別ウィンドウに出れば完了です。

---

## 使い方

**Neovimでの操作**

| コマンド | 動作 |
| --- | --- |
| `:MdPeek` | 現在のバッファを開く・切り替えます。 |
| `:MdPeekClose` | 閉じます。 |

**プレビューでの操作**

| 操作 | 動作 |
| --- | --- |
| `T` | テーマを system → light → dark の順に切り替えます。 |
| Cmd+クリック（macOS）<br>Ctrl+クリック（Linux） | Neovimのカーソルをそのブロックの開始行へ移動します。 |
| リンク | `#`はページ内移動、`http:` / `https:`は既定のブラウザで開きます。 |

入力が約200ms止まると、保存せずに更新します。Neovimのカーソルに追従し、対応ブロックが見えなければ上から1/3の位置へ移動します。

---

## 開発環境の準備

| 環境 | 準備 |
| --- | --- |
| Linux | WebKitGTKなどの開発パッケージ |
| macOS | Xcodeコマンドラインツール |

Fedoraの例：

```sh
sudo dnf install nodejs24 nodejs24-npm
sudo dnf install webkit2gtk4.1-devel gtk3-devel libsoup3-devel \
  librsvg2-devel openssl-devel gcc gcc-c++ make file
```

その他は[Tauriの前提条件](https://v2.tauri.app/start/prerequisites/)を見てください。

RustとTauri CLI：

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"
cargo install tauri-cli --locked --version "^2.0"
```

## ソースコードからのビルド

手順3つで`src-tauri/target/release/mdpeek`ができます。これを`bin`に指定してください。
フロントエンドのビルドは自動で実行されます。配布用パッケージは作りません。

```sh
git clone https://github.com/tkrkmb/mdpeek.git
cd mdpeek
```

```sh
cd ui && npm install
cd ..
```

```sh
cargo tauri build --no-bundle
```

`v`で始まるタグを押すと、GitHub ActionsがLinuxとmacOSの実行ファイルをビルドし、Releaseに添付します。

---

## 既知の制限

- ウィンドウは1枚で、同時に見られるバッファも1つだけです。
- 生HTMLは描画しません。
- 画像は相対パスと`https:`のみで、拡張子は png / jpg / jpeg / gif / webp / svg に限ります。
- スクロール同期はブロック単位で、長い段落や表の内部では概算になります。
- 図や数式に構文エラーがあると、そのブロックだけにエラーを表示します。

---

## 開発

Rustとフロントエンドのテストは以下です。

```sh
cd src-tauri && cargo test   # Rust
cd ui && npm test            # フロントエンド（vitest）
```
