# MdPeek

Neovimで編集中のMarkdownを、保存しなくてもGitHub風の表示で別ウィンドウにプレビューします。カーソル位置に追従してスクロールし、プレビュー側のクリックでNeovimの該当行へ移動できます。`mdpeek ファイル名`で、Neovimなしの読み取り専用ビューアとしても使えます。

## インストール

macOS（Intel / Apple Silicon）とLinux（x86_64）、Neovim 0.10以上に対応しています。LinuxではWebKitGTK 4.1が必要です（Fedoraなら`sudo dnf install webkit2gtk4.1`）。

実行ファイルとプラグインを、Neovimのパッケージ用ディレクトリに展開します。プラグインマネージャーは要りません。コマンドは常に最新の版を取得します。

```sh
DIR=~/.local/share/nvim/site/pack/mdpeek/start/mdpeek
mkdir -p "$DIR"
# macOS
curl -fL https://github.com/tkrkmb/mdpeek/releases/latest/download/mdpeek-universal-macos.tar.gz \
  | tar -xz --strip-components=1 -C "$DIR"
# Linux
curl -fL https://github.com/tkrkmb/mdpeek/releases/latest/download/mdpeek-x86_64-linux.tar.gz \
  | tar -xz --strip-components=1 -C "$DIR"
```

`~/.config/nvim/init.lua`に次の1行を足します（`init.vim`なら先頭に`lua `を付けます）。

```lua
require("mdpeek").setup({ bin = vim.fn.stdpath("data") .. "/site/pack/mdpeek/start/mdpeek/mdpeek" })
```

Markdownファイルを開いて`:MdPeek`を実行し、別ウィンドウに本文が出れば完了です。

データディレクトリを変えている場合は、`~/.local/share/nvim`を`:lua print(vim.fn.stdpath("data"))`の結果に置き換えてください。macOSでブラウザからダウンロードした場合は、署名していないため起動を拒まれることがあります。そのときは`xattr -dr com.apple.quarantine ~/.local/share/nvim/site/pack/mdpeek/start/mdpeek/mdpeek`を実行してください。

### 更新・削除

