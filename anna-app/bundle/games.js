// 🎮 Mini games. Rules come from the desktop app's pure modules
// (lib/games-core.js → window.GamesCore, lib/mines-core.js → window.MinesCore).

const el = (tag, cls, text) => {
  const n = document.createElement(tag);
  if (cls) n.className = cls;
  if (text != null) n.textContent = text;
  return n;
};
const fill = (s, vars) => String(s).replace(/\{(\w+)\}/g, (_, k) => (vars[k] != null ? vars[k] : ''));

export function mountGames(root, ctx) {
  const G = window.GamesCore;
  const M = window.MinesCore;
  const picker = el('div', 'seg');
  const body = el('div', 'game');
  root.append(picker, body);
  const games = { rps, guess, mines };
  let current = 'rps';

  function renderPicker() {
    picker.textContent = '';
    for (const id of Object.keys(games)) {
      const b = el('button', id === current ? 'active' : '', ctx.t('game' + id[0].toUpperCase() + id.slice(1)));
      b.addEventListener('click', () => {
        current = id;
        render();
      });
      picker.appendChild(b);
    }
  }

  function render() {
    renderPicker();
    body.textContent = '';
    games[current]();
  }

  // ✊ rock paper scissors — first to 3, pet plays "normal" (random).
  function rps() {
    let state = G.rpsNew();
    const history = [];
    const score = el('p', 'score');
    const status = el('p', 'status', '');
    const row = el('div', 'row center');
    const again = el('button', 'ghost', ctx.t('newGame'));
    const draw = () => (score.textContent = fill(ctx.t('rpsScore'), state));
    for (const move of G.MOVES) {
      const b = el('button', 'primary', ctx.t(move));
      b.addEventListener('click', () => {
        if (state.over) return;
        const pet = G.rpsPetMove(history, 'normal', Math.random);
        history.push(move);
        const result = G.rpsRound(move, pet);
        state = G.rpsScore(state, result);
        status.textContent = `${ctx.t(move)} vs ${ctx.t(pet)} — ${ctx.t(result === 'win' ? 'rpsWin' : result === 'lose' ? 'rpsLose' : 'rpsDraw')}`;
        ctx.react(result === 'win' ? 'sad' : result === 'lose' ? 'happy' : 'surprised');
        if (state.over) {
          const won = state.winner === 'user';
          ctx.say(ctx.t(won ? 'rpsGameWin' : 'rpsGameLose'));
          if (won) ctx.earn('rpsWin');
        }
        draw();
      });
      row.appendChild(b);
    }
    again.addEventListener('click', () => render());
    draw();
    body.append(score, row, status, again);
  }

  // 🔢 guess the number 1..100 in 7 turns.
  function guess() {
    let state = G.guessNew(Math.random);
    const intro = el('p', '', ctx.t('guessPrompt'));
    const form = el('form', 'row');
    const input = el('input');
    input.type = 'number';
    input.min = String(G.GUESS_MIN);
    input.max = String(G.GUESS_MAX);
    const go = el('button', 'primary', ctx.t('guessGo'));
    go.type = 'submit';
    form.append(input, go);
    const status = el('p', 'status');
    const left = el('p', 'hint');
    const again = el('button', 'ghost', ctx.t('newGame'));
    const drawLeft = () => (left.textContent = `${ctx.t('guessLeft')}${state.maxTurns - state.turns} · ${state.low}–${state.high}`);
    form.addEventListener('submit', (e) => {
      e.preventDefault();
      if (state.done) return;
      const r = G.guess(state, input.value);
      state = r.state;
      input.value = '';
      if (r.hint === 'invalid') status.textContent = ctx.t('guessInvalid');
      else if (r.hint === 'correct') {
        status.textContent = fill(ctx.t('guessWin'), { n: state.secret });
        ctx.react('happy');
        ctx.earn('guessWin');
      } else if (state.done) {
        status.textContent = fill(ctx.t('guessLose'), { n: state.secret });
        ctx.react('wink');
      } else {
        status.textContent = ctx.t(r.hint === 'higher' ? 'guessHigher' : 'guessLower');
        const w = G.warmth(state.secret, Number(state.history.at(-1).n));
        ctx.react(w === 'hot' ? 'surprised' : w === 'warm' ? 'happy' : 'thinking');
      }
      drawLeft();
    });
    again.addEventListener('click', () => render());
    drawLeft();
    body.append(intro, form, status, left, again);
    input.focus();
  }

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
    grid.style.gridTemplateColumns = `repeat(${state.cols}, 1fr)`;
    const status = el('p', 'status');
    const again = el('button', 'ghost', ctx.t('newGame'));
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
    body.append(levels, info, grid, status, again);
  }

  render();
  return { refresh: render };
}
