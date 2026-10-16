// Trang Mini Games (arcade.html). Rust: game_info (ngôn ngữ, tên pet, xưng hô,
// điểm thân thiết) và open_mini_game (ẩn trang này, mở cửa sổ game). Thành
// tích đọc từ localStorage mà các cửa sổ game ghi (rpsStats, guessStats,
// minesStats); cùng origin nên đọc chung được.
(function () {
  const { invoke, listen, log } = window.DC;
  const G = window.GamesCore;
  const $ = (id) => document.getElementById(id);
  const GAMES = ['rps', 'guess', 'mines'];

  // {you}/{You} = gọi người chơi, {me}/{Me} = pet tự xưng, {pet} = tên pet.
  const T = {
    vi: {
      me: 'Em', you: 'Bạn',
      sub: 'Cùng {pet} chơi những mini game dễ thương nhé!',
      bubble: 'Hôm nay chúng ta chơi game gì nhỉ? {Pet} đã chuẩn bị nhiều trò thú vị đó!',
      chibi: 'Chơi game cùng {me} nhé? {Me} luôn chờ {you} đấy!',
      today: 'Hôm nay', todayPoints: 'Điểm từ game', todayGame: 'Game của ngày',
      bond: 'Điểm thân thiết', bondNums: '{have} / {need} · {level}', bondMax: '{points} điểm · {level}',
      names: { rps: 'Oẳn tù tì', guess: 'Đoán số', mines: 'Gỡ bom' },
      tagRps: 'Phổ biến', tagGuess: 'Thử thách', tagMines: 'Logic', todayTag: '★ Hôm nay',
      descRps: 'Oẳn tù tì cùng {pet}! Ai sẽ thắng hôm nay nhỉ?',
      descGuess: 'Đoán số bí mật mà {pet} đã chọn! {You} có tìm ra không?',
      descMines: 'Giúp {pet} gỡ hết bom nhé! Hãy suy nghĩ thật kỹ!',
      quick: 'Chơi nhanh', quickRps: '10–20 giây', round: 'Mỗi ván', quickGuess: '30–60 giây',
      time: 'Thời gian', quickMines: 'Tùy ý',
      wins: 'Đã thắng', streak: 'Chuỗi thắng', fastest: 'Nhanh nhất', winsN: '{n} ván',
      play: 'Chơi ngay',
      achTitle: 'Thành tích', achSub: 'Xem thống kê và kỷ lục của {you}',
      rewardTitle: 'Phần thưởng', rewardSub: 'Hôm nay: {got}/{cap} điểm thân thiết',
      dailyTitle: 'Thử thách hằng ngày', dailySub: 'Hôm nay chơi {game} nha!',
      colPlays: 'Số ván', colWins: 'Thắng', colBest: 'Kỷ lục',
      rewardHelp: [
        'Chơi xong mỗi ván (thắng hay thua) {you} được +1 điểm thân thiết với {pet}, tối đa {cap} điểm mỗi ngày.',
        'Hôm nay {you} đã nhận {got}/{cap} điểm từ game.',
      ],
      help: [
        'Bấm "Chơi ngay" trên một thẻ để mở trò đó trong cửa sổ riêng.',
        'Mỗi ngày có một "game của ngày" được gắn nhãn ★ Hôm nay.',
        'Esc để đóng trang này.',
      ],
    },
    ja: {
      me: 'わたし', you: 'あなた',
      sub: 'いっしょにかわいいミニゲームで遊ぼう！',
      bubble: '今日はどのゲームで遊ぶ？いろいろ用意したよ！',
      chibi: 'いっしょに遊ぼう？ずっと待ってるよ！',
      today: '今日', todayPoints: 'ゲームのポイント', todayGame: '今日のゲーム',
      bond: 'なかよし度', bondNums: '{have} / {need} · {level}', bondMax: '{points}ポイント · {level}',
      names: { rps: 'じゃんけん', guess: '数当て', mines: 'ボム処理班' },
      tagRps: '人気', tagGuess: 'チャレンジ', tagMines: 'ロジック', todayTag: '★ 今日',
      descRps: 'わたしとじゃんけん！今日はどっちが勝つかな？',
      descGuess: 'わたしが選んだ秘密の数を当ててね！',
      descMines: '爆弾をぜんぶ見つけてね！よく考えて！',
      quick: 'サクッと', quickRps: '10〜20秒', round: '1回', quickGuess: '30〜60秒',
      time: '時間', quickMines: '自由',
      wins: '勝った数', streak: '連勝', fastest: '最速', winsN: '{n}回',
      play: '遊ぶ',
      achTitle: '成績', achSub: '記録と統計を見る',
      rewardTitle: 'ごほうび', rewardSub: '今日：なかよし {got}/{cap}',
      dailyTitle: '今日のチャレンジ', dailySub: '今日は{game}で遊ぼう！',
      colPlays: '回数', colWins: '勝ち', colBest: '記録',
      rewardHelp: [
        '1回遊ぶごとに（勝っても負けても）なかよし度+1、1日{cap}ポイントまで。',
        '今日はゲームで{got}/{cap}ポイントもらったよ。',
      ],
      help: [
        'カードの「遊ぶ」を押すと、そのゲームが別ウィンドウで開くよ。',
        '毎日ひとつ「★ 今日」のゲームがあるよ。',
        'Escでこのページを閉じる。',
      ],
    },
    en: {
      me: 'I', you: 'You',
      sub: 'Play cute mini games with me!',
      bubble: 'What shall we play today? I prepared lots of fun games!',
      chibi: "Play with me? I'm always waiting for you!",
      today: 'Today', todayPoints: 'Points from games', todayGame: 'Game of the day',
      bond: 'Friendship', bondNums: '{have} / {need} · {level}', bondMax: '{points} points · {level}',
      names: { rps: 'Rock paper scissors', guess: 'Guess my number', mines: 'Bomb Squad' },
      tagRps: 'Popular', tagGuess: 'Challenge', tagMines: 'Logic', todayTag: '★ Today',
      descRps: 'Rock paper scissors with me! Who wins today?',
      descGuess: 'Guess the secret number I picked! Can you find it?',
      descMines: 'Help me clear every bomb! Think carefully!',
      quick: 'Quick play', quickRps: '10–20 sec', round: 'Each round', quickGuess: '30–60 sec',
      time: 'Time', quickMines: 'Any',
      wins: 'Wins', streak: 'Best streak', fastest: 'Fastest', winsN: '{n}',
      play: 'Play now',
      achTitle: 'Achievements', achSub: 'See your stats and records',
      rewardTitle: 'Rewards', rewardSub: 'Today: {got}/{cap} friendship points',
      dailyTitle: 'Daily challenge', dailySub: "Today's pick: {game}!",
      colPlays: 'Games', colWins: 'Wins', colBest: 'Record',
      rewardHelp: [
        'Every finished game (win or lose) gives +1 friendship point, up to {cap} a day.',
        'Today you got {got}/{cap} points from games.',
      ],
      help: [
        'Press "Play now" on a card to open that game in its own window.',
        'Each day one game is marked ★ Today.',
        'Esc closes this page.',
      ],
    },
  };

  let L = T.vi;
  let lang = 'vi';
  let petName = '';
  let youName = 'Bạn';
  let meWord = 'em';
  let info = {};

  const cap = (w) => (w ? w.charAt(0).toUpperCase() + w.slice(1) : w);
  const low = (w) => (w ? w.charAt(0).toLowerCase() + w.slice(1) : w);
  // {pet}: tên pet nếu đã đặt, chưa thì là chính từ pet tự xưng ("em"), để câu
  // giữa chừng không bị viết hoa "Cùng Em chơi…". Câu ja/en không dùng {pet}.
  function words(extra) {
    const pet = petName || meWord;
    return { you: low(youName), You: youName, me: meWord, Me: cap(meWord), pet, Pet: cap(pet), ...(extra || {}) };
  }
  const fill = (s, vars) => String(s).replace(/\{(\w+)\}/g, (m, k) => (k in vars ? vars[k] : m));

  function readJson(key) {
    try { return JSON.parse(localStorage.getItem(key) || '{}') || {}; } catch (_) { return {}; }
  }
  function fmtTime(sec) {
    return String(Math.floor(sec / 60)).padStart(2, '0') + ':' + String(sec % 60).padStart(2, '0');
  }

  /** Thành tích từng trò: { plays, wins, best (chuỗi đã định dạng) }. */
  function gameStats() {
    const rps = readJson('rpsStats');
    const guess = readJson('guessStats');
    const mines = readJson('minesStats');
    const times = Object.values(mines.best || {}).filter((v) => Number.isFinite(v));
    return {
      rps: { plays: rps.plays || 0, wins: rps.wins || 0, best: rps.wins ? fill(L.winsN, { n: rps.wins }) : '—' },
      guess: { plays: guess.plays || 0, wins: guess.wins || 0, best: guess.best ? `${L.streak} ${guess.best}` : '—' },
      mines: { plays: mines.plays || 0, wins: mines.wins || 0, best: times.length ? fmtTime(Math.min(...times)) : '—' },
      bestStreak: guess.best || 0,
      fastest: times.length ? fmtTime(Math.min(...times)) : '—',
    };
  }

  function daily() {
    return G.dailyPick(new Date(), GAMES);
  }

  function draw() {
    document.documentElement.lang = lang;
    // Ô hồ sơ cần một cái tên, chưa đặt tên thì dùng tên nhân vật trong game.
    $('petName').textContent = (info.pet && String(info.pet).trim()) || 'Miu';
    $('subText').textContent = fill(L.sub, words());
    $('bubble').textContent = fill(L.bubble, words());
    $('chibiSay').textContent = fill(L.chibi, words());
    document.querySelectorAll('[data-t]').forEach((el) => {
      const v = L[el.dataset.t];
      if (typeof v === 'string') el.textContent = fill(v, words());
    });

    const got = info.points_today || 0;
    const capPts = info.points_cap || 3;
    $('todayPoints').textContent = `${got}/${capPts}`;
    const pick = daily();
    $('todayGame').textContent = L.names[pick];
    $('rewardSub').textContent = fill(L.rewardSub, { got, cap: capPts });
    $('dailySub').textContent = fill(L.dailySub, { game: L.names[pick] });

    const level = info.level_name || '';
    if (info.progress) {
      const { have, need } = info.progress;
      $('bondNums').textContent = fill(L.bondNums, { have, need, level });
      $('bondBar').style.width = need ? Math.min(100, Math.round((have * 100) / need)) + '%' : '100%';
    } else {
      $('bondNums').textContent = fill(L.bondMax, { points: info.points || 0, level });
      $('bondBar').style.width = '100%';
    }

    const st = gameStats();
    $('statRps').textContent = fill(L.winsN, { n: st.rps.wins });
    $('statGuess').textContent = String(st.bestStreak);
    $('statMines').textContent = st.fastest;

    // Nhãn "Hôm nay" trên thẻ game của ngày.
    document.querySelectorAll('.card').forEach((card) => {
      const old = card.querySelector('.ribbon');
      if (old) old.remove();
      if (card.dataset.game === pick) {
        const r = document.createElement('div');
        r.className = 'ribbon';
        r.textContent = L.todayTag;
        card.querySelector('.artBox').appendChild(r);
      }
    });

    const body = $('statsBody');
    body.textContent = '';
    for (const g of GAMES) {
      const tr = document.createElement('tr');
      const cells = [L.names[g], String(st[g].plays), String(st[g].wins), st[g].best];
      cells.forEach((text, i) => {
        const td = document.createElement('td');
        td.textContent = text;
        if (i) td.className = 'n';
        tr.appendChild(td);
      });
      body.appendChild(tr);
    }
  }

  function sheet(id, lines, fromHelp) {
    const other = id === 'statsSheet' ? 'helpSheet' : 'statsSheet';
    $(other).hidden = true;
    const el = $(id);
    if (lines) {
      el.textContent = '';
      for (const line of lines) {
        const p = document.createElement('p');
        p.textContent = line;
        el.appendChild(p);
      }
    }
    el.hidden = !el.hidden;
    $('statsBtn').classList.toggle('on', !$('statsSheet').hidden);
    $('helpBtn').classList.toggle('on', !$('helpSheet').hidden && !!fromHelp);
  }

  function closeSheets() {
    $('statsSheet').hidden = true;
    $('helpSheet').hidden = true;
    $('statsBtn').classList.remove('on');
    $('helpBtn').classList.remove('on');
  }

  async function play(kind) {
    closeSheets();
    try {
      await invoke('open_mini_game', { kind });
    } catch (e) {
      log('[arcade] open_mini_game: ' + e);
    }
  }

  // Ẩn bằng lệnh Rust (ẩn luôn nhân vật bám mép), lỗi thì mới dùng API cửa sổ của JS.
  function hideWindow() {
    closeSheets();
    invoke('arcade_close').catch(() => {
      const w = window.__TAURI__ && window.__TAURI__.window && window.__TAURI__.window.getCurrentWindow();
      if (w) w.hide().catch(() => {});
    });
  }

  // ─── Sự kiện ─────────────────────────────────────────────────────────────
  document.querySelectorAll('.card').forEach((card) => {
    card.querySelector('.play').addEventListener('click', () => play(card.dataset.game));
  });
  $('closeBtn').addEventListener('click', hideWindow);
  $('statsBtn').addEventListener('click', () => sheet('statsSheet'));
  $('tileStats').addEventListener('click', () => sheet('statsSheet'));
  $('helpBtn').addEventListener('click', () => sheet('helpSheet', L.help.map((l) => fill(l, words())), true));
  $('tileReward').addEventListener('click', () => {
    const vars = words({ got: info.points_today || 0, cap: info.points_cap || 3 });
    sheet('helpSheet', L.rewardHelp.map((l) => fill(l, vars)));
  });
  $('tileDaily').addEventListener('click', () => play(daily()));
  window.addEventListener('keydown', (e) => {
    if (e.key !== 'Escape') return;
    if (!$('statsSheet').hidden || !$('helpSheet').hidden) closeSheets();
    else hideWindow();
  });

  // Mỗi lần mở trang (dựng sẵn một lần): đọc lại ngôn ngữ, điểm, thành tích.
  async function open() {
    try {
      info = (await invoke('game_info')) || {};
      lang = T[info.lang] ? info.lang : 'vi';
      L = T[lang];
      petName = (info.pet && String(info.pet).trim()) || '';
      const you = String(info.you || '').trim();
      youName = lang === 'vi' && you ? cap(you) : L.you;
      const me = String(info.me || '').trim();
      meWord = lang === 'vi' && me ? me : lang === 'vi' ? 'em' : low(L.me);
    } catch (e) {
      log('[arcade] game_info: ' + e);
    }
    closeSheets();
    draw();
  }

  listen('arcade-refresh', open);
  open();
})();
