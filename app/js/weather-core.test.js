// node --test app/js/  (npm test)
const test = require('node:test');
const assert = require('node:assert/strict');
const W = require('./weather-core.js');

test('mỗi ngôn ngữ có đủ chữ như tiếng Việt', () => {
  const keys = Object.keys(W.T.vi).sort();
  for (const l of ['ja', 'en']) assert.deepEqual(Object.keys(W.T[l]).sort(), keys, l);
  for (const l of ['vi', 'ja', 'en']) assert.equal(W.T[l].days.length, 7);
});

test('mã WMO → mô tả và icon, ngày / đêm', () => {
  assert.equal(W.describe(0, 'vi'), 'Trời quang');
  assert.equal(W.describe(80, 'en'), 'Rain showers');
  assert.equal(W.describe(95, 'ja'), '雷雨');
  assert.equal(W.describe(12345, 'vi'), 'Không rõ');
  assert.equal(W.describe(61, 'xx'), 'Mưa nhẹ', 'ngôn ngữ lạ → tiếng Việt');
  assert.equal(W.icon(0, true), '☀️');
  assert.equal(W.icon(0, false), '🌙');
  assert.equal(W.icon(95, true), '⛈️');
  for (const code of [0, 1, 2, 3, 45, 51, 61, 63, 65, 66, 71, 73, 75, 80, 82, 95, 999]) {
    assert.ok(W.icon(code, true), String(code));
    for (const l of ['vi', 'ja', 'en']) assert.ok(W.describe(code, l), `${code}/${l}`);
  }
});

test('°C ↔ °F', () => {
  assert.equal(W.temp(31.6, 'c'), 32);
  assert.equal(W.temp(0, 'f'), 32);
  assert.equal(W.temp(100, 'f'), 212);
  assert.equal(W.temp(-3.6, 'c'), -4);
});

test('cập nhật bao lâu trước', () => {
  assert.equal(W.ago(0, 1000, 'vi'), 'Chưa có dữ liệu');
  assert.equal(W.ago(1000, 1030, 'vi'), 'Vừa cập nhật');
  assert.equal(W.ago(1000, 1000 + 5 * 60, 'vi'), 'Cập nhật 5 phút trước');
  assert.equal(W.ago(1000, 1000 + 2 * 3600, 'en'), 'Updated 2 h ago');
});

test('nhãn ngày và giờ', () => {
  assert.equal(W.dayName('2026-10-16', 0, 'vi'), 'Hôm nay');
  assert.equal(W.dayName('2026-10-17', 1, 'vi'), 'T7 17');
  assert.equal(W.dayName('2026-10-18', 2, 'en'), 'Sun 18');
  assert.equal(W.dayName('2026-10-19', 3, 'ja'), '19(月)');
  assert.equal(W.dayName('rác', 3, 'vi'), '');
  assert.equal(W.hourLabel('2026-10-16T15:00', 'en'), '3 PM');
  assert.equal(W.hourLabel('2026-10-16T00:00', 'en'), '12 AM');
  assert.equal(W.hourLabel('2026-10-16T12:00', 'en'), '12 PM');
  assert.equal(W.hourLabel('2026-10-16T08:00', 'vi'), '08:00');
  assert.equal(W.clock('2026-10-16T14:15'), '14:15');
});

test('biểu đồ: điểm nằm trong khung, nóng nhất ở cao nhất', () => {
  const hours = [28, 30, 33, 31, 27, 26].map((t, i) => ({ time: `2026-10-16T${10 + i}:00`, temp: t, rain: i * 10 }));
  const c = W.chart(hours, 300, 120, { every: 2 });
  assert.equal(c.points.length, 6);
  assert.equal(c.points[0].x, 0);
  assert.equal(c.points[5].x, 300);
  const top = c.points.reduce((a, b) => (b.y < a.y ? b : a));
  assert.equal(top.hour.temp, 33);
  for (const p of c.points) assert.ok(p.y >= 22 && p.y <= 120 - 30);
  assert.ok(c.line.startsWith('M0,') && c.area.endsWith('Z'));
  assert.deepEqual(c.marks.map((m) => m.hour.temp), [28, 33, 27]);
  assert.deepEqual(W.chart([{ temp: 1 }], 300, 120).points, [], 'một điểm thì không vẽ');
  // Nhiệt độ phẳng: không chia cho 0.
  const flat = W.chart([{ temp: 20 }, { temp: 20 }], 100, 100);
  assert.ok(flat.points.every((p) => Number.isFinite(p.y)));
});

test('biểu cảm Miu theo trời', () => {
  assert.equal(W.face(0, true, false), 'cheer');
  assert.equal(W.face(1, true, false), 'cheer');
  assert.equal(W.face(3, true, false), 'wink');
  assert.equal(W.face(63, true, false), 'wink');
  assert.equal(W.face(95, true, false), 'surprised');
  assert.equal(W.face(95, false, false), 'surprised');
  assert.equal(W.face(0, false, false), 'sleepy');
  assert.equal(W.face(0, true, true), 'shy');
});
