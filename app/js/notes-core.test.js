// node --test app/js/  (chạy bằng `npm test`).
const test = require('node:test');
const assert = require('node:assert/strict');
const { fold, matches, sortNotes, visibleNotes } = require('./notes-core.js');

const note = (id, extra = {}) => ({ id, title: '', body: '', pinned: false, created_at: '', updated_at: '', ...extra });

test('tìm không dấu, không phân biệt hoa thường', () => {
  assert.equal(fold('Mật Khẩu Đường'), 'mat khau duong');
  const n = note('1', { title: 'Wifi văn phòng', body: 'Mật khẩu: abc' });
  assert.ok(matches(n, 'mat khau'));
  assert.ok(matches(n, 'WIFI'));
  assert.ok(matches(n, '  '));
});

test('mọi từ phải có, ở tiêu đề hoặc nội dung', () => {
  const n = note('1', { title: 'Số điện thoại', body: 'Lễ tân 1234' });
  assert.ok(matches(n, 'điện 1234'));
  assert.ok(!matches(n, 'điện 9999'));
});

test('ghim trước, rồi sửa gần nhất trước', () => {
  const list = [
    note('a', { updated_at: '100' }),
    note('b', { updated_at: '300' }),
    note('c', { updated_at: '50', pinned: true }),
    note('d', { created_at: '200' }), // file cũ chưa có updated_at
  ];
  assert.deepEqual(sortNotes(list).map((n) => n.id), ['c', 'b', 'd', 'a']);
  assert.deepEqual(list.map((n) => n.id), ['a', 'b', 'c', 'd'], 'không đổi mảng gốc');
});

test('visibleNotes lọc rồi sắp xếp', () => {
  const list = [
    note('a', { title: 'git', updated_at: '1' }),
    note('b', { title: 'Git lệnh hay dùng', updated_at: '2' }),
    note('c', { title: 'wifi', updated_at: '3' }),
  ];
  assert.deepEqual(visibleNotes(list, 'git').map((n) => n.id), ['b', 'a']);
  assert.deepEqual(visibleNotes(null, ''), []);
});
