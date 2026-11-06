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
    // Lớp rời: vây và ảnh thẳng cũng là PNG nhỏ có thật.
    const extra = [...(m.parts || []).map((p) => p.src), ...(m.front ? [m.front.src] : [])];
    for (const src of extra) {
      const f = path.join(__dirname, '..', src);
      assert.ok(fs.existsSync(f), `${species}: thiếu ${src}`);
      const b = fs.readFileSync(f);
      assert.equal(b.toString('latin1', 1, 4), 'PNG', `${src}: không phải PNG`);
      assert.ok(b.length <= 80 * 1024 && b.readUInt32BE(16) <= 512 && b.readUInt32BE(20) <= 512, `${src}: quá lớn`);
    }
    for (const p of m.parts || []) {
      const at = `${species} ${p.src}`;
      for (const k of ['x', 'y', 'w', 'px', 'py']) assert.ok(p[k] >= 0 && p[k] <= 1, `${at}: ${k}`);
      assert.ok(p.x + p.w <= 1.001, `${at}: vây thò ra ngoài ảnh thân`);
      // Gốc vây nằm trong (hoặc sát) hộp của vây: xoay quanh chỗ khác là vây bay đi.
      assert.ok(p.px >= p.x - 0.02 && p.px <= p.x + p.w + 0.02 && p.py >= p.y - 0.05, `${at}: gốc vây`);
      assert.ok(p.rot >= 0 && p.rot <= 0.5 && p.flap >= 0 && p.flap < 0.6, `${at}: biên độ`);
      assert.ok(p.under == null || typeof p.under === 'boolean', `${at}: under`);
    }
    if (m.front) assert.ok(m.front.h > 0.3 && m.front.h <= 1.5 && m.front.cy > 0 && m.front.cy < 1, `${species}: front`);
    // Có ảnh thẳng thì phải quay chậm (SHAPES.turnSec), không thì hoà ảnh lướt qua trong một khung.
    if (m.front) assert.ok(A.shape(species).turnSec >= 1, `${species}: có ảnh thẳng mà thiếu turnSec`);
  }
});

test('cá có vây rời + ảnh thẳng: ghép vây, quay đầu qua ảnh thẳng mà không lỗi', () => {
  const { A, D, sandbox } = load(true);
  const drawn = [];
  // document giả: canvas phụ (ghép vây, quay đầu) ghi lệnh vào cùng log.
  sandbox.document = { createElement: () => ({ width: 0, height: 0, getContext: () => fakeCtx(drawn) }) };
  const layered = Object.keys(D.SPRITES).filter((s) => D.SPRITES[s].parts && D.SPRITES[s].front);
  assert.ok(layered.length, 'có ít nhất một loài dùng vây rời + ảnh thẳng');
  const bounds = { w: 276, h: 126, sand: D.SAND };
  const ctx = fakeCtx(drawn);
  for (const sp of layered) {
    const fish = A.syncSim([], [{ uid: 1, species: sp, stage: 2 }], bounds, A.rng(4))[0];
    for (const face of [1, 0.6, 0.2, 0, -0.3, -1]) {
      fish.face = face;
      fish.wag = face * 3;
      drawn.length = 0;
      D.drawFish(ctx, fish, { mode: 'day' });
      const imgs = drawn.filter((p) => p === 'drawImage').length;
      // thân + mọi vây lên canvas ghép, rồi các dải (bỏ qua khi đã hoà hẳn sang ảnh thẳng)
      const mix = A.frontTurn(fish).mix;
      if (mix >= 0.99) assert.ok(imgs >= 1, `${sp} face ${face}: giữa vòng quay, vẽ ảnh thẳng`);
      else assert.ok(imgs >= 1 + D.SPRITES[sp].parts.length + 8, `${sp} face ${face}: ghép vây + dải (${imgs})`);
    }
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
