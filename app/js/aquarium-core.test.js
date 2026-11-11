// node --test app/js/  (npm test)
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const A = require('./aquarium-core.js');

const BOUNDS = { w: 276, h: 120, sand: 14 };

function tank(n, species = 'fish.goldfish', stage = 2) {
  return Array.from({ length: n }, (_, i) => ({ uid: i + 1, species, stage, name: '', age: 0 }));
}

test('mọi loài bên Rust (aquarium::SPECIES) đều có hình dáng, và ngược lại', () => {
  const src = fs.readFileSync(path.join(__dirname, '..', '..', 'src-tauri', 'src', 'aquarium.rs'), 'utf8');
  const block = src.match(/pub const SPECIES: \[&str; \d+\] = \[([\s\S]*?)\];/);
  assert.ok(block, 'tìm thấy SPECIES trong aquarium.rs');
  const rust = [...block[1].matchAll(/"([^"]+)"/g)].map((m) => m[1]).sort();
  assert.deepEqual([...A.SPECIES].sort(), rust);
});

test('mọi loài có tên đủ 3 thứ tiếng, mọi chuỗi đủ 3 thứ tiếng', () => {
  const keys = Object.keys(A.STR.vi).sort();
  for (const lang of ['vi', 'ja', 'en']) {
    assert.deepEqual(Object.keys(A.STR[lang]).sort(), keys, lang);
    assert.equal(A.STR[lang].stages.length, 3, lang);
    for (const id of A.SPECIES) assert.ok(A.STR[lang].names[id], `${lang} ${id}`);
  }
  assert.equal(A.strings('xx'), A.STR.vi);
});

test('cỡ cá theo stage, stage lạ bị kẹp', () => {
  assert.equal(A.fishLen('fish.goldfish', 2), A.SHAPES['fish.goldfish'].len);
  assert.ok(A.fishLen('fish.goldfish', 0) < A.fishLen('fish.goldfish', 1));
  assert.equal(A.fishLen('fish.goldfish', 9), A.fishLen('fish.goldfish', 2));
  assert.equal(A.fishLen('fish.goldfish', -1), A.fishLen('fish.goldfish', 0));
  assert.equal(A.shape('fish.unknown'), A.SHAPES['fish.goldfish']);
});

test('rng có hạt giống cho kết quả lặp lại, trong [0, 1)', () => {
  const a = A.rng(7);
  const b = A.rng(7);
  for (let i = 0; i < 100; i++) {
    const x = a();
    assert.equal(x, b());
    assert.ok(x >= 0 && x < 1);
  }
});

test('cá không bao giờ ra ngoài bể sau 10.000 bước', () => {
  const rand = A.rng(1);
  const sim = A.syncSim([], [...tank(4), ...tank(4, 'fish.neon', 0).map((f) => ({ ...f, uid: f.uid + 10 }))], BOUNDS, rand);
  let food = [];
  for (let i = 0; i < 10000; i++) {
    if (i % 500 === 0) food = A.dropFood(food, rand() * BOUNDS.w, BOUNDS, rand, 3);
    if (i % 1500 === 0) A.scatter(sim, rand() * BOUNDS.w, rand() * BOUNDS.h);
    A.step(sim, 1 / 30, { bounds: BOUNDS, food, rand, night: i > 8000 });
    food = A.eat(sim, A.sinkFood(food, 1 / 30, BOUNDS));
    for (const f of sim) {
      const box = A.swimBox(BOUNDS, f.len);
      assert.ok(f.x >= box.x0 && f.x <= box.x1 && f.y >= box.y0 && f.y <= box.y1, `bước ${i}: ${f.x},${f.y}`);
      assert.ok(Number.isFinite(f.vx) && Number.isFinite(f.vy));
    }
  }
});

