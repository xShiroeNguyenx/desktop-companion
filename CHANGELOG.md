# Changelog

All notable changes to Desktop Companion are documented here.
The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [0.3.0] - 2026-09-12

### Added
- **Radial menu on the selection flower.** Press-and-hold the 🌸 now fans the
  actions out on a ring around the cursor instead of opening the 3-tab panel:
  🌐 translate · 📋 copy · 💬 reply · 📝 save as task · 🗂️ full panel. Each entry
  goes straight to the window it needs — picking **Reply** or **Save as task**
  lands you on that tab, rather than always on Translate.
- **Radial right-click menu on the pet**, ported from the VS Code companion.
  Hovering an icon shows its name in the centre pill; clicking a category swaps
  the ring for that category's entries and turns the pill into a back button.
  The classic list menu is still reachable by setting `window.__MENU_STYLE__`
  to `'list'`.

### Changed
- **The 📋 icon above the flower is gone.** Copying now lives on the radial menu
  (📋), and **Ctrl+C** while the flower is on screen still works exactly as
  before. The flower window shrank to a single 30×30 button, so the flower lands
  directly on the cursor instead of being offset upward to make room for the icon.

### Fixed
- **The full panel honours the tab it was asked for.** `popup-refresh` carried a
  tab name from Rust but the popup ignored it and always reset to Translate.

## [0.2.3] - 2026-07-01

### Fixed
- **The selection flower no longer steals keyboard focus.** When you drag-select
  text, the 🌸 flower now pops up _without_ pulling focus away from the app you
  selected in — your caret and the highlighted text stay exactly where they were,
  so you can keep typing or editing right away. Previously the (pre-created,
  reused) flower window called `set_focus()` on every show, which activated the
  overlay and moved focus off the highlighted paragraph. The flower stays fully
  clickable thanks to its `WS_EX_NOACTIVATE` style and remains on top via
  `always_on_top`.

## [0.2.2] - 2026-06-29

### Added
- **Copy the original selected text.** A 📋 icon now appears above the flower as
  soon as text is captured. Click it — or press **Ctrl+C** while the flower is on
  screen — to put the captured text on the clipboard. This recovers the selection
  that the auto-capture would otherwise discard (the capture restores your
  previous clipboard, and the on-screen highlight is often cleared when the flower
  appears).
- **"Copy original text" button** in both the quick-translate window and the full
  3-tab panel, next to the existing "copy translation" button.

### Fixed
- **Saving Settings no longer shows _"The system cannot find the file specified.
  (os error 2)"_** on a fresh install. The "Start with Windows" (autostart) toggle
  used to unconditionally delete a registry entry that does not exist yet on a new
  machine; it now only updates the entry when needed.

### Notes
- The Ctrl+C copy shortcut is registered **only while the flower is visible** and
  is released the moment it disappears, so it never permanently hijacks the system
  Ctrl+C. While the flower is on screen (a few seconds), Ctrl+C copies the captured
  text instead of the focused app's selection.

## [0.2.1] - 2026-06-28

### Added
- **Signed auto-updates.** The app checks GitHub Releases for a newer signed
  version on launch and offers to update; Settings also has a manual
  "Kiểm tra cập nhật / Check for updates" button.

## [0.2.0] - 2026-06-27

### Added
- Initial public release: Live2D desktop pet with select-to-translate, quick
  reply, save-task, translation contexts, look-at-cursor, `Ctrl`+wheel resize,
  custom Live2D models, interaction sounds, Anthropic Claude / Google Gemini
  providers, and a customizable capture hotkey.
- Landing page published to GitHub Pages.

[0.3.0]: https://github.com/xShiroeNguyenx/desktop-companion/releases/tag/v0.3.0
[0.2.3]: https://github.com/xShiroeNguyenx/desktop-companion/releases/tag/v0.2.3
[0.2.2]: https://github.com/xShiroeNguyenx/desktop-companion/releases/tag/v0.2.2
[0.2.1]: https://github.com/xShiroeNguyenx/desktop-companion/releases/tag/v0.2.1
[0.2.0]: https://github.com/xShiroeNguyenx/desktop-companion/releases/tag/v0.2.0
