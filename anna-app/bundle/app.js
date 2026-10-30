import {
  LANGS,
  TONES,
  applyTaskTags,
  award,
  bondLevel,
  chatSystemPrompt,
  isUnlocked,
  newProgress,
  newTaskId,
  normalizePayload,
  parseReply,
  pickLine,
  replyPrompt,
  streamingText,
  translatePrompt,
  trimHistory,
  uiLang,
  unwrap,
} from './core.js';
import { STRINGS } from './i18n.js';
import { Stage } from './stage.js';
import { Voice } from './voice.js';
import { mountGames } from './games.js';
import { mountCalendar, mountCollection, mountFocus, mountNotes } from './extras.js';
import { connectHost, load, save } from './host.js';
import { runPet } from './pet.js';

const $ = (id) => document.getElementById(id);
const KEY = { settings: 'settings', history: 'chat/history', tasks: 'tasks', progress: 'progress' };
const IDLE_MS = [4 * 60_000, 7 * 60_000];

const state = {
  anna: null,
  lang: 'en',
  settings: { model: 'gothic-pastel', translateTarget: '', voice: false },
  history: [],
  tasks: [],
  progress: newProgress(),
  models: [],
  busy: false,
  lastLine: '',
  panels: {},
};

const t = (key) => {
  const own = STRINGS[state.lang] && STRINGS[state.lang][key];
  return own !== undefined ? own : STRINGS.en[key] !== undefined ? STRINGS.en[key] : key;
};

let stage;
const voice = new Voice((level) => stage && stage.setMouth(level));

// ---------------------------------------------------------------- errors / LLM

function errorText(e) {
  if (!state.anna) return t('errNoHost');
  const code = (e && (e.code || (e.error && e.error.code))) || '';
  if (/QUOTA/i.test(code)) return t('errQuota');
  return t('errGeneric') + ((e && e.message) || String(e));
}

/**
 * Ask the user's Anna model. Streams through `onText(partial)` when the host
 * supports llm.stream, otherwise falls back to a single llm.complete.
 */
async function ask(args, onText) {
  if (!state.anna) throw new Error('no host');
  const llm = state.anna.llm;
  if (onText && llm && typeof llm.stream === 'function') {
    try {
      let stream = llm.stream(args);
      if (stream && typeof stream.then === 'function' && !stream[Symbol.asyncIterator]) stream = await stream;
      let text = '';
      for await (const frame of stream) {
        const f = frame && frame.payload ? frame.payload : frame;
        if (f && f.event === 'model_token' && f.text) {
          text += f.text;
          onText(text);
        } else if (f && f.event === 'complete') {
          const final = f.content && f.content.text;
          return final || text;
        }
      }
      return text;
    } catch (e) {
      const code = String((e && e.code) || '');
      if (code !== '-32601' && !/not.?found|not.?implemented|unknown method/i.test(code + ' ' + (e && e.message))) throw e;
      console.warn('[companion] llm.stream unavailable, falling back to complete');
    }
  }
  const res = unwrap(await llm.complete(args));
  const text = (res && res.content && res.content.text) || '';
  if (onText) onText(text);
  return text;
}

// ---------------------------------------------------------------- pet: bubble, voice, rewards

function say(text, spoken = false) {
  $('bubbleText').textContent = text;
  $('bubble').hidden = !text;
  if (spoken) speak(text);
}

function speak(text) {
  if (!state.settings.voice || !state.anna || !text) return;
  voice.speak(state.anna, text, state.lang).catch((e) => {
    console.warn('[companion] voice', e);
    const code = String((e && e.code) || '');
    if (/NOT_GRANTED|permission/i.test(code + (e && e.message))) {
      state.settings.voice = false;
      renderVoice();
      save(KEY.settings, state.settings);
    }
    say(t('errVoice') + ((e && e.message) || code));
  });
}

function renderVoice() {
  const b = $('voiceBtn');
  b.textContent = state.settings.voice ? '🔊' : '🔇';
  b.title = t(state.settings.voice ? 'voiceOn' : 'voiceOff');
}

function renderStatus() {
  const b = bondLevel(state.progress.bond);
  $('status').textContent = `🍬 ${state.progress.candy} · 💗 ${b.level}${state.badge ? ' · 🍅 ' + state.badge : ''}`;
}

async function setProgress(p) {
  state.progress = p;
  renderStatus();
  if (state.panels.collection) state.panels.collection.refresh();
  await save(KEY.progress, p);
}

async function earn(event) {
  const { progress, gained } = award(state.progress, event);
  if (gained.candy > 0) stage.burst(['🍬', '✨'], null, 4);
  await setProgress(progress);
}

