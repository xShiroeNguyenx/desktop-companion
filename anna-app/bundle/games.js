// 🎮 Mini games: the desktop Rock paper scissors / Guess the number pages
// (bundle/desk/, see desk-shim.js) and Bomb Squad, whose rules come from lib/mines-core.js.

const el = (tag, cls, text) => {
  const n = document.createElement(tag);
  if (cls) n.className = cls;
  if (text != null) n.textContent = text;
  return n;
};

export function mountGames(root, ctx) {
  const M = window.MinesCore;
  // Two screens: a list of games, then the chosen game taking the whole window
  // (html.game-full hides the character and tabs) with only a back button.
  const picker = el('div', 'game-list');
  const head = el('div', 'game-head');
  const back = el('button', 'ghost game-back');
  const title = el('span', 'game-title');
  head.append(back, title);
  const body = el('div', 'game');
  root.append(picker, head, body);
  const games = { mines };
  const ids = ['rps', 'guess', 'mines'];
  const label = (id) => ctx.t('game' + id[0].toUpperCase() + id.slice(1));
  let current = null;

  back.addEventListener('click', () => leave());
  window.addEventListener('keydown', (e) => {
    if (e.key === 'Escape' && current && !e.defaultPrevented) leave();
  });

  function renderPicker() {
    picker.textContent = '';
    for (const id of ids) {
      const b = el('button', 'game-pick');
      b.append(el('b', '', label(id)), el('small', '', ctx.t('game' + id[0].toUpperCase() + id.slice(1) + 'Desc')));
      b.addEventListener('click', () => {
        if (DESK[id]) return ctx.playDesk(DESK[id], id);
        current = id;
        render();
      });
      picker.appendChild(b);
    }
  }

  function render() {
    const playing = !!current;
    document.documentElement.classList.toggle('game-full', playing);
    picker.hidden = playing;
    head.hidden = !playing;
    body.hidden = !playing;
    body.textContent = '';
    if (!playing) {
      if (fitObserver) fitObserver.disconnect();
      renderPicker();
      return;
    }
    back.textContent = '← ' + ctx.t('menuBack');
    title.textContent = label(current);
    games[current]();
  }

  /** Back to the list of games (also when another tab is opened). */
  function leave() {
    if (!current) return;
    current = null;
    render();
  }

  // ✊ Rock paper scissors and 🔢 Guess the number: the desktop app's own pages
  // (bundle/desk/, copied by scripts/sync-assets.mjs) with Miu's faces and sounds.
  // Anna blocks frames, so the window switches to that page (app.js playDesk)
  // and desk-shim.js brings it back here.
  const DESK = { rps: 'game.html', guess: 'guess.html' };

  // 💣 minesweeper.
  let level = 'easy';
  function mines() {
    const state = M.minesNew(level);
    const levels = el('div', 'seg small');
    for (const id of Object.keys(M.LEVELS)) {
      const b = el('button', id === level ? 'active' : '', ctx.t('mines' + id[0].toUpperCase() + id.slice(1)));
      b.addEventListener('click', () => {
        level = id;
        render();
      });
      levels.appendChild(b);
    }
    const info = el('p', 'hint');
    const grid = el('div', 'mines');
    grid.style.gridTemplateColumns = `repeat(${state.cols}, minmax(0, 1fr))`;
    grid.style.gridTemplateRows = `repeat(${state.rows}, minmax(0, 1fr))`;
    const status = el('p', 'status');
    // "New game" sits on the level row so the board gets the height.
    const again = el('button', 'again', '↻');
    again.title = ctx.t('newGame');
    levels.appendChild(again);
    const cells = state.cells.map((_, i) => {
      const b = el('button', 'cell');
      b.addEventListener('click', () => open(i));
      b.addEventListener('contextmenu', (e) => {
        e.preventDefault();
        flag(i);
      });
      let timer;
      b.addEventListener('touchstart', () => (timer = setTimeout(() => flag(i), 450)), { passive: true });
      b.addEventListener('touchend', () => clearTimeout(timer));
      grid.appendChild(b);
      return b;
    });
    const draw = () => {
      state.cells.forEach((c, i) => {
        const b = cells[i];
        b.className = 'cell' + (c.open ? ' open' : '') + (c.open && c.adj ? ` n${c.adj}` : '');
        b.textContent = c.open ? (c.mine ? '💣' : c.adj || '') : c.flag ? '🚩' : state.done && c.mine ? '💣' : '';
      });
      info.textContent = `${ctx.t('minesLeft')}${M.minesLeft(state)} · ${ctx.t('minesHelp')}`;
    };
    const finish = () => {
      if (!state.done) return;
      status.textContent = ctx.t(state.won ? 'minesWin' : 'minesLose');
      ctx.react(state.won ? 'happy' : 'surprised');
      if (state.won) ctx.earn('minesWin');
    };
    function open(i) {
      const c = state.cells[i];
      if (c && c.open) M.chord(state, i, Math.random);
      else M.reveal(state, i, Math.random);
      draw();
      finish();
    }
    function flag(i) {
      M.toggleFlag(state, i);
      draw();
    }
    again.addEventListener('click', () => render());
    draw();
    body.append(levels, info, grid, status);
    fitBoard(grid, status, state.cols);
  }

  // Square board as big as the panel allows WITHOUT scrolling: the smaller of
  // the panel width and the height left under the level row. Cells never go
  // below MIN_CELL px (then the panel scrolls, as before).
  const MIN_CELL = 14;
  let fitObserver = null;
  function fitBoard(grid, status, cols) {
    if (fitObserver) fitObserver.disconnect();
    const panel = grid.closest('.panel') || root;
    const fit = () => {
      if (!grid.isConnected) return fitObserver && fitObserver.disconnect();
      const box = panel.getBoundingClientRect();
      const pad = parseFloat(getComputedStyle(panel).paddingBottom) || 0;
      const room = box.bottom - pad - grid.getBoundingClientRect().top - status.offsetHeight - 8;
      const size = Math.max(cols * MIN_CELL, Math.floor(Math.min(body.clientWidth, room)));
      grid.style.width = grid.style.height = size + 'px';
      grid.style.setProperty('--fs', Math.max(9, Math.round((size / cols) * 0.6)) + 'px');
      grid.classList.toggle('dense', size / cols < 16);
    };
    fitObserver = new ResizeObserver(fit);
    fitObserver.observe(panel);
    fit();
  }

  render();
  return { refresh: render, leave };
}