test('cùng hạt giống thì cùng đường bơi', () => {
  const run = () => {
    const rand = A.rng(42);
    const sim = A.syncSim([], tank(3), BOUNDS, rand);
    for (let i = 0; i < 300; i++) A.step(sim, 1 / 30, { bounds: BOUNDS, food: [], rand });
    return sim.map((f) => [f.x, f.y]);
  };
  assert.deepEqual(run(), run());
});

test('syncSim giữ chỗ cá cũ, thêm cá mới, bỏ cá đã vớt', () => {
  const rand = A.rng(3);
  const sim = A.syncSim([], tank(2), BOUNDS, rand);
  const first = { x: sim[0].x, y: sim[0].y };
  const next = A.syncSim(sim, [{ ...tank(1)[0], stage: 1 }, { uid: 9, species: 'fish.koi', stage: 0 }], BOUNDS, rand);
  assert.equal(next.length, 2);
  assert.equal(next[0], sim[0], 'cùng đối tượng, không nhảy chỗ');
  assert.deepEqual({ x: next[0].x, y: next[0].y }, first);
  assert.equal(next[0].len, A.fishLen('fish.goldfish', 1), 'lên cỡ thì đổi chiều dài');
  assert.equal(next[1].uid, 9);
  assert.equal(A.syncSim(next, [], BOUNDS, rand).length, 0);
});

test('thức ăn chìm tới cát, nằm lại rồi tan', () => {
  const rand = A.rng(5);
  let food = A.dropFood([], 100, BOUNDS, rand, 3);
  assert.equal(food.length, 3);
  const floor = BOUNDS.h - BOUNDS.sand - 2;
  for (let t = 0; t < 20; t += 0.1) food = A.sinkFood(food, 0.1, BOUNDS);
  assert.ok(food.every((p) => p.y === floor), 'đã nằm ở đáy');
  for (let t = 0; t < A.REST_SECS + 1; t += 0.1) food = A.sinkFood(food, 0.1, BOUNDS);
  assert.equal(food.length, 0);
});

test('tối đa MAX_FOOD viên trong bể', () => {
  const rand = A.rng(6);
  let food = [];
  for (let i = 0; i < 50; i++) food = A.dropFood(food, 50, BOUNDS, rand, 3);
  assert.equal(food.length, A.MAX_FOOD);
});

test('cá đuổi theo thức ăn và đớp được', () => {
  const rand = A.rng(8);
  const sim = A.syncSim([], tank(1), BOUNDS, rand);
  let food = A.dropFood([], 60, BOUNDS, rand, 1);
  let eaten = false;
  for (let i = 0; i < 30 * 20 && !eaten; i++) {
    A.step(sim, 1 / 30, { bounds: BOUNDS, food, rand });
    food = A.eat(sim, A.sinkFood(food, 1 / 30, BOUNDS));
    eaten = food.length === 0;
  }
  assert.ok(eaten, 'trong 20 giây cá phải đớp được viên thức ăn');
});

test('ban đêm và reduced motion bơi chậm hơn; giật mình thì nhanh hơn', () => {
  const f = { species: 'fish.goldfish', startle: 0 };
  const base = A.maxSpeed(f, {});
  assert.ok(A.maxSpeed(f, { night: true }) < base);
  assert.ok(A.maxSpeed(f, { reduced: true }) < base);
  assert.ok(A.maxSpeed({ ...f, startle: 1 }, {}) > base);
  // Reduced motion: giật mình không tăng tốc.
  assert.equal(A.maxSpeed({ ...f, startle: 1 }, { reduced: true }), A.maxSpeed(f, { reduced: true }));
});

test('cá đói bơi chậm dần, no thì như thường; mã lạ coi như no', () => {
  const f = { species: 'fish.goldfish', startle: 0 };
  const base = A.maxSpeed(f, {});
  assert.equal(A.maxSpeed(f, { hunger: 'full' }), base);
  const peckish = A.maxSpeed(f, { hunger: 'peckish' });
  const hungry = A.maxSpeed(f, { hunger: 'hungry' });
  assert.ok(peckish < base && hungry < peckish, `${hungry} < ${peckish} < ${base}`);
  assert.ok(hungry > 0, 'đói vẫn bơi, không đứng im');
  assert.equal(A.hungerScale('lạ'), 1);
  assert.equal(A.hungerScale(undefined), 1);
});

