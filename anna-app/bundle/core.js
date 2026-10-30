// Pure logic for the Anna App: prompts, reply-tag parsing, history and task
// bookkeeping. No DOM, no SDK — tested by test/core.test.js under node --test.

export const EMOTIONS = ['happy', 'sad', 'angry', 'shy', 'surprised', 'wink', 'thinking'];

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
    'Personality: upbeat, a bit playful, caring, occasionally a little pouty. Reply in the language the user writes in.',
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
  const intentLine = intent.trim()
    ? `What the user wants to say (may be in another language): ${intent.trim()}`
    : 'The user has not said what to reply; write the most natural, helpful reply.';
  return {
    systemPrompt:
      'You write replies to messages on behalf of the user. ' +
      'Write the reply in the SAME language as the original message. ' +
      `Tone: ${toneLine}. Output only the reply text, ready to paste — no quotes, no explanations.`,
    messages: [{ role: 'user', content: `Original message:\n${message}\n\n${intentLine}` }],
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
