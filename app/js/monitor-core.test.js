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

test('site cần đăng nhập: 401 / 403 hiện 🔐, gợi ý bật khi đang tính là sập', () => {
  assert.equal(M.resultText({ state: 'up', ms: 90, status: 401 }, 'vi'), '🔐 90 ms');
  assert.equal(M.resultText({ state: 'up', ms: 90, status: 403 }, 'en'), '🔐 90 ms');
  assert.equal(M.needsLogin({ state: 'down', reason: 'http401' }), true);
  assert.equal(M.needsLogin({ state: 'down', reason: 'http403' }), true);
  for (const o of [
    { state: 'down', reason: 'http404' },
    { state: 'down', reason: 'http500' },
    { state: 'down', reason: 'timeout' },
    { state: 'up', ms: 90, status: 401 },
    null,
  ]) {
    assert.equal(M.needsLogin(o), false, JSON.stringify(o));
  }
});

test('Cài đặt và Rust nối cờ 🔐 login', () => {
  const fs = require('node:fs');
  const path = require('node:path');
  const js = fs.readFileSync(path.join(__dirname, 'settings.js'), 'utf8');
  assert.match(js, /invoke\('monitor_add', \{[^}]*login: \$\('monLogin'\)\.checked/);
  assert.match(js, /invoke\('monitor_test', \{[^}]*login: \$\('monLogin'\)\.checked/);
  assert.match(js, /login: !s\.login/);
  const html = fs.readFileSync(path.join(__dirname, '../settings.html'), 'utf8');
  assert.match(html, /id="monLogin"/);
  const rs = fs.readFileSync(path.join(__dirname, '../../src-tauri/src/commands.rs'), 'utf8');
  assert.match(rs, /pub fn monitor_add\([^)]*login: Option<bool>/);
  assert.match(rs, /pub async fn monitor_test\([^)]*login: Option<bool>/);
  const net = fs.readFileSync(path.join(__dirname, '../../src-tauri/src/monitor_net.rs'), 'utf8');
  assert.match(net, /"login": s\.login/);
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