| やること | 方法 |
| --- | --- |
| 入っている版を見る | `~/.local/share/nvim/site/pack/mdpeek/start/mdpeek/mdpeek --version` |
| 最新の版に更新する | 展開のコマンドをもう一度実行する（変更点は[CHANGELOG.md](CHANGELOG.md)） |
| 特定の版を入れる | URLの`latest/download`を`download/<版>`に置き換える（版は[Releases](https://github.com/tkrkmb/mdpeek/releases)） |
| 削除する | `~/.local/share/nvim/site/pack/mdpeek`を消し、`init.lua`の1行を消す |

## 使い方

### Neovimから

| コマンド | 動作 |
| --- | --- |
| `:MdPeek` | 現在のバッファをプレビューする。開いていれば表示するバッファを切り替え、隠れていたウィンドウを前に出す（入力はNeovimに残る） |
| `:MdPeekClose` | プレビューを閉じる |

対象にできるのは、`filetype`が`markdown`でファイル名のある通常のバッファだけです。入力が約200ms止まると表示が更新されます。カーソルのあるブロックが画面外に出たときだけ、そのブロックが画面の上から1/3に来るようにスクロールします。ウィンドウのタイトルは`MdPeek — Linked to Neovim`です。

### ファイルを直接開く

```sh
mdpeek path/to/note.md
```

ウィンドウはバックグラウンドで開き、コマンドはすぐに終わります。実行ファイルは`~/.local/share/nvim/site/pack/mdpeek/start/mdpeek/mdpeek`にあるので、PATHに加えるかエイリアスを作ってください。

- ファイルを保存し直すと、表示が自動で更新されます。
- 同じファイルを表示している読み取り専用のウィンドウ（タイトルが`MdPeek — Read Only`）があれば、新しく開かずにそれを前面に出します。Neovimのプレビューは対象外です。
- `--foreground`を付けると、ターミナルを占有したまま動き、エラーもターミナルに出ます。

### プレビューでの操作

| 操作 | 動作 |
| --- | --- |
| `T` | テーマを system → light → dark の順に切り替える |
| Cmd+クリック（macOS）、Ctrl+クリック（Linux） | Neovimのカーソルを、クリックしたブロックの開始行へ移す |
| 相対パスの`.md`／`.markdown`リンクをクリック | そのファイルを開く（`#見出し`があればそこへ移動） |
| `http:`／`https:`のリンクをクリック | 既定のブラウザで開く |
| 左上の‹ ›、Cmd+[ / Cmd+]（macOS）、Alt+← / Alt+→（Linux）、2本指の横スワイプ | リンクで開いた文書を戻る／進む |

Neovimのプレビューでリンクを辿ると、Neovim側の対象も同じファイルに切り替わります。保存していない変更があるなどでNeovimが開けなかったときは、通知が出てプレビューも元のままです。`:MdPeek`で対象を切り替えると、戻る／進むの履歴は空になります。

ウィンドウの上端に、表示中のファイルのパスが出ます。リンクを開けなかったときや、ファイルを読み直せないときは、その理由が下端に出ます。

## 既知の制限

- 生HTMLは描画しません。
- 画像は、相対パスと`https:`のものだけを表示します。拡張子は png / jpg / jpeg / gif / webp / svg に限ります。
- スクロール同期はブロック単位です。長い段落や表の中では位置がおおよそになります。
- リンクを辿って戻ると、文書の先頭（または`#見出し`）に移動します。読んでいた位置は覚えていません。
- Neovimで`gf`や`Ctrl-O`で別のファイルへ移っても、プレビューは追従しません。`:MdPeek`をやり直してください。
- LinuxのWaylandでは、`:MdPeek`で隠れたプレビューが前に出ないことがあります。フォーカスを移さずにウィンドウを前に出すことを、OSが許さないためです。
- アイコンは、プレビューのテーマ（ライト／ダーク）に合わせて切り替わります。ただしLinuxのWaylandでは、切り替わらないことがあります。ウィンドウのアイコンを使わない環境があるためです。

## ソースコードからのビルド

RustとTauri CLI（`cargo install tauri-cli --locked --version "^2.0"`）、Node.jsが必要です。macOSではXcodeコマンドラインツールも要ります。Linuxで必要な開発パッケージは、Fedoraなら次のとおりです。ほかの環境は[Tauriの前提条件](https://v2.tauri.app/start/prerequisites/)を参照してください。

```sh
sudo dnf install nodejs24 nodejs24-npm webkit2gtk4.1-devel gtk3-devel libsoup3-devel \
  librsvg2-devel openssl-devel gcc gcc-c++ make file
```

ビルドして、配布版と同じ場所に置きます。`setup`の書き方も同じです。

```sh
git clone https://github.com/tkrkmb/mdpeek.git
cd mdpeek
(cd ui && npm install)
cargo tauri build --no-bundle   # フロントエンドも一緒にビルドされる
DIR=~/.local/share/nvim/site/pack/mdpeek/start/mdpeek
mkdir -p "$DIR"
cp -R lua src-tauri/target/release/mdpeek "$DIR/"
```

テストは`(cd src-tauri && cargo test)`と`(cd ui && npm test)`です。`v`で始まるタグをpushすると、GitHub ActionsがLinuxとmacOSの版をビルドしてReleaseに添付します。

同梱ライブラリのライセンス表示（`THIRD_PARTY_NOTICES.md`）はCIが生成します。手元で作るときは次のとおりです。

```sh
cargo install cargo-about --locked --version 0.9.2 --features cli
(cd ui && npm run build)
scripts/third-party-notices.sh
```

## ライセンス

MdPeekは[MIT License](LICENSE)です。実行ファイルに含まれるTauri、comrak、mermaid、KaTeXなどのライセンスは、リリースのアーカイブ内の`THIRD_PARTY_NOTICES.md`にあります。

Neovimとの通信に使う[nvim-rs](https://github.com/KillTheMule/nvim-rs)はLGPL-3.0です。ソースコードをすべて公開しているので、nvim-rsを差し替えて作り直せます（上の「ソースコードからのビルド」）。
