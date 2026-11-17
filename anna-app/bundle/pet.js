// Floating pet mode (view "pet", a transparent window): only the character,
// her bubble and a 🎀 button that opens the full panel (view "main"). Runs in
// index.html — app.js hands over here when the host says this window is "pet".
import { award, bondLevel, fmtClock, focusFinish, focusLeft, isUnlocked, newProgress, pageQuery, PANEL_CHANNEL, petMenuItems, pickLang, pickLine, voiceDenied } from './core.js';
import { STRINGS } from './i18n.js';
import { load, save } from './host.js';
import { createRadialMenu } from './radial.js';
import { Stage } from './stage.js';
import { Voice } from './voice.js';

const $ = (id) => document.getElementById(id);
const IDLE_MS = [4 * 60_000, 7 * 60_000];
let lang = 'en';
let progress = newProgress();
let lastLine = '';
let bubbleTimer;

const t = (key) => {
  const own = STRINGS[lang] && STRINGS[lang][key];
  return own !== undefined ? own : STRINGS.en[key];
};

/** Set by runPet once the voice exists: says a line out loud when the voice is on. */
let speakLine = null;

function say(text, spoken = false) {
  $('bubbleText').textContent = text;
  $('bubble').hidden = !text;
  clearTimeout(bubbleTimer);
  // The window is small and see-through: let the bubble go away on its own.
  if (text) bubbleTimer = setTimeout(() => ($('bubble').hidden = true), 8000);
  if (text && spoken && speakLine) speakLine(text);
}

function line(kind) {
  lastLine = pickLine(t(kind), Math.random, lastLine);
  return lastLine;
}

/** The focus timer started in the full panel (storage key "focus"), shown and finished here too. */
let focus = null;

function renderStatus() {
  const left = focusLeft(focus);
  $('status').textContent = `🍬 ${progress.candy} · 💗 ${bondLevel(progress.bond).level}${left > 0 ? ' · 🍅 ' + fmtClock(left) : ''}`;
}

async function earn(stage, event) {
  // Re-read first: the main panel may have changed candy since we loaded.
  const fresh = await load('progress', progress);
  const r = award({ ...newProgress(), ...fresh }, event);
  progress = r.progress;
  if (r.gained.candy > 0) stage.burst(['🍬', '✨'], null, 4);
  renderStatus();
  await save('progress', progress);
}

