// node --test app/js/  (npm test)
// aquarium-draw.js chỉ chạy trong WebView (canvas thật). Ở đây vẽ lên một canvas
// giả ghi lại mọi lệnh, để bắt lỗi chạy (biến chưa có, nền thiếu màu cho một giờ,
// vật trang trí chưa có hàm vẽ…) mà test cú pháp không thấy. Không kiểm hình đẹp.
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');

function fakeCtx(log) {
  const gradient = { addColorStop: () => {} };
  return new Proxy(
    {},
    {
      get(_, prop) {
        if (prop === 'createLinearGradient' || prop === 'createRadialGradient') return () => gradient;
        return (...args) => {
          for (const a of args) assert.ok(typeof a !== 'number' || Number.isFinite(a), `${String(prop)}(${args.join(', ')})`);
          log.push(prop);
        };
      },
      set(_, prop, value) {
        if (typeof value === 'number') assert.ok(Number.isFinite(value), `${String(prop)} = ${value}`);
        if (/style|Color/.test(String(prop))) assert.ok(value, `${String(prop)} rỗng`);
        return true;
      },
    },
  );
}

function load(withImage) {
  const sandbox = { Math, Number, String, Array, Object, Map, Set, console };
  if (withImage) {
    // Image giả đã nạp xong (WebView có Image thật; node thì không).
    sandbox.Image = class {
      constructor() {
        sandbox.Image.made = (sandbox.Image.made || 0) + 1;
        this.complete = true;
        this.naturalWidth = 256;
        this.naturalHeight = 143;
      }
      addEventListener() {}
    };
  }
  sandbox.window = sandbox;
  sandbox.globalThis = sandbox;
  vm.createContext(sandbox);
  for (const f of ['aquarium-core.js', 'aquarium-draw.js']) {
    vm.runInContext(fs.readFileSync(path.join(__dirname, f), 'utf8'), sandbox, { filename: f });
  }
  return { A: sandbox.AquariumCore, D: sandbox.AquariumDraw, sandbox };
}

test('vẽ được mọi nền × giờ × trang trí × loài mà không lỗi', () => {
  const { A, D } = load();
  const bounds = { w: 276, h: 126, sand: D.SAND };
  const scene = D.makeScene(bounds);
  const rand = A.rng(3);
  const sim = A.syncSim([], A.SPECIES.map((species, i) => ({ uid: i + 1, species, stage: i % 3 })), bounds, rand);
  const food = A.dropFood([], 100, bounds, rand, 5);
  const decorSets = [[], ['castle', 'chest', 'coral', 'bubbler'], ['', 'bubbler', '', 'lạ']];
  for (const bg of [...Object.keys(D.BG), 'không-có']) {
    for (const mode of ['day', 'dusk', 'night']) {
      for (const decor of decorSets) {
        const log = [];
        D.stepBubbles(scene, 0.5, bounds, decor, rand, false);
        D.drawTank(fakeCtx(log), { bounds, scene, bg, mode, clock: 12.3, reduced: false, decor, food, sim });
        assert.ok(log.length > 50, `${bg}/${mode}: có vẽ`);
      }
    }
  }
});

test('mỗi vật trang trí có hàm vẽ; máy sủi bọt làm ra bọt', () => {
  const { A, D } = load();
  for (const k of ['castle', 'chest', 'coral', 'bubbler']) assert.equal(typeof D.DECOR[k], 'function', k);
  const bounds = { w: 276, h: 126, sand: D.SAND };
  const scene = D.makeScene(bounds);
  const rand = A.rng(9);
  for (let i = 0; i < 100; i++) D.stepBubbles(scene, 0.1, bounds, ['', '', 'bubbler', ''], rand, false);
  assert.ok(scene.bubbles.length > 0);
  // Reduced motion: không có bọt.
  D.stepBubbles(scene, 0.1, bounds, ['bubbler'], rand, true);
  assert.equal(scene.bubbles.length, 0);
});

test('ảnh tĩnh cho thẻ: cá, nền, trang trí', () => {
  const { A, D } = load();
  const log = [];
  const canvas = { clientWidth: 150, clientHeight: 96, width: 0, height: 0, getContext: () => fakeCtx(log) };
  for (const sp of A.SPECIES) D.drawIcon(canvas, 'fish', sp);
  for (const bg of Object.keys(D.BG)) D.drawIcon(canvas, 'bg', bg);
  for (const d of Object.keys(D.DECOR)) D.drawIcon(canvas, 'decor', d);
  assert.ok(canvas.width > 0 && log.length > 100);
});

test('mỗi ảnh cá: có file, đúng loài, PNG ≤ 512 px và ≤ 80 KB, điểm nối hợp lệ', () => {
  const { A, D } = load();
  assert.ok(Object.keys(D.SPRITES).length > 0);
  for (const [species, m] of Object.entries(D.SPRITES)) {
    assert.ok(A.SPECIES.includes(species), `${species}: không phải loài trong SPECIES`);
    const file = path.join(__dirname, '..', m.src);
    assert.ok(fs.existsSync(file), `${species}: thiếu ${m.src}`);
    const buf = fs.readFileSync(file);
    assert.ok(buf.length <= 80 * 1024, `${species}: ${Math.round(buf.length / 1024)} KB > 80 KB`);
    assert.equal(buf.toString('latin1', 1, 4), 'PNG', `${species}: không phải PNG`);
    const w = buf.readUInt32BE(16);
    const h = buf.readUInt32BE(20);
    assert.ok(w <= 512 && h <= 512 && w >= h * 0.8, `${species}: ${w}×${h} (không dựng đứng, ≤ 512 px)`);
    assert.ok(m.joint > 0 && m.joint < 1 && m.cy > 0 && m.cy < 1, species);
    if (m.wag != null) assert.ok(m.wag >= 0 && m.wag <= 0.3, species + ': wag');
  }
});

test('loài có ảnh được vẽ bằng ảnh (đuôi + thân), loài khác vẽ bằng code', () => {
  const { A, D, sandbox } = load(true);
  const drawn = [];
  const ctx = fakeCtx(drawn);
  const bounds = { w: 276, h: 126, sand: D.SAND };
  // Một loài chưa có ảnh (tự chọn, để test không vỡ mỗi lần thêm ảnh).
  const noSprite = A.SPECIES.find((s) => !D.SPRITES[s]);
  const mk = (species) => A.syncSim([], [{ uid: 1, species, stage: 2 }], bounds, A.rng(1));
  D.drawTank(ctx, { bounds, scene: D.makeScene(bounds), bg: 'glow', mode: 'night', decor: [], sim: mk('fish.guppy') });
  // Ảnh cá vẽ theo dải dọc (uốn thân, phối cảnh lúc quay đầu).
  assert.ok(drawn.filter((p) => p === 'drawImage').length >= 8, 'ảnh cá: vẽ theo nhiều dải');
  if (noSprite) {
    drawn.length = 0;
    D.drawTank(ctx, { bounds, scene: D.makeScene(bounds), bg: 'fresh', mode: 'day', decor: [], sim: mk(noSprite) });
    assert.equal(drawn.filter((p) => p === 'drawImage').length, 0, `${noSprite} chưa có ảnh: vẽ bằng code`);
  }
  assert.ok(sandbox.Image.made >= 1);
});