function lineFor(kind) {
  const line = pickLine(t(kind), Math.random, state.lastLine);
  state.lastLine = line;
  return line;
}

function scheduleIdle() {
  const wait = IDLE_MS[0] + Math.random() * (IDLE_MS[1] - IDLE_MS[0]);
  setTimeout(() => {
    if (!state.busy && document.visibilityState === 'visible') {
      const h = new Date().getHours();
      const pool = h >= 5 && h < 10 ? 'linesMorning' : h >= 23 || h < 4 ? 'linesNight' : 'linesIdle';
      say(lineFor(pool));
      stage.react(pickLine(['happy', 'wink', 'thinking'], Math.random));
    }
    scheduleIdle();
  }, wait);
}

// ---------------------------------------------------------------- chat

function renderChat() {
  const log = $('chatLog');
  log.textContent = '';
  for (const m of state.history) {
    const div = document.createElement('div');
    div.className = `msg ${m.role}`;
    div.textContent = m.role === 'assistant' ? parseReply(m.content).text : m.content;
    log.appendChild(div);
  }
  log.scrollTop = log.scrollHeight;
}

function chatError(e) {
  const div = document.createElement('div');
  div.className = 'msg error';
  div.textContent = errorText(e);
  $('chatLog').appendChild(div);
  $('chatLog').scrollTop = $('chatLog').scrollHeight;
}

async function sendChat(text) {
  if (state.busy || !text.trim()) return;
  state.busy = true;
  $('chatSend').disabled = true;
  const history = trimHistory([...state.history, { role: 'user', content: text.trim() }]);
  state.history = history;
  renderChat();
  say(t('thinking'));
  stage.react('thinking');
  try {
    const raw = await ask(
      {
        systemPrompt: chatSystemPrompt({ petName: state.settings.petName || '', tasks: state.tasks }),
        messages: history,
        maxTokens: 600,
        temperature: 0.8,
      },
      (partial) => say(streamingText(partial) || t('thinking')),
    );
    const reply = parseReply(raw);
    state.history = trimHistory([...history, { role: 'assistant', content: raw }]);
    say(reply.text, true);
    stage.react(reply.emotions[0]);
    if (reply.tasks.length || reply.done.length) {
      state.tasks = applyTaskTags(state.tasks, reply);
      renderTasks();
      await save(KEY.tasks, state.tasks);
      for (let i = 0; i < reply.done.length; i++) await earn('task');
    }
    renderChat();
    await save(KEY.history, state.history);
    await earn('chat');
  } catch (e) {
    console.error('[companion] chat', e);
    say('');
    chatError(e);
  } finally {
    state.busy = false;
    $('chatSend').disabled = false;
  }
}

// ---------------------------------------------------------------- translate / reply

async function runTool(button, outBox, outText, args) {
  button.disabled = true;
  outBox.hidden = false;
  outText.textContent = t('working');
  stage.react('thinking');
  try {
    const text = await ask({ ...args, maxTokens: 2000, temperature: 0.3 }, (p) => {
      outText.textContent = p;
    });
    outText.textContent = text.trim();
    stage.react('happy');
    await earn('tool');
  } catch (e) {
    console.error('[companion] tool', e);
    outText.textContent = errorText(e);
  } finally {
    button.disabled = false;
  }
}

const runTranslate = () => {
  const text = $('trInput').value.trim();
  if (text) runTool($('trGo'), $('trOut'), $('trText'), translatePrompt(text, $('trTarget').value));
};
const runReply = () => {
  const message = $('rpMessage').value.trim();
  if (message) runTool($('rpGo'), $('rpOut'), $('rpText'), replyPrompt({ message, intent: $('rpIntent').value, tone: $('rpTone').value }));
};

async function copyText(id, button) {
  const text = $(id).textContent;
  try {
    await navigator.clipboard.writeText(text);
  } catch {
    const ta = document.createElement('textarea');
    ta.value = text;
    document.body.appendChild(ta);
    ta.select();
    document.execCommand('copy');
    ta.remove();
  }
  const label = button.textContent;
  button.textContent = t('copied');
  setTimeout(() => (button.textContent = label), 1200);
}

// ---------------------------------------------------------------- tasks

