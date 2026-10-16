// Cửa sổ Oẳn tù tì (game.html). Luật: window.GamesCore (games-core.js, có test).
// Rust: game_info (ngôn ngữ, tên pet, điểm thân thiết hôm nay từ game) và
// game_result (hết ván → cộng điểm + pet đổi nét mặt). Cài đặt game chỉ là
// tiện ích trên máy này nên để trong localStorage.
(function () {
  const { invoke, listen, log } = window.DC;
  const G = window.GamesCore;
  const $ = (id) => document.getElementById(id);

  const T = {
    vi: {
      title: 'Oẳn tù tì', logo: 'Oẳn<br>Tù Tì', ribbon: 'Mình chơi cùng nhau nha! ♡', score: 'Tỉ số', you: 'Bạn', me: 'Em',
      firstTo: 'Ai thắng {n} trận trước', moves: { rock: 'Búa', paper: 'Bao', scissors: 'Kéo' },
      idle: 'Chơi một ván không nè?', turn: 'Tới lượt {you}! Chọn đi~', think: 'Hmm… để xem nào!',
      win: 'Hừm… lần này {me} thua rồi, {you} giỏi ghê >.<', lose: 'Hehe, {pet} thắng nè! 🎉', draw: 'Hoà rồi! Chơi tiếp nha',
      bWin: '{You} thắng rồi!', bLose: '{You} thua rồi…', bDraw: 'Hoà!',
      finalWin: '{You} thắng cả ván! 🎉', finalLose: '{pet} thắng ván này~ 💕', overSay: 'Chơi thêm ván nữa không?',
      again: 'Chơi lại', quit: 'Nghỉ thôi',
      help: 'Chọn Búa, Bao hoặc Kéo (phím Q · W · E). Búa thắng Kéo, Kéo thắng Bao, Bao thắng Búa. Ai thắng đủ số trận trước thì thắng ván.',
      set: 'Cài đặt game', target: 'Số trận thắng', char: 'Nhân vật', diff: 'Độ khó', diffs: { easy: 'Dễ', normal: 'Vừa', hard: 'Khó' },
      sound: 'Âm thanh', anim: 'Hiệu ứng', reward: 'Phần thưởng', rewardSub: 'Chơi xong một ván được cộng điểm thân thiết (tối đa {cap}/ngày).',
      vs: 'VS',
    },
    ja: {
      title: 'じゃんけん', logo: 'じゃん<br>けん', ribbon: 'いっしょに遊ぼう！♡', score: 'スコア', you: 'あなた', me: 'わたし',
      firstTo: '先に{n}勝で勝ち', moves: { rock: 'グー', paper: 'パー', scissors: 'チョキ' },
      idle: '一回遊ばない？', turn: 'あなたの番！選んでね～', think: 'うーん…どれにしよう！',
      win: 'むぅ…今回はわたしの負け、すごいね >.<', lose: 'えへへ、{pet}の勝ち！🎉', draw: 'あいこ！もう一回ね',
      bWin: '勝ち！', bLose: '負け…', bDraw: 'あいこ！',
      finalWin: 'あなたの優勝！🎉', finalLose: '{pet}の優勝～💕', overSay: 'もう一回やる？',
      again: 'もう一回', quit: 'おしまい',
      help: 'グー・パー・チョキを選んでね（キー Q · W · E）。グーはチョキに、チョキはパーに、パーはグーに勝ち。先に決めた数だけ勝った方の優勝。',
      set: 'ゲーム設定', target: '勝利数', char: 'キャラクター', diff: '難易度', diffs: { easy: 'やさしい', normal: 'ふつう', hard: 'むずかしい' },
      sound: '効果音', anim: 'アニメーション', reward: 'ごほうび', rewardSub: '1ゲーム終わるごとになかよしポイント（1日{cap}まで）。',
      vs: 'VS',
    },
    en: {
      title: 'Rock Paper Scissors', logo: 'Rock<br>Paper<br>Scissors', ribbon: "Let's play together! ♡", score: 'Score', you: 'You', me: 'Me',
      firstTo: 'First to {n} wins!', moves: { rock: 'Rock', paper: 'Paper', scissors: 'Scissors' },
      idle: 'Wanna play a round?', turn: 'Your turn! Choose one~', think: "Hmm… let's see!",
      win: "Hmph… I lost this one, you're good >.<", lose: 'Hehe, {pet} wins! 🎉', draw: "It's a tie! Let's go again",
      bWin: 'You Win!', bLose: 'You Lose…', bDraw: 'Draw!',
      finalWin: 'You won the game! 🎉', finalLose: '{pet} won this game~ 💕', overSay: 'Another game?',
      again: 'Play again', quit: "I'm done",
      help: 'Pick Rock, Paper or Scissors (keys Q · W · E). Rock beats Scissors, Scissors beats Paper, Paper beats Rock. First to the target number of wins takes the game.',
      set: 'Game Settings', target: 'Target win', char: 'Character', diff: 'Difficulty', diffs: { easy: 'Easy', normal: 'Normal', hard: 'Hard' },
      sound: 'Sound effects', anim: 'Show animations', reward: 'Rewards', rewardSub: 'Each finished game earns friendship points (up to {cap} a day).',
      vs: 'VS',
    },
  };
  const ICON = { rock: '✊', paper: '✋', scissors: '✌️' };
  const ORDER = ['rock', 'paper', 'scissors'];
  const KEYS = { q: 'rock', w: 'paper', e: 'scissors', 1: 'rock', 2: 'paper', 3: 'scissors' };
  // 4 biểu cảm của Miu hiện trong khung vuông trên hàng thẻ (media/game/rps-*.png).
  // Nhân vật chọn trong ⚙ chỉ là hình nền lớn bên phải.
  const MIU_FACES = {
    idle: 'media/game/rps-wink.png', // chờ / mời chơi
    think: 'media/game/rps-think.png', // đang chọn
    cheer: 'media/game/rps-cheer.png', // em (pet) thắng
    pout: 'media/game/rps-pout.png', // em (pet) thua
    draw: 'media/game/rps-draw.png', // hoà
  };
  const CHARACTERS = [
    { id: 'miu', img: MIU_FACES.idle, bust: true },
    { id: 'cat', img: 'media/side/popup-girl.png', flip: false },
    { id: 'idol', img: 'media/side/settings-girl.png', flip: false },
    { id: 'silver', img: 'media/side/tasks-girl.png', flip: true },
  ];
  const RESULT_MS = 1600;
  const THINK_MS = 550;

  let L = T.vi;
  let petName = 'Em';
  // Tên gọi người chơi: tiếng Việt theo xưng hô đã chọn ("Anh", "Chị"…), còn lại theo L.you.
  let youName = 'Bạn';
  // Pet tự xưng (tiếng Việt theo xưng hô đã chọn: "em", "mình", "tớ"…).
  let meWord = 'em';
  const cap = (w) => (w ? w.charAt(0).toUpperCase() + w.slice(1) : w);
  const low = (w) => (w ? w.charAt(0).toLowerCase() + w.slice(1) : w);
  /** Biến xưng hô cho câu: {you} {You} {me} {Me} {pet}. */
  function words() {
    return { you: low(youName), You: youName, me: meWord, Me: cap(meWord), pet: petName };
  }
  let opts = loadOpts();
  let state = G.rpsNew();
  let history = [];
  let busy = false;
  let timers = [];

  function loadOpts() {
    const def = { target: 3, difficulty: 'normal', character: 'miu', sound: true, anim: true };
    try {
      const saved = JSON.parse(localStorage.getItem('rpsSettings') || '{}');
      return { ...def, ...saved };
    } catch (_) {
      return def;
    }
  }
  function saveOpts() {
    try { localStorage.setItem('rpsSettings', JSON.stringify(opts)); } catch (_) { /* ignore */ }
  }

  const fill = (s, vars) => String(s).replace(/\{(\w+)\}/g, (m, k) => (k in vars ? vars[k] : m));
  const later = (fn, ms) => timers.push(setTimeout(fn, ms));
  function clearTimers() {
    timers.forEach(clearTimeout);
    timers = [];
  }

  function rand() {
    const a = new Uint32Array(1);
    crypto.getRandomValues(a);
    return a[0] / 4294967296;
  }

  function say(text) {
    const b = $('bubble');
    b.textContent = fill(text, words());
    b.classList.remove('pop');
    void b.offsetWidth;
    b.classList.add('pop');
  }

  function character() {
    return CHARACTERS.find((x) => x.id === opts.character) || CHARACTERS[0];
  }

  /**
   * Tâm trạng: 'idle' (chờ / mời chơi), 'draw' (hoà), 'think' (đang chọn), 'cheer' (em —
   * pet — thắng), 'pout' (em thua). Biểu cảm đổi trong khung vuông trên hàng thẻ
   * (#face); hình nhân vật lớn bên phải là nền, đứng yên.
   */
  function girlMood(mood) {
    const want = MIU_FACES[mood] ? mood : 'idle';
    // Cả 4 ảnh nằm sẵn trong khung (game.html): chỉ đổi ảnh nào đang hiện.
    document.querySelectorAll('#portrait .face').forEach((f) => {
      const on = f.dataset.mood === want;
      f.classList.remove('swap', 'cheer', 'pout');
      f.classList.toggle('on', on);
      if (on) {
        void f.offsetWidth;
        f.classList.add(want === 'cheer' || want === 'pout' ? want : 'swap');
      }
    });
  }

  // Âm thanh: dùng lại tiếng của pet theo ngôn ngữ giọng đã chọn ở menu Âm thanh.
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

  function drawScore(bump) {
    for (const [id, v] of [['youScore', state.user], ['petScore', state.pet]]) {
      const el = $(id);
      if (el.textContent !== String(v) && bump) {
        el.classList.remove('bump');
        void el.offsetWidth;
        el.classList.add('bump');
      }
      el.textContent = String(v);
    }
  }

  function drawCards() {
    const box = $('cards');
    box.textContent = '';
    box.classList.remove('locked');
    ORDER.forEach((m, i) => {
      const b = document.createElement('button');
      b.type = 'button';
      b.className = 'card';
      b.dataset.move = m;
      b.innerHTML = '';
      const icon = document.createElement('div');
      icon.className = 'icon';
      icon.textContent = ICON[m];
      const label = document.createElement('div');
      label.className = 'label';
      label.textContent = L.moves[m];
      const key = document.createElement('div');
      key.className = 'key';
      key.textContent = 'QWE'[i];
      b.append(icon, label, key);
      b.addEventListener('click', () => play(m));
      box.appendChild(b);
    });
  }

  function showBanner(kind) {
    const b = $('banner');
    b.className = 'banner ' + kind;
    b.textContent = fill(kind === 'win' ? L.bWin : kind === 'lose' ? L.bLose : L.bDraw, words());
    void b.offsetWidth;
    b.classList.add('show');
  }

  function vsCard(side, move, won) {
    const c = document.createElement('div');
    c.className = 'vs-card ' + side + (won ? ' win' : '');
    const tag = document.createElement('div');
    tag.className = 'tag';
    tag.textContent = side === 'you' ? youName : petName;
    const icon = document.createElement('div');
    icon.className = 'icon';
    icon.textContent = ICON[move];
    c.append(tag, icon);
    return c;
  }

  function toChoosing() {
    busy = false;
    $('versus').hidden = true;
    $('over').hidden = true;
    $('cards').hidden = false;
    // Xoá cả class kết quả (win/lose/draw) của lượt trước, không chỉ chữ.
    $('banner').textContent = '';
    $('banner').className = 'banner';
    drawCards();
    say(L.turn);
    girlMood('idle');
  }

  function play(move) {
    if (busy || state.over) return;
    busy = true;
    $('help').hidden = true;
    // 1) Đang chọn: thẻ người chơi sáng lên, pet "để xem nào".
    const box = $('cards');
    box.classList.add('locked');
    box.querySelector(`[data-move="${move}"]`)?.classList.add('picked');
    say(L.think);
    girlMood('think');
    const petMove = G.rpsPetMove(history, opts.difficulty, rand);
    history.push(move);

    later(() => {
      // 2) Kết quả: You [thẻ] VS [thẻ] Pet + chữ lớn.
      const r = G.rpsRound(move, petMove);
      state = G.rpsScore(state, r, opts.target);
      const vs = $('versus');
      vs.textContent = '';
      const text = document.createElement('div');
      text.className = 'vs-text';
      text.textContent = L.vs;
      vs.append(vsCard('you', move, r === 'win'), text, vsCard('pet', petMove, r === 'lose'));
      box.hidden = true;
      vs.hidden = false;
      showBanner(r);
      drawScore(true);
      if (r === 'win') {
        say(L.win);
        girlMood('pout');
        sound('poke.mp3');
      } else if (r === 'lose') {
        say(fill(L.lose, { pet: petName }));
        girlMood('cheer');
        sound('headpat.mp3');
      } else {
        say(L.draw);
        girlMood('draw');
      }
      // 3) Hết ván → bảng kết thúc; chưa hết → quay lại chọn.
      later(() => (state.over ? finish() : toChoosing()), RESULT_MS);
    }, THINK_MS);
  }

  async function finish() {
    const userWon = state.winner === 'user';
    $('versus').hidden = true;
    $('banner').textContent = '';
    $('banner').className = 'banner';
    $('final').textContent = fill(userWon ? L.finalWin : L.finalLose, words());
    $('over').hidden = false;
    say(L.overSay);
    girlMood(userWon ? 'pout' : 'cheer');
    // Thành tích cho trang Mini Games (arcade.html đọc cùng khoá).
    try {
      const st = { plays: 0, wins: 0, ...JSON.parse(localStorage.getItem('rpsStats') || '{}') };
      st.plays += 1;
      if (userWon) st.wins += 1;
      localStorage.setItem('rpsStats', JSON.stringify(st));
    } catch (_) { /* ignore */ }
    try {
      await invoke('game_result', { won: userWon });
    } catch (e) {
      log('[rps] game_result: ' + e);
    }
    refreshReward();
  }

  function newGame() {
    clearTimers();
    state = G.rpsNew();
    history = [];
    drawScore(false);
    toChoosing();
  }

  // ─── Cài đặt game ──────────────────────────────────────────────────────────
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

  function applyCharacter() {
    const c = character();
    const g = $('girl');
    g.src = c.img;
    g.classList.toggle('flip', !!c.flip);
    // Bán thân (Miu) đặt khác nhân vật toàn thân: thấp hơn, sát góc phải.
    g.classList.toggle('bust', !!c.bust);
  }

  function drawSettings() {
    seg('segTarget', [3, 5, 7], opts.target, String, (v) => {
      opts.target = v;
      saveOpts();
      drawSettings();
      $('firstTo').textContent = fill(L.firstTo, { n: v });
      newGame();
    });
    seg('segDiff', ['easy', 'normal', 'hard'], opts.difficulty, (v) => L.diffs[v], (v) => {
      opts.difficulty = v;
      saveOpts();
      drawSettings();
    });
    const chars = $('chars');
    chars.textContent = '';
    for (const c of CHARACTERS) {
      const b = document.createElement('button');
      b.type = 'button';
      b.classList.toggle('on', c.id === opts.character);
      const img = document.createElement('img');
      img.src = c.img;
      img.alt = '';
      b.appendChild(img);
      b.addEventListener('click', () => {
        opts.character = c.id;
        saveOpts();
        applyCharacter();
        drawSettings();
      });
      chars.appendChild(b);
    }
    $('optSound').checked = !!opts.sound;
    $('optAnim').checked = !!opts.anim;
    document.body.classList.toggle('no-anim', !opts.anim);
  }

  async function refreshReward() {
    try {
      const info = await invoke('game_info');
      const cap = Math.max(1, info.points_cap || 1);
      const got = Math.min(cap, info.points_today || 0);
      $('rFill').style.width = Math.round((got * 100) / cap) + '%';
      $('rNum').textContent = `${got}/${cap}`;
      $('rSub').textContent = fill(L.rewardSub, { cap });
    } catch (e) {
      log('[rps] game_info: ' + e);
    }
  }

  function applyTexts() {
    document.documentElement.lang = L === T.ja ? 'ja' : L === T.en ? 'en' : 'vi';
    $('titleText').textContent = L.title;
    $('logo').innerHTML = L.logo; // chuỗi cố định trong file này, không có dữ liệu người dùng
    $('ribbon').textContent = L.ribbon;
    $('scoreTitle').textContent = L.score;
    $('youName').textContent = youName;
    $('petName').textContent = petName;
    $('firstTo').textContent = fill(L.firstTo, { n: opts.target });
    $('help').textContent = L.help;
    $('againBtn').textContent = L.again;
    $('quitBtn').textContent = L.quit;
    $('setTitle').textContent = L.set;
    $('lblTarget').textContent = L.target;
    $('lblChar').textContent = L.char;
    $('lblDiff').textContent = L.diff;
    $('lblSound').textContent = L.sound;
    $('lblAnim').textContent = L.anim;
    $('rTitle').textContent = L.reward;
  }

  function hideWindow() {
    clearTimers();
    const w = window.__TAURI__ && window.__TAURI__.window && window.__TAURI__.window.getCurrentWindow();
    if (w) w.hide().catch(() => {});
  }

  // ─── Sự kiện ───────────────────────────────────────────────────────────────
  $('closeBtn').addEventListener('click', hideWindow);
  $('quitBtn').addEventListener('click', hideWindow);
  $('againBtn').addEventListener('click', newGame);
  $('helpBtn').addEventListener('click', () => { $('help').hidden = !$('help').hidden; });
  $('setBtn').addEventListener('click', () => {
    const s = $('settings');
    s.hidden = !s.hidden;
    $('setBtn').classList.toggle('on', !s.hidden);
    if (!s.hidden) refreshReward();
  });
  $('optSound').addEventListener('change', (e) => { opts.sound = e.target.checked; saveOpts(); });
  $('optAnim').addEventListener('change', (e) => { opts.anim = e.target.checked; saveOpts(); drawSettings(); });
  window.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') {
      if (!$('settings').hidden) { $('settings').hidden = true; $('setBtn').classList.remove('on'); }
      else hideWindow();
      return;
    }
    const m = KEYS[e.key.toLowerCase()];
    if (m && !$('cards').hidden) play(m);
    else if ((e.key === 'Enter' || e.key === ' ') && !$('over').hidden) newGame();
  });

  // Mỗi lần mở cửa sổ (dựng sẵn một lần): lấy ngôn ngữ + tên pet, bắt đầu ván mới.
  async function open() {
    try {
      const info = await invoke('game_info');
      L = T[info.lang] || T.vi;
      petName = (info.pet && String(info.pet).trim()) || L.me;
      const you = String(info.you || '').trim();
      youName = info.lang === 'vi' && you ? cap(you) : L.you;
      const me = String(info.me || '').trim();
      meWord = info.lang === 'vi' && me ? me : 'em';
    } catch (e) {
      log('[rps] game_info: ' + e);
    }
    applyTexts();
    applyCharacter();
    drawSettings();
    refreshReward();
    newGame();
    say(L.idle);
    later(() => say(L.turn), 1800);
  }

  listen('game-refresh', open);
  open();
})();
