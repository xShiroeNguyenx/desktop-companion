// node --test app/js/  (npm test)
const test = require('node:test');
const assert = require('node:assert/strict');
const L = require('./launcher-core.js');

const BM = [
  { title: 'Tài liệu API', url: 'https://example.com/docs', group: 'Công việc' },
  { title: 'Bảng Jira', url: 'https://example.atlassian.net/board', group: 'Công việc' },
  { title: 'Nhạc', url: 'https://example.net/music', group: 'Giải trí' },
];

test('bang mặc định, tiếng Việt được mã hoá', () => {
  const r = L.results('!yt mèo con', { engine: 'google' });
  assert.equal(r.length, 1);
  assert.equal(r[0].type, 'bang');
  assert.equal(r[0].url, 'https://www.youtube.com/results?search_query=m%C3%A8o%20con');
  assert.equal(L.results('!YT a&b', {})[0].url, 'https://www.youtube.com/results?search_query=a%26b');
});

test('bang tự thêm thắng bang mặc định trùng tên, bang lạ thành tìm thường', () => {
  const data = { bangs: [['!jira', 'https://example.atlassian.net/search?q={q}'], ['!g', 'https://example.org/s?q={q}']] };
  assert.equal(L.results('!jira PROJ-12', data)[0].url, 'https://example.atlassian.net/search?q=PROJ-12');
  assert.equal(L.results('!g x', data)[0].url, 'https://example.org/s?q=x');
  const r = L.results('!khongco abc', { engine: 'bing' });
  assert.equal(r[r.length - 1].type, 'search');
  assert.equal(r[r.length - 1].url, 'https://www.bing.com/search?q=!khongco%20abc');
});

test('đang gõ tên bang thì gợi ý', () => {
  const r = L.results('!g', {});
  assert.ok(r.every((x) => x.type === 'hint'));
  assert.deepEqual(r.map((x) => x.insert), ['!g ', '!gh ']);
  // Gõ xong tên + dấu cách mà chưa có từ khoá: mở trang chính của bang.
  assert.equal(L.results('!yt ', {})[0].url, 'https://www.youtube.com/results');
});

test('template độc bị chặn', () => {
  assert.equal(L.fill('javascript:alert({q})', 'x'), null);
  assert.equal(L.fill('file:///C:/{q}', 'x'), null);
  assert.deepEqual(L.results('!bad x', { bangs: [['!bad', 'javascript:alert({q})']] }), []);
});

test('nhận ra địa chỉ web', () => {
  assert.equal(L.toUrl('example.com/docs'), 'https://example.com/docs');
  assert.equal(L.toUrl('app-01.example.net:8443'), 'https://app-01.example.net:8443');
  assert.equal(L.toUrl('http://example.com'), 'http://example.com');
  assert.equal(L.toUrl('hello world'), null);
  assert.equal(L.toUrl('1.5'), null, 'số thập phân không phải tên miền');
  assert.equal(L.toUrl('javascript:alert(1)'), null);
  assert.equal(L.results('example.com', {})[0].type, 'url');
});

test('bookmark: không dấu, mọi từ phải khớp, tên trước nhóm / URL', () => {
  assert.deepEqual(L.rank(BM, 'tai lieu').map((b) => b.title), ['Tài liệu API']);
  assert.deepEqual(L.rank(BM, 'cong viec').map((b) => b.title), ['Bảng Jira', 'Tài liệu API']);
  assert.deepEqual(L.rank(BM, 'jira tai'), []);
  const r = L.results('jira', { bookmarks: BM, engine: 'google' });
  assert.deepEqual(r.map((x) => x.type), ['bookmark', 'search']);
  assert.equal(r[1].url, 'https://www.google.com/search?q=jira');
  assert.deepEqual(L.results('   ', { bookmarks: BM }), []);
});
