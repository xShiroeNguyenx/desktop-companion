// node --test app/js/  (npm test)
const test = require('node:test');
const assert = require('node:assert/strict');
const G = require('./games-core.js');

const fixed = (...values) => {
  let i = 0;
  return () => values[i++ % values.length];
};

test('số ngẫu nhiên nằm trong khoảng, kể cả rand biên', () => {
  assert.equal(G.randInt(1, 100, () => 0), 1);
  assert.equal(G.randInt(1, 100, () => 0.999999), 100);
  assert.equal(G.randInt(1, 100, () => 1), 100); // rand lỗi trả đúng 1
  assert.equal(G.randInt(1, 100, () => NaN), 1);
  assert.equal(G.randInt(1, 100, () => -3), 1);
  assert.deepEqual([0, 0.34, 0.67].map((r) => G.rpsPick(() => r)), ['rock', 'scissors', 'paper']);
});

test('oẳn tù tì: luật thắng thua', () => {
  assert.equal(G.rpsRound('rock', 'scissors'), 'win');
  assert.equal(G.rpsRound('scissors', 'paper'), 'win');
  assert.equal(G.rpsRound('paper', 'rock'), 'win');
  assert.equal(G.rpsRound('rock', 'paper'), 'lose');
  assert.equal(G.rpsRound('paper', 'paper'), 'draw');
  assert.throws(() => G.rpsRound('lizard', 'rock'));
});

test('oẳn tù tì: ai thắng 3 trận trước thì hết ván', () => {
  let s = G.rpsNew();
  for (const r of ['win', 'draw', 'lose', 'win', 'lose']) s = G.rpsScore(s, r);
  assert.deepEqual([s.user, s.pet, s.draws, s.over], [2, 2, 1, false]);
  s = G.rpsScore(s, 'lose');
  assert.equal(s.over, true);
  assert.equal(s.winner, 'pet');
  // Hết ván thì không ghi thêm.
  assert.equal(G.rpsScore(s, 'win').user, 2);
});

test('oẳn tù tì: số trận thắng chỉnh được', () => {
  let s = G.rpsNew();
  for (let i = 0; i < 4; i++) s = G.rpsScore(s, 'win', 5);
  assert.equal(s.over, false);
  s = G.rpsScore(s, 'win', 5);
  assert.deepEqual([s.over, s.winner], [true, 'user']);
  // Giá trị lạ → mặc định 3.
  let t = G.rpsNew();
  for (let i = 0; i < 3; i++) t = G.rpsScore(t, 'lose', 'abc');
  assert.equal(t.over, true);
});

test('oẳn tù tì: độ khó của pet', () => {
  assert.equal(G.favouriteMove([]), null);
  assert.equal(G.favouriteMove(['rock', 'paper', 'rock']), 'rock');
  assert.equal(G.favouriteMove(['rock', 'paper']), 'paper'); // hoà → tay gần đây hơn
  const hist = ['rock', 'rock', 'paper'];
  // rand < 0.6 → đoán theo tay hay ra (rock).
  assert.equal(G.rpsPetMove(hist, 'hard', fixed(0.1)), 'paper'); // giấy thắng búa
  assert.equal(G.rpsPetMove(hist, 'easy', fixed(0.1)), 'scissors'); // kéo thua búa
  // rand ≥ 0.6 → ngẫu nhiên (lượt rand kế tiếp chọn tay).
  assert.equal(G.rpsPetMove(hist, 'hard', fixed(0.7, 0)), 'rock');
  // Normal / độ khó lạ / chưa có lịch sử → ngẫu nhiên.
  assert.equal(G.rpsPetMove(hist, 'normal', fixed(0.67)), 'paper');
  assert.equal(G.rpsPetMove(hist, 'xyz', fixed(0)), 'rock');
  assert.equal(G.rpsPetMove([], 'hard', fixed(0.34)), 'scissors');
});

