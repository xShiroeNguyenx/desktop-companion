// Luật của mini game (PLAN.md 12.8): oẳn tù tì và đoán số. Hàm thuần, không
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

  /** Ghi một lượt vào tỉ số; ai đủ RPS_TARGET trận thắng trước thì hết ván. */
  function rpsScore(state, result) {
    if (state.over) return state;
    const s = { ...state };
    if (result === 'win') s.user += 1;
    else if (result === 'lose') s.pet += 1;
    else s.draws += 1;
    if (s.user >= RPS_TARGET || s.pet >= RPS_TARGET) {
      s.over = true;
      s.winner = s.user > s.pet ? 'user' : 'pet';
    }
    return s;
  }

  const GUESS_MIN = 1;
  const GUESS_MAX = 100;
  const GUESS_TURNS = 7;

  function guessNew(rand) {
    return { secret: randInt(GUESS_MIN, GUESS_MAX, rand), turns: 0, done: false, won: false };
  }

  /**
   * Một lượt đoán. Trả { state, hint } với hint:
   * 'invalid' (không phải số nguyên 1..100, KHÔNG tính lượt) | 'higher' | 'lower' | 'correct'.
   * Hết lượt mà chưa đúng thì state.done = true, state.won = false.
   */
  function guess(state, value) {
    if (state.done) return { state, hint: 'invalid' };
    const n = Number(String(value).trim());
    if (!Number.isInteger(n) || n < GUESS_MIN || n > GUESS_MAX) return { state, hint: 'invalid' };
    const s = { ...state, turns: state.turns + 1 };
    if (n === s.secret) {
      s.done = true;
      s.won = true;
      return { state: s, hint: 'correct' };
    }
    if (s.turns >= GUESS_TURNS) s.done = true;
    return { state: s, hint: n < s.secret ? 'higher' : 'lower' };
  }

  /** Độ gần để pet đổi nét mặt: 'hot' (≤ 5), 'warm' (≤ 15), 'cold'. */
  function warmth(secret, n) {
    const d = Math.abs(secret - n);
    return d <= 5 ? 'hot' : d <= 15 ? 'warm' : 'cold';
  }

  const api = {
    MOVES, RPS_TARGET, GUESS_MIN, GUESS_MAX, GUESS_TURNS,
    randInt, rpsPick, rpsRound, rpsNew, rpsScore, guessNew, guess, warmth,
  };
  if (typeof module !== 'undefined' && module.exports) module.exports = api;
  else root.GamesCore = api;
})(typeof window !== 'undefined' ? window : globalThis);
