// node --test app/js/  (npm test)
const test = require('node:test');
const assert = require('node:assert/strict');
const S = require('./sysmon-core.js');

test('mọi ngôn ngữ có đủ chữ', () => {
  const keys = Object.keys(S.T.vi).sort();
  for (const l of ['ja', 'en']) assert.deepEqual(Object.keys(S.T[l]).sort(), keys, l);
});

test('định dạng byte và tốc độ', () => {
  assert.equal(S.fmtBytes(0), '0 B');
  assert.equal(S.fmtBytes(512), '512 B');
  assert.equal(S.fmtBytes(1536), '1.5 KB');
  assert.equal(S.fmtBytes(1024 * 1024), '1 MB');
  assert.equal(S.fmtBytes(9.84 * 1024 ** 3), '9.8 GB');
  assert.equal(S.fmtBytes(476 * 1024 ** 3), '476 GB');
  assert.equal(S.fmtBytes(2 * 1024 ** 4), '2 TB');
  assert.equal(S.fmtBytes(-1), '–');
  assert.equal(S.fmtBytes('x'), '–');
  assert.equal(S.fmtRate(120 * 1024), '120 KB/s');
});

test('% CPU của ứng dụng', () => {
  assert.equal(S.fmtPct(null), '…');
  assert.equal(S.fmtPct(0), '0%');
  assert.equal(S.fmtPct(0.44), '0.4%');
  assert.equal(S.fmtPct(9.96), '10%');
  assert.equal(S.fmtPct(37.6), '38%');
});

test('thời gian máy đã chạy', () => {
  assert.equal(S.fmtUptime(8 * 60 + 30, 'vi'), '8 phút');
  assert.equal(S.fmtUptime(5 * 3600 + 12 * 60, 'vi'), '5 giờ 12 phút');
  assert.equal(S.fmtUptime(3 * 86400 + 4 * 3600 + 59, 'vi'), '3 ngày 4 giờ');
  assert.equal(S.fmtUptime(3 * 86400 + 4 * 3600, 'en'), '3d 4h');
  assert.equal(S.fmtUptime(3600, 'ja'), '1時間0分');
  assert.equal(S.fmtUptime(null, 'vi'), '0 phút');
});

test('mức màu và mặt Miu', () => {
  assert.deepEqual([null, 10, 69, 70, 89, 90, 100].map(S.level), ['', 'ok', 'ok', 'warn', 'warn', 'hot', 'hot']);
  assert.equal(S.face({ cpu: 95, ram: { pct: 40 } }), 'surprised');
  assert.equal(S.face({ cpu: 20, ram: { pct: 92 } }), 'surprised', 'RAM đầy cũng tính');
  assert.equal(S.face({ cpu: 75, ram: { pct: 40 } }), 'fight');
  assert.equal(S.face({ cpu: 3, ram: { pct: 40 } }), 'sleepy');
  assert.equal(S.face({ cpu: 30, ram: { pct: 40 } }), 'wink');
  assert.equal(S.face({ cpu: null }), 'wink', 'lần đo đầu chưa có CPU');
  assert.equal(S.face(null), 'wink');
});

test('biểu đồ CPU: điểm mới nhất ở mép phải', () => {
  assert.equal(S.sparkPoints([], 100, 20, 30), '');
  assert.equal(S.sparkPoints([0, 100], 100, 20, 2), '0,20 100,0');
  // 2 điểm trên khung 3 chỗ: dồn sang phải.
  assert.equal(S.sparkPoints([50, 50], 100, 20, 3), '50,10 100,10');
  assert.equal(S.sparkPoints([150, -5], 100, 20, 2), '0,0 100,20', 'kẹp 0..100');
});
