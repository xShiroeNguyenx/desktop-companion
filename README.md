# Desktop Companion

🌸 A standalone Live2D desktop pet for Windows with built-in **AI translation**,
**quick reply**, and **task saving** — works over any app (Teams, browsers, Word…).

🌐 **[Website / Landing page →](https://desktop.techdecoded.net/)**

**Languages:** **English** · [Tiếng Việt](README.vi.md) · [日本語](README.ja.md)

![Live2D pet](src-tauri/icons/128x128.png)

---

## Screenshots

<table>
  <tr>
    <td width="50%"><img src="docs/landing/img/arcade.webp" alt="Mini Games page" /><br /><sub>Mini Games page</sub></td>
    <td width="50%"><img src="docs/landing/img/widgets.webp" alt="Calendar, clock &amp; weather widgets" /><br /><sub>Calendar, clock &amp; weather widgets</sub></td>
  </tr>
  <tr>
    <td><img src="docs/landing/img/rps.webp" alt="Rock paper scissors" /><br /><sub>Rock paper scissors</sub></td>
    <td><img src="docs/landing/img/mines.webp" alt="Bomb Squad (minesweeper)" /><br /><sub>Bomb Squad (minesweeper)</sub></td>
  </tr>
  <tr>
    <td><img src="docs/landing/img/collection.webp" alt="🍬 Collection" /><br /><sub>🍬 Collection</sub></td>
    <td><img src="docs/landing/img/weather.webp" alt="Weather forecast card" /><br /><sub>Weather forecast card</sub></td>
  </tr>
  <tr>
    <td><img src="docs/landing/img/settings-snow.webp" alt="Settings — ❄️ Snow Lounge theme" /><br /><sub>Settings — ❄️ Snow Lounge theme</sub></td>
    <td><img src="docs/landing/img/settings-neko.webp" alt="Settings — 🦇 Gothic Lolita theme" /><br /><sub>Settings — 🦇 Gothic Lolita theme</sub></td>
  </tr>
</table>

<p align="center">
  <img src="docs/landing/img/girl-bunny.webp" height="180" alt="Bunny Lounge" />
  <img src="docs/landing/img/girl-neko.webp" height="180" alt="Neko Lounge" />
  <img src="docs/landing/img/girl-fox.webp" height="180" alt="Fox Lounge" />
  <img src="docs/landing/img/girl-snow.webp" height="180" alt="Snow Lounge" />
  <br /><sub>Character sets: 🐰 Bunny · 🐱 Neko · 🦊 Fox · ❄️ Snow Lounge</sub>
</p>

<p align="center">
  <img src="docs/landing/img/model-gothic-aristocrat.png" height="180" alt="Gothic Aristocrat" />
  <img src="docs/landing/img/model-pink-angel.png" height="180" alt="Pink Angel" />
  <img src="docs/landing/img/model-snow-aristocrat.png" height="180" alt="Snow Aristocrat" />
  <img src="docs/landing/img/model-pink-aristocrat.png" height="180" alt="Pink Aristocrat" />
  <img src="docs/landing/img/model-gothic-pastel.png" height="180" alt="Gothic Pastel" />
  <img src="docs/landing/img/model-pink-pastel.png" height="180" alt="Pink Pastel" />
  <br />
  <img src="docs/landing/img/model-snow-pastel.png" height="180" alt="Snow Pastel" />
  <img src="docs/landing/img/model-snow-lolita.png" height="180" alt="Snow Lolita" />
  <img src="docs/landing/img/model-gothic-lolita.png" height="180" alt="Gothic Lolita" />
  <img src="docs/landing/img/model-snow-dancer.png" height="180" alt="Snow Dancer" />
  <img src="docs/landing/img/model-pink-kawaii.png" height="180" alt="Pink Kawaii" />
  <img src="docs/landing/img/model-snow-kawaii.png" height="180" alt="Snow Kawaii" />
  <br /><sub>🎭 Live2D models in the Collection: Gothic Aristocrat · Pink Angel · Snow Aristocrat · Pink Aristocrat · Gothic Pastel · Pink Pastel · Snow Pastel · Snow Lolita · Gothic Lolita · Snow Dancer · Pink Kawaii · Snow Kawaii</sub>
</p>

---

## Features

- **Live2D desktop pet** — a transparent, always-on-top, draggable companion that
  starts with Windows. Right-click for the **radial menu**.
- **Translate** — select text in **any** app → a 🌸 flower appears near the cursor
  → **click** for an instant translation, or **press-and-hold** to fan out a
  **radial menu**: 🌐 translate · 📋 copy · 💬 reply · 💭 ask the pet · 📝 save as task ·
  🗒️ save as note · 🗂️ full panel. **Ctrl+C** (while the flower is up) still copies the selected text.
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
- **🗒️ Notes for everyday info** — a board of its own (tray, right-click → Ghi
  chú, or 💞 on the ring) for the things you look up every day: Wi-Fi details,
  phone numbers, links, commands you type often. No deadlines, nothing to tick
  off. Search as you type, with or without Vietnamese accents ("mat khau" finds
  "Mật khẩu"); ⭐ keeps a note at the top, 📋 copies it in one click, ✏️ or a
  double-click edits it in place. 📌 pins a note to the desktop as a yellow
  sticky note with its own 📋 Copy button. Select text anywhere and pick
  🗒️ **Save as note** on the flower to start a note with it.
- **🍅 Focus sessions** — 25 minutes of focus, then a 5-minute break (both
  adjustable). A ring on the pet counts down, she wears her focused face, and
  only a deadline may interrupt you.
- **She notices when you step away** — after 10 minutes without mouse or
  keyboard (or with the screen locked) she dozes off, and greets you when you
  come back. An optional warning (off by default) tells you when CPU or RAM
  stays high. Only on/off states and percentages are read — never window
  titles, app names or screen content. She also warns you (on by default) when
  the battery drops below 15% without the charger, again at 5%, and when the
  Windows drive has less than 10% free.
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
  Click a day for its full lunar date, what is due and its events, or **＋ Add**
  a ✓ task or a 🎗 **event** on that day; your birthday gets a 🎂. Events
  (memorials, birthdays, anniversaries) are separate from tasks: they never go
  overdue and can repeat **every year on the same date** or **every year on the
  same lunar date** (a leap month does not repeat it, and a 30th falls back to
  the 29th in a short month). Events show as a gold dot; click one to edit it,
  switch it between task and event, or delete it. The 🎗 tab of the Notes board
  lists every event by how soon it comes next, and clicking one opens the
  calendar on that day. It stays above other windows unless you
  turn 📍 off. The pet mentions holidays, the full moon (*Rằm*) and the 1st of
  each lunar month, and the evening before reminds you they are tomorrow.
  Add an event and pick **early reminders** (1 day, 3 days, 1 week, 10 days or
  1 month before, any mix); she tells you at 7 am that day, or Windows shows a
  notification. Events with early reminders show a 🔔. On the morning of an
  event the pet's daily greeting names it too.
- **🕐 Clock with world clocks** — your PC's time with the lunar date, plus up
  to two more clocks (Tokyo, Seoul, New York, London…) with the time
  difference and "tomorrow" / "yesterday"; summer time is handled. Its 🍅 row
  starts a focus session and counts down.
- **📊 System monitor** — CPU with a one-minute graph, RAM, free space on the
  Windows drive, download / upload speed, battery (laptops only) and how long
  the PC has been on, refreshed every 2 seconds and only while it is pinned.
  Bars turn gold from 70% and red from 90%, and Miu dozes when the PC is idle.
  Click CPU or RAM to see the five apps using the most (grouped like Task
  Manager), with a button that opens Task Manager. App names are read only
  while that list is open and only shown there: never saved or sent anywhere.
- **🍬 Candy & Collection** — finished tasks, focus sessions, chat, headpats
  and game wins also earn candy (same daily caps as friendship points). Trade it
  in the 🍬 Collection (💞 → 🍬 on the ring) for widget skins (Sakura, Night
  sky, Mint, Sunset, Ocean, Matcha, Cotton candy, Café latte, Neon night),
  🎀 widget decor (ribbon bows, washi tape, sakura petals, lace trim, a star
  garland, a snowcap, a rainbow holo frame), reward effects (fireworks, sakura
  shower, confetti, heart shower, soap bubbles, shooting star), speech-bubble
  styles (Cloud, Heart, Gothic lace, Scroll), the flower shown when you select
  text (⭐ 🍀 🐾 ❄️), character sets (🐰 Bunny, 🐱 Neko, 🦊 Fox, ❄️ Snow
  Lounge), Miu face sets (❄️ Snow, 🦇 Gothic) and Live2D models. Snow, Neko
  and Fox Lounge also restyle every panel (icy blue / gothic lolita / fox
  shrine). Decorations only; never sold for real money.
- **🎭 Live2D models in the Collection** — twelve extra characters, each with
  smile, sad, angry, surprised, shy and wink expressions and her own motions:

  | Model | Price | Capsule rate |
  |---|---|---|
  | 🦇 Gothic Aristocrat · 🌸 Pink Angel · ❄️ Snow Aristocrat · 🌸 Pink Aristocrat | 10000 🍬 | 0.1% each |
  | 🦇 Gothic Pastel · 🌸 Pink Pastel · ❄️ Snow Pastel | 7500 🍬 | 0.2% each |
  | ❄️ Snow Lolita · 🦇 Gothic Lolita · ❄️ Snow Dancer · 🐰 Pink Kawaii · ❄️ Snow Kawaii | 5000 🍬 | 0.4% each |

  They are not in the installer: once unlocked, the model downloads (a few MB,
  progress on its card), is checked against a signed list, and updates itself
  when a newer version is published, without an app update. Press **Use** and
  she replaces the character on screen right away. Hiyori, Haru, Mao and Miara
  stay free, and so do models you add from a folder.
- **🎰 Candy Capsule** — spin the bunny capsule machine in the Collection:
  100 🍬 a spin, 1000 🍬 for ten, rates shown on the machine (Common 62% ·
  Rare 25% · Epic 10% · 🌈 Legendary 3%, Live2D models only). Every 10th spin
  without a Rare is a sure Rare or better, every 50th without an Epic a sure
  Epic or better, and duplicates refund 10% of the item's price. Each rarity
  bursts open with its own light show, with rainbow rays and heavy confetti
  for a Legendary. Trading directly in the shop always stays available.
- **🎁 A free spin every day** — each day a small capsule appears next to the
  character; click it for one free Candy Capsule spin, opened right there on
  the pet. It counts like any other spin (guarantees, history, duplicate refund).
- **🧩 Widgets in one place** — Settings → *🧩 Widget* pins or unpins the
  calendar, the clock, the weather widget and the system monitor, picks the
  extra clocks, and 🧹 tidies every pinned widget into a column on the right.
  They all share one
  width and padding, so they line up on both edges.
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
- **Appearance menu** — locked Collection models show a 🔒; picking one opens
  the Collection on the Models tab.
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

Show/Hide pet · Tasks · 🗒️ Notes · Settings · Toggle click-through · 🍅 Start / stop focus · 🔕 Do not disturb (1 hour) · 🌦 Weather effects on / off · ☁️ Weather forecast · 📅 Pin / unpin the month calendar · 📊 Pin / unpin the system monitor · Quit

### Right-click the pet

A radial menu fans out around the cursor. Hovering an icon shows its name in the
centre pill; clicking a category swaps the ring for its entries and turns the
pill into a back button.

Appearance (change model, look-at toggle, motion, poke) · Sound (voice language,
ambient, mute) · Quick Chat · Tasks · Settings (the bottom slot) · 💞 Đồng hành
(🍅 focus, 💗 bond, 🎮 Mini Games, 🍬 Collection, 🗒️ Notes, 📅 month calendar, 📊 system monitor, 🔕 Do not disturb) · ☁️ Weather (forecast,
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

- **Code:** MIT — see [LICENSE](LICENSE).
- **Original artwork** (the Live2D models in the Collection, side characters, Miu faces, Arcade art): [CC BY-NC 4.0](https://creativecommons.org/licenses/by-nc/4.0/) — free to share and remix with credit, not for commercial use. See [ASSETS-LICENSE.md](ASSETS-LICENSE.md).
- **Live2D sample models** (Hiyori, Haru, Mao, Miara): © Live2D Inc., Free Material License.
