// node --test app/js/  (npm test)
const test = require('node:test');
const assert = require('node:assert/strict');
const F = require('./fx-core.js');

const seq = () => { let k = 0; return () => ((k++ * 0.37) % 1); };

test('số hạt theo diện tích và cường độ, có trần', () => {
  const mid = F.particleCount('rain', 'medium', 1920, 1080);
  assert.equal(mid, F.BASE.rain);
  assert.ok(F.particleCount('rain', 'light', 1920, 1080) < mid);
  assert.ok(F.particleCount('rain', 'heavy', 1920, 1080) > mid);
  assert.equal(F.particleCount('rain', 'heavy', 3840, 2160), F.MAX.rain, '4K mức Dày chạm trần');
  assert.equal(F.particleCount('snow', 'medium', 0, 1080), 0);
  assert.equal(F.particleCount('lạ', 'medium', 1920, 1080), 0);
  assert.ok(F.particleCount('sakura', 'light', 200, 100) >= 1, 'màn bé vẫn có ít nhất 1 hạt');
});

test('hạt lúc bật rải khắp màn, hạt sinh sau nằm trên mép trên', () => {
  for (const kind of F.FALLING) {
    const r = seq();
    for (let i = 0; i < 40; i++) {
      const a = F.spawn(kind, 800, 600, r, true);
      assert.ok(a.x >= 0 && a.x <= 800 && a.y >= 0 && a.y <= 600, kind);
      const b = F.spawn(kind, 800, 600, r, false);
      assert.ok(b.y < 0 || b.x < 0, kind + ': sinh ngoài màn');
    }
  }
});

test('mưa rơi nhanh hơn tuyết, mọi hạt đều đi xuống', () => {
  const r = seq();
  const rain = F.spawn('rain', 800, 600, r, true);
  const snow = F.spawn('snow', 800, 600, r, true);
  assert.ok(rain.vy > snow.vy * 5);
  for (const kind of F.FALLING) {
    const p = F.spawn(kind, 800, 600, seq(), true);
    const y0 = p.y;
    F.step(p, 0.1, 1);
    assert.ok(p.y > y0, kind);
  }
});

test('sương trôi ngang, nắng và sao đứng yên, không bao giờ "hết"', () => {
  const fog = F.spawn('fog', 800, 600, seq(), false);
  assert.ok(fog.x < 0, 'mảng sương mới vào từ mép trái');
  assert.ok(!F.gone(fog, 800, 600), 'vừa sinh ngoài mép trái chưa tính là hết');
  const x0 = fog.x;
  F.step(fog, 1, 1);
  assert.ok(fog.x > x0 && fog.vy === 0);
  for (const kind of ['sun', 'stars']) {
    const p = F.spawn(kind, 800, 600, seq(), true);
    const at = [p.x, p.y];
    F.step(p, 5, 3);
    assert.deepEqual([p.x, p.y], at, kind);
    assert.ok(!F.gone(p, 800, 600), kind);
  }
  const star = F.spawn('stars', 800, 600, seq(), true);
  assert.ok(star.y <= 600 * 0.45, 'sao ở nửa trên');
  for (let t = 0; t < 10; t += 0.7) {
    const v = F.twinkle(star, t);
    assert.ok(v >= 0.25 && v <= 1);
  }
  assert.equal(F.particleCount('sun', 'heavy', 3840, 2160), 1);
});

test('hạt chạm mặt đất hoặc bay khỏi mép là hết', () => {
  assert.ok(F.gone({ x: 100, y: 501 }, 800, 500));
  assert.ok(F.gone({ x: 900, y: 10 }, 800, 500));
  assert.ok(!F.gone({ x: 100, y: 10 }, 800, 500));
});

test('tuyết đọng có trần và tan dần', () => {
  const g = F.groundNew(80);
  assert.equal(g.length, 10);
  for (let i = 0; i < 100; i++) F.groundAdd(g, 12, 1);
  assert.equal(g[1], F.GROUND_MAX);
  F.groundAdd(g, -5, 3);
  F.groundAdd(g, 999, 3);
  assert.equal(g[0], 0, 'ngoài bàn thì bỏ qua');
  F.groundMelt(g, 1000);
  assert.ok(g.every((v) => v === 0));
});

test('chớp: cách nhau ≥ 8 giây, sáng nhẹ rồi tắt hẳn', () => {
  for (const r of [() => 0, () => 0.5, () => 0.999]) {
    const at = F.nextFlash(100, r);
    assert.ok(at - 100 >= F.FLASH_GAP_MIN && at - 100 <= 20);
  }
  assert.equal(F.flashAlpha(0), F.FLASH_PEAK);
  assert.ok(F.FLASH_PEAK <= 0.2, 'không chớp loá');
  assert.ok(F.flashAlpha(0.2) < F.FLASH_PEAK);
  assert.equal(F.flashAlpha(1), 0);
  assert.equal(F.flashAlpha(-1), 0);
});

test('lỗ: không vẽ đè lên bảng của app', () => {
  const holes = [{ x: 100, y: 100, w: 200, h: 100 }];
  assert.ok(F.inHoles(holes, 150, 150));
  assert.ok(!F.inHoles(holes, 300, 150), 'mép phải không tính');
  assert.ok(!F.inHoles([], 150, 150));
  assert.ok(!F.inHoles(null, 150, 150));
});
