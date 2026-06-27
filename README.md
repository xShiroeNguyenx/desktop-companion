# Desktop Companion

🌸 A standalone Live2D desktop pet for Windows with built-in **AI translation**,
**quick reply**, and **task saving** — works over any app (Teams, browsers, Word…).

🌐 **[Website / Landing page →](https://xshiroenguyenx.github.io/desktop-companion/)**

**Languages:** **English** · [Tiếng Việt](README.vi.md) · [日本語](README.ja.md)

![Live2D pet](src-tauri/icons/128x128.png)

---

## Features

- **Live2D desktop pet** — a transparent, always-on-top, draggable companion that
  starts with Windows. Right-click for the menu.
- **Translate** — select text in **any** app → a 🌸 flower appears near the cursor
  → **click** for an instant translation, or **press-and-hold** for the full panel
  (translate / reply / save task).
- **Quick reply** — select a message, type your reply in Vietnamese, and the AI
  composes a natural reply in the message's language (English / Japanese / …).
- **Save tasks** — turn any selected text into a TODO; manage them in the Tasks window.
- **Translation contexts** — preset contexts in Settings (e.g. "formal work chat")
  so quick-translate matches the right tone. Pick a default or none.
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
3. **Reply** — select a message → press-and-hold the flower → **Reply** tab → type
   in Vietnamese → **Generate**.
4. **Tasks** — tray → **Tasks**, or save a selection via the flower's hold menu.

### Tray menu

Show/Hide pet · Tasks · Settings · Toggle click-through · Quit

### Right-click the pet

Appearance (change model, look-at toggle, motion, poke) · Sound (voice language,
ambient, mute) · AI Chat · Tasks · Settings

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