function renderTasks() {
  const list = $('taskList');
  list.textContent = '';
  if (!state.tasks.length) {
    const li = document.createElement('li');
    li.className = 'empty';
    li.textContent = t('noTasks');
    list.appendChild(li);
    return;
  }
  for (const task of state.tasks) {
    const li = document.createElement('li');
    li.className = task.done ? 'done' : '';
    const box = document.createElement('input');
    box.type = 'checkbox';
    box.checked = !!task.done;
    box.addEventListener('change', async () => {
      await updateTasks(state.tasks.map((x) => (x.id === task.id ? { ...x, done: box.checked } : x)));
      if (box.checked) {
        stage.react('happy');
        await earn('task');
      }
    });
    const span = document.createElement('span');
    span.textContent = task.text;
    const del = document.createElement('button');
    del.textContent = '✕';
    del.title = t('remove');
    del.addEventListener('click', () => updateTasks(state.tasks.filter((x) => x.id !== task.id)));
    li.append(box, span, del);
    list.appendChild(li);
  }
}

async function updateTasks(next) {
  state.tasks = next;
  renderTasks();
  await save(KEY.tasks, state.tasks);
}

// ---------------------------------------------------------------- models

function fillModelPick() {
  const pick = $('modelPick');
  pick.textContent = '';
  for (const m of state.models) if (isUnlocked(state.progress, m.id)) pick.add(new Option(m.name, m.id));
  pick.value = state.settings.model;
}

async function useModel(id) {
  const entry = state.models.find((m) => m.id === id) || state.models[0];
  if (!isUnlocked(state.progress, entry.id)) {
    fillModelPick();
    return;
  }
  $('loading').hidden = false;
  try {
    await stage.load(entry.url);
    state.settings.model = entry.id;
    await save(KEY.settings, state.settings);
  } catch (e) {
    console.error('[companion] Live2D failed', e);
  } finally {
    $('loading').hidden = true;
    fillModelPick();
    if (state.panels.collection) state.panels.collection.refresh();
  }
}

// ---------------------------------------------------------------- tabs & Anna chat payloads

function showTab(name) {
  document.querySelectorAll('.tabs button').forEach((b) => b.classList.toggle('active', b.dataset.tab === name));
  document.querySelectorAll('.panel').forEach((p) => p.classList.toggle('active', p.dataset.panel === name));
  const btn = document.querySelector(`.tabs button[data-tab="${name}"]`);
  if (btn) btn.scrollIntoView({ inline: 'nearest', block: 'nearest' });
}

/** Anna's assistant opened us (open_app_view) with a payload, e.g. "#Desktop Companion translate …". */
function applyPayload(raw) {
  const p = normalizePayload(raw);
  if (!p) return;
  showTab(p.mode);
  if (p.mode === 'translate') {
    if (p.text) $('trInput').value = p.text;
    if (p.target) $('trTarget').value = p.target;
    if (p.text && p.autorun) runTranslate();
  } else if (p.mode === 'reply') {
    if (p.text) $('rpMessage').value = p.text;
    if (p.intent) $('rpIntent').value = p.intent;
    if (p.tone) $('rpTone').value = p.tone;
    if (p.text && p.autorun) runReply();
  } else if (p.mode === 'chat' && p.text) {
    if (p.autorun) sendChat(p.text);
    else $('chatInput').value = p.text;
  } else if (p.mode === 'tasks' && p.text) {
    updateTasks([...state.tasks, { id: newTaskId(state.tasks), text: p.text, done: false }]);
  }
}

// ---------------------------------------------------------------- UI wiring

function applyI18n() {
  document.documentElement.lang = state.lang;
  document.querySelectorAll('[data-i18n]').forEach((el) => (el.textContent = t(el.dataset.i18n)));
  document.querySelectorAll('[data-i18n-placeholder]').forEach((el) => (el.placeholder = t(el.dataset.i18nPlaceholder)));
  const tone = $('rpTone');
  const toneValue = tone.value || 'friendly';
  tone.textContent = '';
  for (const id of TONES) tone.add(new Option(t('tone' + id[0].toUpperCase() + id.slice(1)), id));
  tone.value = toneValue;
  renderVoice();
}

