# Changelog

English | [日本語](CHANGELOG.ja.md)

User-visible changes to MdSight, by version.
The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

To release a new version, rename the "Unreleased" heading to `## [vX.Y.Z] - YYYY-MM-DD` and then tag it. The GitHub release notes are taken from that section.

## [Unreleased]

### Added

- A copy button appears at the top right of a code block when you hover over it.
- Vim keys in the preview: `j` / `k`, `Ctrl-d` / `Ctrl-u` and `gg` / `G` scroll, `/` opens find in page, and `n` / `N` move between matches. They do not move the Neovim cursor.
- Going back or forward returns to where you were reading in that document, instead of its top.
- Find in page accepts Neovim search patterns (Vim regular expressions such as `\<word\>`, `foo\|bar`, `\v`, `\zs`, `\c`). Unsupported or invalid patterns show "Invalid pattern".

### Fixed

- Find in page no longer matches across the end of one paragraph and the start of the next.

## [v0.10.0] - 2026-09-26

### Added

- YAML front matter is shown as a table at the top, as on GitHub, instead of turning into a rule and a heading. If it is not valid YAML, it is shown as a code block.
- Footnotes (`[^1]`) and alerts (`> [!NOTE]`, `[!TIP]`, `[!IMPORTANT]`, `[!WARNING]`, `[!CAUTION]`) are rendered.
- Code blocks with a language name are syntax highlighted, in light and dark themes.
- Searches in Neovim (`/`, `?`, `*`, `#`) are highlighted in the preview too, with the match under the cursor in a darker color. `:noh` clears them.
- Find in the page with Cmd+F (macOS) or Ctrl+F (Linux). Matches are highlighted, Enter / Shift+Enter moves between them, and the search is kept while the text updates.
- The Neovim preview follows moves to another Markdown file in the previewed window (`gf`, `:e`, `Ctrl-O`, `:b`, and so on) without coming to the front. Moves in other windows are still ignored.

### Changed

- Deleting the previewed buffer with `:bd` no longer closes the preview while the previewed window remains; the preview follows the next Markdown file shown there.

### Fixed

- Following a link or going back / forward in a Neovim preview could show an error even though the document opened, and ignored the `#heading` in the link.

## [v0.9.0] - 2026-09-25

### Changed

- Renamed from MdPeek to MdSight, because other tools already use the name `mdpeek`. The repository, executable, archives, Neovim module and commands all use the new name. To upgrade, install again with the README steps, change your config to `require("mdsight").setup({ bin = ... })` with the new executable path, and use `:MdSight` / `:MdSightClose`. Delete the old `~/.local/share/nvim/site/pack/mdpeek`. The theme chosen with `T` resets to system once.

## [v0.8.0] - 2026-09-25

### Added

- English versions of the README and the changelog. The Japanese versions moved to `README.ja.md` and `CHANGELOG.ja.md`.

### Changed

- New app icon. It switches between light and dark versions to match the preview theme (light / dark). On macOS, the icon now also shows in the Dock for release builds.

### Fixed

- Moving to another document via a link or back/forward now shows it from the top (or from `#heading` for such links), instead of at the reading position of the previous document.

## [v0.7.0] - 2026-09-25

### Added

- Released under the MIT License (`LICENSE`). Release archives now include the license notices for bundled libraries (`THIRD_PARTY_NOTICES.md`).
- `mdpeek --version` shows the version.

### Changed

- Removed the version from the names of the release archives (`mdpeek-universal-macos.tar.gz`, `mdpeek-x86_64-linux.tar.gz`). The installation steps in the README always fetch the latest version.

## [v0.6.0] - 2026-09-25

### Added

- The path of the shown file appears at the top of the window. Long paths are truncated on the left and scroll to show the whole path while the mouse is over them.
- `mdpeek <file>` brings an existing window showing the same file to the front instead of opening a new one.
- Running `:MdPeek` on another buffer in Neovim brings the preview to the front if it is hidden behind other windows. Input stays in Neovim.

### Changed

- Moved the back/forward buttons from the bottom right of the text to the left end of the path bar, so they no longer cover the text.
- Text in the window and output to the terminal are now in English.
- The window title tells a window linked to Neovim (`MdPeek — Linked to Neovim`) from a read-only window that just opened a file (`MdPeek — Read Only`).
- `mdpeek <file>` opens the window in the background and returns immediately. Add `--foreground` to keep it attached to the terminal.
- Problems such as a file that cannot be reloaded or watched are shown at the bottom of the window, and disappear once the file can be reloaded.
- When a link cannot be opened or back/forward fails, the reason is shown at the bottom of the window for a few seconds (previously nothing happened).

## [v0.5.0] - 2026-09-24

### Added

- `mdpeek <file>` starts the app on its own, without Neovim. The preview updates automatically when the file is saved in another editor.
- Clicking a relative `.md` / `.markdown` link opens that document. Links with `#heading` jump to that heading.
- Back/forward through documents opened from links, with the buttons at the bottom right, the keyboard (`Cmd+[` / `Cmd+]` on macOS, `Alt+←` / `Alt+→` on Linux), or a two-finger horizontal swipe on the trackpad.
- Opening a link while previewing from Neovim switches Neovim's target window to the same file.

### Changed

- The minimum Rust version for building from source is now 1.88.0, matching the actual dependencies.

## [v0.4.0] - 2026-09-24

### Changed

- The executable and the Neovim plugin ship in one archive. Extracting it into a Neovim package directory is all it takes to install.
- Reorganized the README so it can be followed from installation to building.

## [v0.3.0] - 2026-09-21

### Fixed

- Fixed the theme sometimes not changing after a single press of `T`.

## [v0.2.0] - 2026-09-21

### Changed

- The text now uses the full width of the window.

### Fixed

- Fixed the cursor line being lost when it was sent before the preview was ready.
- Fixed the preview position not matching the cursor after switching the target with `:MdPeek`.

## [v0.1.0] - 2026-09-21

First release.

### Added

- `:MdPeek` previews the Markdown being edited in Neovim in GitHub style. The preview updates when you stop typing, without saving.
- `:MdPeekClose` closes the preview.
- Mermaid diagrams, KaTeX math, and images (relative paths and `https:`).
- Theme switching between system / light / dark with the `T` key.
- Scrolling that follows the Neovim cursor, and modifier-click to move Neovim to the corresponding line.
- CI that attaches executables for macOS (Intel / Apple Silicon) and Linux (x86_64) to a release from a tag.
