// node --test app/js/  (npm test)
const test = require('node:test');
const assert = require('node:assert/strict');
const K = require('./clock-core.js');

// Mốc cố định (UTC) để không phụ thuộc giờ máy chạy test.
const at = (iso) => Date.parse(iso);

test('mỗi ngôn ngữ có đủ chữ như tiếng Việt', () => {
  const keys = Object.keys(K.T.vi).sort();
  for (const l of ['ja', 'en']) assert.deepEqual(Object.keys(K.T[l]).sort(), keys, l);
  for (const l of ['vi', 'ja', 'en']) assert.equal(K.T[l].weekdays.length, 7);
});

test('danh sách múi giờ: múi nào cũng dùng được với Intl, tên đủ 3 thứ tiếng', () => {
  for (const [zone, vi, ja, en] of K.ZONES) {
    assert.ok(K.partsIn(at('2026-10-16T00:00:00Z'), zone), zone);
    assert.ok(vi && ja && en, zone);
  }
  assert.equal(new Set(K.ZONES.map((z) => z[0])).size, K.ZONES.length, 'không trùng múi');
});

test('giờ ở một múi', () => {
  const ms = at('2026-10-16T07:32:00Z');
  assert.equal(K.hhmm(K.partsIn(ms, 'Asia/Ho_Chi_Minh')), '14:32');
  assert.equal(K.hhmm(K.partsIn(ms, 'Asia/Tokyo')), '16:32');
  assert.equal(K.hhmm(K.partsIn(ms, 'Asia/Kolkata')), '13:02');
  assert.equal(K.hhmm(K.partsIn(at('2026-10-16T00:05:00Z'), 'UTC')), '00:05');
  assert.equal(K.partsIn(ms, 'Không/Có'), null);
});

test('giờ mùa hè do Intl tự tính', () => {
  // New York: tháng 7 là EDT (−4), tháng 1 là EST (−5).
  assert.equal(K.partsIn(at('2026-07-01T12:00:00Z'), 'America/New_York').hour, 8);
  assert.equal(K.partsIn(at('2026-01-15T12:00:00Z'), 'America/New_York').hour, 7);
});

test('so với giờ máy: chênh giờ và lệch ngày', () => {
  const ms = at('2026-10-16T16:30:00Z');
  // Giả lập máy ở Việt Nam: 23:30 ngày 16.
  const local = K.partsIn(ms, 'Asia/Ho_Chi_Minh');
  const tokyo = K.compare(ms, 'Asia/Tokyo', local);
  assert.equal(tokyo.diffMin, 120);
  assert.equal(tokyo.dayShift, 1);
  assert.equal(K.hhmm(tokyo.parts), '01:30');
  const ny = K.compare(ms, 'America/New_York', local);
  assert.equal(ny.diffMin, -11 * 60);
  assert.equal(ny.dayShift, 0);
  assert.equal(K.compare(ms, 'Asia/Ho_Chi_Minh', local).diffMin, 0);
  assert.equal(K.compare(ms, 'Không/Có', local), null);
  // Sáng sớm ở Việt Nam thì New York còn hôm qua.
  const early = at('2026-10-16T00:30:00Z');
  assert.equal(K.compare(early, 'America/New_York', K.partsIn(early, 'Asia/Ho_Chi_Minh')).dayShift, -1);
});

test('nhãn chênh giờ và tên múi', () => {
  assert.equal(K.diffLabel(120), '+2h');
  assert.equal(K.diffLabel(-660), '−11h');
  assert.equal(K.diffLabel(-90), '−1h30');
  assert.equal(K.diffLabel(0), '');
  assert.equal(K.zoneName('Asia/Tokyo', 'ja'), '東京');
  assert.equal(K.zoneName('Asia/Ho_Chi_Minh', 'vi'), 'Việt Nam');
  assert.equal(K.zoneName('America/Sao_Paulo', 'en'), 'Sao Paulo');
});

test('đếm lùi', () => {
  assert.equal(K.countdown(1122), '18:42');
  assert.equal(K.countdown(59), '0:59');
  assert.equal(K.countdown(3900), '1:05:00');
  assert.equal(K.countdown(-5), '0:00');
  assert.equal(K.countdown('x'), '0:00');
});

test('biểu cảm Miu trên đồng hồ', () => {
  assert.equal(K.face('focus', 23), 'fight');
  assert.equal(K.face('idle', 23), 'sleepy');
  assert.equal(K.face('idle', 4), 'sleepy');
  assert.equal(K.face('idle', 5), 'wink');
  assert.equal(K.face('idle', 22), 'wink');
  assert.equal(K.face('break', 2), 'wink');
});
