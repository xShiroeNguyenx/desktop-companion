# Changelog

All notable changes to Desktop Companion are documented here.
The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [0.4.1] - 2026-10-14

### Changed
- **A new app icon** — a pink-haired, cat-eared girl with headphones and a
  lilac laptop, the same art as the VS Code extension's new icon. Every size
  in `src-tauri/icons/` (Windows `.ico`, macOS `.icns`, the PNG sizes, the
  Store logos, Android and iOS) is regenerated with `tauri icon` from one
  1024 × 1024 transparent source; the tray icon reads `icons/icon.png`, so it
  changes too.
- **New fallback art in the pet window** — `app/media/icon.png` and
  `app/media/character.png` (shown while no Live2D model is loaded, and as the
  chat avatar before a chibi is captured) match the new look.

### Fixed
- **A chat reply that never arrives is now really cancelled** — after 30
  seconds without a first word, the bubble still says no reply came, and the
  request is now also stopped in the app, so a late answer no longer slips
  into the chat history behind your back.

## [0.4.0] - 2026-10-13

### Added
- **The pet changes its face while it talks.** The model tags its reply with
  emotions (`[happy]`, `[shy]`, `[surprised]`…); the tags are hidden from the
  bubble and switch the Live2D expression as the reply streams in.
- **Quick Chat remembers the conversation** (last 10 turns), also across
  restarts. It is saved only on this machine, and the 🗑 button in the chat
  panel clears it.
- **"Hỏi em" on the selection flower.** Hold the 🌸 and pick 💭 to ask the pet
  about the text you just selected; the panel shows the attached snippet.
- **The pet knows the time and your unfinished tasks**, so "hôm nay còn việc gì
  không em?" gets a real answer.
- **The pet starts the conversation now and then.** After a quiet stretch
  (Settings → *Em tự bắt chuyện*: off / 30 min / 1 h / 2 h, default 1 h) it says
  one line: nudges a task that has been sitting longest, or reminds you to rest
  your eyes. It stays quiet while hidden, docked in a panel, or mid-reply.
- **Ask the pet to remember a task.** Say "nhắc anh gửi báo cáo cho sếp" and it
  adds the task to your list. The bubble confirms with "📝 Đã lưu task: …" only
  once the task is really saved, or says why it could not be saved. Nothing is
  saved if you stop the reply halfway.
- **A ⏹ button on the bubble stops the pet mid-reply.**

### Changed
- **Quick Chat is one click away.** The right-click menu used to open a "Chat AI"
  ring that held nothing but Quick Chat; **💬 Chat nhanh** now sits directly on
  the first ring.

### Fixed
- **"Hỏi em" works with click-through on.** The chat panel used to ignore your
  clicks and typing. Click-through is now paused while you ask, and turned back
  on once the pet has answered. If the pet is sitting in the Settings or Tasks
  panel, it steps out first so the chat panel has room.
- **Your Gemini API key no longer shows in error messages.** Translate, Quick
  Reply and the model list put the key in the request address, and a
  connection error printed that address on screen. The key now travels in a
  header.
- **Saving Settings no longer forgets "Bỏ qua bản này"**, so a skipped update
  stays skipped.
- **Quick Chat (right-click → Chat AI) now answers.** The question was sent but
  nothing handled it, so the bubble sat on "Em đang suy nghĩ…" until it timed
  out. Replies now stream in word by word from Anthropic or Gemini.

## [0.3.2] - 2026-09-15

### Added
- **An update dialog that opens itself on launch.** The app already checked for
  a new version ~5s after start, but it only nudged you with a pet bubble saying
  to go find the button in Settings — so unless you went looking, you never
  updated. It now opens a centred dialog with the new version number, the one
  you are on, the release notes, and a **Tải & cài ngay** button that downloads
  and installs right there.
- **A progress bar while the update downloads.** The download reports real
  percentages (falling back to an indeterminate sweep when the server sends no
  content length) instead of one line of text and then silence until the app
  restarts itself.
- **"Bỏ qua bản này".** The automatic dialog stays quiet about that exact
  version on later launches; checking manually from Settings clears the skip.

### Changed
- **Kiểm tra cập nhật** in Settings now opens that same dialog — release notes
  and progress bar included — instead of installing from within the panel.

## [0.3.1] - 2026-09-14

### Added
- **The pet steps into the Settings and Tasks windows.** Opening either panel
  moves the live Live2D pet into the gap left between their two columns and
  makes it click-through, so it can't be grabbed or poked while you work. Close
  the panel and the pet returns to its exact previous position and size, with
  interaction restored. Dragging or resizing the panel carries the pet along.
- **Tasks: separate "to do" and "done" lists.** Completing a task moves it to
  the ✅ column, most recently finished first; each column shows a count.
- **Tasks: timestamps.** Every task shows when it was created (`🕒 Tạo`) and,
  once finished, when it was completed (`✓ Xong`). Same-day times show only the
  clock; older ones include the date. Un-checking a task clears its completion
  time and returns it to the to-do column.
- **Tasks: editable notes.** Click a task's note line to edit it in place —
  Enter saves, Shift+Enter adds a line, Escape cancels.
- **Model and Motion pickers as two side columns**, ported from the VS Code
  companion. The pickers used to be one card centred over the character, so
  listing a dozen motions hid the very model you were dressing. They now sit in
  two narrow columns either side, leaving the middle clear, and stay open across
  choices — only × or Escape closes them.

### Changed
- **Settings is a two-column panel.** Provider / API key / model and the system
  options sit on the left, translation and appearance on the right, with the pet
  between them. The window grew to 880×600 and collapses to two even columns
  below 620px wide.
- **The quick-translate window is smaller and auto-sizing.** It now shows only
  the translation by default and grows or shrinks to fit the text instead of
  leaving a fixed empty box. The two "copy" buttons became 📋 icons inside each
  box, and a 👁 icon reveals the original text — which, when shown, is labelled
  so the two boxes can be told apart.

### Fixed
- **The Motion list reflects the model.** It was three hard-coded names
  (`TapBody` / `TapHead` / `Idle`), so a model naming its groups `待机` or `摸头`
  showed three buttons that did nothing. The list is now read from the loaded
  model, with a note when a model declares no motions at all.
- **Docking the pet no longer loses its position.** The window-moved handler
  saves the pet's position on every move, which would have overwritten the
  remembered spot with the temporary one inside a panel; it now skips saving
  while the pet is docked, and quitting from the tray un-docks first.

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

[0.4.0]: https://github.com/xShiroeNguyenx/desktop-companion/releases/tag/v0.4.0
[0.3.2]: https://github.com/xShiroeNguyenx/desktop-companion/releases/tag/v0.3.2
[0.3.1]: https://github.com/xShiroeNguyenx/desktop-companion/releases/tag/v0.3.1
[0.3.0]: https://github.com/xShiroeNguyenx/desktop-companion/releases/tag/v0.3.0
[0.2.3]: https://github.com/xShiroeNguyenx/desktop-companion/releases/tag/v0.2.3
[0.2.2]: https://github.com/xShiroeNguyenx/desktop-companion/releases/tag/v0.2.2
[0.2.1]: https://github.com/xShiroeNguyenx/desktop-companion/releases/tag/v0.2.1
[0.2.0]: https://github.com/xShiroeNguyenx/desktop-companion/releases/tag/v0.2.0
