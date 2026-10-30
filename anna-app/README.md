# Desktop Companion — Anna App

The Desktop Companion pet as an [Anna](https://anna.partners) App: a UI bundle only,
no executa. AI calls go through the user's own Anna model (`anna.llm`), voice through
Anna TTS (`anna.audio.speak`, opt-in), and all data through Anna storage (`anna.storage`).

Features: Live2D pet (look-at, head pats, pokes, idle lines, lip-sync), chat with emotions
and task tags, translate, reply drafting, tasks, notes, 🍅 focus timer, mini games
(rock paper scissors, guess the number, Bomb Squad), lunar calendar, and a collection
where candy from using the app unlocks more models.

## Layout

| Path | What |
|---|---|
| `bundle/core.js` | Pure logic (prompts, reply tags, rewards, focus, payloads) — tested in `test/` |
| `bundle/app.js` | Wiring: SDK, storage, chat / translate / reply / tasks, tabs, Anna chat payloads |
| `bundle/stage.js` | Live2D stage: framing, look-at, pats / pokes, reactions, lip-sync, particles |
| `bundle/voice.js` | Anna TTS playback through Web Audio |
| `bundle/games.js`, `bundle/extras.js` | Games, notes, focus, calendar, collection panels |
| `bundle/i18n.js` | en / vi / ja strings (a test keeps the keys in sync) |
| `scripts/sync-assets.mjs` | Copies Live2D libs, the shared `app/js/*-core.js` modules, every model from `../models/manifest.json` and their previews into `bundle/` |

`bundle/lib/` and `bundle/models/` are generated — run `npm run assets` after a clone.

## Commands

The `anna-app` CLI needs Node 22+.

```bash
npm run assets     # libs + shared cores + models → bundle/
npm test           # node --test
npm run validate   # anna-app validate --strict
npm run dev        # harness on http://localhost:5180 (real LLM + storage after `anna-app login`)
anna-app apps push     # update the working draft (installable as 0.0.0-draft)
anna-app apps publish  # cut the version in app.json
```

Asking Anna's assistant with `#Desktop Companion …` opens the app with a payload
(`{mode, text, target, intent, tone}`, see `system_prompt_addendum` in `manifest.json`).