function wire() {
  document.querySelectorAll('.tabs button').forEach((btn) => btn.addEventListener('click', () => showTab(btn.dataset.tab)));

  $('chatForm').addEventListener('submit', (e) => {
    e.preventDefault();
    const text = $('chatInput').value;
    $('chatInput').value = '';
    voice.unlock();
    sendChat(text);
  });
  $('chatInput').addEventListener('keydown', (e) => {
    if (e.key === 'Enter' && !e.shiftKey && !e.isComposing) {
      e.preventDefault();
      $('chatForm').requestSubmit();
    }
  });
  $('chatClear').addEventListener('click', async () => {
    state.history = [];
    renderChat();
    await save(KEY.history, []);
  });

  $('trGo').addEventListener('click', runTranslate);
  $('trTarget').addEventListener('change', () => {
    state.settings.translateTarget = $('trTarget').value;
    save(KEY.settings, state.settings);
  });
  $('rpGo').addEventListener('click', runReply);
  document.querySelectorAll('[data-copy]').forEach((btn) => btn.addEventListener('click', () => copyText(btn.dataset.copy, btn)));

  $('taskForm').addEventListener('submit', (e) => {
    e.preventDefault();
    const text = $('taskInput').value.trim();
    if (!text) return;
    $('taskInput').value = '';
    updateTasks([...state.tasks, { id: newTaskId(state.tasks), text, done: false }]);
  });

  $('modelPick').addEventListener('change', () => useModel($('modelPick').value));
  $('voiceBtn').addEventListener('click', () => {
    voice.unlock();
    state.settings.voice = !state.settings.voice;
    if (!state.settings.voice) voice.stop();
    renderVoice();
    save(KEY.settings, state.settings);
  });
}

async function main() {
  state.anna = await connectHost();
  const anna = state.anna;
  // The floating pet window (view "pet") shares this page but runs its own small mode.
  const view = new URLSearchParams(location.search).get('view') || (anna && anna.viewMeta && anna.viewMeta.name);
  if (view === 'pet') return runPet(anna);
  state.lang = uiLang((anna && anna.locale) || navigator.language);
  if (anna && typeof anna.onThemeChange === 'function') {
    anna.onThemeChange(({ theme }) => (document.documentElement.dataset.theme = theme));
  }

  const [settings, history, tasks, progress, models] = await Promise.all([
    load(KEY.settings, {}),
    load(KEY.history, []),
    load(KEY.tasks, []),
    load(KEY.progress, null),
    fetch('./models/index.json').then((r) => r.json()),
  ]);
  state.settings = { ...state.settings, ...settings };
  state.history = trimHistory(history);
  state.tasks = Array.isArray(tasks) ? tasks : [];
  state.progress = { ...newProgress(), ...(progress || {}) };
  state.models = models;
  // A saved model may be locked (or gone): fall back to the first one the user owns.
  if (!models.some((m) => m.id === state.settings.model && isUnlocked(state.progress, m.id))) {
    state.settings.model = (models.find((m) => isUnlocked(state.progress, m.id)) || models[0]).id;
  }

  stage = new Stage($('stage'), $('live2d'), {
    onPat: () => {
      say(lineFor('linesPat'), true);
      earn('pat');
    },
    onPoke: () => say(lineFor('linesPoke'), true),
  });

  applyI18n();
  const target = $('trTarget');
  for (const [code, name] of Object.entries(LANGS)) target.add(new Option(name, code));
  target.value = state.settings.translateTarget || (state.lang === 'en' ? 'vi' : 'en');
  wire();
  renderChat();
  renderTasks();

  const ctx = {
    t,
    lang: () => state.lang,
    load,
    save,
    say,
    react: (emotion) => stage.react(emotion),
    earn,
    chime: () => voice.chime(),
    unlockAudio: () => voice.unlock(),
    setBadge: (text) => {
      if (state.badge !== text) {
        state.badge = text;
        renderStatus();
      }
    },
    progress: () => state.progress,
    setProgress,
    settings: () => state.settings,
    models: state.models,
    useModel,
  };
  state.panels.games = mountGames($('gamesPanel'), ctx);
  state.panels.calendar = mountCalendar($('calendarPanel'), ctx);
  state.panels.collection = mountCollection($('collectionPanel'), ctx);
  await Promise.all([mountNotes($('notesPanel'), ctx), mountFocus($('focusPanel'), ctx)]);

  if (anna && typeof anna.onLocaleChange === 'function') {
    anna.onLocaleChange(({ lang }) => {
      const next = uiLang(lang);
      if (next === state.lang) return;
      state.lang = next;
      applyI18n();
      renderTasks();
      Object.values(state.panels).forEach((p) => p && p.refresh && p.refresh());
    });
  }
  if (anna) {
    anna.window.set_title({ title: 'Desktop Companion' }).catch(() => {});
    anna.on('entry_payload', applyPayload);
  }

  renderStatus();
  await useModel(state.settings.model);
  const before = state.progress.day;
  await earn('login');
  say(state.history.length ? parseReply(state.history.at(-1).content).text : t('greeting'));
  stage.react('happy');
  if (before !== state.progress.day) stage.burst(['🍬', '✨', '💗'], null, 6);
  if (anna && anna.entryPayload) applyPayload(anna.entryPayload);
  scheduleIdle();
}

main();
