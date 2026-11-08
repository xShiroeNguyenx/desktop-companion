// Cửa sổ Ghép Miu (merge.html). Luật: window.MergeCore (merge-core.js, có test).
// Rust: game_info (ngôn ngữ, tên pet, xưng hô, bộ mặt Miu) và game_result (hết
// ván → cộng điểm thân thiết + nhiệm vụ "chơi game"). Ván đang chơi, cài đặt và
// thành tích là tiện ích trên máy này nên để trong localStorage.
(function () {
  const { invoke, listen, log } = window.DC;
  const M = window.MergeCore;
  const $ = (id) => document.getElementById(id);
  // Khoá có số 4 (thang thứ 4, 13 bậc): ván/thành tích của các thang thử nghiệm trước đọc sai bậc nên bỏ.
  const GAME_KEY = 'mergeGame4';

  // {you}/{You} = gọi người chơi, {me}/{Me} = pet tự xưng, {pet} = tên pet.
  const T = {
    vi: {
      title: 'Ghép Miu', me: 'Em', you: 'Bạn',
      score: 'Điểm', best: 'Kỷ lục', ladder: 'Thang tiến hoá', series: 'Dòng model', sound: 'Âm thanh',
      newGame: '↺ Ván mới',
      series_gothic: 'Gothic', series_snow: 'Snow', series_pink: 'Pink',
      models: { lolita: 'Lolita', kawaii: 'Kawaii', pastel: 'Pastel', aristocrat: 'Aristocrat', angel: 'Angel' },
      mius: { gothic: 'Miu tím', snow: 'Miu tuyết', pink: 'Miu hồng' },
      miuFaces: { sleepy: '{miu} ngái ngủ', cheer: '{miu} cổ vũ' },
      angels: { gothic: 'Angel tím', snow: 'Angel tuyết', pink: 'Angel hồng' },
      next: 'Kế tiếp: {name}', nextDone: 'Đã ghép ra Angel hồng! Chơi tiếp lấy điểm nha~',
      intro: 'Ghép hai Miu giống nhau để tiến hoá! Tới được Angel không nè?',
      sayMerge: 'Yay, {name} nè! ♡', sayModel: 'Woa! {You} ghép ra {name} rồi!',
      sayTense: 'Bàn sắp đầy rồi, cẩn thận nha!', sayAgain: 'Ván mới! Cố lên {you} ơi~',
      unlock: 'MỞ KHOÁ', win: 'Angel xuất hiện! {You} thắng rồi!', winSub: '{You} đã ghép ra {name} sau {n} lượt! Giỏi quá ♡ Chơi tiếp để lên Angel tuyết, Angel hồng và kiếm thêm điểm nha~',
      over: 'Hết nước đi rồi~', overSub: 'Xa nhất: {name} · {score} điểm',
      keep: 'Chơi tiếp', again: 'Ván mới', close: 'Đóng',
      help: [
        'Dùng phím mũi tên (hoặc W A S D), hay vuốt chuột trên bàn để dồn mọi ô về một phía.',
        'Hai ô giống nhau chạm nhau sẽ ghép thành hình kế tiếp. Mỗi màu Miu có 2 bậc (ngái ngủ → cổ vũ): Miu tím → Miu tuyết → Miu hồng → Lolita → Kawaii → Pastel → Aristocrat → Angel tím → Angel tuyết → Angel hồng.',
        'Ghép ra Angel tím (bậc 11, như ô 2048) là thắng! Sau đó vẫn chơi tiếp được tới Angel tuyết, Angel hồng để kiếm thêm điểm. Bàn đầy mà không còn cặp nào ghép được là hết ván.',
        'Mỗi ván xong được +1 điểm thân thiết (tối đa theo ngày, như các game khác). Ván dở được giữ lại khi đóng cửa sổ.',
      ],
    },
    ja: {
      title: 'ミウ合体', me: 'わたし', you: 'あなた',
      score: 'スコア', best: 'ベスト', ladder: '進化の段階', series: 'モデルの系統', sound: 'サウンド',
      newGame: '↺ 新しいゲーム',
      series_gothic: 'ゴシック', series_snow: 'スノー', series_pink: 'ピンク',
      models: { lolita: 'ロリータ', kawaii: 'カワイイ', pastel: 'パステル', aristocrat: 'アリストクラット', angel: 'エンジェル' },
      mius: { gothic: 'ゴシックのミウ', snow: '雪のミウ', pink: 'ピンクのミウ' },
      miuFaces: { sleepy: 'ねむい{miu}', cheer: '応援する{miu}' },
      angels: { gothic: 'ゴシックのエンジェル', snow: '雪のエンジェル', pink: 'ピンクのエンジェル' },
      next: '次：{name}', nextDone: 'ピンクのエンジェル完成！続けてスコアを伸ばそう～',
      intro: '同じミウを合わせて進化させよう！エンジェルまで行けるかな？',
      sayMerge: 'やった、{name}だよ！♡', sayModel: 'わあ！{name}ができた！',
      sayTense: 'もうすぐいっぱい、気をつけて！', sayAgain: '新しいゲーム！がんばって～',
      unlock: 'アンロック', win: 'エンジェル登場！あなたの勝ち！', winSub: '{n}手で{name}を作ったよ！すごい♡ 続けて雪・ピンクのエンジェルを目指してスコアを伸ばそう～',
      over: 'もう動かせないよ～', overSub: '最高：{name} · {score}点',
      keep: '続ける', again: 'もう一回', close: '閉じる',
      help: [
        '矢印キー（または W A S D）か、盤の上でドラッグして全部のタイルを一方向に寄せるよ。',
        '同じタイルがぶつかると次の姿に。ミウは色ごとに2段階（ねむい → 応援）：ゴシックのミウ → 雪のミウ → ピンクのミウ → ロリータ → カワイイ → パステル → アリストクラット → ゴシック・雪・ピンクのエンジェル。',
        'ゴシックのエンジェル（11段階目、2048と同じ）を作れば勝ち！そのあとも雪・ピンクのエンジェルを目指して続けられるよ。盤がいっぱいで合わせられなくなったら終わり。',
        '1ゲームごとになかよし度+1（1日の上限あり）。途中のゲームは閉じても残るよ。',
      ],
    },
    en: {
      title: 'Miu Merge', me: 'I', you: 'You',
      score: 'Score', best: 'Best', ladder: 'Evolution', series: 'Model line', sound: 'Sound',
      newGame: '↺ New game',
      series_gothic: 'Gothic', series_snow: 'Snow', series_pink: 'Pink',
      models: { lolita: 'Lolita', kawaii: 'Kawaii', pastel: 'Pastel', aristocrat: 'Aristocrat', angel: 'Angel' },
      mius: { gothic: 'Gothic Miu', snow: 'Snow Miu', pink: 'Pink Miu' },
      miuFaces: { sleepy: 'Sleepy {miu}', cheer: 'Cheering {miu}' },
      angels: { gothic: 'Gothic Angel', snow: 'Snow Angel', pink: 'Pink Angel' },
      next: 'Next: {name}', nextDone: 'Pink Angel reached! Keep going for points~',
      intro: 'Merge two matching Miu to evolve them! Can you reach Angel?',
      sayMerge: 'Yay, {name}! ♡', sayModel: 'Wow! You made {name}!',
      sayTense: 'The board is almost full, careful!', sayAgain: 'New game! You can do it~',
      unlock: 'UNLOCKED', win: 'Angel appears! You win!', winSub: 'You made {name} in {n} moves! Amazing ♡ Keep going for Snow Angel and Pink Angel and more points~',
      over: 'No moves left~', overSub: 'Furthest: {name} · {score} points',
      keep: 'Keep going', again: 'New game', close: 'Close',
      help: [
        'Use the arrow keys (or W A S D), or drag on the board, to slide every tile to one side.',
        'Two matching tiles that touch merge into the next one. Each Miu colour has 2 steps (sleepy → cheering): Gothic Miu → Snow Miu → Pink Miu → Lolita → Kawaii → Pastel → Aristocrat → Gothic Angel → Snow Angel → Pink Angel.',
        'Making Gothic Angel (step 11, like the 2048 tile) wins! You can then keep going for Snow Angel and Pink Angel to earn more points. A full board with nothing left to merge ends the game.',
        'Every finished game gives +1 friendship point (daily cap, like the other games). An unfinished game is kept when you close the window.',
      ],
    },
  };
  const UNLOCK_MS = 1600;

  let L = T.vi;
  let lang = 'vi';
  let petName = 'Em';
  let youName = 'Bạn';
  let meWord = 'em';
  let miuDir = 'media/game';
  let opts = loadJson('mergeSettings', { series: '', sound: true });
  let stats = loadJson('mergeStats4', { plays: 0, wins: 0, bestScore: 0, bestLevel: 0 });
  let state = null;
  // Ván này đã báo game_result chưa (thắng báo một lần, chơi tiếp rồi thua không báo lại).
  let reported = false;
  let unlockTimer = 0;

  function loadJson(key, def) {
    try { return { ...def, ...JSON.parse(localStorage.getItem(key) || '{}') }; } catch (_) { return { ...def }; }
  }
  function saveJson(key, value) {
    try { localStorage.setItem(key, JSON.stringify(value)); } catch (_) { /* ignore */ }
  }
  function saveGame() {
    try {
      if (!state || state.over) localStorage.removeItem(GAME_KEY);
      else localStorage.setItem(GAME_KEY, JSON.stringify({ ...state, reported }));
    } catch (_) { /* ignore */ }
  }
  function loadGame() {
    try {
      const saved = JSON.parse(localStorage.getItem(GAME_KEY) || 'null');
      const s = M.resumable(saved);
      return s ? { state: s, reported: !!saved.reported } : null;
    } catch (_) { return null; }
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

  const series = () => (M.SERIES.includes(opts.series) ? opts.series : M.seriesForMiu(miuDir));
  const src = (level) => M.tileSrc(level, series());
  function tierName(level) {
    const t = M.tier(level);
    if (!t) return '';
    if (t.kind === 'miu') return fill(L.miuFaces[t.face], { miu: L.mius[t.name] });
    if (t.kind === 'angel') return L.angels[t.color];
    return `${L['series_' + series()]} ${L.models[t.name]}`;
  }

  function pop(el) {
    el.classList.remove('pop');
    void el.offsetWidth;
    el.classList.add('pop');
  }
  function say(text) {
    $('bubble').textContent = text;
    pop($('bubble'));
  }

  /** Nhân vật: 'start' | 'happy' | 'tense' | 'out'. */
  function mood(m) {
    document.querySelectorAll('#hero .face').forEach((f) => {
      const on = f.dataset.mood === m;
      f.classList.remove('swap', 'cheer');
      f.classList.toggle('on', on);
      if (on) {
        void f.offsetWidth;
        f.classList.add(m === 'happy' ? 'cheer' : 'swap');
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

  // ─── Vẽ ──────────────────────────────────────────────────────────────────
  function img(level) {
    const el = document.createElement('img');
    el.src = src(level);
    el.alt = tierName(level);
    el.draggable = false;
    // Ô Miu cố định màu theo bậc: miu-skin.js đừng đổi theo bộ đang chọn.
    el.setAttribute('data-miu-fixed', '');
    return el;
  }

  function drawBoard(merged, spawned) {
    const box = $('board');
    box.textContent = '';
    state.board.forEach((v, i) => {
      const cell = document.createElement('div');
      cell.className = 'cell';
      if (v) {
        const t = M.tier(v);
        cell.classList.add('t');
        if (t.kind !== 'miu') cell.classList.add('model');
        if (t.kind === 'angel') cell.classList.add('angel');
        if (merged && merged.includes(i)) cell.classList.add('merged');
        if (i === spawned) cell.classList.add('spawned');
        cell.title = tierName(v);
        cell.appendChild(img(v));
        const lv = document.createElement('span');
        lv.className = 'lv';
        lv.textContent = String(v);
        cell.appendChild(lv);
      }
      box.appendChild(cell);
    });
  }

  function drawSide() {
    $('score').textContent = String(state.score);
    $('bestScore').textContent = String(Math.max(stats.bestScore, state.score));
    const ladder = $('ladder');
    ladder.textContent = '';
    for (let l = 1; l <= M.MAX_LEVEL; l++) {
      const r = document.createElement('div');
      r.className = 'rung';
      if (M.tier(l).kind !== 'miu') r.classList.add('model');
      if (M.tier(l).kind === 'angel') r.classList.add('angel');
      if (l <= state.best) r.classList.add('got');
      r.title = `${l}. ${tierName(l)}`;
      r.appendChild(img(l));
      ladder.appendChild(r);
    }
    $('nextText').textContent = state.best >= M.MAX_LEVEL
      ? L.nextDone
      : fill(L.next, { name: tierName(state.best + 1) });
  }

  function drawSeries() {
    const box = $('segSeries');
    box.textContent = '';
    for (const s of M.SERIES) {
      const b = document.createElement('button');
      b.type = 'button';
      b.textContent = L['series_' + s];
      b.classList.toggle('on', s === series());
      b.addEventListener('click', () => {
        opts.series = s;
        saveJson('mergeSettings', opts);
        drawSeries();
        drawBoard();
        drawSide();
        $('board').focus();
      });
      box.appendChild(b);
    }
  }

  function draw() {
    drawBoard();
    drawSide();
    drawSeries();
  }

  function showUnlock(level) {
    clearTimeout(unlockTimer);
    $('unlockImg').src = src(level);
    $('unlockLbl').textContent = '🔓 ' + L.unlock;
    $('unlockName').textContent = tierName(level);
    $('unlock').hidden = false;
    pop($('unlock'));
    unlockTimer = setTimeout(() => { $('unlock').hidden = true; }, UNLOCK_MS);
  }

  function showResult(kind, level, verdict, sub, main, onMain, alt, onAlt) {
    const box = $('result');
    box.className = 'result ' + kind;
    $('resImg').src = src(level);
    $('resVerdict').textContent = verdict;
    $('resSub').textContent = sub;
    $('resMain').textContent = main;
    $('resMain').onclick = onMain;
    $('resAlt').textContent = alt;
    $('resAlt').onclick = onAlt;
    box.hidden = false;
    pop(box);
  }
  function hideResult() {
    $('result').hidden = true;
  }

  // ─── Luồng chơi ──────────────────────────────────────────────────────────
  async function report(won) {
    if (reported) return;
    reported = true;
    saveGame();
    try {
      await invoke('game_result', { won, game: 'merge' });
    } catch (e) {
      log('[merge] game_result: ' + e);
    }
  }

  function recordEnd(won) {
    if (!reported) stats.plays += 1;
    if (won) stats.wins += 1;
    stats.bestScore = Math.max(stats.bestScore, state.score);
    stats.bestLevel = Math.max(stats.bestLevel, state.best);
    saveJson('mergeStats4', stats);
  }

  function step(dir) {
    if (!state || !$('result').hidden) return;
    const wasWon = state.won;
    const r = M.move(state, dir, rand);
    if (!r.moved) return;
    state = r.state;
    drawBoard(r.merged, r.spawned);
    drawSide();
    saveGame();

    const top = r.reached.length ? r.reached[r.reached.length - 1] : 0;
    if (state.won && !wasWon) {
      recordEnd(true);
      mood('happy');
      sound('headpat.mp3');
      say(fill(L.sayModel, words({ name: tierName(M.WIN_LEVEL) })));
      showResult('win', M.WIN_LEVEL, '👼 ' + L.win, fill(L.winSub, words({ name: tierName(M.WIN_LEVEL), n: state.moves })),
        L.keep, () => { hideResult(); $('board').focus(); }, L.again, newGame);
      report(true);
      return;
    }
    if (state.over) {
      // Thắng rồi chơi tiếp tới hết: đã tính ván thắng, chỉ cập nhật kỷ lục.
      if (wasWon) { stats.bestScore = Math.max(stats.bestScore, state.score); saveJson('mergeStats4', stats); }
      else recordEnd(false);
      mood('out');
      say(fill(L.over, words()));
      showResult('out', state.best, L.over, fill(L.overSub, words({ name: tierName(state.best), score: state.score })),
        L.again, newGame, L.close, hideWindow);
      report(false);
      return;
    }
    if (top >= M.FIRST_MODEL) {
      showUnlock(top);
      mood('happy');
      sound('headpat.mp3');
      say(fill(L.sayModel, words({ name: tierName(top) })));
    } else if (top) {
      mood('happy');
      say(fill(L.sayMerge, words({ name: tierName(top) })));
    } else if (M.emptyCells(state.board).length <= 2) {
      mood('tense');
      say(fill(L.sayTense, words()));
    } else if (r.merged.length) {
      sound('poke.mp3');
    }
  }

  function newGame() {
    hideResult();
    $('unlock').hidden = true;
    state = M.newGame(rand);
    reported = false;
    saveGame();
    draw();
    mood('start');
    say(fill(L.sayAgain, words()));
    $('board').focus();
  }

  function applyTexts() {
    document.documentElement.lang = lang;
    document.title = L.title;
    $('titleText').textContent = L.title;
    $('lblScore').textContent = L.score;
    $('lblBest').textContent = L.best;
    $('lblLadder').textContent = L.ladder;
    $('lblSeries').textContent = L.series;
    $('lblSound').textContent = L.sound;
    $('newBtn').textContent = L.newGame;
    const help = $('help');
    help.textContent = '';
    for (const line of L.help) {
      const p = document.createElement('p');
      p.textContent = fill(line, words());
      help.appendChild(p);
    }
    $('optSound').checked = !!opts.sound;
  }

  // Đóng hẳn (không chỉ ẩn): cửa sổ được dựng khi mở (windows::LAZY). Ván dở đã lưu.
  function hideWindow() {
    saveGame();
    const w = window.__TAURI__ && window.__TAURI__.window && window.__TAURI__.window.getCurrentWindow();
    if (w) w.close().catch(() => w.hide().catch(() => {}));
  }

  // ─── Sự kiện ─────────────────────────────────────────────────────────────
  $('closeBtn').addEventListener('click', hideWindow);
  $('newBtn').addEventListener('click', newGame);
  $('helpBtn').addEventListener('click', () => {
    $('help').hidden = !$('help').hidden;
    $('helpBtn').classList.toggle('on', !$('help').hidden);
  });
  $('optSound').addEventListener('change', (e) => { opts.sound = e.target.checked; saveJson('mergeSettings', opts); });
  window.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') {
      if (!$('help').hidden) { $('help').hidden = true; $('helpBtn').classList.remove('on'); }
      else hideWindow();
      return;
    }
    if (e.key === 'Enter' && !$('result').hidden) { $('resMain').click(); return; }
    const dir = M.keyDir(e.key);
    if (dir) {
      e.preventDefault();
      step(dir);
    }
  });
  // Vuốt bằng chuột / cảm ứng trên bàn.
  let start = null;
  $('board').addEventListener('pointerdown', (e) => { start = { x: e.clientX, y: e.clientY }; });
  window.addEventListener('pointerup', (e) => {
    if (!start) return;
    const dir = M.swipeDir(e.clientX - start.x, e.clientY - start.y);
    start = null;
    if (dir) step(dir);
  });

  function applyInfo(info) {
    if (!info) return;
    if (info.miuDir) miuDir = info.miuDir;
  }

  // Mỗi lần mở cửa sổ: lấy ngôn ngữ, tên, xưng hô, bộ mặt Miu; chơi tiếp ván dở.
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
      applyInfo(info);
    } catch (e) {
      log('[merge] game_info: ' + e);
    }
    applyTexts();
    hideResult();
    const saved = loadGame();
    if (saved) {
      state = saved.state;
      reported = saved.reported;
      draw();
      mood('start');
      say(fill(L.intro, words()));
      $('board').focus();
    } else {
      newGame();
      say(fill(L.intro, words()));
    }
  }

  listen('merge-refresh', open);
  // Đổi bộ mặt Miu trong Bộ sưu tập → vẽ lại ô bằng bộ mới.
  listen('collection-changed', (e) => {
    applyInfo(e && e.payload);
    if (state) draw();
  });
  open();
})();
