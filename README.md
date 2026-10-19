# Desktop Companion

🌸 A standalone Live2D desktop pet for Windows with built-in **AI translation**,
**quick reply**, and **task saving** — works over any app (Teams, browsers, Word…).

🌐 **[Website / Landing page →](https://desktop.techdecoded.net/)**

**Languages:** **English** · [Tiếng Việt](README.vi.md) · [日本語](README.ja.md)

![Live2D pet](src-tauri/icons/128x128.png)

---

## Features

- **Live2D desktop pet** — a transparent, always-on-top, draggable companion that
  starts with Windows. Right-click for the **radial menu**.
- **Translate** — select text in **any** app → a 🌸 flower appears near the cursor
  → **click** for an instant translation, or **press-and-hold** to fan out a
  **radial menu**: 🌐 translate · 📋 copy · 💬 reply · 💭 ask the pet · 📝 save as task ·
  🗂️ full panel. **Ctrl+C** (while the flower is up) still copies the selected text.
- **Chat with the pet** — right-click → 💬 **Quick Chat**. The reply streams into
  the bubble word by word and the pet's face changes with the mood of what it
  says. It knows the time and your unfinished tasks, remembers the last 10 turns
  (stored only on your machine; 🗑 clears them), and ⏹ stops it mid-reply.
- **Ask about a selection** — pick 💭 on the flower's radial menu to open the chat
  with the selected text attached ("explain this", "is this polite?"…).
- **Tasks by chat** — "nhắc anh gửi báo cáo trước 5 giờ chiều mai" adds the task
  with its deadline; "xong báo cáo rồi" ticks it off. The bubble confirms only
  once it is really saved, and the pet asks instead of guessing when it is not
  sure which task you meant.
- **Name your companion** — she asks for a name (a small card right on the
  pet), thanks you by it, and asks how to address you: em – anh, em – chị,
  mình – bạn, tớ – cậu, em – sếp, or your own words. She can speak and chat in
  **Vietnamese, Japanese or English**; change everything in Settings →
  *Bạn đồng hành*.
- **The pet speaks up** — after a quiet stretch (Settings: off / 30 min / 1 h / 2 h)
  it says one line: nudges your oldest unfinished task or reminds you to rest.
  It never talks to an empty room: it waits while you are away, the screen is
  locked, or something is full screen, and keeps its own lines at least 10
  minutes apart. 🔕 **Do not disturb** (right-click ring or tray) silences it
  for an hour.
- **Deadlines** — 📅 on any task. The Tasks panel sorts by the nearest deadline
  and colours it by urgency. The pet reminds you on the morning it is due, an
  hour before, when it is due, and once a day for up to three days after; when
  the pet is hidden or you are presenting, a **Windows notification** comes
  instead.
- **Rest reminders** (on by default, no API key needed) — rest your eyes after
  45 minutes at the computer, drink water and stand up after 60. Stepping away
  for 5 minutes counts as a rest; reminders that fall close together become one.
- **Cheers when you finish** — the pet celebrates every finished task, with
  something special for the last one on the list, finishing early, and your
  3rd / 5th / 10th task of the day.
- **Pin tasks to the desktop** — 📌 turns a task into a pink sticky note held
  by a pushpin with your companion's face. Drag it anywhere; ✓ stamps it done,
  ✕ unpins it, 📍 keeps it above other windows. Links in tasks and notes are
  clickable.
- **🍅 Focus sessions** — 25 minutes of focus, then a 5-minute break (both
  adjustable). A ring on the pet counts down, she wears her focused face, and
  only a deadline may interrupt you.
- **She notices when you step away** — after 10 minutes without mouse or
  keyboard (or with the screen locked) she dozes off, and greets you when you
  come back. An optional warning (off by default) tells you when CPU or RAM
  stays high. Only on/off states and percentages are read — never window
  titles, app names or screen content.
- **💗 A bond that grows** — days together, finished tasks and focus sessions,
  chats, head-pats and games slowly raise your bond through five levels
  (capped per day, never taken away). She says hello each morning with what is
  due today, celebrates milestones (7 / 30 / 100 / 365 days, anniversaries,
  streaks, new levels) and your birthday, and talks more casually as you get
  closer.
- **🎮 Mini Games** — a page of its own (💞 → 🎮) with three games, each in
  its own window where Miu reacts to every move: rock paper scissors (first to
  3, 5 or 7), guess the number (1–10 up to 1–100, with hints) and Bomb Squad,
  a minesweeper in three sizes. A "★ Today" game of the day, achievements and
  the friendship points games gave you today sit on the same page.
- **🌦 Weather** — rain, snow, cherry blossoms, falling leaves, fog, sunshine
  or a starry night across your whole screen, never blocking the mouse and
  pausing by itself when you watch something full screen, lock the PC or step
  away. Pick an effect, let it follow the season and time of day, or follow
  the **real weather** in a city you choose. A forecast card (now, 7 days, a
  24-hour chart with the chance of rain, °C / °F) and a weather widget you can
  pin to the desktop; the pet comments when the weather changes and gives a
  short morning report. Toggle the effect with **Ctrl+Alt+W**. Your location is
  never looked up — only the coordinates of the city you pick are sent to
  [Open-Meteo](https://open-meteo.com) (no API key needed).
- **📅 Month calendar on your screen** — pin a small month calendar from the
  tray, 💞 → 📅 on the ring or Settings. Each day shows its Vietnamese lunar
  date, Vietnamese holidays (Tết, Mid-Autumn, Hùng Kings Day…) are marked and
  days off are in red, and days with tasks due get a dot (red when overdue).
  Click a day for its full lunar date and what is due, or to add a task due
  that day; your birthday gets a 🎂. It stays above other windows unless you
  turn 📍 off. The pet mentions holidays, the full moon (*Rằm*) and the 1st of
  each lunar month, and the evening before reminds you they are tomorrow.
- **🕐 Clock with world clocks** — your PC's time with the lunar date, plus up
  to two more clocks (Tokyo, Seoul, New York, London…) with the time
  difference and "tomorrow" / "yesterday"; summer time is handled. Its 🍅 row
  starts a focus session and counts down.
- **🍬 Candy & Collection** — finished tasks, focus sessions, chat, headpats
  and game wins also earn candy (same daily caps as friendship points). Trade it
  in the 🍬 Collection (💞 → 🍬 on the ring) for widget skins, reward effects
  (fireworks, sakura shower, confetti), character sets (🐰 Bunny, 🐱 Neko,
  🦊 Fox, ❄️ Snow Lounge) and Miu face sets (❄️ Snow, 🦇 Gothic). Snow Lounge
  and Neko Lounge also restyle every panel (icy blue / gothic lolita).
  Decorations only; never sold for real money.
- **🧩 Widgets in one place** — Settings → *🧩 Widget* pins or unpins the
  calendar, the clock and the weather widget, picks the extra clocks, and
  🧹 tidies every pinned widget into a column on the right.
- **Quick reply** — select a message, type your reply in Vietnamese, and the AI
  composes a natural reply in the message's language (English / Japanese / …).
- **Text assistant without selecting** — right-click → 🗂️ **Mở đầy đủ** opens
  the assistant next to the pet with an empty box: type or paste text, then
  translate, reply or save it as a task.
- **Save tasks** — turn any selected text into a TODO. The Tasks window keeps
  "to do" and "done" in two columns, stamps each task with when it was created
  and finished, and lets you edit a note on any of them.
- **Translation contexts** — preset contexts in Settings (e.g. "formal work chat")
  so quick-translate matches the right tone. Pick a default or none.
- **The pet joins your panels** — open Settings or Tasks and the pet steps into
  the gap between their two columns (click-through, so it stays out of the way),
  then returns to where it was when you close or minimize them.
- **🌸 Bunny Lounge** (optional, Settings → *Diện mạo*) — companion girls lean
  on the edge of Settings, Tasks, the text assistant, the weather card and the
  Mini Games page; the part over the panel fades so everything stays readable.
- **Look-at cursor** — the model's head/eyes follow the mouse when it's near the pet.
- **Resize** — `Ctrl` + mouse wheel scales the pet; size is remembered.
- **Custom Live2D models** — load your own `*.model3.json` models from any folder.
- **Sounds** — interaction SFX (poke / headpat / etc.) in Japanese / Vietnamese / English.
- **AI providers** — Anthropic Claude or Google Gemini (bring your own API key).
  Model lists can be fetched live from each provider.
- **Custom hotkey** — rebind the capture shortcut (default `Ctrl+Shift+Space`).

---

## Install

Download the latest installer from the [Releases](../../releases) page:

- **`Desktop Companion_x.y.z_x64-setup.exe`** (NSIS) or **`…_x64_en-US.msi`** (MSI)

Run it and the app launches automatically. It lives in the system tray.

> Windows only for now. Requires the WebView2 runtime (preinstalled on Windows 10/11).

---

## Getting started

1. **Set your API key** — tray icon → **Cài đặt / Settings** → choose a provider
   (Anthropic or Gemini) → paste your key → **Save**.
2. **Translate** — select text anywhere → click the 🌸 flower.
3. **Reply** — select a message → press-and-hold the flower → pick 💬 **Reply** →
   type in Vietnamese → **Generate**.
4. **Tasks** — tray → **Tasks**, or pick 📝 **Save as task** on the flower's radial
   menu.
5. **Chat** — right-click the pet → 💬 **Quick Chat** → type → `Enter`.

### Tray menu

Show/Hide pet · Tasks · Settings · Toggle click-through · 🍅 Start / stop focus · 🔕 Do not disturb (1 hour) · 🌦 Weather effects on / off · ☁️ Weather forecast · 📅 Pin / unpin the month calendar · Quit

### Right-click the pet

A radial menu fans out around the cursor. Hovering an icon shows its name in the
centre pill; clicking a category swaps the ring for its entries and turns the
pill into a back button.

Appearance (change model, look-at toggle, motion, poke) · Sound (voice language,
ambient, mute) · Quick Chat · Tasks · Settings (the bottom slot) · 💞 Đồng hành
(🍅 focus, 💗 bond, 🎮 Mini Games, 📅 month calendar, 🔕 Do not disturb) · ☁️ Weather (forecast,
effects on / off, pin to desktop) · 🗂️ Mở đầy đủ (text assistant)

**Model** and **Motion** open as two columns either side of the character, so it
stays visible while you pick. They stay open across choices — close with × or
`Escape`.

---

## Build from source

### Prerequisites

- [Rust](https://rustup.rs) (stable) + the MSVC C++ build tools
- [Node.js](https://nodejs.org) 18+
- Tauri prerequisites — see <https://v2.tauri.app/start/prerequisites/>

### Commands

```bash
npm install
npm run dev      # run in development
npm run build    # produce installers in src-tauri/target/release/bundle/
npm test         # JS unit tests (app/js/*.test.js)
cd src-tauri && cargo test   # Rust unit tests
```

---

## Tech stack

- **[Tauri 2](https://tauri.app)** (Rust core + WebView2)
- **Live2D Cubism 4** rendered with **[PIXI.js](https://pixijs.com)**
- Global mouse hook + simulated copy for cross-app text capture (Windows)
- Anthropic / Gemini HTTP APIs for translation & reply
- [Open-Meteo](https://open-meteo.com) for weather forecasts and city search

---

## Credits

- Live2D sample models © Live2D Inc. (used under their Free Material License).
- Built on the rendering runtime from the Anime Companion project.
- Weather data by [Open-Meteo.com](https://open-meteo.com) (CC BY 4.0).

## License

MIT — see [LICENSE](LICENSE).
