// node --test app/js/  (npm test)
const test = require('node:test');
const assert = require('node:assert/strict');
const M = require('./monitor-core.js');

test('mọi ngôn ngữ có đủ chữ', () => {
  const keys = Object.keys(M.T.vi).sort();
  const reasons = Object.keys(M.T.vi.reasons).sort();
  for (const l of ['ja', 'en']) {
    assert.deepEqual(Object.keys(M.T[l]).sort(), keys, l);
    assert.deepEqual(Object.keys(M.T[l].reasons).sort(), reasons, l);
  }
});

test('chữ kết quả', () => {
  assert.equal(M.fmtMs(85.4), '85 ms');
  assert.equal(M.fmtMs(1234), '1.2 s');
  assert.equal(M.fmtMs(null), '');
  assert.equal(M.resultText({ state: 'up', ms: 120, status: 200 }, 'vi'), '120 ms');
  assert.equal(M.resultText({ state: 'down', reason: 'http502' }, 'vi'), '502');
  assert.equal(M.resultText({ state: 'down', reason: 'timeout' }, 'vi'), 'Hết giờ');
  assert.equal(M.resultText({ state: 'down', reason: 'dns' }, 'en'), 'DNS error');
  assert.equal(M.resultText({ state: 'down', reason: 'lạ' }, 'vi'), 'Lỗi kết nối');
  assert.equal(M.resultText(null, 'vi'), 'đang kiểm…');
});

test('chấm màu và tóm tắt', () => {
  const up = { up: true, last: { state: 'up', ms: 50 } };
  const down = { up: false, last: { state: 'down', reason: 'http500' } };
  // Lỗi một lần, Rust chưa kết luận sập: vàng, vẫn tính là đang chạy.
  const once = { up: true, last: { state: 'down', reason: 'timeout' } };
  const fresh = { up: null, last: null };
  assert.deepEqual([up, down, once, fresh].map(M.dot), ['up', 'down', 'warn', 'wait']);
  assert.equal(M.summary([up, down, once, fresh], 'vi'), '3/4 đang chạy');
  assert.equal(M.summary([], 'en'), '0/0 up');
});
