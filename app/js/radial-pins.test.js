const test = require('node:test');
const assert = require('node:assert/strict');
const P = require('./radial-pins.js');

function memStorage(initial) {
  const m = new Map(initial ? [[P.KEY, initial]] : []);
  return { getItem: (k) => (m.has(k) ? m.get(k) : null), setItem: (k, v) => m.set(k, String(v)) };
}

test('ghim / gỡ / đầy', () => {
  let r = P.toggle([], 'open-notes');
  assert.deepEqual(r, { pins: ['open-notes'], result: 'pinned' });
  r = P.toggle(r.pins, 'toggle-focus');
  assert.deepEqual(r.pins, ['open-notes', 'toggle-focus']);
  assert.deepEqual(P.toggle(r.pins, 'show-games'), { pins: r.pins, result: 'full' });
  assert.deepEqual(P.toggle(r.pins, 'open-notes'), { pins: ['toggle-focus'], result: 'unpinned' });
});

test('tìm mục của ghim theo thứ tự ghim, bỏ ghim không còn', () => {
  const groups = [
    { items: [{ action: 'a', icon: 'A' }, { action: 'b', icon: 'B' }] },
    { items: [{ action: 'c', icon: 'C' }] },
  ];
  assert.deepEqual(P.resolve(['c', 'gone', 'a'], groups).map((i) => i.icon), ['C', 'A']);
});

test('đọc storage hỏng hoặc bị chặn thì rỗng', () => {
  assert.deepEqual(P.load(memStorage()), []);
  assert.deepEqual(P.load(memStorage('{bad')), []);
  assert.deepEqual(P.load(memStorage('{"a":1}')), []);
  assert.deepEqual(P.load(memStorage('["x","x",3,"y","z"]')), ['x', 'y']);
  const blocked = { getItem() { throw new Error('denied'); }, setItem() { throw new Error('denied'); } };
  assert.deepEqual(P.load(blocked), []);
  P.save(blocked, ['x']); // không ném lỗi
  const s = memStorage();
  P.save(s, ['x']);
  assert.deepEqual(P.load(s), ['x']);
});
