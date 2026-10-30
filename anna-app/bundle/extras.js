// 🗒️ Notes, 🍅 Focus, 📅 Calendar and 🍬 Collection panels.
// Notes search and the lunar calendar reuse the desktop app's pure modules
// (lib/notes-core.js → window.NotesCore, lib/calendar-core.js → window.CalendarCore).
import { bondLevel, dayOf, fmtClock, focusLeft, focusStart, isUnlocked, modelCost, unlockModel } from './core.js';

const el = (tag, cls, text) => {
  const n = document.createElement(tag);
  if (cls) n.className = cls;
  if (text != null) n.textContent = text;
  return n;
};
const fill = (s, vars) => String(s).replace(/\{(\w+)\}/g, (_, k) => (vars[k] != null ? vars[k] : ''));
const uid = () => Date.now().toString(36) + Math.random().toString(36).slice(2, 6);

// ---------------------------------------------------------------- notes

export async function mountNotes(root, ctx) {
  let notes = await ctx.load('notes', []);
  if (!Array.isArray(notes)) notes = [];
  let query = '';
  let openId = null;
  let saveTimer;

  const top = el('div', 'row');
  const search = el('input');
  search.type = 'text';
  search.placeholder = ctx.t('notesSearch');
  const add = el('button', 'primary', ctx.t('notesNew'));
  top.append(search, add);
  const list = el('ul', 'notes');
  const editor = el('div', 'editor');
  editor.hidden = true;
  const title = el('input');
  title.type = 'text';
  title.placeholder = ctx.t('noteTitle');
  const body = el('textarea');
  body.rows = 6;
  body.placeholder = ctx.t('noteBody');
  const bar = el('div', 'row');
  const back = el('button', 'ghost', '←');
  const pin = el('button', 'ghost', '📌 ' + ctx.t('notePin'));
  const del = el('button', 'ghost danger', ctx.t('noteDelete'));
  bar.append(back, pin, del);
  editor.append(bar, title, body);
  root.append(top, list, editor);

  const persist = () => {
    clearTimeout(saveTimer);
    saveTimer = setTimeout(() => ctx.save('notes', notes), 500);
  };
  const current = () => notes.find((n) => n.id === openId);

  function renderList() {
    list.textContent = '';
    const shown = window.NotesCore.visibleNotes(notes, query);
    if (!shown.length) list.appendChild(el('li', 'empty', ctx.t('noNotes')));
    for (const n of shown) {
      const li = el('li');
      li.append(el('b', '', (n.pinned ? '📌 ' : '') + (n.title || ctx.t('untitled'))), el('span', '', (n.body || '').slice(0, 80)));
      li.addEventListener('click', () => openNote(n.id));
      list.appendChild(li);
    }
  }
  function openNote(id) {
    openId = id;
    const n = current();
    title.value = n.title || '';
    body.value = n.body || '';
    editor.hidden = false;
    list.hidden = top.hidden = true;
    body.focus();
  }
  function closeNote() {
    const n = current();
    if (n && !n.title && !n.body) notes = notes.filter((x) => x.id !== n.id);
    openId = null;
    editor.hidden = true;
    list.hidden = top.hidden = false;
    persist();
    renderList();
  }
  const edit = () => {
    const n = current();
    if (!n) return;
    n.title = title.value;
    n.body = body.value;
    n.updated_at = Date.now();
    persist();
  };
  title.addEventListener('input', edit);
  body.addEventListener('input', edit);
  search.addEventListener('input', () => {
    query = search.value;
    renderList();
  });
  add.addEventListener('click', () => {
    const now = Date.now();
    notes.push({ id: uid(), title: '', body: '', pinned: false, created_at: now, updated_at: now });
    openNote(notes.at(-1).id);
  });
  back.addEventListener('click', closeNote);
  pin.addEventListener('click', () => {
    const n = current();
    if (n) n.pinned = !n.pinned;
    persist();
  });
  del.addEventListener('click', () => {
    notes = notes.filter((n) => n.id !== openId);
    openId = null;
    closeNote();
  });
  renderList();
}

// ---------------------------------------------------------------- focus

export async function mountFocus(root, ctx) {
  let state = await ctx.load('focus', { phase: 'work', running: false, endsAt: 0, day: '', sessions: 0 });
  const label = el('p', 'focus-label');
  const clock = el('div', 'clock');
  const row = el('div', 'row center');
  const start = el('button', 'primary', ctx.t('focusStart'));
  const rest = el('button', 'ghost', ctx.t('focusBreakBtn'));
  const stop = el('button', 'ghost', ctx.t('focusStop'));
  row.append(start, rest, stop);
  const today = el('p', 'hint');
  root.append(label, clock, row, today);

  const begin = (phase) => {
    ctx.unlockAudio();
    state = { ...state, ...focusStart(phase) };
    ctx.save('focus', state);
    ctx.react(phase === 'work' ? 'happy' : 'wink');
    draw();
  };
  start.addEventListener('click', () => begin('work'));
  rest.addEventListener('click', () => begin('break'));
  stop.addEventListener('click', () => {
    state = { ...state, running: false };
    ctx.save('focus', state);
    draw();
  });

  function draw() {
    if (state.day !== dayOf()) state = { ...state, day: dayOf(), sessions: 0 };
    const left = focusLeft(state);
    label.textContent = state.running ? ctx.t(state.phase === 'work' ? 'focusWork' : 'focusBreak') : ctx.t('focusIdle');
    clock.textContent = fmtClock(state.running ? left : 25 * 60_000);
    clock.classList.toggle('break', state.running && state.phase === 'break');
    start.hidden = rest.hidden = state.running;
    stop.hidden = !state.running;
    today.textContent = ctx.t('focusToday') + (state.sessions || 0) + ' 🍅'.repeat(Math.min(8, state.sessions || 0));
    ctx.setBadge(state.running ? fmtClock(left) : '');
  }

  setInterval(() => {
    if (state.running && focusLeft(state) === 0) {
      const finishedWork = state.phase === 'work';
      state = { ...state, running: false, sessions: (state.sessions || 0) + (finishedWork ? 1 : 0) };
      ctx.save('focus', state);
      ctx.chime();
      ctx.say(ctx.t(finishedWork ? 'focusDone' : 'focusBack'), true);
      ctx.react(finishedWork ? 'happy' : 'wink');
      if (finishedWork) ctx.earn('focus');
    }
    draw();
  }, 1000);
  draw();
}

