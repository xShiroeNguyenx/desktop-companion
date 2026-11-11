// 🐠 Bể cá (aquarium-pin.html): hình dáng từng loài, mô phỏng bơi, thức ăn,
// ánh sáng theo giờ, chữ vi/ja/en. Hàm thuần, dùng trong cửa sổ
// (window.AquariumCore) và node --test. `rand` luôn truyền vào để test được.
// Cá và việc cho ăn do Rust giữ (aquarium.rs); trang chỉ vẽ.
(function (root) {
  // ─── Loài ─────────────────────────────────────────────────────────────────
  // ĐỒNG BỘ với aquarium::SPECIES. len = dài thân (px) ở cỡ lớn; h = cao / dài;
  // speed = px/giây; school = bơi theo đàn cùng loài. turnSec (tuỳ chọn) = thời gian một
  // vòng quay đầu, cho loài có ảnh nhìn thẳng (aquarium-draw SPRITES.front): quay chậm để
  // ảnh hoà dần sang ảnh thẳng, dừng một lúc, rồi hoà lại (frontTurn). Mặc định 2 / TURN_RATE.
  const SHAPES = {
    'fish.goldfish': { len: 34, h: 0.55, body: '#ff9a3c', belly: '#ffd08a', fin: '#ffb766', tail: 'fan', pattern: 'none', speed: 26, school: false },
    'fish.neon': { len: 20, h: 0.32, body: '#5fd2ff', belly: '#e8f7ff', fin: '#b7ecff', tail: 'fork', pattern: 'neon', speed: 34, school: true },
    'fish.guppy': { len: 21, h: 0.38, body: '#c9d6e3', belly: '#f2f6fa', fin: '#ff6fb1', tail: 'fan', pattern: 'spots', speed: 30, school: true, turnSec: 1.4 },
    'fish.molly': { len: 26, h: 0.45, body: '#f4f1f6', belly: '#ffffff', fin: '#ffb04a', tail: 'round', pattern: 'none', speed: 24, school: true },
    'fish.betta': { len: 32, h: 0.42, body: '#7a5cff', belly: '#b6a4ff', fin: '#ff5f9e', tail: 'long', pattern: 'none', speed: 18, school: false },
    'fish.angel': { len: 30, h: 1.0, body: '#e9e4d4', belly: '#fffaf0', fin: '#d8d0b8', tail: 'fork', pattern: 'stripe', speed: 16, school: false },
    'fish.clown': { len: 26, h: 0.5, body: '#ff7a1a', belly: '#ff9a4a', fin: '#ff8a2a', tail: 'round', pattern: 'stripe', speed: 22, school: false },
    'fish.koi': { len: 46, h: 0.36, body: '#fffaf2', belly: '#ffffff', fin: '#ffe9d8', tail: 'fan', pattern: 'koi', speed: 14, school: false },
    'fish.miu': { len: 56, h: 0.6, body: '#c7a6ff', belly: '#f1e6ff', fin: '#ff9fd0', tail: 'fan', pattern: 'none', speed: 20, school: false, ears: true, turnSec: 1.4 },
  };
  const SPECIES = Object.keys(SHAPES);
  const STAGE_SCALE = [0.6, 0.8, 1];
  /** Quay đầu: `face` chạy từ -1 tới 1 (và ngược lại) với tốc độ này mỗi giây → lật hết
   * hướng mất 2 / TURN_RATE giây. Vẽ bằng scale(face, 1): thân hẹp dần, lật, nở ra. */
  const TURN_RATE = 6;
  /** Miệng cá cách tâm thân (× dài thân), và bán kính đớp mồi (× dài thân, cộng 4 px). */
  const MOUTH = 0.42;
  /** Ngóc / chúi tối đa (rad, ~20°). */
  const PITCH_MAX = 0.35;
  const EAT_RADIUS = 0.2;

  function shape(species) {
    return SHAPES[species] || SHAPES['fish.goldfish'];
  }

  /** Dài thân (px) theo loài và cỡ (0 bé · 1 nhỡ · 2 lớn). */
  function fishLen(species, stage) {
    const i = Math.max(0, Math.min(2, Number(stage) || 0));
    return shape(species).len * STAGE_SCALE[i];
  }

  // ─── Ngẫu nhiên có hạt giống (mulberry32) ─────────────────────────────────
  function rng(seed) {
    let a = seed >>> 0;
    return function () {
      a = (a + 0x6d2b79f5) >>> 0;
      let t = a;
      t = Math.imul(t ^ (t >>> 15), t | 1);
      t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
      return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
    };
  }

  // ─── Bể ───────────────────────────────────────────────────────────────────
  /**
   * Vùng cá được bơi: w × h là cỡ bể; `sand` = độ cao lớp cát ở đáy.
   * Trả hộp {x0, y0, x1, y1} cho tâm cá dài `len`.
   */
  function swimBox(bounds, len) {
    const mx = len / 2 + 2;
    const my = len * 0.35 + 3;
    return {
      x0: mx,
      y0: 8 + my,
      x1: Math.max(mx + 1, bounds.w - mx),
      y1: Math.max(8 + my + 1, bounds.h - (bounds.sand || 0) - my),
    };
  }

  function randomIn(box, rand) {
    return { x: box.x0 + rand() * (box.x1 - box.x0), y: box.y0 + rand() * (box.y1 - box.y0) };
  }

  /**
   * Giữ cá cũ (theo uid) đúng chỗ đang bơi, thêm cá mới ở chỗ ngẫu nhiên, bỏ
   * cá không còn trong danh sách. `list` là tank.fish từ Rust.
   */
  /**
   * scale: phóng to cá (bể full trên desktop to hơn bể widget nhiều lần).
   * depthRange: độ sâu (cảm giác 3D) — mỗi con có depth 0 (xa) … 1 (gần); con xa nhỏ đi
   * tới (1 − depthRange) lần, bơi chậm hơn, vẽ mờ hơn và nằm sau. 0 = tắt (mặc định).
   */
  function syncSim(prev, list, bounds, rand, scale = 1, depthRange = 0) {
    const old = new Map((prev || []).map((f) => [f.uid, f]));
    return (list || []).map((d) => {
      const f = old.get(d.uid);
      const depth = f && Number.isFinite(f.depth) ? f.depth : depthRange > 0 ? rand() : 1;
      const len = fishLen(d.species, d.stage) * scale * depthScale(depth, depthRange);
      const box = swimBox(bounds, len);
      if (f) {
        f.species = d.species;
        f.stage = d.stage;
        f.len = len;
        f.depth = depth;
        f.depthRange = depthRange;
        f.x = clamp(f.x, box.x0, box.x1);
        f.y = clamp(f.y, box.y0, box.y1);
        return f;
      }
      const p = randomIn(box, rand);
      const t = randomIn(box, rand);
      const d0 = rand() < 0.5 ? -1 : 1;
      return {
        uid: d.uid, species: d.species, stage: d.stage, len,
        x: p.x, y: p.y, vx: 0, vy: 0, tx: t.x, ty: t.y,
        dir: d0, face: d0, wag: rand() * Math.PI * 2, startle: 0, pitch: 0, depth, depthRange,
      };
    });
  }

  /** Cỡ theo độ sâu: gần (1) = 100%, xa (0) = 1 − range. */
  function depthScale(depth, range) {
    const d = Number.isFinite(depth) ? Math.max(0, Math.min(1, depth)) : 1;
    return 1 - (range || 0) * (1 - d);
  }

  /** Độ mờ khi vẽ theo độ sâu: con xa nhạt đi (tối đa 35% khi range = 0.3). */
  function depthAlpha(f) {
    const r = f.depthRange || 0;
    const d = Number.isFinite(f.depth) ? f.depth : 1;
    return 1 - Math.min(0.35, r * 1.2) * (1 - d);
  }

  /** Thứ tự vẽ: xa trước, gần sau (gần đè lên xa). Không đổi mảng gốc. */
  function byDepth(sim) {
    return [...(sim || [])].sort((a, b) => (Number.isFinite(a.depth) ? a.depth : 1) - (Number.isFinite(b.depth) ? b.depth : 1));
  }

  /** Góc ngóc (+) / chúi (−) mong muốn theo hướng bơi, tối đa PITCH_MAX rad. */
  function pitchFor(vx, vy) {
    if (Math.hypot(vx, vy) < 4) return 0;
    return Math.max(-PITCH_MAX, Math.min(PITCH_MAX, Math.atan2(-vy, Math.abs(vx))));
  }

  function clamp(v, lo, hi) {
    return v < lo ? lo : v > hi ? hi : v;
  }

  /** Lâu không được ăn thì bơi chậm lại (Rust: aquarium::hunger). Lạ / thiếu = no. */
  const HUNGER_SCALE = { full: 1, peckish: 0.8, hungry: 0.55 };
  function hungerScale(h) {
    return HUNGER_SCALE[h] || 1;
  }

  /** Tốc độ tối đa (px/s) của một con lúc này. */
  function maxSpeed(f, opts) {
    let v = shape(f.species).speed * ((opts && opts.speedScale) || 1);
    v *= hungerScale(opts && opts.hunger);
    // Con ở xa (nhỏ hơn) bơi chậm hơn một chút: đúng phối cảnh.
    v *= depthScale(f.depth, f.depthRange);
    if (opts && opts.night) v *= 0.25;
    if (opts && opts.reduced) v *= 0.5;
    if (f.startle > 0) v *= opts && opts.reduced ? 1 : 3;
    return v;
  }

  /**
   * Một bước mô phỏng. opts: { bounds, food, rand, night, reduced, hunger }.
   * Cá lang thang tới một điểm, loài bơi đàn kéo về giữa đàn, có thức ăn thì
   * đuổi theo viên gần nhất (ban đêm cá ngủ, không đuổi).
   */
  function step(sim, dt, opts) {
    const { bounds, food = [], rand, night = false } = opts;
    const d = Math.max(0, Math.min(0.1, dt));
    for (const f of sim) {
      const s = shape(f.species);
      const box = swimBox(bounds, f.len);
      let tx = f.tx;
      let ty = f.ty;
      let urge = 1;
      const pellet = night ? null : nearest(food, f.x, f.y);
      if (pellet) {
        // Bơi sao cho MIỆNG (trước tâm thân MOUTH × dài) chạm viên mồi, không phải tâm thân:
        // cá to (bể full ×2.7) mà nhắm tâm vào mồi thì bu quanh mãi không đớp được.
        const side = pellet.x >= f.x ? 1 : -1;
        tx = pellet.x - side * f.len * MOUTH;
        ty = pellet.y;
        urge = 2;
      } else {
        if (Math.hypot(f.tx - f.x, f.ty - f.y) < 8 || rand() < d * 0.15) {
          const t = randomIn(box, rand);
          f.tx = t.x;
          f.ty = t.y;
        }
        if (s.school) {
          const mates = sim.filter((o) => o !== f && o.species === f.species);
          if (mates.length) {
            const cx = mates.reduce((a, o) => a + o.x, 0) / mates.length;
            const cy = mates.reduce((a, o) => a + o.y, 0) / mates.length;
            tx = tx * 0.6 + cx * 0.4;
            ty = ty * 0.6 + cy * 0.4;
          }
        }
      }
      const vmax = maxSpeed(f, opts) * urge;
      const dx = tx - f.x;
      // Lang thang: bơi ngang nhiều hơn dọc. Đuổi mồi: đuổi thẳng cả chiều dọc.
      const dy = (ty - f.y) * (pellet ? 1 : 0.7);
      const dist = Math.hypot(dx, dy) || 1;
      // Lang thang thì chậm dần khi tới nơi. Đuổi mồi thì KHÔNG: mồi đang rơi, cá giảm tốc
      // theo khoảng cách sẽ luôn trễ sau viên mồi (bể full: mồi rơi ~43 px/s) mà không kịp đớp.
      const want = pellet ? vmax : Math.min(vmax, dist * 2);
      const k = Math.min(1, d * (f.startle > 0 ? 4 : 1.5));
      f.vx += ((dx / dist) * want - f.vx) * k;
      f.vy += ((dy / dist) * want - f.vy) * k;
      const sp = Math.hypot(f.vx, f.vy);
      const cap = maxSpeed(f, opts) * Math.max(urge, 1);
      if (sp > cap) {
        f.vx *= cap / sp;
        f.vy *= cap / sp;
      }
      f.x += f.vx * d;
      f.y += f.vy * d;
      if (f.x < box.x0 || f.x > box.x1) {
        f.x = clamp(f.x, box.x0, box.x1);
        f.vx = -f.vx * 0.5;
      }
      if (f.y < box.y0 || f.y > box.y1) {
        f.y = clamp(f.y, box.y0, box.y1);
        f.vy = -f.vy * 0.5;
      }
      // Quay đầu có trễ, để cá không lật qua lật lại khi gần đứng yên.
      // Đang quay đầu (face chưa tới dir) thì KHOÁ hướng: phải quay hết vòng rồi mới đổi
      // tiếp. Không thì bị gõ / đổi ý liên tục, vận tốc ngang đổi dấu vài lần mỗi giây,
      // cá đi tới giữa vòng lại quay ngược — kẹt ở tư thế nằm nghiêng (thử thật 2026-11-05).
      const turning = f.face != null && Math.abs(f.face - f.dir) > 1e-9;
      if (!turning) {
        if (f.vx > 3) f.dir = 1;
        else if (f.vx < -3) f.dir = -1;
      }
      f.face = turnToward(f.face, f.dir, d, turnRate(f.species));
      // Ngóc lên / chúi xuống theo hướng bơi (mượt, nhỏ dần khi đang quay đầu).
      const pitchWant = opts.reduced ? 0 : pitchFor(f.vx, f.vy) * Math.abs(f.face == null ? 1 : f.face);
      f.pitch = (f.pitch || 0) + (pitchWant - (f.pitch || 0)) * Math.min(1, d * 3);
      f.wag += d * (3 + Math.hypot(f.vx, f.vy) / 6);
      if (f.startle > 0) f.startle = Math.max(0, f.startle - d);
    }
    return sim;
  }

  /** face tiến dần về dir (mượt, không lật tức thì). Cá cũ chưa có face thì nhận luôn dir. */
  function turnToward(face, dir, dt, rate = TURN_RATE) {
    if (face == null || !Number.isFinite(face)) return dir;
    const step = rate * Math.max(0, dt);
    return Math.abs(dir - face) <= step ? dir : face + Math.sign(dir - face) * step;
  }

  /** Tốc độ quay đầu (face / giây) của loài: loài có turnSec quay chậm hơn. */
  function turnRate(species) {
    const s = shape(species).turnSec;
    return s > 0 ? 2 / s : TURN_RATE;
  }

  // ─── Quay đầu (xoay quanh trục dọc, thân cong chữ C, phối cảnh) ────────────
  // Ảnh cá chụp ngang không có "mặt kia": xoay kiểu gì cũng có lúc nằm nghiêng hẳn.
  // Đã thử (2026-11-05, dựng ảnh từng khung bằng Edge headless): bóp ngang = lật tờ
  // giấy; xoay vòng cung trong mặt phẳng = hình 2D lộn nhào. Cách giữ lại: mỗi dải
  // thân xoay một góc hơi khác (đầu trước, đuôi theo — cong chữ C), chiếu phối cảnh
  // (đầu quay về phía người nhìn thì to ra), đổ tối dải đang nghiêng; và đi RẤT NHANH
  // qua đoạn giữa (chậm ở hai đầu) để khoảnh khắc mỏng nhất gần như không lọt khung nào.

  /** Độ chậm ở hai đầu / nhanh ở giữa vòng quay (0 = đều). */
  const TURN_EASE = 0.6;
  /** Độ cong chữ C lúc quay (rad chênh giữa đầu và đuôi). */
  const TURN_BEND = 0.75;
  /** Khoảng cách camera (× bề ngang ảnh) cho phép chiếu phối cảnh. */
  const TURN_CAM = 2.4;

  /** Góc xoay quanh trục dọc: 0 = nhìn phải, π = nhìn trái. Theo face (đi đều theo thời gian). */
  function turnPhi(f) {
    const face = f.face == null ? f.dir || 1 : Math.max(-1, Math.min(1, f.face));
    const p = (1 - face) / 2;
    return Math.PI * (p - (TURN_EASE * Math.sin(2 * Math.PI * p)) / (2 * Math.PI));
  }

  /**
   * Các dải thân khi xoay góc phi (W = bề ngang ảnh, len = dài thân, n dải). Đi dọc sống
   * lưng từ miệng (u = 1) về đuôi (u = 0), mỗi dải hướng yaw riêng; toạ độ tính từ tâm
   * thân (cách miệng len/2, như khi bơi thẳng). Trả mỗi dải: u0, u1, yaw, pa / pb (x đã
   * chiếu ở mép phía miệng / phía đuôi), s (tỉ lệ phối cảnh), z (gần + / xa −).
   * phi = 0: thẳng, miệng ở +len/2, đuôi ở len/2 − W (đúng như vẽ không quay).
   */
  function spine(phi, W, n, len, bend = TURN_BEND) {
    const ds = W / n;
    let x = 0;
    let z = 0;
    let walked = 0;
    let cx = null;
    let cz = 0;
    const segs = [];
    for (let i = n - 1; i >= 0; i--) {
      const u0 = i / n;
      const u1 = (i + 1) / n;
      const uc = (u0 + u1) / 2;
      const yaw = clamp(phi + bend * (uc - 0.5) * Math.sin(phi) * 2, 0, Math.PI);
      const x1 = x - ds * Math.cos(yaw);
      const z1 = z - ds * Math.sin(yaw);
      if (cx == null && walked + ds >= len / 2) {
        const t = (len / 2 - walked) / ds;
        cx = x + (x1 - x) * t;
        cz = z + (z1 - z) * t;
      }
      segs.push({ u0, u1, yaw, xa: x, za: z, xb: x1, zb: z1 });
      x = x1;
      z = z1;
      walked += ds;
    }
    if (cx == null) {
      cx = x;
      cz = z;
    }
    const cam = W * TURN_CAM;
    return segs.map((g) => {
      const sa = cam / (cam - (g.za - cz));
      const sb = cam / (cam - (g.zb - cz));
      return { u0: g.u0, u1: g.u1, yaw: g.yaw, pa: (g.xa - cx) * sa, pb: (g.xb - cx) * sb, s: (sa + sb) / 2, z: (g.za + g.zb) / 2 - cz };
    });
  }

  /**
   * Quay đầu của loài có ảnh nhìn thẳng (SPRITES.front, quay trong turnSec giây). Theo tiến
   * độ vòng quay (đều theo thời gian), đối xứng hai nửa:
   *   - thân bắt đầu xoay vào trong (cong chữ C), ảnh thẳng hiện mờ dần lên trên;
   *   - giữa vòng: dừng ở ảnh thẳng (FRONT_HOLD của vòng);
   *   - ảnh thẳng mờ dần đi, thân (đã quay sang hướng mới) xoay nốt ra ngang.
   * Trả { phi: góc xoay cho ảnh nghiêng, mix: 0 (chỉ ảnh nghiêng) … 1 (chỉ ảnh thẳng) }.
   */
  const FRONT_HOLD = 0.25;
  const FRONT_FADE = 0.5;
  function frontTurn(f) {
    const face = f.face == null ? f.dir || 1 : clamp(f.face, -1, 1);
    const p = (1 - face) / 2;
    const u = Math.abs(2 * p - 1); // 1 = hai đầu vòng, 0 = giữa vòng
    const mix = 1 - smooth((u - FRONT_HOLD) / FRONT_FADE);
    const half = (Math.PI / 2) * smooth((1 - u) / (1 - FRONT_HOLD));
    return { phi: p <= 0.5 ? half : Math.PI - half, mix };
  }

  /** Smoothstep trên t (kẹp 0..1): đi chậm ở hai đầu. */
  function smooth(t) {
    const x = clamp(t, 0, 1);
    return x * x * (3 - 2 * x);
  }

  /**
   * Tư thế một vây rời theo nhịp bơi `wag`: góc xoay quanh gốc (rad) và bề ngang
   * (1 = như ảnh, nhỏ hơn = vây quạt ra sau / vào trong). part: { rot, flap, speed, lag }.
   * reduced (giảm chuyển động): còn 30% biên độ.
   */
  function finPose(part, wag, reduced) {
    const k = reduced ? 0.3 : 1;
    const t = (wag || 0) * (part.speed || 1) + (part.lag || 0);
    const s = Math.sin(t);
    return { angle: (part.rot || 0) * k * s, scaleX: 1 - (part.flap || 0) * k * (0.5 + 0.5 * s) };
  }

  function nearest(food, x, y) {
    let best = null;
    let bd = Infinity;
    for (const p of food) {
      const dd = Math.hypot(p.x - x, p.y - y);
      if (dd < bd) {
        bd = dd;
        best = p;
      }
    }
    return best;
  }

  // ─── Thức ăn ──────────────────────────────────────────────────────────────
  const MAX_FOOD = 30;
  const SINK = 16; // px/s
  const REST_SECS = 6; // nằm ở đáy bao lâu rồi tan

  /** Thả `n` viên quanh x, từ mặt nước. Tối đa MAX_FOOD viên trong bể. */
  function dropFood(food, x, bounds, rand, n = 3) {
    for (let i = 0; i < n && food.length < MAX_FOOD; i++) {
      food.push({ x: clamp(x + (rand() - 0.5) * 18, 4, bounds.w - 4), y: 4 + rand() * 4, rest: 0 });
    }
    return food;
  }

  /** Thả n viên quanh (x, y) (bể full: bấm ở đâu rơi ở đó). Tối đa MAX_FOOD viên. */
  function dropFoodAt(food, x, y, bounds, rand, n = 3) {
    for (let i = 0; i < n && food.length < MAX_FOOD; i++) {
      food.push({
        x: clamp(x + (rand() - 0.5) * 18, 4, bounds.w - 4),
        y: clamp(y + (rand() - 0.5) * 10, 4, bounds.h - (bounds.sand || 0) - 2),
        rest: 0,
      });
    }
    return food;
  }

  /** Con cá gần điểm (x, y) nhất trong bán kính r (rê chuột để xem tên). */
  function fishNear(sim, x, y, r) {
    let best = null;
    let bd = Infinity;
    for (const f of sim) {
      const d = Math.hypot(f.x - x, f.y - y);
      if (d <= Math.max(r, f.len * 0.8) && d < bd) {
        bd = d;
        best = f;
      }
    }
    return best;
  }

  /** Viên chìm dần, chạm cát thì nằm lại REST_SECS giây rồi mất. Trả mảng mới. */
  function sinkFood(food, dt, bounds) {
    const floor = bounds.h - (bounds.sand || 0) - 2;
    const out = [];
    for (const p of food) {
      if (p.y < floor) p.y = Math.min(floor, p.y + SINK * dt);
      else p.rest += dt;
      if (p.rest < REST_SECS) out.push(p);
    }
    return out;
  }

  /** Cá đớp viên ở gần miệng. Trả mảng thức ăn còn lại. */
  function eat(sim, food) {
    return food.filter((p) => !sim.some((f) => {
      // Vùng đớp bầu dục, cao gấp đôi rộng: cá không chạm đáy được (swimBox chừa
      // khoảng ~0.35 × dài) mà mồi lại nằm sát đáy — tròn thì cá bu trên mồi mãi không ăn.
      const mx = f.x + f.dir * f.len * MOUTH;
      const rx = 4 + f.len * EAT_RADIUS;
      const ry = 4 + f.len * EAT_RADIUS * 2;
      return ((p.x - mx) / rx) ** 2 + ((p.y - f.y) / ry) ** 2 <= 1;
    }));
  }

  /** Gõ kính: cá trong bán kính r giật mình bơi ra xa. */
  function scatter(sim, x, y, r = 50) {
    let n = 0;
    for (const f of sim) {
      const dx = f.x - x;
      const dy = f.y - y;
      const dist = Math.hypot(dx, dy);
      if (dist > r) continue;
      const ux = dist ? dx / dist : 1;
      const uy = dist ? dy / dist : 0;
      f.tx = f.x + ux * 60;
      f.ty = f.y + uy * 30;
      f.vx = ux * shape(f.species).speed * 2;
      f.vy = uy * shape(f.species).speed;
      f.startle = 1;
      n++;
    }
    return n;
  }

  /** Cá dưới con trỏ (con nằm trên cùng = vẽ sau cùng). */
  function fishAt(sim, x, y) {
    for (let i = sim.length - 1; i >= 0; i--) {
      const f = sim[i];
      const rx = f.len / 2;
      const ry = Math.max(4, f.len * shape(f.species).h / 2);
      if (((x - f.x) / rx) ** 2 + ((y - f.y) / ry) ** 2 <= 1) return f;
    }
    return null;
  }

  // ─── Ánh sáng ─────────────────────────────────────────────────────────────
  /** 'day' 7–17h · 'dusk' 5–7h và 17–19h · 'night' còn lại. */
  function lightAt(hour) {
    const h = Number(hour);
    if (h >= 7 && h < 17) return 'day';
    if ((h >= 5 && h < 7) || (h >= 17 && h < 19)) return 'dusk';
    return 'night';
  }

  const WATER = {
    day: ['#8fdcf0', '#3a9cc8', '#1d6b9a'],
    dusk: ['#a99be0', '#5a6bb8', '#2e3d7a'],
    night: ['#2a3a6e', '#1a2550', '#0e1636'],
  };

  // ─── Chữ ──────────────────────────────────────────────────────────────────
  const STR = {
    vi: {
      title: 'Bể cá', fed: 'Đã cho ăn', notFed: 'Chưa cho ăn hôm nay', feed: 'Cho cá ăn',
      peckish: 'Cá hơi đói rồi', hungry: 'Cá đói lắm, cho ăn nha!',
      bigger: 'Bể lớn', smaller: 'Bể nhỏ', top: 'Luôn nổi', unpin: 'Gỡ bể cá',
      empty: 'Bể đang trống', age: '{n} ngày tuổi', newborn: 'mới thả',
      stages: ['bé', 'nhỡ', 'lớn'], grew: 'Cá lớn thêm rồi!', alreadyFed: '🍤 Hôm nay cá ăn rồi, thả thêm cho vui thôi~',
      fullOn: '⛶ Thả cá ra hình nền', fullOff: '🐠 Thu cá về bể', fullHint: 'Cá bơi trên hình nền màn hình chính, sau mọi cửa sổ. Widget bể tạm ẩn.',
      manage: 'Quản lý bể cá', dblHint: 'Bấm: thả thức ăn · bấm đúp: quản lý bể',
      tabTank: 'Trong bể', tabStored: 'Bể chờ', tabDecor: 'Trang trí',
      add: 'Thả thêm', store: 'Vớt ra', putBack: 'Thả lại', rename: 'Đặt tên', save: 'Lưu',
      namePh: 'Để trống = tên loài', emptyTank: 'Bể đang trống, thả cá từ Bể chờ nha~',
      emptyStored: 'Chưa vớt con nào ra.', ownedTitle: 'Loài đã có', fedDays: 'đã ăn {n} ngày',
      count: '{n}/{max} con trong bể', perSpecies: 'Mỗi loài tối đa {n} con (tính cả bể chờ)',
      bg: 'Nền bể', changeBg: 'Đổi nền', slot: 'Ô {n}', none: '— Trống —',
      noDecor: 'Chưa có vật trang trí nào.', shopHint: 'Mua thêm cá, nền và đồ trang trí ở 🍬 Bộ sưu tập.',
      openShop: '🍬 Bộ sưu tập', close: 'Đóng', saved: 'Đã lưu~',
      errors: {
        aq_not_found: 'Không tìm thấy món này.', aq_not_owned: 'Chưa mở khoá món này ở Bộ sưu tập.',
        aq_tank_full: 'Bể đầy rồi (tối đa 8 con). Vớt bớt một con ra trước nha.',
        aq_species_limit: 'Loài này đã đủ 3 con rồi.', aq_no_fish: 'Không còn con cá này nữa.',
        aq_bad_slot: 'Ô trang trí không hợp lệ.',
      },
      names: {
        'fish.goldfish': 'Cá vàng', 'fish.neon': 'Cá neon', 'fish.guppy': 'Cá bảy màu',
        'fish.molly': 'Cá molly', 'fish.betta': 'Cá betta', 'fish.angel': 'Cá thần tiên',
        'fish.clown': 'Cá hề', 'fish.koi': 'Cá koi', 'fish.miu': 'Tiên cá Miu',
      },
    },
    ja: {
      title: '水槽', fed: 'ごはん済み', notFed: '今日はまだごはん前', feed: 'ごはんをあげる',
      peckish: 'ちょっとおなかがすいてる', hungry: 'おなかぺこぺこ、ごはんをあげてね！',
      bigger: '大きい水槽', smaller: '小さい水槽', top: '常に手前', unpin: '水槽を外す',
      empty: '水槽は空っぽ', age: '{n}日目', newborn: '新入り',
      stages: ['ちび', '中', '大'], grew: '大きくなった！', alreadyFed: '🍤 今日はもうごはん済み、これはおまけだよ～',
      fullOn: '⛶ 壁紙で泳がせる', fullOff: '🐠 水槽に戻す', fullHint: 'メイン画面の壁紙の上、すべてのウィンドウの後ろで泳ぎます。水槽ウィジェットは一時的に隠れます。',
      manage: '水槽の管理', dblHint: 'クリック：えさ・ダブルクリック：水槽の管理',
      tabTank: '水槽', tabStored: '待機中', tabDecor: '飾り',
      add: '入れる', store: '出す', putBack: '戻す', rename: '名前', save: '保存',
      namePh: '空欄＝種類の名前', emptyTank: '水槽は空っぽ。待機中からお魚を入れてね～',
      emptyStored: '待機中のお魚はいません。', ownedTitle: '持っている種類', fedDays: 'ごはん{n}日',
      count: '水槽に{n}/{max}匹', perSpecies: '1種類につき最大{n}匹（待機中も含む）',
      bg: '背景', changeBg: '背景を変える', slot: '{n}番', none: '— なし —',
      noDecor: '飾りはまだありません。', shopHint: 'お魚・背景・飾りは🍬コレクションで手に入ります。',
      openShop: '🍬 コレクション', close: '閉じる', saved: '保存したよ～',
      errors: {
        aq_not_found: '見つかりません。', aq_not_owned: 'コレクションでまだアンロックしていません。',
        aq_tank_full: '水槽がいっぱい（最大8匹）。先に1匹出してね。',
        aq_species_limit: 'この種類はもう3匹います。', aq_no_fish: 'このお魚はもういません。',
        aq_bad_slot: '飾りの場所が正しくありません。',
      },
      names: {
        'fish.goldfish': '金魚', 'fish.neon': 'ネオンテトラ', 'fish.guppy': 'グッピー',
        'fish.molly': 'モーリー', 'fish.betta': 'ベタ', 'fish.angel': 'エンゼルフィッシュ',
        'fish.clown': 'クマノミ', 'fish.koi': '錦鯉', 'fish.miu': '人魚ミウ',
      },
    },
    en: {
      title: 'Aquarium', fed: 'Fed today', notFed: 'Not fed yet today', feed: 'Feed the fish',
      peckish: 'A little hungry', hungry: 'Very hungry, please feed them!',
      bigger: 'Bigger tank', smaller: 'Smaller tank', top: 'Always on top', unpin: 'Remove aquarium',
      empty: 'The tank is empty', age: '{n} days old', newborn: 'just arrived',
      stages: ['small', 'medium', 'big'], grew: 'The fish grew!', alreadyFed: '🍤 Already fed today, this is just a snack~',
      fullOn: '⛶ Let them swim on the wallpaper', fullOff: '🐠 Back to the tank', fullHint: 'Fish swim on the main screen wallpaper, behind every window. The tank widget hides meanwhile.',
      manage: 'Manage the tank', dblHint: 'Click: drop food · double-click: manage the tank',
      tabTank: 'In the tank', tabStored: 'Waiting', tabDecor: 'Decor',
      add: 'Add one', store: 'Take out', putBack: 'Put back', rename: 'Name', save: 'Save',
      namePh: 'Empty = species name', emptyTank: 'The tank is empty, add fish from Waiting~',
      emptyStored: 'No fish waiting.', ownedTitle: 'Species you have', fedDays: 'fed {n} days',
      count: '{n}/{max} fish in the tank', perSpecies: 'Up to {n} per species (waiting included)',
      bg: 'Background', changeBg: 'Change', slot: 'Spot {n}', none: '— Empty —',
      noDecor: 'No decorations yet.', shopHint: 'Get more fish, backgrounds and decor in the 🍬 Collection.',
      openShop: '🍬 Collection', close: 'Close', saved: 'Saved~',
      errors: {
        aq_not_found: 'That item was not found.', aq_not_owned: 'Unlock it in the Collection first.',
        aq_tank_full: 'The tank is full (8 fish max). Take one out first.',
        aq_species_limit: 'You already have 3 of this species.', aq_no_fish: 'That fish is no longer here.',
        aq_bad_slot: 'That decor spot is not valid.',
      },
      names: {
        'fish.goldfish': 'Goldfish', 'fish.neon': 'Neon tetra', 'fish.guppy': 'Guppy',
        'fish.molly': 'Molly', 'fish.betta': 'Betta', 'fish.angel': 'Angelfish',
        'fish.clown': 'Clownfish', 'fish.koi': 'Koi', 'fish.miu': 'Miu mermaid',
      },
    },
  };

  function strings(lang) {
    return STR[lang] || STR.vi;
  }

  /** Dòng trạng thái ăn của bể: đã ăn hôm nay / đói lắm / hơi đói / chưa ăn hôm nay. */
  function fedText(tank, s) {
    if (tank && tank.fedToday) return s.fed;
    if (tank && tank.hunger === 'hungry') return s.hungry;
    if (tank && tank.hunger === 'peckish') return s.peckish;
    return s.notFed;
  }

  function fill(tpl, vars) {
    return String(tpl).replace(/\{(\w+)\}/g, (m, k) => (vars && k in vars ? String(vars[k]) : m));
  }

  /** Dòng tooltip của một con: "Mập · Cá vàng (nhỡ) · 3 ngày tuổi". */
  function fishLabel(d, lang) {
    const t = strings(lang);
    const kind = t.names[d.species] || d.species;
    const age = d.age > 0 ? fill(t.age, { n: d.age }) : t.newborn;
    const who = d.name ? `${d.name} · ${kind}` : kind;
    return `${who} (${t.stages[d.stage] || t.stages[0]}) · ${age}`;
  }

  /** Mã lỗi từ Rust (aq_tank_full…, kèm chữ "Error: " tuỳ nơi gọi) → câu đọc được. */
  function errorText(err, lang) {
    const s = String(err || '');
    const code = (s.match(/aq_[a-z_]+/) || [])[0];
    return (code && strings(lang).errors[code]) || s;
  }

  /** Loài đã có thả thêm được không (giới hạn loài tính cả bể chờ, bể chưa đầy). */
  function canAdd(tank, species) {
    const all = [...((tank && tank.fish) || []), ...((tank && tank.stored) || [])];
    const per = (tank && tank.perSpecies) || 3;
    const max = (tank && tank.max) || 8;
    if (((tank && tank.fish) || []).length >= max) return { ok: false, why: 'aq_tank_full' };
    if (all.filter((f) => f.species === species).length >= per) return { ok: false, why: 'aq_species_limit' };
    return { ok: true };
  }

  const api = {
    SHAPES, SPECIES, STAGE_SCALE, MAX_FOOD, REST_SECS, WATER, STR,
    TURN_RATE, PITCH_MAX, TURN_BEND, turnToward, turnRate, turnPhi, spine, frontTurn, finPose, depthScale, depthAlpha, byDepth, pitchFor,
    shape, fishLen, rng, swimBox, syncSim, step, maxSpeed, hungerScale, fedText,
    dropFood, dropFoodAt, fishNear, sinkFood, eat, scatter, fishAt, lightAt, strings, fill, fishLabel, errorText, canAdd,
  };
  if (typeof module !== 'undefined' && module.exports) module.exports = api;
  else root.AquariumCore = api;
})(typeof window !== 'undefined' ? window : globalThis);
