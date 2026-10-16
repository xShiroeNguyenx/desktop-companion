// Luật của mini game (PLAN.md 12.8): oẳn tù tì, đoán số, game của ngày. Hàm thuần, không
// đụng DOM; nguồn ngẫu nhiên được truyền vào (`rand` trả số trong [0, 1)) để
// test được. Dùng trong pet (window.GamesCore) và node --test.
(function (root) {
  const MOVES = ['rock', 'scissors', 'paper'];
  // Mỗi tay thắng tay nào.
  const BEATS = { rock: 'scissors', scissors: 'paper', paper: 'rock' };
  const RPS_TARGET = 3;

  /** Số nguyên ngẫu nhiên trong [min, max]. */
  function randInt(min, max, rand) {
    const r = Math.min(Math.max(Number(rand()) || 0, 0), 0.999999999);
    return min + Math.floor(r * (max - min + 1));
  }

  function rpsPick(rand) {
    return MOVES[randInt(0, MOVES.length - 1, rand)];
  }

  /** Kết quả một lượt, theo góc nhìn người chơi: 'win' | 'lose' | 'draw'. */
  function rpsRound(user, pet) {
    if (!BEATS[user] || !BEATS[pet]) throw new Error('nước đi lạ: ' + user + ' / ' + pet);
    if (user === pet) return 'draw';
    return BEATS[user] === pet ? 'win' : 'lose';
  }

  function rpsNew() {
    return { user: 0, pet: 0, draws: 0, over: false, winner: null };
  }

  /** Ghi một lượt vào tỉ số; ai đủ `target` trận thắng trước thì hết ván. */
  function rpsScore(state, result, target = RPS_TARGET) {
    if (state.over) return state;
    const goal = Math.max(1, Number(target) || RPS_TARGET);
    const s = { ...state };
    if (result === 'win') s.user += 1;
    else if (result === 'lose') s.pet += 1;
    else s.draws += 1;
    if (s.user >= goal || s.pet >= goal) {
      s.over = true;
      s.winner = s.user > s.pet ? 'user' : 'pet';
    }
    return s;
  }

  // Tay thắng / thua một tay cho trước.
  const BEATEN_BY = { scissors: 'rock', paper: 'scissors', rock: 'paper' };

  /** Tay người chơi ra nhiều nhất trong lịch sử (hoà thì lấy tay gần đây hơn). */
  function favouriteMove(history) {
    const count = { rock: 0, scissors: 0, paper: 0 };
    let best = null;
    history.forEach((m) => {
      if (!(m in count)) return;
      count[m] += 1;
      if (!best || count[m] >= count[best]) best = m;
    });
    return best;
  }

  /**
   * Pet ra tay theo độ khó, dựa vào các tay người chơi đã ra trong ván:
   *  - 'easy':   60% đoán tay người chơi hay ra và cố tình THUA nó, còn lại ngẫu nhiên.
   *  - 'normal': ngẫu nhiên hoàn toàn.
   *  - 'hard':   60% đoán tay người chơi hay ra và đánh THẮNG nó, còn lại ngẫu nhiên.
   * Chưa có lịch sử thì luôn ngẫu nhiên.
   */
  function rpsPetMove(history, difficulty, rand) {
    const fav = favouriteMove(history || []);
    if (!fav || difficulty === 'normal' || (difficulty !== 'easy' && difficulty !== 'hard')) return rpsPick(rand);
    if (rand() >= 0.6) return rpsPick(rand);
    return difficulty === 'hard' ? BEATEN_BY[fav] : BEATS[fav];
  }

  const GUESS_MIN = 1;
  const GUESS_MAX = 100;
  const GUESS_TURNS = 7;

  /**
   * Ván đoán số mới. opts: { min, max, turns } (mặc định 1..100, 7 lượt).
   * low / high = khoảng còn có thể chứa số bí mật (thu hẹp sau mỗi lượt sai).
   */
  function guessNew(rand, opts) {
    const o = opts || {};
    const min = Number.isInteger(o.min) ? o.min : GUESS_MIN;
    const max = Number.isInteger(o.max) && o.max > min ? o.max : GUESS_MAX;
    const maxTurns = Number.isInteger(o.turns) && o.turns > 0 ? o.turns : GUESS_TURNS;
    return { secret: randInt(min, max, rand), min, max, maxTurns, low: min, high: max,
      turns: 0, history: [], done: false, won: false };
  }

  /**
   * Một lượt đoán. Trả { state, hint } với hint:
   * 'invalid' (không phải số nguyên trong khoảng, KHÔNG tính lượt) | 'higher' | 'lower' | 'correct'.
   * Hết lượt mà chưa đúng thì state.done = true, state.won = false.
   */
  function guess(state, value) {
    if (state.done) return { state, hint: 'invalid' };
    const min = state.min ?? GUESS_MIN;
    const max = state.max ?? GUESS_MAX;
    const maxTurns = state.maxTurns ?? GUESS_TURNS;
    const n = Number(String(value).trim());
    if (String(value).trim() === '' || !Number.isInteger(n) || n < min || n > max) return { state, hint: 'invalid' };
    const s = { ...state, turns: state.turns + 1, history: [...(state.history || [])] };
    let hint;
    if (n === s.secret) {
      s.done = true;
      s.won = true;
      hint = 'correct';
    } else {
      hint = n < s.secret ? 'higher' : 'lower';
      if (hint === 'higher') s.low = Math.max(s.low ?? min, n + 1);
      else s.high = Math.min(s.high ?? max, n - 1);
      if (s.turns >= maxTurns) s.done = true;
    }
    s.history.push({ n, hint });
    return { state: s, hint };
  }

  /** Độ gần để pet đổi nét mặt, theo độ rộng phạm vi (mặc định 100): 'hot' (≤ 5%), 'warm' (≤ 15%), 'cold'. */
  function warmth(secret, n, span = 100) {
    const d = Math.abs(secret - n);
    const s = Math.max(1, span);
    return d <= Math.max(1, Math.round(s * 0.05)) ? 'hot' : d <= Math.max(2, Math.round(s * 0.15)) ? 'warm' : 'cold';
  }

  /** Số lần được xin gợi ý theo độ khó. */
  function hintsAllowed(difficulty) {
    return difficulty === 'easy' ? 3 : difficulty === 'hard' ? 0 : 1;
  }

  /**
   * Gợi ý thứ `index` (đếm từ 0), từ dễ dùng tới chi tiết:
   *  0 → { kind: 'parity', even }        số chẵn hay lẻ
   *  1 → { kind: 'range', from, to }     nửa khoảng còn lại chứa số bí mật
   *  2 → { kind: 'digit', last }         chữ số tận cùng
   */
  function guessHint(state, index) {
    const low = state.low ?? state.min ?? GUESS_MIN;
    const high = state.high ?? state.max ?? GUESS_MAX;
    if (index === 0) return { kind: 'parity', even: state.secret % 2 === 0 };
    if (index === 1) {
      const mid = Math.floor((low + high) / 2);
      return state.secret <= mid ? { kind: 'range', from: low, to: mid } : { kind: 'range', from: mid + 1, to: high };
    }
    return { kind: 'digit', last: state.secret % 10 };
  }

  /**
   * "Game của ngày" cho trang Mini Games: xoay vòng theo ngày lịch địa
   * phương (cùng một ngày luôn ra cùng một trò, sang ngày mới đổi trò).
   */
  function dailyPick(date, list) {
    if (!list || !list.length) return null;
    const d = date instanceof Date ? date : new Date(date);
    const day = Math.floor(Date.UTC(d.getFullYear(), d.getMonth(), d.getDate()) / 86400000);
    return list[((day % list.length) + list.length) % list.length];
  }

  const api = {
    MOVES, RPS_TARGET, GUESS_MIN, GUESS_MAX, GUESS_TURNS,
    randInt, rpsPick, rpsRound, rpsNew, rpsScore, favouriteMove, rpsPetMove, guessNew, guess, warmth, hintsAllowed, guessHint, dailyPick,
  };
  if (typeof module !== 'undefined' && module.exports) module.exports = api;
  else root.GamesCore = api;
})(typeof window !== 'undefined' ? window : globalThis);
