// node --test app/js/  (chạy bằng `npm test`).
const test = require('node:test');
const assert = require('node:assert/strict');
const W = require('./week-stats.js');

const day = (focusMin, extra = {}) => ({ tasks: 0, sessions: 0, focusMin, quests: false, ...extra });

test('minutesText', () => {
  assert.equal(W.minutesText(0), '0 phút');
  assert.equal(W.minutesText(45), '45 phút');
  assert.equal(W.minutesText(60), '1 giờ');
  assert.equal(W.minutesText(135), '2 giờ 15 phút');
  assert.equal(W.minutesText(undefined), '0 phút');
});

test('summary hides the streak when it is broken', () => {
  const week = { total: { tasks: 5, focusMin: 75 }, streak: 4 };
  assert.equal(W.summary(week), '✅ 5 việc · 🍅 1 giờ 15 phút · 🔥 4 ngày liền');
  assert.equal(W.summary({ ...week, streak: 0 }), '✅ 5 việc · 🍅 1 giờ 15 phút');
  assert.equal(W.summary(null), '✅ 0 việc · 🍅 0 phút');
});

test('delta against last week', () => {
  assert.equal(W.delta(5, 3), '+2');
  assert.equal(W.delta(3, 5), '−2');
  assert.equal(W.delta(4, 4), 'bằng');
  assert.equal(W.delta(50, 120, 'min'), '−1 giờ 10 phút');
});

test('bars scale to the busiest day, at least 25 minutes', () => {
  assert.deepEqual(W.bars([day(0), day(10), day(25)]), [0, 40, 100]);
  assert.deepEqual(W.bars([day(100), day(50), day(2)]), [100, 50, 8]);
  assert.deepEqual(W.bars([]), []);
});

test('footer and day title', () => {
  const week = { total: { tasks: 6, focusMin: 100, questDays: 3 }, last: { tasks: 4, focusMin: 100 } };
  assert.equal(W.footer(week), 'So với tuần trước: việc +2 · tập trung bằng · 📋 xong nhiệm vụ 3/7 ngày');
  assert.equal(W.dayTitle(day(50, { tasks: 2, sessions: 2, quests: true }), 6), 'CN: 2 việc · 2 phiên 🍅 (50 phút) · xong cả 3 nhiệm vụ');
});

test('theo ngôn ngữ', () => {
  const week = { total: { tasks: 1, focusMin: 135, questDays: 2 }, last: { tasks: 1, focusMin: 100 }, streak: 3 };
  assert.equal(W.summary(week, 'ja'), '✅ 1件 · 🍅 2時間15分 · 🔥 3日連続');
  assert.equal(W.summary(week, 'en'), '✅ 1 task · 🍅 2 h 15 min · 🔥 3-day streak');
  assert.equal(W.footer(week, 'en'), 'Vs last week: tasks same · focus +35 min · 📋 quests done 2/7 days');
  assert.equal(W.dayTitle(day(25, { tasks: 2, sessions: 1 }), 0, 'en'), 'Mon: 2 tasks · 1 🍅 session (25 min)');
  assert.deepEqual(W.dayLabels('ja').length, 7);
  assert.equal(W.minutesText(45, 'xx'), '45 phút');
});
