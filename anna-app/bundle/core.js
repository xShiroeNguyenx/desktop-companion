// Pure logic for the Anna App: prompts, reply-tag parsing, history and task
// bookkeeping. No DOM, no SDK — tested by test/core.test.js under node --test.

export const EMOTIONS = ['happy', 'sad', 'angry', 'shy', 'surprised', 'wink', 'thinking'];

/**
 * Face parameters a reaction motion writes, and the value each one rests at (same
 * neutral face as the desktop app). The idle motions drive none of them, so nothing
 * else brings them back once a reaction ends.
 */
export const FACE_REST = {
  ParamMouthForm: 0,
  ParamEyeLSmile: 0,
  ParamEyeRSmile: 0,
  ParamCheek: 0,
  ParamBrowLY: 0,
  ParamBrowRY: 0,
  ParamBrowLAngle: 0,
  ParamBrowRAngle: 0,
  ParamEyeLOpen: 1,
  ParamEyeROpen: 1,
};

/**
 * One step of easing `value` back to `rest`: closes ~63% of the gap every `tauMs`,
 * whatever the frame rate, and lands exactly on `rest` once it is close enough.
 */
export function easeToRest(value, rest, dtMs, tauMs = 120) {
  if (!Number.isFinite(value)) return rest;
  if (!(dtMs > 0)) return value;
  const next = rest + (value - rest) * Math.exp(-dtMs / tauMs);
  return Math.abs(next - rest) < 0.001 ? rest : next;
}

/**
 * Should the blink advance this frame? Always while she is resting. During a reaction
 * its curves own the eyes, except that a blink already under way (Cubism eye states
 * 2 closing, 3 closed, 4 opening) is finished, so the eyes are not left half shut.
 */
export function shouldBlink(reacting, blinkState) {
  return !reacting || (blinkState >= 2 && blinkState <= 4);
}

/** Keep the last `maxTurns` user/assistant pairs (2 messages per turn). */
export const HISTORY_TURNS = 10;

export const LANGS = {
  en: 'English',
  vi: 'Vietnamese',
  ja: 'Japanese',
  zh: 'Chinese (Simplified)',
  ko: 'Korean',
  fr: 'French',
  de: 'German',
  es: 'Spanish',
};

/** Map an Anna locale ("en", "zh-CN", "vi-VN"…) to one of our UI languages. */
export function uiLang(locale) {
  const base = String(locale || '').toLowerCase().split(/[-_]/)[0];
  return base === 'vi' || base === 'ja' ? base : 'en';
}

/**
 * Did audio.speak fail because the user has not granted the app voice (so the voice
 * switch should turn itself off), as opposed to a passing error?
 */
export function voiceDenied(e) {
  return /NOT_GRANTED|permission/i.test(String((e && e.code) || '') + ' ' + String((e && e.message) || ''));
}

/**
 * A bubble line as it should be read out: no emoji, hearts, stars or trailing "~",
 * which a voice either skips or spells out.
 */
export function spokenText(text) {
  return String(text || '')
    .replace(/[\p{Extended_Pictographic}☀-➿️‍~～]/gu, ' ')
    .replace(/\s+/g, ' ')
    .replace(/\s+([!?.,…。！？])/g, '$1')
    .trim();
}

/** The languages the app's own text exists in, by their own names. */
export const UI_LANGS = { en: 'English', vi: 'Tiếng Việt', ja: '日本語' };

/**
 * The language to show: the one the user picked in the app (settings.lang), else
 * Anna's. Anything that is not one of ours ("", "auto", an old value) means "follow Anna".
 */
export function pickLang(choice, hostLocale) {
  return Object.hasOwn(UI_LANGS, String(choice)) ? choice : uiLang(hostLocale);
}

/** Unwrap an SDK RPC response that may be `{ok, result}` / `{ok:false, error}` or a bare result. */
export function unwrap(res) {
  if (res && typeof res === 'object' && 'ok' in res) {
    if (res.ok) return res.result;
    const err = new Error((res.error && res.error.message) || 'request failed');
    err.code = res.error && res.error.code;
    throw err;
  }
  return res;
}

// The pet window tells an already-open full panel to pick up "openTab" now.
export const PANEL_CHANNEL = 'desktop-companion/panel';

