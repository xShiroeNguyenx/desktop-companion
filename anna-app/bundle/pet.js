// Floating pet mode (view "pet", a transparent window): only the character,
// her bubble and a 💬 button that opens the full panel (view "main"). Runs in
// index.html — app.js hands over here when the host says this window is "pet".
import { award, bondLevel, isUnlocked, newProgress, pickLine, uiLang } from './core.js';
import { STRINGS } from './i18n.js';
import { load, save } from './host.js';
import { Stage } from './stage.js';

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

function say(text) {
  $('bubbleText').textContent = text;
  $('bubble').hidden = !text;
  clearTimeout(bubbleTimer);
  // The window is small and see-through: let the bubble go away on its own.
  if (text) bubbleTimer = setTimeout(() => ($('bubble').hidden = true), 8000);
}

function line(kind) {
  lastLine = pickLine(t(kind), Math.random, lastLine);
  return lastLine;
}

function renderStatus() {
  $('status').textContent = `🍬 ${progress.candy} · 💗 ${bondLevel(progress.bond).level}`;
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
  lang = uiLang((anna && anna.locale) || navigator.language);
  document.documentElement.lang = lang;
  if (anna && typeof anna.onThemeChange === 'function') {
    anna.onThemeChange(({ theme }) => (document.documentElement.dataset.theme = theme));
  }
  if (anna && typeof anna.onLocaleChange === 'function') anna.onLocaleChange(({ lang: l }) => (lang = uiLang(l)));

  const [settings, saved, models] = await Promise.all([
    load('settings', {}),
    load('progress', null),
    fetch('./models/index.json').then((r) => r.json()),
  ]);
  progress = { ...newProgress(), ...(saved || {}) };
  const entry =
    models.find((m) => m.id === settings.model && isUnlocked(progress, m.id)) ||
    models.find((m) => isUnlocked(progress, m.id)) ||
    models[0];

  const stage = new Stage($('stage'), $('live2d'), {
    framing: 'full',
    onPat: () => {
      say(line('linesPat'));
      earn(stage, 'pat');
    },
    onPoke: () => say(line('linesPoke')),
  });

  const open = document.createElement('button');
  open.id = 'openBtn';
  open.type = 'button';
  open.className = 'open';
  open.textContent = '💬';
  open.title = t('openPanel');
  open.addEventListener('click', async () => {
    try {
      if (!anna) throw new Error('no host');
      await anna.window.open_view({ view: 'main' });
    } catch (e) {
      // Hosts without window.open_view: turn this window into the full panel instead.
      console.warn('[companion] open_view unavailable, switching in place', e);
      if (anna) await anna.window.resize({ w: 440, h: 720 }).catch(() => {});
      const url = new URL(location.href);
      url.searchParams.set('view', 'main');
      location.replace(url);
    }
  });
  $('stage').querySelector('.stage-bar').appendChild(open);
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