test('đoán số: gợi ý cao / thấp, đúng, hết lượt', () => {
  let s = G.guessNew(fixed(0.41)); // 42
  assert.equal(s.secret, 42);
  let r = G.guess(s, '10');
  assert.equal(r.hint, 'higher');
  r = G.guess(r.state, 80);
  assert.equal(r.hint, 'lower');
  assert.equal(r.state.turns, 2);
  r = G.guess(r.state, ' 42 ');
  assert.deepEqual([r.hint, r.state.done, r.state.won], ['correct', true, true]);

  // Nhập sai không mất lượt.
  s = G.guessNew(fixed(0.41));
  for (const bad of ['', 'abc', '0', '101', '4.5']) {
    const x = G.guess(s, bad);
    assert.equal(x.hint, 'invalid', bad);
    assert.equal(x.state.turns, 0);
  }

  // 7 lượt sai thì thua.
  s = G.guessNew(fixed(0.41));
  for (let i = 0; i < G.GUESS_TURNS; i++) s = G.guess(s, 1).state;
  assert.deepEqual([s.done, s.won, s.turns], [true, false, 7]);
  assert.equal(G.guess(s, 42).hint, 'invalid');
});

test('đoán số: phạm vi, số lượt, khoảng thu hẹp và lịch sử', () => {
  let s = G.guessNew(fixed(0.5), { min: 1, max: 20, turns: 5 });
  assert.equal(s.secret, 11);
  assert.deepEqual([s.min, s.max, s.maxTurns, s.low, s.high], [1, 20, 5, 1, 20]);
  assert.equal(G.guess(s, 21).hint, 'invalid'); // ngoài phạm vi 1–20
  assert.equal(G.guess(s, 0).hint, 'invalid');
  let r = G.guess(s, 5);
  assert.equal(r.hint, 'higher');
  assert.equal(r.state.low, 6);
  r = G.guess(r.state, 15);
  assert.equal(r.hint, 'lower');
  assert.deepEqual([r.state.low, r.state.high], [6, 14]);
  assert.deepEqual(r.state.history, [{ n: 5, hint: 'higher' }, { n: 15, hint: 'lower' }]);
  // Hết 5 lượt mà chưa đúng.
  for (let i = 0; i < 3; i++) r = G.guess(r.state, 6);
  assert.deepEqual([r.state.done, r.state.won, r.state.turns], [true, false, 5]);
  // Thông số lạ → mặc định 1–100, 7 lượt.
  const d = G.guessNew(fixed(0), { min: 5, max: 3, turns: -1 });
  assert.deepEqual([d.min, d.max, d.maxTurns], [5, 100, 7]);
});

test('đoán số: gợi ý theo độ khó', () => {
  assert.deepEqual(['easy', 'normal', 'hard', 'x'].map(G.hintsAllowed), [3, 1, 0, 1]);
  const s = G.guessNew(fixed(0.5), { min: 1, max: 20 }); // 11
  assert.deepEqual(G.guessHint(s, 0), { kind: 'parity', even: false });
  assert.deepEqual(G.guessHint(s, 1), { kind: 'range', from: 11, to: 20 });
  assert.deepEqual(G.guessHint(s, 2), { kind: 'digit', last: 1 });
  // Gợi ý khoảng dựa trên khoảng đã thu hẹp.
  const r = G.guess(s, 15).state; // high = 14
  assert.deepEqual(G.guessHint(r, 1), { kind: 'range', from: 8, to: 14 });
});

test('độ gần', () => {
  assert.equal(G.warmth(10, 11, 20), 'hot');
  assert.equal(G.warmth(10, 13, 20), 'warm');
  assert.equal(G.warmth(10, 18, 20), 'cold');
  assert.equal(G.warmth(50, 53), 'hot');
  assert.equal(G.warmth(50, 40), 'warm');
  assert.equal(G.warmth(50, 10), 'cold');
});

test('game của ngày: cố định trong ngày, đổi khi sang ngày mới', () => {
  const list = ['rps', 'guess', 'mines'];
  const morning = new Date(2026, 9, 16, 0, 5);
  const night = new Date(2026, 9, 16, 23, 55);
  assert.equal(G.dailyPick(morning, list), G.dailyPick(night, list));
  const days = [16, 17, 18].map((d) => G.dailyPick(new Date(2026, 9, d, 12), list));
  assert.deepEqual([...days].sort(), [...list].sort(), '3 ngày liền đi qua đủ 3 trò');
  assert.equal(G.dailyPick(morning, []), null);
});