export async function runPet(anna) {
  document.documentElement.classList.add('pet-mode');
  // `choice` = the language picked in the app (settings.lang, "" = follow Anna).
  let choice = '';
  let hostLocale = (anna && anna.locale) || navigator.language;
  const useLang = () => {
    lang = pickLang(choice, hostLocale);
    document.documentElement.lang = lang;
    if ($('openBtn')) $('openBtn').title = t('openPanel');
    // The voice button's hint follows the language too (🔊 = on).
    if ($('voiceBtn').textContent) $('voiceBtn').title = t($('voiceBtn').textContent === '🔊' ? 'voiceOn' : 'voiceOff');
  };
  useLang();
  if (anna && typeof anna.onThemeChange === 'function') {
    anna.onThemeChange(({ theme }) => (document.documentElement.dataset.theme = theme));
  }
  if (anna && typeof anna.onLocaleChange === 'function') {
    anna.onLocaleChange(({ lang: l }) => {
      hostLocale = l || hostLocale;
      useLang();
    });
  }

  const [settings, saved, models] = await Promise.all([
    load('settings', {}),
    load('progress', null),
    fetch('./models/index.json').then((r) => r.json()),
  ]);
  choice = settings.lang || '';
  useLang();
  progress = { ...newProgress(), ...(saved || {}) };
  const entry =
    models.find((m) => m.id === settings.model && isUnlocked(progress, m.id)) ||
    models.find((m) => isUnlocked(progress, m.id)) ||
    models[0];

  const stage = new Stage($('stage'), $('live2d'), {
    framing: 'full',
    onPat: () => {
      say(line('linesPat'), true);
      earn(stage, 'pat');
    },
    onPoke: () => say(line('linesPoke'), true),
  });

  // Voice: the same switch as the full panel (settings.voice), and the same lines
  // spoken (head pats and pokes). It is billed on the user's Anna quota, so off by default.
  const voice = new Voice((level) => stage.setMouth(level));
  let voiceOn = !!settings.voice;
  const renderVoice = () => {
    $('voiceBtn').textContent = voiceOn ? '🔊' : '🔇';
    $('voiceBtn').title = t(voiceOn ? 'voiceOn' : 'voiceOff');
  };
  const setVoice = async (on) => {
    voiceOn = on;
    if (!on) voice.stop();
    renderVoice();
    await save('settings', { ...(await load('settings', {})), voice: on });
  };
  speakLine = (text) => {
    if (!voiceOn || !anna) return;
    voice.speak(anna, text, lang).catch((e) => {
      console.warn('[companion] voice', e);
      if (voiceDenied(e)) setVoice(false);
      say(t('errVoice') + ((e && e.message) || String((e && e.code) || '')));
    });
  };
  // Browsers only let audio start after a click or key press.
  for (const type of ['pointerdown', 'keydown']) window.addEventListener(type, () => voice.unlock(), true);
  $('voiceBtn').addEventListener('click', () => setVoice(!voiceOn));
  renderVoice();

  // Focus timer: the panel that started it closes when this window opens, so the
  // countdown and its end (chime, line, candy) carry on here.
  focus = await load('focus', null);
  let ringing = false;
  setInterval(async () => {
    renderStatus();
    if (ringing || !focusFinish(focus)) return;
    ringing = true;
    try {
      // Re-read first: a panel opened meanwhile may have stopped or finished it.
      focus = await load('focus', focus);
      const done = focusFinish(focus);
      if (!done) return;
      focus = done.state;
      await save('focus', focus);
      voice.chime();
      say(t(done.finishedWork ? 'focusDone' : 'focusBack'), true);
      stage.react(done.finishedWork ? 'happy' : 'wink');
      if (done.finishedWork) await earn(stage, 'focus');
    } finally {
      ringing = false;
      renderStatus();
    }
  }, 1000);

  const openMain = async () => {
    try {
      if (!anna) throw new Error('no host');
      await anna.window.open_view({ view: 'main' });
      // One window at a time: the panel's 🐾 button brings this one back.
      anna.window.close().catch((err) => console.warn('[companion] window.close', err));
    } catch (e) {
      // Hosts without window.open_view: turn this window into the full panel instead.
      console.warn('[companion] open_view unavailable, switching in place', e);
      if (anna) await anna.window.resize({ w: 440, h: 570 }).catch(() => {});
      location.replace(location.pathname + '?' + pageQuery(location.search, { view: 'main' }, anna && anna.token));
    }
  };
  const open = document.createElement('button');
  open.id = 'openBtn';
  open.type = 'button';
  open.className = 'open-panel';
  // Not a chat bubble: the panel holds every tool, not only chat.
  open.textContent = '🎀';
  open.title = t('openPanel');
  open.addEventListener('click', openMain);
  $('stage').querySelector('.stage-bar').appendChild(open);

  // Right-click menu, like the desktop pet. Tabs open the full panel on that
  // tab (app.js reads "openTab" when it starts, gets focus or hears PANEL_CHANNEL).
  let shown = entry;
  const menu = createRadialMenu({
    getItems: () =>
      petMenuItems({
        view: 'pet',
        models: models.filter((m) => isUnlocked(progress, m.id)),
        current: shown.id,
        lang: choice,
        t,
      }),
    onAction: async (action) => {
      const [kind, arg] = action.split(':');
      if (kind === 'open') openMain();
      else if (kind === 'tab') {
        await save('openTab', { tab: arg, at: Date.now() });
        if (typeof BroadcastChannel === 'function') new BroadcastChannel(PANEL_CHANNEL).postMessage({ tab: arg });
        openMain();
      } else if (kind === 'lang') {
        choice = arg === 'auto' ? '' : arg;
        useLang();
        // Answer in the new language, so the change shows at once.
        say(t('greeting'));
        // Same setting the full panel uses.
        await save('settings', { ...(await load('settings', {})), lang: choice });
      } else if (kind === 'pat') stage.pat(null);
      else if (kind === 'model' && arg !== shown.id) {
        const next = models.find((m) => m.id === arg);
        if (!next) return;
        $('loading').hidden = false;
        try {
          await stage.load(next.url);
          shown = next;
          // Same setting the full panel uses, so both windows show her.
          await save('settings', { ...(await load('settings', {})), model: next.id });
        } finally {
          $('loading').hidden = true;
        }
      }
    },
    labels: () => ({ title: t('menuTitle'), back: t('menuBack'), close: t('menuClose') }),
  });
  menu.attach(document.documentElement);
  if (anna) anna.window.set_title({ title: 'Desktop Companion' }).catch(() => {});

  renderStatus();
  $('loading').hidden = false;
  try {
    await stage.load(entry.url);
  } finally {
    $('loading').hidden = true;
  }
  stage.react('happy');
  say(t('greeting'));

  const idle = () =>
    setTimeout(() => {
      if (document.visibilityState === 'visible') {
        const h = new Date().getHours();
        say(line(h >= 5 && h < 10 ? 'linesMorning' : h >= 23 || h < 4 ? 'linesNight' : 'linesIdle'));
        stage.react(pickLine(['happy', 'wink', 'thinking'], Math.random));
      }
      idle();
    }, IDLE_MS[0] + Math.random() * (IDLE_MS[1] - IDLE_MS[0]));
  idle();
}
