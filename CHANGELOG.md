# Changelog

All notable changes to Desktop Companion are documented here.
The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.7.0] - 2026-10-16

### Added
- **A Mini Games page.** 💞 → 🎮 now opens a page of its own instead of the
  small card on the pet: three illustrated cards (Rock paper scissors, Guess
  my number, Bomb Squad) with your best result on each and a *Play now*
  button, a "★ Today" game of the day, your friendship level and how many
  points games gave you today, plus Achievements (plays, wins and records for
  every game) and Rewards.
- **New mini game: Bomb Squad (minesweeper).** Pick it on the Mini Games page.
  Choose Easy (8 × 8, 10 bombs), Normal (12 × 12, 25) or Hard (16 × 16, 40);
  left-click opens a tile, right-click plants a flag, and clicking a number
  whose bombs are all flagged opens the rest around it. Your first tile is
  always safe. The top bar shows the timer, bombs left and flags placed. Miu
  sits on the right and reacts as you play: nervous while you hold a tile,
  happy when it is safe, startled by a bomb and cheering when the board is
  clear. Her tip box explains the rules and can light up a safe tile
  (3 hints on Easy, 2 on Normal, 1 on Hard). ⚙ turns sound, quick-open and
  her chatter on or off and shows your plays, win rate and fastest time per
  level. Closing the window mid-game pauses the clock and keeps the board.

### Changed
- **⚙ Settings sits at the bottom of the round menu** (it swapped places with
  🗂️ *Mở đầy đủ*), so it is easy to find.
- **Rock paper scissors has its own big window now.** A playful card with the
  game's title, a You vs. pet scoreboard, the character on the right with a
  speech bubble, and three large cards to pick from (or keys Q · W · E). Your
  pick lights up while she thinks, then the round shows both hands side by
  side with a big *Win! / Lose… / Draw!*. A framed portrait above the cards
  shows Miu reacting to the game: a wink while she waits, a puzzled "?" while
  she picks, cheering when she wins, a pout when she loses and a big smile on
  a draw. ⚙ opens game settings: play to 3, 5 or 7 wins, choose the character
  (Miu, the cat girl or either bunny), difficulty (easy lets you win more,
  hard learns which hand you like), sound effects, animations, and how many
  friendship points games have given you today.
- **Guess the number has its own window too.** Miu sits on the left and
  reacts to every guess (puzzled when it is too low, pouting when it is too
  high, cheering when you get it). Type with the on-screen keypad or your
  keyboard, then *Guess* (Enter), *Clear* or ask for a *Hint*. The right panel
  shows the range, hearts for the guesses you have left and a history of every
  try. ⚙ lets you pick the range (1–10, 20, 50 or 100), 5, 7 or 10 guesses,
  whether to show "hot / cold" hints, sound, and difficulty (easy gives 3
  hints, normal 1, hard none), plus your plays, win rate and best streak.

## [0.6.0] - 2026-10-15

### Added
- **Focus sessions (Pomodoro).** Pick 🍅 (right-click → 💞 *Đồng hành*) or use
  the tray: 25 minutes of focus, then a 5-minute break (both adjustable in
  Settings → *Tập trung*). A ring on the pet counts down, the pet wears its
  focused face, and nothing interrupts you except a deadline. When the session
  ends she cheers you on and sends you to rest; pick ⏹ to stop early.
- **The pet notices when you step away.** After 10 minutes without mouse or
  keyboard (or with the screen locked) she dozes off, without a word. When you
  come back she wakes up and greets you — differently after a short break, a
  long absence, or the next morning.
- **Optional CPU / RAM warning** (off by default). When CPU or memory stays
  above your threshold for 2 minutes, the pet tells you, at most once every 30
  minutes for each. With it off, nothing is measured at all.
- **A bond that grows over time.** Being at the computer each day, finishing
  tasks and focus sessions, chatting and head-pats slowly raise your bond with
  your companion, through five levels from *Mới quen* to *Không thể thiếu*.
  Each kind of activity counts only up to a small amount per day, and nothing
  is ever taken away when you skip a day. Pick 💗 (right-click → 💞) to
  see the level, progress to the next one, days together and your streak. The
  closer you are, the more casually she talks in chat.
- **💞 Đồng hành** on the right-click ring gathers 🍅 focus, 💗 bond and 🎮
  games, so the first ring stays tidy.
- **Mini games.** Right-click → 💞 *Đồng hành* → 🎮 *Chơi*: rock paper scissors
  (first to 3 wins) or guess the number (1–100 in 7 tries, with higher /
  lower hints). The pet reacts to every round with her face and shows her
  move (✊ ✌️ ✋), your latest guess, and finally her secret number in a
  bubble above her head. Games take a minute or two, and each finished game
  adds a little to your bond.
- **🗂️ Mở đầy đủ on the right-click ring** opens the text assistant next to the
  pet with an empty box to type or paste your own text — translate, reply or
  save it as a task without selecting anything first.
- **A hello to start the day**, with how many tasks are due today.
- **Milestones and birthdays.** She celebrates 7, 30, 100 and 365 days
  together, each yearly anniversary, every new bond level, 7- and 30-day
  streaks — each one once — and your birthday, if you add it in Settings →
  *Bạn đồng hành* (day and month only).