/**
 * Anna opens the panel fitted to the whole workspace (measured: 1416×737 for a
 * declared 440×720), which stretches the layout. Returns the declared size to ask
 * for when the window starts out much wider than that, else null. `done` = this
 * window was already sized once (a page switch inside it must not undo the user's
 * own resize).
 */
export function startSize(inner, declared, done) {
  if (done || !inner || !declared || !(declared.w > 0) || !(declared.h > 0)) return null;
  return inner.w > declared.w * 1.5 ? { w: declared.w, h: declared.h } : null;
}

/**
 * Dragging the tab strip with the mouse: `start` is { x, left } at pointer-down
 * (pointer x and the strip's scrollLeft). Returns where the strip should be scrolled
 * to, and whether the pointer has moved far enough to be a drag and not a click.
 */
export function tabDrag(start, x, threshold = 5) {
  const dx = x - start.x;
  return { scrollLeft: Math.max(0, start.left - dx), moved: Math.abs(dx) > threshold };
}

/**
 * The tab in a stored "openTab" request ({ tab, at }) while it is recent enough
 * to be the click that made it, else null: an old request must not hijack a
 * later start. A bare string is the format from before timestamps.
 */
export function pendingTab(value, now = Date.now(), maxAgeMs = 15000) {
  if (typeof value === 'string') return value || null;
  if (!value || typeof value.tab !== 'string') return null;
  return now - Number(value.at) <= maxAgeMs ? value.tab : null;
}

// The app's own URL keys; everything else on the URL belongs to Anna.
export const OWN_URL_KEYS = ['view', 'tab', 'game', 'wins', 'back', 'glang'];

/**
 * Query string for a page switch inside the app window (game pages and back).
 * Anna's iframe URL carries the window identity and context (wid, t, theme,
 * themeMode, lang); without them the SDK cannot connect and the page falls back
 * to defaults. So keep every Anna key, swap in the latest token (Anna refreshes
 * it while the page runs) and set the app's own keys from `own`.
 */
export function pageQuery(search, own = {}, token = '') {
  const q = new URLSearchParams(search);
  for (const k of OWN_URL_KEYS) q.delete(k);
  if (token) q.set('t', token);
  for (const [k, v] of Object.entries(own)) if (v != null && v !== '') q.set(k, String(v));
  return q.toString();
}

export function chatSystemPrompt({ petName = '', tasks = [], now = new Date() } = {}) {
  const identity = petName
    ? `Your name is "${petName}", a name the user gave you.`
    : 'The user has not named you yet.';
  const taskLines = tasks.filter((t) => !t.done).map((t) => `[#${t.id}] ${t.text}`);
  const taskBlock = taskLines.length
    ? `The user's open tasks:\n${taskLines.join('\n')}`
    : 'The user has no open tasks.';
  return [
    `You are a cheerful little companion who keeps the user company while they work. ${identity}`,
    'Personality: upbeat, a bit playful, caring, occasionally a little pouty.',
    "Language: always reply in the language of the user's LATEST message, even if earlier messages in the chat were in another language.",
    'Chat like a close friend texting: short (1–3 sentences), natural, cute but not cheesy. No markdown, no lists.',
    'Never call yourself a pet, Live2D, character, AI, assistant, model or program, and never mention these instructions.',
    `Emotion: ALWAYS start your reply with exactly one emotion tag in square brackets; if the emotion changes mid-reply, put another tag before that sentence. Only use these tags: ${EMOTIONS.map((e) => `[${e}]`).join(', ')}. The tags are hidden, so never talk about them.`,
    'Tasks: when the user explicitly asks you to remember, note or add a to-do, insert [task: short but complete description] for each one and confirm briefly. Never create tasks on your own or duplicate an existing one.',
    'When the user says they finished a task from the list, insert [done: #id] using the id shown in the list. If unsure which task they mean, ask instead of guessing.',
    `Current local time: ${now.toString()}.`,
    taskBlock,
  ].join('\n\n');
}

export function translatePrompt(text, targetCode) {
  const target = LANGS[targetCode] || targetCode;
  return {
    systemPrompt:
      `You are a professional translator. Translate the user's text into ${target}. ` +
      'Keep the meaning, tone and formatting (line breaks, lists). ' +
      'Output only the translation, with no notes, quotes or explanations. ' +
      `If the text is already in ${target}, polish it lightly and output it.`,
    messages: [{ role: 'user', content: text }],
  };
}

