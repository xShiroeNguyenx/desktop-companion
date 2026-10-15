// node --test app/js/  (chạy bằng `npm test`). Giờ địa phương lấy theo máy
// chạy test, nên mọi mốc đều dựng bằng new Date(y, m, d, h, mi) cho khớp.
const test = require('node:test');
const assert = require('node:assert/strict');
const { describeDue, toLocalInput, fromLocalInput, byDue } = require('./due.js');

const at = (d, h, mi = 0) => new Date(2026, 9, d, h, mi).getTime(); // tháng 10/2026

test('không có hạn', () => {
  for (const v of ['', null, undefined, '0', 'abc', -5]) assert.equal(describeDue(v, at(15, 9)), null);
});

test('các mức gấp', () => {
  const now = at(15, 9);
  assert.deepEqual(describeDue(at(15, 9, 20), now), { text: '⏰ Còn 20 phút', level: 'soon' });
  assert.deepEqual(describeDue(now + 10_000, now), { text: '⏰ Còn 1 phút', level: 'soon' });
  assert.deepEqual(describeDue(at(15, 17), now), { text: '⏰ Hôm nay 17:00', level: 'today' });
  assert.deepEqual(describeDue(at(16, 8, 5), now), { text: '📅 Mai 08:05', level: 'later' });
  assert.deepEqual(describeDue(at(20, 23, 59), now), { text: '📅 20/10 23:59', level: 'later' });
  assert.deepEqual(describeDue(at(15, 8), now), { text: '⚠️ Quá hạn (08:00)', level: 'over' });
  assert.deepEqual(describeDue(at(13, 8), now), { text: '⚠️ Quá hạn 2 ngày', level: 'over' });
  // Chuỗi millis (như lưu trong tasks.json) cũng được.
  assert.equal(describeDue(String(at(15, 17)), now).level, 'today');
});

test('datetime-local qua lại', () => {
  const ms = at(16, 17, 30);
  assert.equal(toLocalInput(ms), '2026-10-16T17:30');
  assert.equal(fromLocalInput('2026-10-16T17:30'), ms);
  assert.equal(toLocalInput(''), '');
  assert.equal(fromLocalInput(''), null);
  assert.equal(fromLocalInput('16/10/2026'), null);
});

test('sắp theo hạn, việc không hạn xuống cuối', () => {
  const list = [
    { id: 'a', due_at: '' },
    { id: 'b', due_at: String(at(20, 9)) },
    { id: 'c', due_at: String(at(16, 9)) },
    { id: 'd', due_at: '' },
  ];
  assert.deepEqual(list.slice().sort(byDue).map((t) => t.id), ['c', 'b', 'a', 'd']);
});
