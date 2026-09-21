# MdPeek

Neovimで編集中のMarkdownを、GitHub風の見た目で別ウィンドウにプレビューするツール。保存しなくても、入力が止まると表示が追いつく。

- 本文はNeovimのバッファから直接受け取る（ファイルは読まない）
- Mermaidの図、KaTeXの数式、画像、ダークテーマに対応する
- Neovimのカーソルにプレビューが追従し、プレビューの修飾クリックでNeovimがその行へ飛ぶ

対応OSは macOS と Linux。Neovimは0.10以上が必要。

---

## インストール

やることは2つだけ。

1. **実行ファイルを置く** … 下のOSごとの手順
2. **Neovimプラグインを入れる** … OS共通

ビルドは不要。自分でビルドしたい場合は[自分でビルドする](#自分でビルドする)を見る。

### 1. 実行ファイルを置く

#### macOS

Intel と Apple Silicon の両方で動く実行ファイルを1つ置いている。

```sh
VERSION=v0.1.0
curl -LO "https://github.com/tkrkmb/mdpeek/releases/download/${VERSION}/mdpeek-${VERSION}-universal-macos.tar.gz"
tar -xzf "mdpeek-${VERSION}-universal-macos.tar.gz"
mkdir -p ~/bin
mv "mdpeek-${VERSION}-universal-macos/mdpeek" ~/bin/
```

追加で入れるものはない（WebViewはmacOS標準のものを使う）。

> **ブラウザでダウンロードした場合**は、次の1行が必要になる。署名も公証もしていないため、macOSが隔離属性を付けるため。上の `curl` で取得したときは付かない。
>
> ```sh
> xattr -dr com.apple.quarantine ~/bin/mdpeek
> ```

#### Linux（x86_64）

WebViewにWebKitGTKを使うので、実行時ライブラリを先に入れる。

```sh
sudo dnf install webkit2gtk4.1   # Fedora
```

ほかのディストリビューションでは、WebKitGTK 4.1 の実行時ライブラリを入れる。

そのうえで実行ファイルを置く。

```sh
VERSION=v0.1.0
curl -LO "https://github.com/tkrkmb/mdpeek/releases/download/${VERSION}/mdpeek-${VERSION}-x86_64-linux.tar.gz"
tar -xzf "mdpeek-${VERSION}-x86_64-linux.tar.gz"
mkdir -p ~/bin
mv "mdpeek-${VERSION}-x86_64-linux/mdpeek" ~/bin/
```

最新の版は[Releases](https://github.com/tkrkmb/mdpeek/releases)で確認する。

### 2. Neovimプラグインを入れる

設定項目は `bin`（実行ファイルのパス）だけ。

**lazy.nvim の場合**

```lua
{
  "tkrkmb/mdpeek",
  config = function()
    require("mdpeek").setup({ bin = vim.fn.expand("~/bin/mdpeek") })
  end,
}
```

**プラグインマネージャを使わない場合**

リポジトリのルートがそのままプラグインになっている。クローンして `runtimepath` に足す。

```lua
vim.opt.runtimepath:append(vim.fn.expand("~/src/mdpeek"))
require("mdpeek").setup({ bin = vim.fn.expand("~/bin/mdpeek") })
```

### 3. 動かしてみる

Markdownファイルを開いて `:MdPeek` を実行する。別ウィンドウにプレビューが出れば導入は完了。

---

## 使い方

**Neovim側のコマンド**

| コマンド | 動き |
| --- | --- |
| `:MdPeek` | 現在のバッファをプレビューする。すでに開いていれば、表示対象をこのバッファに切り替える |
| `:MdPeekClose` | プレビューを閉じる |

対象にできるのは、`buftype` が空で、ファイル名があり、`filetype` が `markdown` のバッファだけ。それ以外は理由を通知して何もしない。

**プレビュー側の操作**

| 操作 | 動き |
| --- | --- |
| `T` | テーマを system → light → dark の順に切り替える（選択は保存される） |
| Cmd+クリック（macOS）<br>Ctrl+クリック（Linux） | Neovimのカーソルをその行へ移動する |
| ふつうのクリック | `#` のリンクはページ内を移動し、`http:` / `https:` は既定のブラウザで開く |

**自動でおきること**

- 入力が200ms止まると、表示が追いつく（保存は不要）
- Neovimのカーソルを動かすと、その行を含むブロックが画面になければ、上から1/3の位置に来るようにスクロールする。すでに見えていれば動かさない
- Neovimを終了するか、プレビューのウィンドウを閉じると、もう一方も終了する

---

## 自分でビルドする

**必要なもの**

- Node.js と npm
- Rust
- Tauri CLI
- Linuxでは、WebKitGTKなどの開発パッケージ

Fedoraの場合:

```sh
sudo dnf install nodejs24 nodejs24-npm
sudo dnf install webkit2gtk4.1-devel gtk3-devel libsoup3-devel \
  librsvg2-devel openssl-devel gcc gcc-c++ make file
```

ほかのディストリビューションでは、[Tauriの前提条件](https://v2.tauri.app/start/prerequisites/)を参照する。macOSでは、Xcodeのコマンドラインツールがあればよい。

RustとTauri CLI:

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"
cargo install tauri-cli --locked --version "^2.0"
```

**ビルド**

```sh
git clone https://github.com/tkrkmb/mdpeek.git
cd mdpeek/ui && npm install
cd .. && cargo tauri build --no-bundle
```

実行ファイルは `src-tauri/target/release/mdpeek` にできる。フロントエンドのビルドは `cargo tauri build` が自動で実行する。配布用のパッケージは作らず、この実行ファイルをそのまま使う。

`v` で始まるタグを押すと、GitHub ActionsがLinuxとmacOSの実行ファイルをビルドし、Releaseに添付する。

---

## 既知の制限

- ウィンドウは1枚だけで、同時に見られるバッファも1つだけ
- Markdown内の生HTMLは描画しない（そのまま文字として表示される）
- 画像は、文書からの相対パスと `https:` のものだけ表示する。ほかは代替テキストになる。拡張子は png / jpg / jpeg / gif / webp / svg に限る
- スクロール同期はブロック単位。長い段落や表の内部では、位置が概算になる
- 図や数式に構文エラーがあると、そのブロックだけにエラーを表示する
- 配布用パッケージ、署名、公証、自動更新は用意しない
- Windowsには対応しない
- 実行ファイルは単体では起動できない。Neovimが `--nvim <ソケット> --token <トークン>` を付けて起動する前提なので、`cargo tauri dev` は使わない

---

## 開発

```sh
cd src-tauri && cargo test   # Rust
cd ui && npm test            # フロントエンド（vitest）
```
