// node --test app/js/  (npm test)
const test = require('node:test');
const assert = require('node:assert/strict');
const { splitLinks } = require('./linkify.js');

const link = (value) => ({ type: 'link', value });
const text = (value) => ({ type: 'text', value });

test('không có link', () => {
  assert.deepEqual(splitLinks('Mua sữa'), [text('Mua sữa')]);
  assert.deepEqual(splitLinks(''), []);
  assert.deepEqual(splitLinks(null), []);
});

test('link giữa câu và ở dòng riêng', () => {
  assert.deepEqual(splitLinks('Xem https://example.com/account/tasks/123 nha'), [
    text('Xem '),
    link('https://example.com/account/tasks/123'),
    text(' nha'),
  ]);
  assert.deepEqual(splitLinks('Sửa lỗi\nhttp://example.net/a?b=1&c=2#x'), [
    text('Sửa lỗi\n'),
    link('http://example.net/a?b=1&c=2#x'),
  ]);
});

test('nhiều link, dấu câu cuối không thuộc link', () => {
  assert.deepEqual(splitLinks('A: https://example.com/a, B: https://example.com/b.'), [
    text('A: '),
    link('https://example.com/a'),
    text(', B: '),
    link('https://example.com/b'),
    text('.'),
  ]);
  assert.deepEqual(splitLinks('(xem https://example.com/x)'), [
    text('(xem '),
    link('https://example.com/x'),
    text(')'),
  ]);
  // Ngoặc là một phần của link.
  assert.deepEqual(splitLinks('https://example.com/wiki/A_(b)'), [link('https://example.com/wiki/A_(b)')]);
});

test('không nhận giao thức khác hoặc link rỗng', () => {
  assert.deepEqual(splitLinks('javascript:alert(1)'), [text('javascript:alert(1)')]);
  assert.deepEqual(splitLinks('file:///C:/x.exe'), [text('file:///C:/x.exe')]);
  assert.deepEqual(splitLinks('https:// rỗng'), [text('https:// rỗng')]);
});
