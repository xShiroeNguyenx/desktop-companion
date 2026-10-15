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

test('độ gần', () => {
  assert.equal(G.warmth(50, 53), 'hot');
  assert.equal(G.warmth(50, 40), 'warm');
  assert.equal(G.warmth(50, 10), 'cold');
});
