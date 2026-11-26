// node --test app/js/  (npm test)
const test = require('node:test');
const assert = require('node:assert/strict');
const A = require('./wallpaper-art.js');

test('mỗi hình nền có tên đủ 3 thứ tiếng, id dùng được làm tên file', () => {
  const ids = new Set();
  for (const [id, vi, ja, en] of A.THEMES) {
    assert.match(id, /^[a-z0-9-]+$/);
    assert.ok(vi && ja && en, id);
    assert.ok(!ids.has(id), 'trùng ' + id);
    ids.add(id);
  }
  assert.equal(A.themeName('night', 'ja'), '星空');
  assert.equal(A.themeName('night', 'xx'), 'Đêm sao', 'ngôn ngữ lạ → tiếng Việt');
});

test('số ngẫu nhiên có hạt giống: lặp lại y hệt, nằm trong [0, 1)', () => {
  const a = A.rng(42), b = A.rng(42);
  for (let i = 0; i < 1000; i++) {
    const x = a();
    assert.equal(x, b());
    assert.ok(x >= 0 && x < 1);
  }
  assert.notEqual(A.rng(1)(), A.rng(2)());
});

test('cảnh vẽ lại y hệt và mọi điểm nằm trong khung', () => {
  for (const [id] of A.THEMES) {
    const s1 = A.scene(id, 1920, 1080);
    assert.deepEqual(s1, A.scene(id, 1920, 1080), id);
    assert.equal(s1.ops[0].t, 'grad', id + ': lớp nền phủ kín trước');
    for (const op of s1.ops) {
      for (const k of ['x', 'y', 'r', 'rx', 'ry', 'alpha']) {
        if (k in op) assert.ok(Number.isFinite(op[k]), `${id} ${op.t}.${k}`);
      }
      if (op.alpha !== undefined) assert.ok(op.alpha >= 0 && op.alpha <= 1, `${id} alpha`);
      if (op.pts) {
        assert.equal(op.pts[0][0], 0, id + ': đường bắt đầu ở mép trái');
        assert.ok(op.pts.at(-1)[0] >= 1919.9, id + ': đường đi hết bề ngang');
        for (const [x, y] of op.pts) assert.ok(Number.isFinite(x) && Number.isFinite(y));
      }
    }
  }
});

test('ảnh nhỏ (xem trước) và ảnh lớn cùng một bố cục', () => {
  const small = A.scene('night', 192, 108);
  const big = A.scene('night', 1920, 1080);
  assert.equal(small.ops.length, big.ops.length);
  const moon = (s) => s.ops.find((o) => o.t === 'circle' && o.color === '#f6f1d5');
  assert.ok(Math.abs(moon(big).x / 10 - moon(small).x) < 1e-9);
});