// ---------------------------------------------------------------- calendar

export function mountCalendar(root, ctx) {
  const C = window.CalendarCore;
  const now = new Date();
  let year = now.getFullYear();
  let month = now.getMonth() + 1;
  const head = el('div', 'row');
  const prev = el('button', 'ghost', '‹');
  const title = el('b', 'cal-title');
  const next = el('button', 'ghost', '›');
  const todayBtn = el('button', 'ghost', ctx.t('calToday'));
  head.append(prev, title, next, todayBtn);
  const grid = el('div', 'cal');
  const info = el('div', 'cal-info');
  root.append(head, grid, info);

  const shift = (d) => {
    ({ year, month } = C.addMonths(year, month, d));
    draw();
  };
  prev.addEventListener('click', () => shift(-1));
  next.addEventListener('click', () => shift(1));
  todayBtn.addEventListener('click', () => {
    year = now.getFullYear();
    month = now.getMonth() + 1;
    draw();
  });

  function describe(c) {
    const lang = ctx.lang();
    const S = C.strings(lang);
    const lunar = C.toLunar(c.day, c.month, c.year);
    const names = C.holidays(c.year, c.month, c.day, lunar).map((h) => S.h[h.id]).filter(Boolean);
    const lunarText = C.fill(S.lunarFull, { d: lunar.day, m: lunar.month, leap: lunar.leap ? S.leap : '', cc: C.canChi(lunar.year, lang) });
    info.textContent = '';
    info.append(el('b', '', `${S.weekdaysLong[(new Date(c.year, c.month - 1, c.day).getDay() + 6) % 7]}, ${c.day}/${c.month}/${c.year}`), el('p', '', lunarText));
    for (const n of names) info.appendChild(el('p', 'holiday', '🎉 ' + n));
  }

  function draw() {
    const lang = ctx.lang();
    const S = C.strings(lang);
    title.textContent = C.monthTitle(year, month, lang);
    grid.textContent = '';
    for (const w of S.weekdays) grid.appendChild(el('span', 'wd', w));
    const todayKey = C.keyOfDate(now);
    for (const c of C.monthGrid(year, month)) {
      const lunar = C.toLunar(c.day, c.month, c.year);
      const hol = C.holidays(c.year, c.month, c.day, lunar);
      const b = el('button', 'day');
      if (!c.inMonth) b.classList.add('out');
      if (c.weekday === 6 || hol.some((h) => h.off)) b.classList.add('off');
      if (hol.length) b.classList.add('hol');
      if (C.dayKey(c.year, c.month, c.day) === todayKey) b.classList.add('today');
      b.append(el('span', 'd', String(c.day)), el('span', 'l', C.lunarLabel(lunar, lang)));
      b.addEventListener('click', () => describe(c));
      grid.appendChild(b);
    }
    describe({ year: now.getFullYear(), month: now.getMonth() + 1, day: now.getDate() });
  }
  draw();
  return { refresh: draw };
}

// ---------------------------------------------------------------- collection

export function mountCollection(root, ctx) {
  const head = el('div', 'stats');
  const grid = el('div', 'models');
  const hint = el('p', 'hint', ctx.t('earnHint'));
  root.append(head, grid, hint);

  function draw() {
    const p = ctx.progress();
    const b = bondLevel(p.bond);
    head.textContent = '';
    const bar = el('div', 'bar');
    const fillBar = el('span');
    fillBar.style.width = b.need ? `${Math.round((b.into / b.need) * 100)}%` : '100%';
    bar.appendChild(fillBar);
    head.append(
      el('div', 'stat', `🍬 ${p.candy} ${ctx.t('candy')}`),
      el('div', 'stat', `💗 ${fill(ctx.t('bondLevel'), { lv: b.level })}`),
      bar,
      el('div', 'hint', fill(ctx.t('streak'), { n: p.streak || 0 })),
    );
    grid.textContent = '';
    ctx.models.forEach((m, i) => {
      const card = el('div', 'card');
      const img = el('img');
      img.src = m.preview;
      img.alt = m.name;
      img.loading = 'lazy';
      const owned = isUnlocked(p, m.id);
      const using = ctx.settings().model === m.id;
      const cost = modelCost(m.id, i);
      const btn = el('button', using ? 'ghost' : 'primary', using ? ctx.t('using') : owned ? ctx.t('use') : fill(ctx.t('unlock'), { cost }));
      btn.disabled = using;
      if (!owned) card.classList.add('locked');
      btn.addEventListener('click', async () => {
        if (!owned) {
          const r = unlockModel(ctx.progress(), m.id, cost);
          if (r.error) {
            ctx.say(ctx.t(r.error === 'candy' ? 'needCandy' : 'owned'));
            ctx.react('sad');
            return;
          }
          await ctx.setProgress(r.progress);
          ctx.say(fill(ctx.t('unlocked'), { name: m.name }));
        }
        await ctx.useModel(m.id);
        draw();
      });
      card.append(img, el('b', '', m.name), btn);
      grid.appendChild(card);
    });
  }
  draw();
  return { refresh: draw };
}