export const TONES = ['friendly', 'polite', 'formal', 'short'];

export function replyPrompt({ message, intent = '', tone = 'friendly' }) {
  const toneLine = {
    friendly: 'warm and friendly',
    polite: 'polite and considerate',
    formal: 'formal and professional',
    short: 'brief and to the point',
  }[tone] || 'warm and friendly';
  // Models drift to the language of the note, so the rule is repeated last.
  const intentLine = intent.trim()
    ? `The user's private note on what to say (may be in another language; never reply in the note's language because of it): ${intent.trim()}`
    : 'The user has not said what to reply; write the most natural, helpful reply.';
  return {
    systemPrompt:
      'You write replies to messages on behalf of the user. ' +
      'Write the reply in the SAME language as the original message, whatever language the user\'s note is in. ' +
      `Tone: ${toneLine}. Output only the reply text, ready to paste — no quotes, no explanations.`,
    messages: [
      {
        role: 'user',
        content: `Original message:\n${message}\n\n${intentLine}\n\nWrite the reply in the language of the original message.`,
      },
    ],
  };
}

const TAG_RE = /\[(\w+)(?::\s*([^\]]*))?\]/g;

/**
 * Split a model reply into what to show and what to do.
 * Returns { text, emotions[], tasks[] (text strings), done[] (ids) }.
 */