- Settings → *Nhận biết máy tính*: doze-off delay (or off), greet on return,
  stay quiet while presenting or full screen, and the CPU / RAM warning. The
  app reads only on/off states and percentages — never window titles, app
  names or screen content.

## [0.5.0] - 2026-10-15

### Added
- **Pin a task to your desktop.** Press 📌 next to a task in the Tasks panel
  and it becomes a pink sticky note on the desktop, held by a pushpin with
  your companion's face on it. Drag it anywhere; it stays where you left it,
  also after a restart. Hover the note for its buttons: **✓ Xong** stamps it
  done, drops it off the screen and the pet cheers you on; **✕** unpins it
  (the task stays in your list); **📍** keeps it above other windows. Double-
  click a note to open the Tasks panel. Up to 8 notes at a time.
- **Companion girls cling to the panels.** A pink-haired bunny idol leans on
  the left edge of the Settings window, a silver-haired bunny on Tasks, and a
  blue-haired cat girl on the text assistant (*Mở đầy đủ* from the flower),
  about a third of each overlapping the frame. The overlapping part
  fades to a faint ghost so the panel stays readable, and clicks go through
  to the panel underneath. They follow the panel when you
  move or resize it, hide with it, and never take clicks. If a panel opens too
  close to the left side of the screen, it slides right just enough for her
  to fit. Turn them off (or back on) with Settings → *Diện mạo* → *🌸 Bunny
  Lounge* (off by default); it applies right away.
- **Links in tasks are clickable.** A web link in a task or its note — on a
  pinned note or in the Tasks panel — opens in your default browser. Only
  `http://` and `https://` links open; anything else stays plain text.
- **Give your companion a name.** Until she has one, she asks you now and then
  (at most three times, a week apart) with a small card right on the pet:
  type a name, or pick *Để sau*. She thanks you by name, then asks how she
  should address you. You can also name her in chat ("từ giờ em tên là Mây
  nhé"); the reply notes the new name only once it is really saved. Change it
  any time in Settings → *Bạn đồng hành*.
- **Choose how she addresses you**: em – anh, em – chị, mình – bạn, tớ – cậu,
  em – sếp, or your own words. When she speaks Japanese or English she uses
  your name instead (with さん / くん / ちゃん / 様 in Japanese).
- **Deadlines for tasks.** Press 📅 on a task to give it a date and time. The
  Tasks panel sorts by the nearest deadline and shows how urgent each one is
  ("⏰ Còn 20 phút", "⚠️ Quá hạn 1 ngày"); pinned notes show it too. The pet
  reminds you once per step: on the morning of the due day, an hour before,
  when it is due, and once a day for up to three days after. Tell the pet "nhắc
  anh gửi báo cáo trước 5 giờ chiều mai" and the task is saved with its
  deadline.
- **Windows notifications for deadlines.** If a deadline arrives while the pet
  is hidden, in a panel, set to do-not-disturb, or you are presenting full
  screen, you get a Windows notification instead (Settings → *Nhắc nhở*, with
  a *Gửi thử* button).
- **Rest reminders, on by default**: rest your eyes every 45 minutes, drink
  water and stand up every 60 minutes of sitting at the computer. Stepping
  away for 5 minutes counts as a rest. Reminders that fall close together are
  combined into one line. They need no API key.
- **The pet cheers when you finish a task** — from the Tasks panel, a pinned
  note, or chat — with something special for the last task on the list,
  finishing before the deadline, and your 3rd, 5th and 10th task of the day.
- **Finish or delete tasks by chat.** Say you are done with a task and the pet
  ticks it off. It only changes a task when it is sure which one you meant,
  and says so when it is not.
- **Do not disturb.** Pick 🔕 on the pet's right-click ring or in the tray
  menu and the pet stops starting conversations for an hour. Pick it again
  (🔔) to turn it back off early. Your questions in chat are still answered.

### Changed
- **The text assistant (*Mở đầy đủ* from the flower) now matches the Settings
  look**: the same soft pink background, a sakura glass card, pink section
  headings, pill tabs and the same inputs and buttons.
- **The pet no longer talks to an empty room.** It waits instead of starting a
  conversation while you are away from the computer (5 minutes without mouse
  or keyboard, or the screen is locked), while a game, video or slideshow is
  full screen, or while it is hidden. Only these on/off states are read, never
  window titles, app names or screen content.
- Lines the pet says on its own are now at least 10 minutes apart (adjustable
  in Settings → *Nhắc nhở*). Deadline reminders do not wait.
- **"Ngôn ngữ chữ" now sticks and reaches the chat.** The language picked in
  the pet's sound menu (or in Settings) is saved, survives a restart, and the
  pet's lines and chat replies follow it in Vietnamese, Japanese or English.
  Menus and panels are still in Vietnamese.

### Fixed
- **Minimizing Settings or Tasks no longer leaves the pet stranded.** The pet
  that stands in the middle of the panel stayed floating where the panel had
  been. It now goes back to its usual spot when the panel is minimized, and
  steps back into the panel when you open it again.

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
