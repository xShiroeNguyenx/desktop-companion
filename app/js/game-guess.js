// Cửa sổ Đoán số (guess.html). Luật: window.GamesCore (games-core.js, có test).
// Rust: game_info (ngôn ngữ, tên pet, xưng hô, điểm thân thiết hôm nay) và
// game_result (hết ván → cộng điểm + pet ngoài desktop đổi nét mặt). Cài đặt
// và thành tích chỉ là tiện ích trên máy này nên để trong localStorage.
(function () {
  const { invoke, listen, log } = window.DC;
  const G = window.GamesCore;
  const $ = (id) => document.getElementById(id);

  // {you}/{You} = gọi người chơi, {me}/{Me} = pet tự xưng, {pet} = tên pet.
  const T = {
    vi: {
      title: 'Đoán số', me: 'Em', you: 'Bạn',
      intro: '{Pet} đã chọn một số bí mật từ {min} đến {max}! {You} đoán xem là số mấy?',
      tipStart: 'Hãy đoán số nào? {Me} sẽ đưa gợi ý nhé! Cố lên! ♡',
      tipLow: 'Số đó thấp quá rồi, thử số lớn hơn đi!', tipHigh: 'Số đó cao quá rồi, thử số nhỏ hơn đi!',
      sayLow: 'Hehe~ số của {you} thấp quá rồi! Thử số lớn hơn đi nè!',
      sayHigh: 'Hmmm… số của {you} cao quá rồi! Thử số nhỏ hơn đi!',
      sayWin: 'Yay! {You} đoán đúng rồi! 🎉', sayOut: 'Hehe, {me} thắng rồi nha~ Chơi lại không?',
      invalid: 'Nhập số từ {min} đến {max} nha~',
      low: 'Quá thấp!', high: 'Quá cao!', correct: 'Chính xác!', out: 'Hết lượt rồi~',
      hot: '🔥 Gần lắm rồi!', warm: '🌤 Hơi gần rồi đó!', cold: '❄️ Còn xa lắm~',
      subWin: '{You} đã đoán đúng sau {n} lần! Tuyệt vời! ♡', subOut: 'Số của {me} là {secret} đó~',
      retry: 'Thử lại', again: 'Chơi lại', close: 'Đóng',
      go: '💡 Đoán', clear: '↺ Xoá', hint: '❓ Gợi ý ({n})', hintNone: '❓ Gợi ý',
      hintParity: 'Gợi ý: số này là số {kind}!', even: 'chẵn', odd: 'lẻ',
      hintRange: 'Gợi ý: số nằm trong khoảng {from} – {to}!', hintDigit: 'Gợi ý: chữ số cuối là {d}!',
      noHint: 'Hết gợi ý rồi~ {You} tự đoán nha!', hardHint: 'Chế độ Khó không có gợi ý đâu nha~',
      range: 'Phạm vi số', lives: 'Số lần đoán còn lại', history: 'Lịch sử đoán', hEmpty: 'Chưa đoán lần nào',
      mascot: 'Gợi ý sẽ giúp {you} tìm ra số nhanh hơn đó!',
      help: 'Gõ số bằng bàn phím trên màn hình (hoặc phím số), rồi bấm Đoán (Enter). {Pet} sẽ nói số đó cao quá hay thấp quá. Đoán đúng trước khi hết lượt là thắng!',
      set: 'Cài đặt game', setRange: 'Phạm vi số', setTurns: 'Số lần đoán', warm: 'Hiển thị độ gần', sound: 'Hiệu ứng âm thanh',
      diff: 'Độ khó', diffs: { easy: 'Dễ', normal: 'Bình thường', hard: 'Khó' },
      stats: 'Thành tích', plays: 'Số lần chơi', rate: 'Tỷ lệ thắng', best: 'Chuỗi thắng cao nhất',
    },
    ja: {
      title: '数当て', me: 'わたし', you: 'あなた',
      intro: '{pet}は{min}〜{max}の秘密の数を選んだよ！何だと思う？',
      tipStart: 'どの数かな？ヒントも出すからがんばって！♡',
      tipLow: 'その数は小さすぎ！もっと大きい数で！', tipHigh: 'その数は大きすぎ！もっと小さい数で！',
      sayLow: 'えへへ～小さすぎだよ！もっと大きく！', sayHigh: 'うーん…大きすぎだよ！もっと小さく！',
      sayWin: 'やった！正解だよ！🎉', sayOut: 'えへへ、わたしの勝ち～もう一回やる？',
      invalid: '{min}〜{max}の数を入れてね～',
      low: '小さすぎ！', high: '大きすぎ！', correct: '正解！', out: '残念～',
      hot: '🔥 すごく近い！', warm: '🌤 ちょっと近いよ！', cold: '❄️ まだ遠いよ～',
      subWin: '{n}回で当たり！すごい！♡', subOut: '答えは{secret}だったよ～',
      retry: 'もう一回', again: 'もう一回遊ぶ', close: '閉じる',
      go: '💡 当てる', clear: '↺ 消す', hint: '❓ ヒント（{n}）', hintNone: '❓ ヒント',
      hintParity: 'ヒント：{kind}だよ！', even: '偶数', odd: '奇数',
      hintRange: 'ヒント：{from}〜{to}のどこかだよ！', hintDigit: 'ヒント：一の位は{d}だよ！',
      noHint: 'ヒントはもうないよ～', hardHint: 'むずかしいモードはヒントなしだよ～',
      range: '数の範囲', lives: '残りの回数', history: 'これまでの予想', hEmpty: 'まだ予想してないよ',
      mascot: 'ヒントを使うと早く見つかるかも！',
      help: '画面のキー（または数字キー）で数を入力して「当てる」（Enter）。大きすぎ・小さすぎを教えるよ。回数内に当てたら勝ち！',
      set: 'ゲーム設定', setRange: '数の範囲', setTurns: '予想できる回数', warm: '近さを表示', sound: '効果音',
      diff: '難易度', diffs: { easy: 'やさしい', normal: 'ふつう', hard: 'むずかしい' },
      stats: '成績', plays: 'プレイ回数', rate: '勝率', best: '最高連勝',
    },
    en: {
      title: 'Guess My Number', me: 'I', you: 'You',
      intro: '{Pet} picked a secret number from {min} to {max}! Can you guess it?',
      tipStart: "Which number is it? I'll give you hints! You got this! ♡",
      tipLow: 'Too low — try a bigger number!', tipHigh: 'Too high — try a smaller number!',
      sayLow: 'Hehe~ too low! Go bigger!', sayHigh: 'Hmmm… too high! Go smaller!',
      sayWin: 'Yay! You got it! 🎉', sayOut: 'Hehe, I win this time~ Play again?',
      invalid: 'Type a number from {min} to {max}~',
      low: 'Too low!', high: 'Too high!', correct: 'Correct!', out: 'Out of guesses~',
      hot: '🔥 So close!', warm: '🌤 Getting close!', cold: '❄️ Still far~',
      subWin: 'You got it in {n} tries! Amazing! ♡', subOut: 'My number was {secret}~',
      retry: 'Try again', again: 'Play again', close: 'Close',
      go: '💡 Guess', clear: '↺ Clear', hint: '❓ Hint ({n})', hintNone: '❓ Hint',
      hintParity: "Hint: it's an {kind} number!", even: 'even', odd: 'odd',
      hintRange: "Hint: it's between {from} and {to}!", hintDigit: 'Hint: the last digit is {d}!',
      noHint: 'No hints left~ you can do it!', hardHint: 'No hints on Hard~',
      range: 'Number range', lives: 'Guesses left', history: 'Guess history', hEmpty: 'No guesses yet',
      mascot: 'Hints help you find it faster!',
      help: 'Type a number with the keypad (or your number keys) and press Guess (Enter). I will tell you if it is too high or too low. Find it before you run out of guesses!',
      set: 'Game Settings', setRange: 'Number range', setTurns: 'Guesses', warm: 'Show how close', sound: 'Sound effects',
      diff: 'Difficulty', diffs: { easy: 'Easy', normal: 'Normal', hard: 'Hard' },
      stats: 'Achievements', plays: 'Games played', rate: 'Win rate', best: 'Best streak',
    },
  };
  const RANGES = [10, 20, 50, 100];
  const TURNS = [5, 7, 10];
  const RESULT_MS = 1500;

  let L = T.vi;
  let lang = 'vi';
  let petName = 'Em';
  let youName = 'Bạn';
  let meWord = 'em';
  let opts = loadJson('guessSettings', { max: 20, turns: 5, warm: true, sound: true, difficulty: 'normal' });
  let stats = loadJson('guessStats', { plays: 0, wins: 0, streak: 0, best: 0 });
  let state = null;
  let input = '';
  let hintsUsed = 0;
  let timer = 0;

  function loadJson(key, def) {
    try { return { ...def, ...JSON.parse(localStorage.getItem(key) || '{}') }; } catch (_) { return { ...def }; }
  }
  function saveJson(key, value) {
    try { localStorage.setItem(key, JSON.stringify(value)); } catch (_) { /* ignore */ }
  }

  const cap = (w) => (w ? w.charAt(0).toUpperCase() + w.slice(1) : w);
  const low = (w) => (w ? w.charAt(0).toLowerCase() + w.slice(1) : w);
  function words(extra) {
    return {
      you: low(youName), You: youName, me: meWord, Me: cap(meWord), pet: petName, Pet: petName,
      min: state ? state.min : 1, max: state ? state.max : opts.max, ...(extra || {}),
    };
  }
  const fill = (s, vars) => String(s).replace(/\{(\w+)\}/g, (m, k) => (k in vars ? vars[k] : m));

  function rand() {
    const a = new Uint32Array(1);
    crypto.getRandomValues(a);
    return a[0] / 4294967296;
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
  function tip(text) {
    $('tip').textContent = text;
    pop($('tip'));
  }

  /** Nhân vật: 'start' | 'low' | 'high' | 'win' | 'out' (5 hình nằm sẵn trong #hero). */
  function mood(m) {
    document.querySelectorAll('#hero .face').forEach((f) => {
      const on = f.dataset.mood === m;
      f.classList.remove('swap', 'cheer', 'pout');
      f.classList.toggle('on', on);
      if (on) {
        void f.offsetWidth;
        f.classList.add(m === 'win' ? 'cheer' : m === 'high' ? 'pout' : 'swap');
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
  const maxDigits = () => String(state ? state.max : opts.max).length;

  function drawSlots(shake) {
    const box = $('slots');
    box.textContent = '';
    for (let i = 0; i < maxDigits(); i++) {
      const s = document.createElement('div');
      const ch = input[i];
      s.className = 'slot' + (ch ? ' filled' : '');
      s.textContent = ch || '?';
      box.appendChild(s);
    }
    if (shake) {
      box.classList.remove('shake');
      void box.offsetWidth;
      box.classList.add('shake');
    }
  }

  function drawKeys() {
    const box = $('keys');
    box.textContent = '';
    for (const d of ['1', '2', '3', '4', '5', '6', '7', '8', '9', '0']) {
      const b = document.createElement('button');
      b.type = 'button';
      b.className = 'key';
      b.textContent = d;
      b.addEventListener('click', () => typeDigit(d));
      box.appendChild(b);
    }
  }

  function drawSide() {
    $('rangeText').textContent = `${state.min} – ${state.max}`;
    const lives = $('lives');
    lives.textContent = '';
    const left = state.maxTurns - state.turns;
    for (let i = 0; i < state.maxTurns; i++) {
      const h = document.createElement('span');
      h.textContent = '💗';
      if (i >= left) h.className = 'lost';
      lives.appendChild(h);
    }
    const n = document.createElement('span');
    n.className = 'n';
    n.textContent = `${left} / ${state.maxTurns}`;
    lives.appendChild(n);

    const hist = $('history');
    hist.textContent = '';
    const rows = (state.history || []).slice(-4).reverse();
    if (!rows.length) {
      const e = document.createElement('div');
      e.className = 'h-empty';
      e.textContent = L.hEmpty;
      hist.appendChild(e);
    }
    for (const r of rows) {
      const row = document.createElement('div');
      row.className = 'h-row ' + r.hint;
      const num = document.createElement('span');
      num.className = 'num';
      num.textContent = String(r.n);
      const lab = document.createElement('span');
      lab.className = 'lab';
      // hint 'higher' = số bí mật lớn hơn → người chơi đoán THẤP quá.
      lab.textContent = r.hint === 'higher' ? L.low : r.hint === 'lower' ? L.high : L.correct;
      const arrow = document.createElement('span');
      arrow.textContent = r.hint === 'higher' ? '⬇' : r.hint === 'lower' ? '⬆' : '✓';
      row.append(num, lab, arrow);
      hist.appendChild(row);
    }
    drawHintBtn();
  }

  function drawHintBtn() {
    const total = G.hintsAllowed(opts.difficulty);
    const left = Math.max(0, total - hintsUsed);
    $('hintBtn').textContent = total ? fill(L.hint, { n: left }) : L.hintNone;
  }

  // ─── Chơi ────────────────────────────────────────────────────────────────
  function locked() {
    return !$('result').hidden || !state || state.done;
  }

  function typeDigit(d) {
    if (locked()) return;
    if (input.length >= maxDigits() || (input === '' && d === '0')) {
      drawSlots(true);
      return;
    }
    const next = input + d;
    if (Number(next) > state.max) {
      drawSlots(true);
      return;
    }
    input = next;
    drawSlots(false);
  }

  function clearInput() {
    if (locked()) return;
    input = '';
    drawSlots(false);
  }

  function useHint() {
    if (locked()) return;
    const total = G.hintsAllowed(opts.difficulty);
    if (!total) {
      tip(fill(L.hardHint, words()));
      return;
    }
    if (hintsUsed >= total) {
      tip(fill(L.noHint, words()));
      return;
    }
    const h = G.guessHint(state, hintsUsed);
    hintsUsed += 1;
    const text =
      h.kind === 'parity' ? fill(L.hintParity, { kind: h.even ? L.even : L.odd })
      : h.kind === 'range' ? fill(L.hintRange, { from: h.from, to: h.to })
      : fill(L.hintDigit, { d: h.last });
    tip(text);
    drawHintBtn();
  }

  function showResult(kind, big, verdict, sub, mainLabel, onMain, altLabel, onAlt) {
    const r = $('result');
    r.className = 'result ' + kind;
    $('resBig').textContent = big;
    $('resVerdict').textContent = verdict;
    $('resSub').textContent = sub || '';
    $('resSub').hidden = !sub;
    const main = $('resMain');
    main.textContent = mainLabel;
    main.onclick = onMain;
    const alt = $('resAlt');
    alt.hidden = !altLabel;
    if (altLabel) {
      alt.textContent = altLabel;
      alt.onclick = onAlt;
    }
    r.hidden = false;
    pop(r);
  }

  function hideResult() {
    clearTimeout(timer);
    $('result').hidden = true;
  }

  function submit() {
    if (locked()) return;
    if (!input) {
      drawSlots(true);
      return;
    }
    const n = Number(input);
    const r = G.guess(state, input);
    if (r.hint === 'invalid') {
      tip(fill(L.invalid, words()));
      drawSlots(true);
      return;
    }
    state = r.state;
    input = '';
    drawSlots(false);
    drawSide();
    if (r.hint === 'correct') return finish(true, n);
    if (state.done) return finish(false, n);

    const tooLow = r.hint === 'higher';
    const span = state.max - state.min + 1;
    const warmLine = opts.warm ? L[G.warmth(state.secret, n, span)] : '';
    mood(tooLow ? 'low' : 'high');
    say(fill(tooLow ? L.sayLow : L.sayHigh, words()));
    tip(tooLow ? L.tipLow : L.tipHigh);
    sound('poke.mp3');
    showResult(tooLow ? 'higher' : 'lower', String(n), (tooLow ? '⬇ ' : '⬆ ') + (tooLow ? L.low : L.high), warmLine, L.retry, hideResult);
    timer = setTimeout(hideResult, RESULT_MS);
  }

  async function finish(won, n) {
    stats.plays += 1;
    if (won) {
      stats.wins += 1;
      stats.streak += 1;
      stats.best = Math.max(stats.best, stats.streak);
    } else {
      stats.streak = 0;
    }
    saveJson('guessStats', stats);
    drawStats();

    if (won) {
      mood('win');
      say(fill(L.sayWin, words()));
      sound('headpat.mp3');
      tip(fill(L.subWin, words({ n: state.turns })));
      showResult('correct', String(n), '✓ ' + L.correct, fill(L.subWin, words({ n: state.turns })), L.again, newGame, L.close, hideWindow);
    } else {
      mood('out');
      say(fill(L.sayOut, words()));
      tip(fill(L.subOut, words({ secret: state.secret })));
      showResult('out', String(state.secret), L.out, fill(L.subOut, words({ secret: state.secret })), L.again, newGame, L.close, hideWindow);
    }
    try {
      await invoke('game_result', { won });
    } catch (e) {
      log('[guess] game_result: ' + e);
    }
  }

  function newGame() {
    hideResult();
    state = G.guessNew(rand, { min: 1, max: opts.max, turns: opts.turns });
    input = '';
    hintsUsed = 0;
    drawSlots(false);
    drawSide();
    mood('start');
    say(fill(L.intro, words()));
    tip(fill(L.tipStart, words()));
    $('mascotSay').textContent = fill(L.mascot, words());
  }

  // ─── Cài đặt + thành tích ────────────────────────────────────────────────
  function seg(boxId, values, current, label, onPick) {
    const box = $(boxId);
    box.textContent = '';
    for (const v of values) {
      const b = document.createElement('button');
      b.type = 'button';
      b.textContent = label(v);
      b.classList.toggle('on', v === current);
      b.addEventListener('click', () => onPick(v));
      box.appendChild(b);
    }
  }

  function drawSettings() {
    seg('segRange', RANGES, opts.max, (v) => `1 - ${v}`, (v) => { opts.max = v; saveJson('guessSettings', opts); drawSettings(); newGame(); });
    seg('segTurns', TURNS, opts.turns, String, (v) => { opts.turns = v; saveJson('guessSettings', opts); drawSettings(); newGame(); });
    seg('segDiff', ['easy', 'normal', 'hard'], opts.difficulty, (v) => L.diffs[v], (v) => {
      opts.difficulty = v;
      saveJson('guessSettings', opts);
      drawSettings();
      drawHintBtn();
    });
    $('optWarm').checked = !!opts.warm;
    $('optSound').checked = !!opts.sound;
  }

  function drawStats() {
    $('statPlays').textContent = String(stats.plays);
    $('statRate').textContent = stats.plays ? Math.round((stats.wins * 100) / stats.plays) + '%' : '0%';
    $('statBest').textContent = String(stats.best);
  }

  function applyTexts() {
    document.documentElement.lang = lang;
    $('titleText').textContent = L.title;
    $('goBtn').textContent = L.go;
    $('clearBtn').textContent = L.clear;
    $('lblRange').textContent = L.range;
    $('lblLives').textContent = L.lives;
    $('lblHistory').textContent = L.history;
    $('help').textContent = fill(L.help, words());
    $('setTitle').textContent = L.set;
    $('lblSetRange').textContent = L.setRange;
    $('lblSetTurns').textContent = L.setTurns;
    $('lblWarm').textContent = L.warm;
    $('lblSound').textContent = L.sound;
    $('lblDiff').textContent = L.diff;
    $('lblStats').textContent = L.stats;
    $('lblPlays').textContent = L.plays;
    $('lblRate').textContent = L.rate;
    $('lblBest').textContent = L.best;
  }

  // Đóng hẳn (không chỉ ẩn): cửa sổ được dựng khi mở (windows::LAZY).
  function hideWindow() {
    hideResult();
    const w = window.__TAURI__ && window.__TAURI__.window && window.__TAURI__.window.getCurrentWindow();
    if (w) w.close().catch(() => w.hide().catch(() => {}));
  }

  // ─── Sự kiện ─────────────────────────────────────────────────────────────
  $('closeBtn').addEventListener('click', hideWindow);
  $('goBtn').addEventListener('click', submit);
  $('clearBtn').addEventListener('click', clearInput);
  $('hintBtn').addEventListener('click', useHint);
  $('helpBtn').addEventListener('click', () => { $('help').hidden = !$('help').hidden; });
  $('setBtn').addEventListener('click', () => {
    const s = $('settings');
    s.hidden = !s.hidden;
    $('setBtn').classList.toggle('on', !s.hidden);
  });
  $('optWarm').addEventListener('change', (e) => { opts.warm = e.target.checked; saveJson('guessSettings', opts); });
  $('optSound').addEventListener('change', (e) => { opts.sound = e.target.checked; saveJson('guessSettings', opts); });
  window.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') {
      if (!$('settings').hidden) { $('settings').hidden = true; $('setBtn').classList.remove('on'); }
      else if (!$('help').hidden) $('help').hidden = true;
      else hideWindow();
      return;
    }
    if (/^[0-9]$/.test(e.key)) typeDigit(e.key);
    else if (e.key === 'Backspace') { if (!locked()) { input = input.slice(0, -1); drawSlots(false); } }
    else if (e.key === 'Enter') {
      if (!$('result').hidden) $('resMain').click();
      else submit();
    }
  });

  // Mỗi lần mở cửa sổ (dựng sẵn một lần): lấy ngôn ngữ, tên, xưng hô; ván mới.
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
      log('[guess] game_info: ' + e);
    }
    if (!RANGES.includes(opts.max)) opts.max = 20;
    if (!TURNS.includes(opts.turns)) opts.turns = 5;
    applyTexts();
    drawKeys();
    drawSettings();
    drawStats();
    newGame();
  }

  listen('guess-refresh', open);
  open();
})();