export function parseReply(raw) {
  const emotions = [];
  const tasks = [];
  const done = [];
  const text = String(raw || '')
    .replace(TAG_RE, (whole, name, arg) => {
      const key = name.toLowerCase();
      if (arg === undefined && EMOTIONS.includes(key)) {
        emotions.push(key);
        return '';
      }
      if (key === 'task' && arg && arg.trim()) {
        tasks.push(arg.split('|')[0].trim());
        return '';
      }
      if (key === 'done' && arg) {
        const id = arg.replace(/[#\s]/g, '');
        if (id) done.push(id);
        return '';
      }
      return whole;
    })
    .replace(/[ \t]{2,}/g, ' ')
    .trim();
  return { text, emotions, tasks, done };
}

/** Text to show while a reply is still streaming: drop finished tags and a dangling "[…". */
export function streamingText(partial) {
  const cut = String(partial || '').replace(/\[[^\]]*$/, '');
  return parseReply(cut).text;
}

/**
 * Copy of the chat for the model with a reply-language reminder on the last
 * user turn: a long chat in one language otherwise outweighs the system prompt
 * when the user switches. The stored history stays as typed.
 */
export function withLanguageHint(history) {
  const list = Array.isArray(history) ? history.slice() : [];
  const last = list.at(-1);
  if (last && last.role === 'user') {
    list[list.length - 1] = { ...last, content: `${last.content}\n\n(Reply in the language of this message.)` };
  }
  return list;
}

export function trimHistory(history, maxTurns = HISTORY_TURNS) {
  const list = Array.isArray(history) ? history : [];
  return list.slice(-maxTurns * 2);
}

/** Short random id, unique within the user's list. */
export function newTaskId(tasks, rand = Math.random) {
  const taken = new Set((tasks || []).map((t) => String(t.id)));
  for (;;) {
    const id = String(1000 + Math.floor(rand() * 9000));
    if (!taken.has(id)) return id;
  }
}

// ---------------------------------------------------------------- candy, bond, collection

/** Candy / bond for each event. `daily` caps how many times per day an event pays. */
export const REWARDS = {
  login: { candy: 5, bond: 2, daily: 1 },
  chat: { candy: 1, bond: 1, daily: 20 },
  pat: { candy: 0, bond: 1, daily: 10 },
  task: { candy: 3, bond: 2, daily: 10 },
  focus: { candy: 5, bond: 3, daily: 8 },
  rpsWin: { candy: 2, bond: 1, daily: 10 },
  guessWin: { candy: 3, bond: 1, daily: 10 },
  minesWin: { candy: 4, bond: 2, daily: 10 },
  tool: { candy: 1, bond: 0, daily: 10 },
};

/** Bond points needed to reach each level (index = level - 1). */
export const BOND_LEVELS = [0, 25, 80, 200, 450];

export function newProgress() {
  return { candy: 0, bond: 0, day: '', counts: {}, unlocked: [], streak: 0, lastDay: '' };
}

/** Local day key "2026-10-29". */
export function dayOf(date = new Date()) {
  const d = new Date(date);
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;
}

/**
 * Pay one event. Returns { progress, gained: {candy, bond} } — gained is zero
 * when the daily cap for that event is used up. Never mutates the input.
 */
export function award(progress, event, now = new Date()) {
  const rule = REWARDS[event];
  const p = { ...newProgress(), ...(progress || {}) };
  p.counts = { ...(p.counts || {}) };
  p.unlocked = [...(p.unlocked || [])];
  const today = dayOf(now);
  if (p.day !== today) {
    p.day = today;
    p.counts = {};
  }
  if (!rule) return { progress: p, gained: { candy: 0, bond: 0 } };
  const used = p.counts[event] || 0;
  if (used >= rule.daily) return { progress: p, gained: { candy: 0, bond: 0 } };
  p.counts[event] = used + 1;
  if (event === 'login') {
    const yesterday = dayOf(new Date(new Date(now).setDate(new Date(now).getDate() - 1)));
    p.streak = p.lastDay === yesterday ? (p.streak || 0) + 1 : 1;
    p.lastDay = today;
  }
  p.candy += rule.candy;
  p.bond += rule.bond;
  return { progress: p, gained: { candy: rule.candy, bond: rule.bond } };
}

/** { level 1..5, into: points past this level, need: points to next (0 at max) } */
export function bondLevel(points) {
  const pts = Math.max(0, Number(points) || 0);
  let level = 1;
  for (let i = 0; i < BOND_LEVELS.length; i++) if (pts >= BOND_LEVELS[i]) level = i + 1;
  const next = BOND_LEVELS[level];
  return { level, into: pts - BOND_LEVELS[level - 1], need: next === undefined ? 0 : next - BOND_LEVELS[level - 1] };
}

/** Models everyone starts with; the rest are unlocked with candy. */
export const FREE_MODELS = ['gothic-pastel', 'pink-kawaii', 'snow-dancer'];

export function modelCost(id, index = 0) {
  return FREE_MODELS.includes(id) ? 0 : 30 + 10 * (index % 4);
}

export function isUnlocked(progress, id) {
  return FREE_MODELS.includes(id) || ((progress && progress.unlocked) || []).includes(id);
}

/** Spend candy on a model. Returns { progress } or { error: 'owned' | 'candy' }. */
export function unlockModel(progress, id, cost) {
  const p = { ...newProgress(), ...(progress || {}) };
  if (isUnlocked(p, id)) return { error: 'owned' };
  if (p.candy < cost) return { error: 'candy' };
  return { progress: { ...p, candy: p.candy - cost, unlocked: [...(p.unlocked || []), id] } };
}

// ---------------------------------------------------------------- focus (pomodoro)

export const FOCUS = { work: 25 * 60_000, break: 5 * 60_000 };

/** Start a phase now. */
export function focusStart(phase = 'work', now = Date.now()) {
  return { phase, endsAt: now + FOCUS[phase], running: true };
}

/** ms left in the current phase (0 when finished or stopped). */
export function focusLeft(state, now = Date.now()) {
  if (!state || !state.running) return 0;
  return Math.max(0, state.endsAt - now);
}

/**
 * A running timer that has reached its end: the stopped state to save, and whether a
 * work session was completed (sessions count per day). Null while it still runs or is
 * already stopped. Shared by the panel and the floating pet: only one of the two
 * windows is open at a time, and whichever it is has to ring.
 */
export function focusFinish(state, now = Date.now()) {
  if (!state || !state.running || focusLeft(state, now) > 0) return null;
  const finishedWork = state.phase === 'work';
  const day = dayOf(new Date(now));
  const sessions = (state.day === day ? state.sessions || 0 : 0) + (finishedWork ? 1 : 0);
  return { state: { ...state, running: false, day, sessions }, finishedWork };
}

export function fmtClock(ms) {
  const s = Math.ceil(Math.max(0, ms) / 1000);
  return `${String(Math.floor(s / 60)).padStart(2, '0')}:${String(s % 60).padStart(2, '0')}`;
}

// ---------------------------------------------------------------- Anna chat → app

const MODES = ['chat', 'translate', 'reply', 'tasks', 'notes', 'focus', 'games', 'calendar', 'collection'];

/**
 * Normalise an `entry_payload` sent by Anna's assistant (open_app_view). Accepts
 * a few loose shapes ({text}, {mode:'translate', text, target}, a bare string).
 * Returns null when there is nothing to do.
 */
export function normalizePayload(raw) {
  if (raw == null || raw === '') return null;
  const p = typeof raw === 'string' ? { text: raw } : typeof raw === 'object' ? raw : null;
  if (!p) return null;
  const str = (v, max = 8000) => (typeof v === 'string' ? v.trim().slice(0, max) : '');
  let mode = str(p.mode || p.tab || p.view, 20).toLowerCase();
  const text = str(p.text || p.message || p.content);
  if (!MODES.includes(mode)) mode = text ? 'chat' : '';
  if (!mode) return null;
  const target = str(p.target || p.to || p.language, 10).toLowerCase();
  const tone = str(p.tone, 20).toLowerCase();
  return {
    mode,
    text,
    target: LANGS[target] ? target : '',
    intent: str(p.intent || p.say, 2000),
    tone: TONES.includes(tone) ? tone : '',
    autorun: p.autorun !== false,
  };
}

/** Pick one line, avoiding an immediate repeat of `last`. */
export function pickLine(lines, rand = Math.random, last = '') {
  const list = (lines || []).filter(Boolean);
  if (!list.length) return '';
  if (list.length === 1) return list[0];
  for (let i = 0; i < 5; i++) {
    const line = list[Math.floor(rand() * list.length) % list.length];
    if (line !== last) return line;
  }
  return list.find((l) => l !== last);
}

/** Apply the tags of one reply to the task list; returns a new list. */
export function applyTaskTags(tasks, { tasks: added = [], done = [] }, rand) {
  let out = (tasks || []).map((t) => ({ ...t }));
  for (const id of done) {
    out = out.map((t) => (String(t.id) === id ? { ...t, done: true } : t));
  }
  for (const text of added) {
    const exists = out.some((t) => !t.done && t.text.toLowerCase() === text.toLowerCase());
    if (!exists) out.push({ id: newTaskId(out, rand), text, done: false });
  }
  return out;
}

// ---------------------------------------------------------------- right-click menu

/** Tab label without its leading emoji ("💬 Chat" → "Chat"). */
const bare = (label) => String(label).replace(/^\S+\s+/, '');

/**
 * Entries of the pet's right-click ring (radial.js), like the desktop pet's.
 * `view` "pet" (floating window: tabs open the full panel) or "main".
 * `models` = the models the user owns ({ id, name, preview }), `current` = shown one.
 * `lang` = the language picked in the app ("" = follow Anna).
 * Actions: "tab:<name>", "model:<id>", "lang:<auto|en|vi|ja>", "voice", "pat", "open".
 */
export function petMenuItems({ view = 'main', voice = false, models = [], current = '', lang = '', t }) {
  const tab = (id, icon) => ({ icon, label: bare(t('tab' + id[0].toUpperCase() + id.slice(1))), action: 'tab:' + id });
  const items = [
    tab('chat', '💬'),
    { id: 'tools', icon: '🛠️', label: t('menuTools'), items: [tab('translate', '🌐'), tab('reply', '✍️')] },
    tab('tasks', '📝'),
    {
      id: 'together',
      icon: '💞',
      label: t('menuTogether'),
      items: [tab('focus', '🍅'), tab('games', '🎮'), tab('calendar', '📅'), tab('notes', '🗒️'), tab('collection', '🍬')],
    },
    {
      id: 'model',
      icon: '👗',
      label: t('menuModel'),
      items: models.map((m) => ({ icon: '👗', img: m.preview, label: m.name, action: 'model:' + m.id, active: m.id === current })),
    },
    { icon: '💗', label: t('menuPat'), action: 'pat' },
    {
      id: 'lang',
      icon: '🌐',
      label: t('menuLang'),
      items: [
        { icon: '🌐', label: t('langAuto'), action: 'lang:auto', active: !Object.hasOwn(UI_LANGS, lang) },
        ...Object.entries(UI_LANGS).map(([id, name]) => ({ icon: id.toUpperCase(), label: name, action: 'lang:' + id, active: lang === id })),
      ],
    },
  ];
  if (view === 'pet') items.unshift({ icon: '🗂️', label: t('menuOpen'), action: 'open' });
  else items.splice(5, 0, { icon: voice ? '🔊' : '🔇', label: t(voice ? 'voiceOn' : 'voiceOff'), action: 'voice' });
  return items;
}
