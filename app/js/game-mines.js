// Cửa sổ Gỡ mìn (mines.html). Luật: window.MinesCore (mines-core.js, có test).
// Rust: game_info (ngôn ngữ, tên pet, xưng hô) và game_result (hết ván → cộng
// điểm thân thiết + pet ngoài desktop đổi nét mặt). Cài đặt và kỷ lục chỉ là
// tiện ích trên máy này nên để trong localStorage.
(function () {
  const { invoke, listen, log } = window.DC;
  const M = window.MinesCore;
  const $ = (id) => document.getElementById(id);

  // {you}/{You} = gọi người chơi, {me}/{Me} = pet tự xưng, {pet} = tên pet.
  const T = {
    vi: {
      title: 'Gỡ bom', sub: 'Giúp {me} gỡ hết bom nhé!', me: 'Em', you: 'Bạn',
      choose: 'Chọn độ khó', levels: { easy: 'Dễ', normal: 'Thường', hard: 'Khó' }, bombs: 'bom',
      goalTitle: 'Mục tiêu', goal: 'Mở tất cả ô an toàn và đánh dấu đúng vị trí bom!',
      ctrlTitle: 'Điều khiển', kLeft: 'Chuột trái:', aOpen: 'Mở ô', kRight: 'Chuột phải:', aFlag: 'Cắm cờ',
      kNum: 'Bấm ô số:', aChord: 'Mở nhanh các ô quanh',
      cheer: 'Cùng cố gắng nào!',
      sayIdle: 'Hình như ở đây có bom đó~ Hãy suy nghĩ thật kỹ nhé!',
      sayThink: 'Hmm… Có vẻ an toàn ở đây nè?', sayFlag: 'Đánh dấu chỗ này là bom ha~',
      saySafe: 'Tuyệt! Không có bom ở đây!', sayBig: 'Oa, mở được cả một vùng luôn!',
      sayBoom: 'Á! Quả bom đây rồi!', sayWin: 'Hoàn thành rồi! {You} thật giỏi!',
      tipsTitle: 'Gợi ý từ {pet}',
      tips: [
        'Con số cho biết có bao nhiêu quả bom trong 8 ô xung quanh đó!',
        '{You} có thể đánh dấu những ô nghi ngờ bằng cờ nhé!',
        'Ô đầu tiên luôn an toàn, cứ mạnh dạn mở thôi~',
        'Số 1 cạnh đúng một ô chưa mở? Ô đó chắc chắn là bom!',
        'Cắm đủ cờ quanh một ô số rồi bấm vào nó để mở nhanh nha!',
      ],
      hint: '🔍 Chỉ ô an toàn ({n})', hintNone: '🔍 Hết lượt gợi ý', hintFirst: 'Mở một ô bất kỳ trước đã nha~',
      hintSay: '{Me} thấy ô sáng xanh kia an toàn đó!',
      won: 'Hoàn thành!', lost: 'Bùm!', subWon: '{You} gỡ xong trong {time}! {record}', record: 'Kỷ lục mới!',
      subLost: 'Không sao đâu, thử lại với {me} nha~', again: 'Chơi lại', close: 'Đóng',
      help: [
        'Mở hết các ô không có bom là thắng.',
        'Mỗi con số cho biết số bom nằm trong 8 ô xung quanh nó. Chuột phải để cắm cờ lên ô nghi là bom; bấm vào ô số đã đủ cờ để mở nhanh các ô còn lại quanh nó.',
        'Phím tắt: R = ván mới, Esc = đóng.',
      ],
      set: 'Cài đặt game', sound: 'Hiệu ứng âm thanh', chordOpt: 'Bấm ô số để mở nhanh', talkOpt: '{Pet} nói chuyện khi chơi',
      stats: 'Thành tích', plays: 'Số lần chơi', rate: 'Tỷ lệ thắng', best: 'Nhanh nhất',
    },
    ja: {
      title: 'ボム処理班', sub: 'ぜんぶの爆弾を見つけてね！', me: 'わたし', you: 'あなた',
      choose: '難易度', levels: { easy: 'かんたん', normal: 'ふつう', hard: 'むずかしい' }, bombs: '個',
      goalTitle: '目標', goal: '安全なマスをすべて開けて、爆弾の場所に旗を立てよう！',
      ctrlTitle: '操作', kLeft: '左クリック：', aOpen: '開ける', kRight: '右クリック：', aFlag: '旗',
      kNum: '数字をクリック：', aChord: 'まわりをまとめて開ける',
      cheer: 'いっしょにがんばろう！',
      sayIdle: 'ここに爆弾があるかも～よく考えてね！',
      sayThink: 'うーん…ここは安全かな？', sayFlag: 'ここに旗を立てておくね～',
      saySafe: 'やった！ここは爆弾なし！', sayBig: 'わあ、一気に開いたね！',
      sayBoom: 'きゃっ！爆弾だ！', sayWin: 'クリア！すごいね！',
      tipsTitle: '{pet}のヒント',
      tips: [
        '数字はまわり8マスにある爆弾の数だよ！',
        'あやしいマスには旗を立てておこうね！',
        '最初のマスは必ず安全だよ～',
        '「1」のとなりに閉じたマスが1つだけ？それは爆弾だね！',
        '旗がそろった数字をクリックすると、まとめて開けられるよ！',
      ],
      hint: '🔍 安全なマス（{n}）', hintNone: '🔍 ヒントはもうないよ', hintFirst: 'まずはどこか1マス開けてね～',
      hintSay: '青く光ったマスは安全だよ！',
      won: 'クリア！', lost: 'ドカン！', subWon: '{time}でクリア！{record}', record: '新記録！',
      subLost: 'だいじょうぶ、もう一回やろう～', again: 'もう一回', close: '閉じる',
      help: [
        '爆弾のないマスをすべて開けたら勝ち。',
        '数字はまわり8マスの爆弾の数。右クリックで旗を立て、旗がそろった数字をクリックすると残りのマスをまとめて開けられるよ。',
        'ショートカット：R = 新しいゲーム、Esc = 閉じる。',
      ],
      set: 'ゲーム設定', sound: '効果音', chordOpt: '数字クリックでまとめて開ける', talkOpt: '{pet}がおしゃべりする',
      stats: '成績', plays: 'プレイ回数', rate: '勝率', best: '最速',
    },
    en: {
      title: 'Bomb Squad', sub: 'Help me clear every bomb!', me: 'I', you: 'You',
      choose: 'Difficulty', levels: { easy: 'Easy', normal: 'Normal', hard: 'Hard' }, bombs: 'bombs',
      goalTitle: 'Goal', goal: 'Open every safe tile and flag where the bombs are!',
      ctrlTitle: 'Controls', kLeft: 'Left click:', aOpen: 'Open', kRight: 'Right click:', aFlag: 'Flag',
      kNum: 'Click a number:', aChord: 'Open around it',
      cheer: "Let's do our best!",
      sayIdle: 'There might be a bomb here~ Think carefully!',
      sayThink: 'Hmm… is it safe here?', sayFlag: "I'll mark that one as a bomb~",
      saySafe: 'Yay! No bomb here!', sayBig: 'Whoa, a whole area opened up!',
      sayBoom: 'Eek! A bomb!', sayWin: 'All clear! You are amazing!',
      tipsTitle: "{Pet}'s tips",
      tips: [
        'A number tells how many bombs are in the 8 tiles around it!',
        'You can mark suspicious tiles with a flag!',
        'The first tile is always safe, go ahead~',
        'A 1 next to just one closed tile? That one is a bomb!',
        'Flag every bomb around a number, then click it to open the rest!',
      ],
      hint: '🔍 Show a safe tile ({n})', hintNone: '🔍 No hints left', hintFirst: 'Open any tile first~',
      hintSay: 'The glowing blue tile is safe!',
      won: 'All clear!', lost: 'Boom!', subWon: 'You cleared it in {time}! {record}', record: 'New record!',
      subLost: "It's okay, let's try again~", again: 'Play again', close: 'Close',
      help: [
        'Open every tile without a bomb to win.',
        'Each number shows how many bombs touch it. Right-click to flag a tile you think is a bomb; click a number that has all its flags to open the rest around it.',
        'Shortcuts: R = new game, Esc = close.',
      ],
      set: 'Game Settings', sound: 'Sound effects', chordOpt: 'Click numbers to open around', talkOpt: '{Pet} chats while you play',
      stats: 'Achievements', plays: 'Games played', rate: 'Win rate', best: 'Fastest',
    },
  };
  const LEVEL_KEYS = ['easy', 'normal', 'hard'];
  const LEVEL_CAT = { easy: '😺', normal: '😸', hard: '😼' };
  const HINTS = { easy: 3, normal: 2, hard: 1 };
  // Chỉ mở ≥ chừng này ô một lần mới khen "mở cả vùng".
  const BIG_OPEN = 8;
  // Nổ / thắng xong chờ một nhịp để người chơi nhìn bàn rồi mới hiện thẻ.
  const RESULT_DELAY_MS = 900;

  let L = T.vi;
  let lang = 'vi';
  let petName = 'Em';
  let youName = 'Bạn';
  let meWord = 'em';
  let opts = loadJson('minesSettings', { level: 'easy', sound: true, chord: true, talk: true });
  let stats = loadJson('minesStats', { plays: 0, wins: 0, best: {} });
  let state = null;
  let cellEls = [];
  let hintsLeft = 0;
  let started = 0;
  let elapsed = 0;
  let ticker = 0;
  let resultTimer = 0;
  let tipIndex = 0;

  function loadJson(key, def) {
    try { return { ...def, ...JSON.parse(localStorage.getItem(key) || '{}') }; } catch (_) { return { ...def }; }
  }
  function saveJson(key, value) {
    try { localStorage.setItem(key, JSON.stringify(value)); } catch (_) { /* ignore */ }
  }

  // Ván đang chơi dở: đóng cửa sổ là huỷ hẳn (trả RAM), nên lưu lại sau mỗi
  // nước đi để mở lại chơi tiếp được. Ván xong / chưa bắt đầu thì xoá.
  const GAME_KEY = 'minesGame';
  function saveGame() {
    try {
      if (!state || !state.placed || state.done) { localStorage.removeItem(GAME_KEY); return; }
      const sec = ticker ? Math.floor((Date.now() - started) / 1000) : elapsed;
      localStorage.setItem(GAME_KEY, JSON.stringify({ level: opts.level, state, elapsed: sec, hints: hintsLeft }));
    } catch (_) { /* ignore */ }
  }
  function loadGame() {
    try { return M.resumable(JSON.parse(localStorage.getItem(GAME_KEY) || 'null'), opts.level); } catch (_) { return null; }
  }

  const cap = (w) => (w ? w.charAt(0).toUpperCase() + w.slice(1) : w);
  const low = (w) => (w ? w.charAt(0).toLowerCase() + w.slice(1) : w);
  function words(extra) {
    return { you: low(youName), You: youName, me: meWord, Me: cap(meWord), pet: petName, Pet: petName, ...(extra || {}) };
  }
  const fill = (s, vars) => String(s).replace(/\{(\w+)\}/g, (m, k) => (k in vars ? vars[k] : m));

  function rand() {
    const a = new Uint32Array(1);
    crypto.getRandomValues(a);
    return a[0] / 4294967296;
  }

  function fmtTime(sec) {
    const s = Math.min(sec, 99 * 60 + 59);
    return String(Math.floor(s / 60)).padStart(2, '0') + ':' + String(s % 60).padStart(2, '0');
  }

  function pop(el) {
    el.classList.remove('pop');
    void el.offsetWidth;
    el.classList.add('pop');
  }
  // `force`: câu quan trọng (nổ / thắng) vẫn nói kể cả khi tắt "nói chuyện".
  function say(text, force) {
    if (!opts.talk && !force) return;
    $('bubble').textContent = text;
    pop($('bubble'));
  }

  /** Nhân vật: 'idle' | 'think' | 'safe' | 'boom' | 'win'. */
  function mood(m) {
    document.querySelectorAll('#hero .face').forEach((f) => {
      const on = f.dataset.mood === m;
      if (on && f.classList.contains('on')) return;
      f.classList.remove('swap', 'cheer', 'shock');
      f.classList.toggle('on', on);
      if (on) {
        void f.offsetWidth;
        f.classList.add(m === 'win' ? 'cheer' : m === 'boom' ? 'shock' : 'swap');
      }
    });
  }

  function sound(name) {
    if (!opts.sound) return;
    let voice = 'ja';
    try { voice = localStorage.getItem('voiceLanguage') || 'ja'; } catch (_) { /* ignore */ }
    try {
      const a = new Audio(`media/audio/${voice}/${name}`);
      a.volume = 0.6;
      a.play().catch(() => {});
    } catch (_) { /* ignore */ }
  }

  // ─── Đồng hồ: chạy từ lần mở ô đầu tiên tới khi hết ván; ẩn cửa sổ thì dừng
  // tạm, mở lại thì chạy tiếp từ `fromSec`.
  function startClock(fromSec) {
    stopClock();
    started = Date.now() - (fromSec || 0) * 1000;
    ticker = setInterval(() => {
      elapsed = Math.floor((Date.now() - started) / 1000);
      $('timeText').textContent = fmtTime(elapsed);
    }, 250);
  }
  function stopClock() {
    if (ticker) clearInterval(ticker);
    ticker = 0;
    if (started) elapsed = Math.floor((Date.now() - started) / 1000);
  }

  // ─── Vẽ ──────────────────────────────────────────────────────────────────
  function drawCounters() {
    $('leftText').textContent = String(M.minesLeft(state));
    $('flagText').textContent = String(state.flags);
    $('mineText').textContent = String(state.mines);
  }

  function drawCell(i, fresh) {
    const c = state.cells[i];
    const el = cellEls[i];
    el.className = 'cell';
    el.textContent = '';
    if (c.open) {
      el.classList.add('open');
      if (c.mine) el.classList.add('mine');
      else if (c.adj) { el.classList.add('n' + c.adj); el.textContent = String(c.adj); }
      if (fresh) el.classList.add('fresh');
    } else if (c.flag) {
      el.classList.add('flag');
      // Hết ván do nổ: cờ cắm sai chỗ thì gạch đi.
      if (state.done && !state.won && !c.mine) el.classList.add('wrong');
    }
    if (i === state.boom) el.classList.add('boom');
  }

  function drawBoard() {
    const board = $('board');
    board.textContent = '';
    const n = state.cols;
    // Ô càng nhỏ thì khe, bo góc và cỡ chữ càng nhỏ theo.
    board.style.gridTemplateColumns = `repeat(${n}, 1fr)`;
    board.style.gridTemplateRows = `repeat(${state.rows}, 1fr)`;
    board.style.setProperty('--gap', n > 12 ? '3px' : n > 8 ? '4px' : '6px');
    board.style.setProperty('--r', n > 12 ? '6px' : n > 8 ? '8px' : '12px');
    board.classList.toggle('dense', n > 12);
    board.style.setProperty('--fs', n > 12 ? '19px' : n > 8 ? '25px' : '34px');
    cellEls = state.cells.map((_, i) => {
      const el = document.createElement('div');
      el.dataset.i = String(i);
      board.appendChild(el);
      return el;
    });
    state.cells.forEach((_, i) => drawCell(i, false));
  }

  function drawLevels() {
    const box = $('levels');
    box.textContent = '';
    for (const key of LEVEL_KEYS) {
      const cfg = M.LEVELS[key];
      const b = document.createElement('button');
      b.type = 'button';
      b.className = 'level' + (opts.level === key ? ' on' : '');
      const cat = document.createElement('span');
      cat.className = 'cat';
      cat.textContent = LEVEL_CAT[key];
      const txt = document.createElement('span');
      const name = document.createElement('b');
      name.textContent = L.levels[key];
      const meta = document.createElement('small');
      meta.textContent = `${cfg.rows} × ${cfg.cols} · ${cfg.mines} ${L.bombs}`;
      txt.append(name, meta);
      b.append(cat, txt);
      b.addEventListener('click', () => {
        opts.level = key;
        saveJson('minesSettings', opts);
        drawLevels();
        newGame();
      });
      box.appendChild(b);
    }
  }

  function drawHintBtn() {
    const b = $('hintBtn');
    b.textContent = hintsLeft > 0 ? fill(L.hint, { n: hintsLeft }) : L.hintNone;
    b.disabled = hintsLeft <= 0 || state.done;
  }

  function nextTip() {
    const list = L.tips;
    $('tipText').textContent = fill(list[tipIndex % list.length], words());
    tipIndex += 1;
  }

  function drawStats() {
    $('statPlays').textContent = String(stats.plays);
    $('statRate').textContent = stats.plays ? Math.round((stats.wins * 100) / stats.plays) + '%' : '0%';
    for (const key of LEVEL_KEYS) {
      const id = 'stat' + cap(key);
      const best = stats.best && stats.best[key];
      $(id).textContent = best != null ? fmtTime(best) : '—';
      $('best' + cap(key)).textContent = `${L.best} · ${L.levels[key]}`;
    }
  }

  function applyTexts() {
    document.documentElement.lang = lang;
    document.title = L.title;
    $('titleText').textContent = L.title;
    $('subText').textContent = fill(L.sub, words());
    document.querySelectorAll('[data-t]').forEach((el) => {
      const v = L[el.dataset.t];
      if (typeof v === 'string') el.textContent = fill(v, words());
    });
    $('tipsTitle').textContent = fill(L.tipsTitle, words());
    const help = $('help');
    help.textContent = '';
    for (const line of L.help) {
      const p = document.createElement('p');
      p.textContent = fill(line, words());
      help.appendChild(p);
    }
    $('optSound').checked = !!opts.sound;
    $('optChord').checked = !!opts.chord;
    $('optTalk').checked = !!opts.talk;
    $('soundBtn').textContent = opts.sound ? '🔊' : '🔇';
  }

  // ─── Luồng chơi ──────────────────────────────────────────────────────────
  function newGame() {
    clearTimeout(resultTimer);
    stopClock();
    started = 0;
    elapsed = 0;
    $('timeText').textContent = fmtTime(0);
    $('result').hidden = true;
    state = M.minesNew(opts.level);
    hintsLeft = HINTS[opts.level] || 1;
    drawBoard();
    drawCounters();
    drawHintBtn();
    mood('idle');
    say(L.sayIdle, true);
    nextTip();
    saveGame();
  }

  // Kết quả của một lần mở (mở thường hoặc mở nhanh).
  function afterOpen(r) {
    if (r.result === 'none') return;
    if (!started) startClock();
    for (const i of r.opened) drawCell(i, true);
    if (r.result === 'boom') {
      // Vẽ lại để gạch các cờ cắm sai.
      state.cells.forEach((c, i) => { if (c.flag) drawCell(i, false); });
      return finish(false);
    }
    if (r.result === 'win') {
      state.cells.forEach((c, i) => { if (c.flag) drawCell(i, false); });
      drawCounters();
      return finish(true);
    }
    mood('safe');
    say(r.opened.length >= BIG_OPEN ? L.sayBig : L.saySafe);
    if (r.opened.length >= BIG_OPEN) sound('poke.mp3');
  }

  function openCell(i) {
    if (!state || state.done) return;
    const c = state.cells[i];
    if (c.open) {
      if (opts.chord) afterOpen(M.chord(state, i, rand));
      return;
    }
    afterOpen(M.reveal(state, i, rand));
    drawHintBtn();
  }

  function openAndSave(i) {
    openCell(i);
    saveGame();
  }

  function flagCell(i) {
    if (!state || state.done) return;
    if (!M.toggleFlag(state, i)) return;
    drawCell(i, false);
    drawCounters();
    saveGame();
    if (state.cells[i].flag) {
      mood('think');
      say(L.sayFlag);
    }
  }

  function useHint() {
    if (!state || state.done || hintsLeft <= 0) return;
    const i = M.safeHint(state);
    if (i < 0) {
      say(fill(L.hintFirst, words()), true);
      return;
    }
    hintsLeft -= 1;
    saveGame();
    drawHintBtn();
    cellEls[i].classList.add('hint');
    mood('think');
    say(fill(L.hintSay, words()), true);
  }

  async function finish(won) {
    stopClock();
    drawHintBtn();
    stats.plays += 1;
    let record = false;
    if (won) {
      stats.wins += 1;
      stats.best = stats.best || {};
      const prev = stats.best[opts.level];
      if (prev == null || elapsed < prev) {
        stats.best[opts.level] = elapsed;
        record = true;
      }
    }
    saveJson('minesStats', stats);
    drawStats();

    if (won) {
      mood('win');
      say(fill(L.sayWin, words()), true);
      sound('headpat.mp3');
    } else {
      mood('boom');
      say(fill(L.sayBoom, words()), true);
      sound('poke.mp3');
    }
    resultTimer = setTimeout(() => {
      const box = $('result');
      box.className = 'result ' + (won ? 'won' : 'lost');
      $('resIcon').textContent = won ? '🎉' : '💥';
      $('resVerdict').textContent = won ? L.won : L.lost;
      $('resSub').textContent = won
        ? fill(L.subWon, words({ time: fmtTime(elapsed), record: record ? L.record : '' })).trim()
        : fill(L.subLost, words());
      $('resMain').textContent = L.again;
      $('resAlt').textContent = L.close;
      box.hidden = false;
    }, RESULT_DELAY_MS);
    try {
      await invoke('game_result', { won });
    } catch (e) {
      log('[mines] game_result: ' + e);
    }
  }

  // Đóng hẳn (Rust huỷ cửa sổ để trả RAM, windows::close_panel); ván dở đã lưu.
  function hideWindow() {
    if (ticker) stopClock();
    saveGame();
    const w = window.__TAURI__ && window.__TAURI__.window && window.__TAURI__.window.getCurrentWindow();
    if (w) w.close().catch(() => w.hide().catch(() => {}));
  }
  // Đóng bằng Alt+F4 / thanh tác vụ không qua hideWindow: lưu lúc cửa sổ bị ẩn.
  document.addEventListener('visibilitychange', () => {
    if (document.hidden) saveGame();
  });

  // ─── Sự kiện ─────────────────────────────────────────────────────────────
  const board = $('board');
  const cellIndex = (e) => {
    const el = e.target.closest('.cell');
    return el ? Number(el.dataset.i) : -1;
  };
  board.addEventListener('click', (e) => {
    const i = cellIndex(e);
    if (i >= 0) openAndSave(i);
  });
  board.addEventListener('contextmenu', (e) => {
    e.preventDefault();
    const i = cellIndex(e);
    if (i >= 0) flagCell(i);
  });
  // Nhấn giữ chuột trái trên ô chưa mở: pet hồi hộp "ở đây an toàn không?".
  board.addEventListener('pointerdown', (e) => {
    const i = cellIndex(e);
    if (e.button !== 0 || i < 0 || !state || state.done || state.cells[i].open || state.cells[i].flag) return;
    cellEls[i].classList.add('press');
    mood('think');
    say(L.sayThink);
  });
  const unpress = () => board.querySelectorAll('.press').forEach((el) => el.classList.remove('press'));
  board.addEventListener('pointerup', unpress);
  board.addEventListener('pointerleave', unpress);

  $('closeBtn').addEventListener('click', hideWindow);
  $('hintBtn').addEventListener('click', useHint);
  $('resMain').addEventListener('click', newGame);
  $('resAlt').addEventListener('click', hideWindow);
  $('helpBtn').addEventListener('click', () => {
    $('help').hidden = !$('help').hidden;
    $('helpBtn').classList.toggle('on', !$('help').hidden);
  });
  $('setBtn').addEventListener('click', () => {
    const s = $('settings');
    s.hidden = !s.hidden;
    $('setBtn').classList.toggle('on', !s.hidden);
  });
  $('soundBtn').addEventListener('click', () => {
    opts.sound = !opts.sound;
    saveJson('minesSettings', opts);
    applyTexts();
  });
  for (const [id, key] of [['optSound', 'sound'], ['optChord', 'chord'], ['optTalk', 'talk']]) {
    $(id).addEventListener('change', (e) => {
      opts[key] = e.target.checked;
      saveJson('minesSettings', opts);
      applyTexts();
    });
  }
  // Mỗi lần ván chạy được một lúc thì đổi mẹo trong khung gợi ý.
  setInterval(() => { if (state && !state.done) nextTip(); }, 15000);

  window.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') {
      if (!$('settings').hidden) { $('settings').hidden = true; $('setBtn').classList.remove('on'); }
      else if (!$('help').hidden) { $('help').hidden = true; $('helpBtn').classList.remove('on'); }
      else hideWindow();
    } else if (e.key === 'r' || e.key === 'R' || e.key === 'F2') {
      newGame();
    } else if (e.key === 'Enter' && !$('result').hidden) {
      newGame();
    }
  });

  // Mỗi lần mở cửa sổ (dựng mới, hoặc hiện lại khi chưa kịp huỷ): lấy ngôn ngữ,
  // tên, xưng hô. Ván đang chơi dở (trong trang hoặc đã lưu) thì chơi tiếp; ván
  // đã xong (hoặc chưa có) thì bắt đầu ván mới.
  async function open() {
    try {
      const info = await invoke('game_info');
      lang = T[info.lang] ? info.lang : 'vi';
      L = T[lang];
      petName = (info.pet && String(info.pet).trim()) || L.me;
      const you = String(info.you || '').trim();
      youName = lang === 'vi' && you ? cap(you) : L.you;
      const me = String(info.me || '').trim();
      meWord = lang === 'vi' && me ? me : lang === 'vi' ? 'em' : low(L.me);
    } catch (e) {
      log('[mines] game_info: ' + e);
    }
    if (!M.LEVELS[opts.level]) opts.level = 'easy';
    applyTexts();
    drawLevels();
    drawStats();
    // Cửa sổ vừa dựng lại: lấy ván dở đã lưu lúc đóng lần trước.
    if (!state) {
      const saved = loadGame();
      if (saved) {
        state = saved.state;
        hintsLeft = saved.hints;
        elapsed = saved.elapsed;
        started = 1; // ván đã bắt đầu: đồng hồ chạy tiếp từ `elapsed` bên dưới
        $('result').hidden = true;
        drawBoard();
        drawCounters();
        mood('idle');
        say(L.sayIdle, true);
      }
    }
    if (!state || state.done || !state.placed) newGame();
    else {
      if (started && !ticker) startClock(elapsed);
      drawHintBtn();
      nextTip();
    }
  }

  listen('mines-refresh', open);
  open();
})();