test('dòng trạng thái ăn: đã ăn > đói lắm > hơi đói > chưa ăn hôm nay, đủ 3 thứ tiếng', () => {
  for (const lang of ['vi', 'ja', 'en']) {
    const s = A.strings(lang);
    assert.equal(A.fedText({ fedToday: true, hunger: 'hungry' }, s), s.fed, lang);
    assert.equal(A.fedText({ fedToday: false, hunger: 'hungry' }, s), s.hungry, lang);
    assert.equal(A.fedText({ fedToday: false, hunger: 'peckish' }, s), s.peckish, lang);
    assert.equal(A.fedText({ fedToday: false, hunger: 'full' }, s), s.notFed, lang);
    assert.equal(A.fedText(null, s), s.notFed, lang);
  }
});

test('Rust gửi mức đói đúng bộ mã mà trang hiểu', () => {
  const fs = require('node:fs');
  const path = require('node:path');
  const rs = fs.readFileSync(path.join(__dirname, '../../src-tauri/src/aquarium.rs'), 'utf8');
  for (const code of ['full', 'peckish', 'hungry']) assert.match(rs, new RegExp(`=> "${code}"`), code);
  assert.match(rs, /"hunger": hunger\(/);
  for (const f of ['aquarium-pin.js', 'aquarium.js', 'aquarium-full.js']) {
    const js = fs.readFileSync(path.join(__dirname, f), 'utf8');
    assert.match(js, /hunger: /, `${f} truyền mức đói cho step()`);
  }
});

test('gõ kính chỉ làm giật mình cá ở gần', () => {
  const rand = A.rng(9);
  const sim = A.syncSim([], tank(2), BOUNDS, rand);
  sim[0].x = 50; sim[0].y = 60;
  sim[1].x = 230; sim[1].y = 60;
  assert.equal(A.scatter(sim, 55, 60, 40), 1);
  assert.equal(sim[0].startle, 1);
  assert.equal(sim[1].startle, 0);
  assert.ok(sim[0].vx < 0, 'bơi ra xa chỗ gõ');
});

test('fishAt tìm đúng con dưới con trỏ', () => {
  const sim = A.syncSim([], tank(1), BOUNDS, A.rng(2));
  sim[0].x = 100; sim[0].y = 50;
  assert.equal(A.fishAt(sim, 100, 50), sim[0]);
  assert.equal(A.fishAt(sim, 10, 10), null);
});

test('ánh sáng theo giờ', () => {
  assert.equal(A.lightAt(12), 'day');
  assert.equal(A.lightAt(6), 'dusk');
  assert.equal(A.lightAt(18), 'dusk');
  assert.equal(A.lightAt(23), 'night');
  assert.equal(A.lightAt(3), 'night');
  for (const k of ['day', 'dusk', 'night']) assert.equal(A.WATER[k].length, 3);
});

test('mọi mã lỗi aq_* bên Rust có câu báo đủ 3 thứ tiếng', () => {
  const src = fs.readFileSync(path.join(__dirname, '..', '..', 'src-tauri', 'src', 'aquarium.rs'), 'utf8');
  const codes = [...new Set([...src.matchAll(/"(aq_[a-z_]+)"/g)].map((m) => m[1]))].sort();
  assert.ok(codes.length >= 6, 'tìm thấy mã lỗi trong aquarium.rs');
  for (const lang of ['vi', 'ja', 'en']) {
    assert.deepEqual(Object.keys(A.STR[lang].errors).sort(), codes, lang);
  }
  assert.equal(A.errorText('Error: aq_tank_full', 'vi'), A.STR.vi.errors.aq_tank_full);
  assert.equal(A.errorText('lỗi lạ', 'vi'), 'lỗi lạ', 'lỗi lạ hiện nguyên văn');
});

test('canAdd: bể đầy, đủ loài (tính cả bể chờ)', () => {
  const t = { max: 8, perSpecies: 3, fish: tank(2, 'fish.koi'), stored: [{ uid: 50, species: 'fish.koi' }] };
  assert.deepEqual(A.canAdd(t, 'fish.koi'), { ok: false, why: 'aq_species_limit' });
  assert.deepEqual(A.canAdd(t, 'fish.neon'), { ok: true });
  assert.deepEqual(A.canAdd({ ...t, fish: tank(8) }, 'fish.neon'), { ok: false, why: 'aq_tank_full' });
  assert.deepEqual(A.canAdd(undefined, 'fish.neon'), { ok: true });
});

test('tooltip cá', () => {
  assert.equal(A.fishLabel({ species: 'fish.goldfish', stage: 1, age: 3, name: '' }, 'vi'), 'Cá vàng (nhỡ) · 3 ngày tuổi');
  assert.equal(A.fishLabel({ species: 'fish.goldfish', stage: 0, age: 0, name: 'Mập' }, 'vi'), 'Mập · Cá vàng (bé) · mới thả');
  assert.equal(A.fishLabel({ species: 'fish.koi', stage: 2, age: 5, name: '' }, 'en'), 'Koi (big) · 5 days old');
});

test('quay đầu mượt: face đi dần từ hướng cũ sang hướng mới, không lật tức thì', () => {
  assert.equal(A.turnToward(-1, 1, 0.05), -1 + A.TURN_RATE * 0.05);
  assert.equal(A.turnToward(0.99, 1, 0.05), 1, 'gần tới thì dừng đúng hướng');
  assert.equal(A.turnToward(undefined, -1, 0.05), -1, 'cá cũ chưa có face thì nhận luôn hướng');
  // Mô phỏng: cá đang bơi sang phải bị gõ kính ở phía trước → quay đầu qua nhiều khung.
  const rand = A.rng(4);
  const sim = A.syncSim([], tank(1), BOUNDS, rand);
  const f = sim[0];
  Object.assign(f, { x: 150, y: 60, vx: 20, vy: 0, dir: 1, face: 1, tx: 250, ty: 60 });
  A.scatter(sim, 170, 60, 60);
  const faces = [];
  let turned = false;
  for (let i = 0; i < 30; i++) {
    A.step(sim, 1 / 30, { bounds: BOUNDS, food: [], rand });
    faces.push(f.face);
    turned = turned || f.dir === -1;
  }
  assert.ok(turned, 'giật mình bơi ngược lại thì quay đầu');
  assert.ok(faces.some((v) => v > -0.9 && v < 0.9), 'có những khung đang lật dở');
  for (let i = 1; i < faces.length; i++) assert.ok(Math.abs(faces[i] - faces[i - 1]) <= A.TURN_RATE / 30 + 1e-9, 'mỗi khung xoay một chút');
});

test('bể full: thả thức ăn ngay chỗ bấm, tìm cá gần con trỏ', () => {
  const rand = A.rng(12);
  const food = A.dropFoodAt([], 500, 300, { w: 1920, h: 1000, sand: 0 }, rand, 3);
  assert.equal(food.length, 3);
  for (const p of food) assert.ok(Math.abs(p.x - 500) <= 9 && Math.abs(p.y - 300) <= 5);
  // Bấm sát mép: viên không rơi ra ngoài bể.
  for (const p of A.dropFoodAt([], 0, 2000, { w: 1920, h: 1000, sand: 0 }, rand, 5)) {
    assert.ok(p.x >= 4 && p.y <= 998);
  }
  const sim = [{ x: 100, y: 100, len: 60 }, { x: 400, y: 100, len: 60 }];
  assert.equal(A.fishNear(sim, 120, 110, 30), sim[0]);
  assert.equal(A.fishNear(sim, 390, 90, 30), sim[1]);
  assert.equal(A.fishNear(sim, 250, 400, 30), null);
});

test('cá to (bể full, phóng ×2.7) bơi tới thức ăn thì đớp được, không chỉ bu quanh', () => {
  for (const seed of [1, 2, 3, 4, 5]) {
    const rand = A.rng(seed);
    const big = { w: 1920, h: 1000, sand: 0 };
    const sim = A.syncSim([], [{ uid: 1, species: 'fish.goldfish', stage: 2 }, { uid: 2, species: 'fish.koi', stage: 2 }], big, rand, 2.7);
    let food = A.dropFoodAt([], 900, 400, big, rand, 3);
    let t = 0;
    let eaten = 0;
    while (food.length && t < 30) {
      A.step(sim, 1 / 15, { bounds: big, food, rand, speedScale: 2.7 * 0.8 });
      const sunk = A.sinkFood(food, (1 / 15) * 2.7, big);
      food = A.eat(sim, sunk);
      eaten += sunk.length - food.length;
      t += 1 / 15;
    }
    // Viên nằm ở đáy quá REST_SECS thì tự tan: phải đếm viên BỊ ĐỚP, không phải viên biến mất.
    assert.equal(eaten, 3, `hạt giống ${seed}: chỉ đớp được ${eaten}/3 viên`);
  }
});

test('mồi nằm sát đáy: cá bơi thấp nhất có thể vẫn đớp được (widget và bể full)', () => {
  for (const [bounds, scale] of [[BOUNDS, 1], [{ w: 1920, h: 1000, sand: 0 }, 2.7]]) {
    const sim = A.syncSim([], tank(1), bounds, A.rng(1), scale);
    const f = sim[0];
    const box = A.swimBox(bounds, f.len);
    const floor = bounds.h - (bounds.sand || 0) - 2;
    // Cá ở sát đáy nhất mà swimBox cho phép, miệng ngay trên viên mồi.
    Object.assign(f, { y: box.y1, dir: 1 });
    f.x = 200 - f.len * 0.42;
    const left = A.eat(sim, [{ x: 200, y: floor, rest: 1 }]);
    assert.equal(left.length, 0, `scale ${scale}: không với tới mồi ở đáy`);
    // Mồi ở xa thì không ăn "xuyên" được.
    assert.equal(A.eat(sim, [{ x: 200 + f.len * 2, y: floor, rest: 1 }]).length, 1);
  }
});

test('độ sâu: con xa nhỏ / mờ / chậm hơn, vẽ trước; tắt (0) thì như cũ', () => {
  const rand = A.rng(21);
  const sim = A.syncSim([], tank(6), BOUNDS, rand, 1, 0.3);
  const full = A.fishLen('fish.goldfish', 2);
  for (const f of sim) {
    assert.ok(f.depth >= 0 && f.depth <= 1);
    assert.ok(Math.abs(f.len - full * A.depthScale(f.depth, 0.3)) < 1e-9);
    assert.ok(f.len <= full + 1e-9 && f.len >= full * 0.7 - 1e-9);
    assert.ok(A.depthAlpha(f) >= 0.65 && A.depthAlpha(f) <= 1);
  }
  const order = A.byDepth(sim).map((f) => f.depth);
  for (let i = 1; i < order.length; i++) assert.ok(order[i] >= order[i - 1], 'xa trước, gần sau');
  // Giữ độ sâu khi cập nhật (lên cỡ), không nhảy cỡ ngẫu nhiên.
  const again = A.syncSim(sim, tank(6), BOUNDS, rand, 1, 0.3);
  assert.deepEqual(again.map((f) => f.depth), sim.map((f) => f.depth));
  // Con xa bơi chậm hơn con gần.
  const far = { ...sim[0], depth: 0, startle: 0 };
  const near = { ...sim[0], depth: 1, startle: 0 };
  assert.ok(A.maxSpeed(far, {}) < A.maxSpeed(near, {}));
  // Tắt độ sâu: cỡ đúng như trước.
  for (const f of A.syncSim([], tank(3), BOUNDS, rand)) assert.equal(f.len, full);
});

test('ngóc lên khi bơi chéo lên, chúi xuống khi bơi xuống, tối đa PITCH_MAX', () => {
  assert.ok(A.pitchFor(30, -30) > 0, 'lên');
  assert.ok(A.pitchFor(-30, 30) < 0, 'xuống, kể cả khi bơi sang trái');
  assert.equal(A.pitchFor(1, 1), 0, 'gần đứng yên thì nằm ngang');
  assert.equal(A.pitchFor(0, -100), A.PITCH_MAX);
  // Trong mô phỏng: pitch đi dần (mượt), không vượt giới hạn.
  const rand = A.rng(22);
  const sim = A.syncSim([], tank(2), BOUNDS, rand, 1, 0.15);
  for (let i = 0; i < 600; i++) {
    A.step(sim, 1 / 30, { bounds: BOUNDS, food: [], rand });
    for (const f of sim) assert.ok(Math.abs(f.pitch) <= A.PITCH_MAX + 1e-9);
  }
  // Reduced motion: về nằm ngang.
  for (let i = 0; i < 120; i++) A.step(sim, 1 / 30, { bounds: BOUNDS, food: [], rand, reduced: true });
  for (const f of sim) assert.ok(Math.abs(f.pitch) < 0.01);
});

test('gõ trúng cá liên tục: cá quay hết vòng rồi mới đổi hướng, không kẹt giữa vòng / giật vòng', () => {
  // Koi: quay nhanh, giữa vòng (nằm nghiêng) chỉ thoáng qua. Miu: quay chậm, dừng ở ảnh
  // thẳng (frontTurn) — có chủ ý, nhưng vẫn không kẹt ở giữa vòng.
  const cases = [['fish.koi', 0.4], ['fish.miu', 0.6]].flatMap(([sp, max]) => [41, 42, 43, 44, 45].map((seed) => [sp, max, seed]));
  for (const [species, maxMid, seed] of cases) {
    const rand = A.rng(seed);
    const sim = A.syncSim([], [{ uid: 1, species, stage: 2 }], { w: 1920, h: 1000, sand: 0 }, rand, 2.7, 0.3);
    const f = sim[0];
    const big = { w: 1920, h: 1000, sand: 0 };
    let midFrames = 0;
    let lastFace = f.face;
    let dirAtStart = f.dir;
    for (let i = 0; i < 15 * 20; i++) {
      if (i % 20 === 0) A.scatter(sim, f.x + (rand() - 0.5) * 20, f.y, 200);
      const turningBefore = Math.abs(f.face - f.dir) > 1e-9;
      A.step(sim, 1 / 15, { bounds: big, food: [], rand, speedScale: 2.16 });
      if (turningBefore) {
        assert.equal(f.dir, dirAtStart, `hạt ${seed}: đổi hướng khi chưa quay xong`);
        // face đi một chiều về dir, không lùi lại.
        assert.ok((f.face - lastFace) * f.dir >= -1e-9, `hạt ${seed}: vòng quay đi lùi`);
      }
      if (Math.abs(f.face) < 0.5) midFrames++;
      dirAtStart = f.dir;
      lastFace = f.face;
    }
    // Bị gõ mỗi 1.3 giây mà phần lớn thời gian vẫn bơi ngang.
    assert.ok(midFrames < 300 * maxMid, `${species} hạt ${seed}: ${midFrames}/300 khung giữa vòng`);
  }
});

test('quay đầu: góc xoay đi chậm ở hai đầu, nhanh ở giữa; đầu / cuối đúng hướng', () => {
  assert.equal(A.turnPhi({ face: 1 }), 0);
  assert.ok(Math.abs(A.turnPhi({ face: -1 }) - Math.PI) < 1e-12);
  assert.ok(Math.abs(A.turnPhi({ face: 0 }) - Math.PI / 2) < 1e-12);
  assert.equal(A.turnPhi({ dir: -1 }), A.turnPhi({ face: -1 }), 'chưa có face thì theo dir');
  // face đi đều theo thời gian → phần trăm thời gian cá ở gần nằm nghiêng (60°–120°).
  let mid = 0;
  const n = 1000;
  for (let i = 0; i <= n; i++) {
    const phi = A.turnPhi({ face: 1 - (2 * i) / n });
    if (phi > Math.PI / 3 && phi < (2 * Math.PI) / 3) mid++;
  }
  assert.ok(mid / n < 0.25, `đoạn nằm nghiêng chiếm ${Math.round((100 * mid) / n)}% (đều là 33%)`);
  // Góc luôn tăng (không lùi) khi face giảm đều.
  let last = -1;
  for (let i = 0; i <= 100; i++) {
    const phi = A.turnPhi({ face: 1 - i / 50 });
    assert.ok(phi >= last - 1e-12);
    last = phi;
  }
});

test('spine: bơi thẳng = vẽ như trước; quay xong = gương; các dải liền nhau', () => {
  const W = 120;
  const len = 70;
  const n = 18;
  const near = (a, b) => Math.abs(a - b) < 1e-9;
  // Thẳng, nhìn phải: miệng ở +len/2, đuôi ở len/2 − W, dải nối tiếp đúng cột ảnh.
  const s0 = A.spine(0, W, n, len);
  assert.equal(s0.length, n);
  assert.ok(near(s0[0].pa, len / 2), 'miệng ở trước tâm thân len/2');
  assert.ok(near(s0[n - 1].pb, len / 2 - W), 'đuôi');
  for (const g of s0) {
    assert.ok(near(g.s, 1) && near(g.z, 0));
    assert.ok(near(g.pa - g.pb, ((g.u1 - g.u0) * W)), 'không bóp, không lật');
    assert.ok(near(g.pb, len / 2 - W + g.u0 * W), 'đúng chỗ của cột ảnh');
  }
  // Nhìn trái: gương ngang của nhìn phải.
  const sPi = A.spine(Math.PI, W, n, len);
  for (let i = 0; i < n; i++) {
    assert.ok(near(sPi[i].pa, -s0[i].pa) && near(sPi[i].pb, -s0[i].pb), `dải ${i}`);
  }
  // Giữa vòng: các dải nối liền (mép đuôi dải trước = mép miệng dải sau khi chưa chiếu
  // phối cảnh), đầu ở gần hơn đuôi (to hơn), và cả con KHÔNG bẹp thành một vạch.
  const sMid = A.spine(Math.PI / 2, W, n, len);
  const head = sMid[0];
  const tail = sMid[n - 1];
  assert.ok(head.s > tail.s, 'đầu quay về phía người nhìn thì to hơn');
  const xs = sMid.flatMap((g) => [g.pa, g.pb]);
  const width = Math.max(...xs) - Math.min(...xs);
  // ~19% bề ngang ảnh với TURN_BEND 0.75 (cong mạnh hơn thì đầu gập sớm thành mép cắt).
  assert.ok(width > W * 0.15, `giữa vòng rộng ${width.toFixed(1)} (bóp ngang thì ~0)`);
  for (const phi of [0.3, 1.2, 2.0, 2.8]) {
    for (const g of A.spine(phi, W, n, len)) assert.ok(g.yaw >= 0 && g.yaw <= Math.PI && g.s > 0);
  }
});

test('frontTurn: hoà dần sang ảnh thẳng, dừng ở giữa vòng, hoà lại; đầu / cuối đúng hướng', () => {
  assert.deepEqual(A.frontTurn({ face: 1 }), { phi: 0, mix: 0 });
  const end = A.frontTurn({ face: -1 });
  assert.ok(Math.abs(end.phi - Math.PI) < 1e-12 && end.mix === 0, 'quay xong: nhìn trái, chỉ ảnh nghiêng');
  const mid = A.frontTurn({ face: 0 });
  assert.ok(Math.abs(mid.phi - Math.PI / 2) < 1e-12 && mid.mix === 1, 'giữa vòng: chỉ ảnh thẳng');
  assert.deepEqual(A.frontTurn({ dir: -1 }), A.frontTurn({ face: -1 }), 'chưa có face thì theo dir');
  // face đi đều theo thời gian: đếm phần vòng quay dừng ở ảnh thẳng / đang hoà.
  const n = 1000;
  let hold = 0;
  let fading = 0;
  let prev = A.frontTurn({ face: 1 });
  for (let i = 1; i <= n; i++) {
    const t = A.frontTurn({ face: 1 - (2 * i) / n });
    if (t.mix >= 0.99) hold++;
    else if (t.mix > 0.01) fading++;
    assert.ok(t.phi >= prev.phi - 1e-12, `góc không lùi (${i})`);
    // mượt: không nhảy cóc giữa hai bước nhỏ
    assert.ok(Math.abs(t.mix - prev.mix) < 0.01 && Math.abs(t.phi - prev.phi) < 0.02, `mượt (${i})`);
    prev = t;
  }
  assert.ok(hold / n > 0.2, `dừng ở ảnh thẳng ${Math.round((100 * hold) / n)}% vòng quay`);
  assert.ok(fading / n > 0.3, `hoà dần ${Math.round((100 * fading) / n)}% vòng quay (không đổi cái rụp)`);
});

test('loài có turnSec quay chậm đúng thời gian đó, loài khác giữ TURN_RATE', () => {
  assert.equal(A.turnRate('fish.koi'), A.TURN_RATE);
  for (const sp of A.SPECIES.filter((s) => A.shape(s).turnSec)) {
    let face = 1;
    let frames = 0;
    while (face !== -1 && frames < 1000) {
      face = A.turnToward(face, -1, 1 / 30, A.turnRate(sp));
      frames++;
    }
    assert.ok(Math.abs(frames / 30 - A.shape(sp).turnSec) <= 1 / 30 + 1e-9, `${sp}: ${frames} khung`);
    assert.ok(A.turnRate(sp) < A.TURN_RATE, `${sp}: chậm hơn mặc định`);
  }
});

test('finPose: vây xoay quanh gốc theo nhịp, co bề ngang, giảm chuyển động thì nhỏ lại', () => {
  const p = { rot: 0.2, flap: 0.3, speed: 2, lag: 0.5 };
  let lo = Infinity;
  let hi = -Infinity;
  for (let i = 0; i < 200; i++) {
    const { angle, scaleX } = A.finPose(p, i * 0.05, false);
    assert.ok(Math.abs(angle) <= 0.2 + 1e-12);
    assert.ok(scaleX >= 0.7 - 1e-12 && scaleX <= 1 + 1e-12);
    lo = Math.min(lo, angle);
    hi = Math.max(hi, angle);
  }
  assert.ok(hi - lo > 0.35, 'có quạt thật');
  const r = A.finPose(p, Math.PI / 4 - 0.5 / 2, true);
  assert.ok(Math.abs(r.angle) <= 0.06 + 1e-12 && r.scaleX >= 0.91 - 1e-12, 'reduced: 30%');
  assert.deepEqual(A.finPose({}, 3, false), { angle: 0, scaleX: 1 }, 'vây không khai báo thì đứng yên');
});
