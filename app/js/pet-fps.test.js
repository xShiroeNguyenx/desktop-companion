// node --test app/js/  (npm test)
const test = require('node:test');
const assert = require('node:assert/strict');
const F = require('./pet-fps.js');

test('nhịp nghỉ khi chưa / lâu rồi không tương tác, nhịp nhanh ngay sau tương tác', () => {
  assert.equal(F.fpsAt(1000, undefined), F.IDLE_FPS);
  assert.equal(F.fpsAt(1000, NaN), F.IDLE_FPS);
  assert.equal(F.fpsAt(10_000, 10_000), F.ACTIVE_FPS);
  assert.equal(F.fpsAt(10_000 + F.ACTIVE_HOLD_MS - 1, 10_000), F.ACTIVE_FPS);
  assert.equal(F.fpsAt(10_000 + F.ACTIVE_HOLD_MS, 10_000), F.IDLE_FPS);
  // Đồng hồ lùi: không kẹt ở nhịp nhanh.
  assert.equal(F.fpsAt(5_000, 10_000), F.IDLE_FPS);
  assert.ok(F.IDLE_FPS <= 30, 'đứng yên không vẽ quá 30 khung/giây');
});

test('trộn biểu cảm theo thời gian, không theo số khung', () => {
  const run = (fps, secs) => {
    let v = 0;
    const dt = 1000 / fps;
    for (let i = 0; i < fps * secs; i++) v += (1 - v) * F.blendAlpha(0.08, dt);
    return v;
  };
  const at60 = run(60, 0.5);
  for (const fps of [30, 120, 144]) assert.ok(Math.abs(run(fps, 0.5) - at60) < 1e-9, `${fps} fps`);
  // Ở 60 khung/giây giữ đúng hệ số cũ.
  assert.ok(Math.abs(F.blendAlpha(0.08, 1000 / 60) - 0.08) < 1e-12);
});

test('hệ số trộn luôn trong [0, 1], kể cả khung rất dài hay đầu vào lạ', () => {
  for (const [p, d] of [[0.08, 0], [0.08, -5], [0.08, 1e9], [2, 16], [-1, 16], [NaN, 16], [0.08, NaN]]) {
    const a = F.blendAlpha(p, d);
    assert.ok(a >= 0 && a <= 1, `${p}, ${d} → ${a}`);
  }
  assert.equal(F.blendAlpha(0.08, 0), 0);
});
