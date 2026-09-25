# MdSight

English | [日本語](README.ja.md)

Preview the Markdown you are editing in Neovim in a separate, GitHub-style window, without saving. The preview scrolls to follow your cursor, and a click in the preview moves Neovim to that line. `mdsight <file>` also works as a read-only viewer without Neovim.

## Installation

Supports macOS (Intel / Apple Silicon) and Linux (x86_64), with Neovim 0.10 or later. Linux needs WebKitGTK 4.1 (on Fedora: `sudo dnf install webkit2gtk4.1`).

Extract the executable and the plugin into a Neovim package directory. No plugin manager is needed. The commands always fetch the latest release.

```sh
DIR=~/.local/share/nvim/site/pack/mdsight/start/mdsight
mkdir -p "$DIR"
# macOS
curl -fL https://github.com/tkrkmb/mdsight/releases/latest/download/mdsight-universal-macos.tar.gz \
  | tar -xz --strip-components=1 -C "$DIR"
# Linux
curl -fL https://github.com/tkrkmb/mdsight/releases/latest/download/mdsight-x86_64-linux.tar.gz \
  | tar -xz --strip-components=1 -C "$DIR"
```

Add this line to `~/.config/nvim/init.lua` (in `init.vim`, prefix it with `lua `).

```lua
require("mdsight").setup({ bin = vim.fn.stdpath("data") .. "/site/pack/mdsight/start/mdsight/mdsight" })
```

Open a Markdown file and run `:MdSight`. If the text shows up in a new window, you are done.

If you have changed your data directory, replace `~/.local/share/nvim` with the output of `:lua print(vim.fn.stdpath("data"))`. On macOS, if you downloaded the archive with a browser, the unsigned executable may be refused. In that case, run `xattr -dr com.apple.quarantine ~/.local/share/nvim/site/pack/mdsight/start/mdsight/mdsight`.

### Updating and uninstalling

| Task | How |
| --- | --- |
| Show the installed version | `~/.local/share/nvim/site/pack/mdsight/start/mdsight/mdsight --version` |
| Update to the latest version | Run the extract command again (see [CHANGELOG.md](CHANGELOG.md) for changes) |
| Install a specific version | Replace `latest/download` in the URL with `download/<version>` (versions are listed under [Releases](https://github.com/tkrkmb/mdsight/releases)) |
| Uninstall | Delete `~/.local/share/nvim/site/pack/mdsight` and remove the line from `init.lua` |

## Usage

### From Neovim

| Command | Action |
| --- | --- |
| `:MdSight` | Preview the current buffer. If the preview is already open, switch it to this buffer and bring a hidden window to the front (input stays in Neovim) |
| `:MdSightClose` | Close the preview |

Only normal buffers with a file name and `filetype` set to `markdown` can be previewed. The preview updates about 200 ms after you stop typing. It scrolls only when the block under the cursor leaves the screen, placing that block one third of the way down. The window title is `MdSight — Linked to Neovim`.

### Opening a file directly

```sh
mdsight path/to/note.md
```

The window opens in the background and the command returns immediately. The executable is at `~/.local/share/nvim/site/pack/mdsight/start/mdsight/mdsight`; add it to your PATH or create an alias.

- The preview updates automatically when the file is saved again.
- If a read-only window (titled `MdSight — Read Only`) already shows the same file, it is brought to the front instead of opening a new one. Neovim previews are not affected.
- With `--foreground`, the command keeps the terminal and prints errors there.

### In the preview

| Input | Action |
| --- | --- |
| `T` | Cycle the theme: system → light → dark |
| Cmd+click (macOS), Ctrl+click (Linux) | Move the Neovim cursor to the first line of the clicked block |
| Click a relative `.md` / `.markdown` link | Open that file (and jump to `#heading` if given) |
| Click an `http:` / `https:` link | Open it in the default browser |
| ‹ › at the top left, Cmd+[ / Cmd+] (macOS), Alt+← / Alt+→ (Linux), two-finger horizontal swipe | Go back / forward through documents opened from links |

When you follow a link in a Neovim preview, Neovim switches to the same file too. If Neovim cannot open it (for example, because of unsaved changes), a notification appears and the preview stays as it was. Switching the target with `:MdSight` clears the back/forward history.

The path of the shown file appears at the top of the window. If a link cannot be opened or the file cannot be reloaded, the reason appears at the bottom.

## Known limitations

- Raw HTML is not rendered.
- Only relative and `https:` images are shown, and only png / jpg / jpeg / gif / webp / svg.
- Scroll sync works per block, so it is approximate inside long paragraphs and tables.
- Going back through links opens the document at the top (or at `#heading`); the previous reading position is not remembered.
- The preview does not follow when you move to another file in Neovim with `gf` or `Ctrl-O`. Run `:MdSight` again.
- On Linux Wayland, `:MdSight` may not bring a hidden preview to the front, because the OS does not allow raising a window without giving it focus.
- The icon follows the preview theme (light / dark), but on Linux Wayland it may not change, because some environments do not use the window icon.

## Building from source

You need Rust, the Tauri CLI (`cargo install tauri-cli --locked --version "^2.0"`), and Node.js. macOS also needs the Xcode command line tools. On Fedora, Linux needs the following development packages; for other systems, see the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/).

```sh
sudo dnf install nodejs24 nodejs24-npm webkit2gtk4.1-devel gtk3-devel libsoup3-devel \
  librsvg2-devel openssl-devel gcc gcc-c++ make file
```

Build it and place it where the release archive goes. The `setup` line is the same.

```sh
git clone https://github.com/tkrkmb/mdsight.git
cd mdsight
(cd ui && npm install)
cargo tauri build --no-bundle   # also builds the frontend
DIR=~/.local/share/nvim/site/pack/mdsight/start/mdsight
mkdir -p "$DIR"
cp -R lua src-tauri/target/release/mdsight "$DIR/"
```

Run the tests with `(cd src-tauri && cargo test)` and `(cd ui && npm test)`. Pushing a tag starting with `v` makes GitHub Actions build the Linux and macOS versions and attach them to a release.

CI generates the license notices for bundled libraries (`THIRD_PARTY_NOTICES.md`). To generate them locally:

```sh
cargo install cargo-about --locked --version 0.9.2 --features cli
(cd ui && npm run build)
scripts/third-party-notices.sh
```

## License

MdSight is under the [MIT License](LICENSE). Licenses for Tauri, comrak, mermaid, KaTeX, and the other libraries in the executable are in `THIRD_PARTY_NOTICES.md` inside the release archive.

[nvim-rs](https://github.com/KillTheMule/nvim-rs), used to talk to Neovim, is under LGPL-3.0. All source code is public, so you can replace nvim-rs and rebuild (see "Building from source" above).
