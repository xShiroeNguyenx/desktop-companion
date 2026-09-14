# Desktop Companion

🌸 A standalone Live2D desktop pet for Windows with built-in **AI translation**,
**quick reply**, and **task saving** — works over any app (Teams, browsers, Word…).

🌐 **[Website / Landing page →](https://xshiroenguyenx.github.io/desktop-companion/)**

**Languages:** **English** · [Tiếng Việt](README.vi.md) · [日本語](README.ja.md)

![Live2D pet](src-tauri/icons/128x128.png)

---

## Features

- **Live2D desktop pet** — a transparent, always-on-top, draggable companion that
  starts with Windows. Right-click for the **radial menu**.
- **Translate** — select text in **any** app → a 🌸 flower appears near the cursor
  → **click** for an instant translation, or **press-and-hold** to fan out a
  **radial menu**: 🌐 translate · 📋 copy · 💬 reply · 📝 save as task · 🗂️ full panel.
  **Ctrl+C** (while the flower is up) still copies the selected text.
- **Quick reply** — select a message, type your reply in Vietnamese, and the AI
  composes a natural reply in the message's language (English / Japanese / …).
- **Save tasks** — turn any selected text into a TODO. The Tasks window keeps
  "to do" and "done" in two columns, stamps each task with when it was created
  and finished, and lets you edit a note on any of them.
- **Translation contexts** — preset contexts in Settings (e.g. "formal work chat")
  so quick-translate matches the right tone. Pick a default or none.
- **The pet joins your panels** — open Settings or Tasks and the pet steps into
  the gap between their two columns (click-through, so it stays out of the way),
  then returns to where it was when you close them.
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

### Tray menu

Show/Hide pet · Tasks · Settings · Toggle click-through · Quit

### Right-click the pet

A radial menu fans out around the cursor. Hovering an icon shows its name in the
centre pill; clicking a category swaps the ring for its entries and turns the
pill into a back button.

Appearance (change model, look-at toggle, motion, poke) · Sound (voice language,
ambient, mute) · AI Chat · Tasks · Settings

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
```

---

## Tech stack

- **[Tauri 2](https://tauri.app)** (Rust core + WebView2)
- **Live2D Cubism 4** rendered with **[PIXI.js](https://pixijs.com)**
- Global mouse hook + simulated copy for cross-app text capture (Windows)
- Anthropic / Gemini HTTP APIs for translation & reply

---

## Credits

- Live2D sample models © Live2D Inc. (used under their Free Material License).
- Built on the rendering runtime from the Anime Companion project.

## License

MIT — see [LICENSE](LICENSE).
