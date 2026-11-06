// 🐠 Vẽ bể cá lên canvas 2D: nước, cát, rong, nền bể, vật trang trí, bọt, thức
// ăn, cá. Dùng chung cho widget (aquarium-pin.js), cửa sổ quản lý (aquarium.js)
// và thẻ xem trước trong Bộ sưu tập (collection.js). Mô phỏng bơi, hình dáng
// loài: aquarium-core.js. Hình vẽ bằng code, không có ảnh (PLAN.md 18.1).
//
// Mỗi nền bể (Bộ sưu tập, nhóm tank_bg) và mỗi vật trang trí (nhóm tank_deco)
// phải có một khoá trong BG / DECOR dưới đây — test fish_catalog_matches_aquarium_species
// (collection.rs) đọc file này để kiểm.
(function (root) {
  const A = root.AquariumCore;
  const SAND = 14;

  // ─── Nền bể ───────────────────────────────────────────────────────────────
  const BG = {
    fresh: {
      water: { day: A.WATER.day, dusk: A.WATER.dusk, night: A.WATER.night },
      sand: { day: '#e9d3a4', dusk: '#c7a98f', night: '#5a5478' },
      weeds: ['#4fbf7a', '#3aa36a'],
      nightWeed: '#2f6a5a',
      rays: true,
    },
    reef: {
      water: {
        day: ['#8af0e6', '#2bb3c0', '#147894'],
        dusk: ['#9fb8e6', '#4f7fb8', '#26457a'],
        night: ['#26406e', '#182a52', '#0d1838'],
      },
      sand: { day: '#f6cfc6', dusk: '#d9a9a6', night: '#5e5070' },
      weeds: ['#ff8fb1', '#ffb36b'],
      nightWeed: '#7a4a6a',
      rays: true,
      backdrop: 'coral',
    },
    glow: {
      // Lúc nào cũng tối; rong và cá toả sáng.
      water: {
        day: ['#4a3a8e', '#2c2066', '#171040'],
        dusk: ['#3e2f7c', '#241a58', '#130d38'],
        night: ['#30236a', '#1c1448', '#0d0828'],
      },
      sand: { day: '#4a3f6e', dusk: '#433864', night: '#3a305a' },
      weeds: ['#6ff7e8', '#b98cff'],
      nightWeed: '#6ff7e8',
      rays: false,
      glow: true,
    },
  };

  function bgOf(id) {
    return BG[id] || BG.fresh;
  }

  // ─── Vật trang trí (ô ở đáy bể) ───────────────────────────────────────────
  /** Tâm x của ô `i` (0..3) trên bể rộng `w`. */
  function slotX(i, w) {
    return [0.13, 0.37, 0.63, 0.87][i] * w;
  }

  function decorSize(bounds) {
    return Math.max(22, Math.min(48, bounds.h * 0.3));
  }

  const DECOR = {
    castle: (ctx, x, base, s, t) => {
      ctx.fillStyle = '#b9b2c9';
      ctx.fillRect(x - s * 0.42, base - s * 0.55, s * 0.84, s * 0.55);
      ctx.fillRect(x - s * 0.5, base - s * 0.85, s * 0.26, s * 0.85);
      ctx.fillRect(x + s * 0.24, base - s * 0.85, s * 0.26, s * 0.85);
      ctx.fillStyle = '#9a92ad';
      for (const tx of [x - s * 0.5, x + s * 0.24]) {
        for (let k = 0; k < 3; k++) ctx.fillRect(tx + k * s * 0.1, base - s * 0.95, s * 0.06, s * 0.1);
      }
      ctx.fillStyle = '#ff8fb8';
      ctx.beginPath();
      ctx.moveTo(x + s * 0.37, base - s * 0.85);
      ctx.lineTo(x + s * 0.37, base - s * 1.1);
      ctx.lineTo(x + s * 0.37 + s * 0.18, base - s * 1.02 + Math.sin(t * 2) * 1.5);
      ctx.closePath();
      ctx.fill();
      ctx.fillStyle = '#3b3550';
      ctx.beginPath();
      ctx.arc(x, base - s * 0.2, s * 0.14, Math.PI, 0);
      ctx.fillRect(x - s * 0.14, base - s * 0.2, s * 0.28, s * 0.2);
      ctx.fill();
    },
    chest: (ctx, x, base, s) => {
      const w = s * 0.8;
      const h = s * 0.42;
      ctx.fillStyle = '#9a5b32';
      ctx.fillRect(x - w / 2, base - h, w, h);
      ctx.fillStyle = '#ffd36e';
      ctx.beginPath();
      ctx.ellipse(x, base - h, w * 0.42, h * 0.28, 0, Math.PI, 0);
      ctx.fill();
      ctx.save();
      ctx.translate(x - w / 2, base - h);
      ctx.rotate(-0.45);
      ctx.fillStyle = '#b06a3b';
      ctx.fillRect(0, -h * 0.45, w, h * 0.45);
      ctx.restore();
      ctx.fillStyle = '#e8b64a';
      ctx.fillRect(x - w / 2, base - h * 0.6, w, h * 0.12);
      ctx.fillRect(x - s * 0.05, base - h * 0.75, s * 0.1, h * 0.3);
    },
    coral: (ctx, x, base, s, t) => {
      ctx.lineCap = 'round';
      const sway = Math.sin(t * 0.8) * 1.5;
      for (const [dx, h, c, w] of [[-0.25, 0.7, '#ff7aa8', 5], [0, 0.95, '#ff9f6b', 6], [0.25, 0.65, '#ff7aa8', 5]]) {
        ctx.strokeStyle = c;
        ctx.lineWidth = w * s / 40;
        ctx.beginPath();
        ctx.moveTo(x + dx * s, base);
        ctx.quadraticCurveTo(x + dx * s + sway, base - h * s * 0.5, x + dx * s * 1.4 + sway, base - h * s);
        ctx.stroke();
        ctx.beginPath();
        ctx.moveTo(x + dx * s + sway * 0.5, base - h * s * 0.45);
        ctx.lineTo(x + dx * s - s * 0.12 + sway, base - h * s * 0.7);
        ctx.stroke();
      }
    },
    bubbler: (ctx, x, base, s) => {
      ctx.fillStyle = '#8f9bb0';
      ctx.beginPath();
      ctx.ellipse(x, base - s * 0.1, s * 0.28, s * 0.14, 0, 0, Math.PI * 2);
      ctx.fill();
      ctx.fillStyle = '#b7c2d4';
      ctx.beginPath();
      ctx.ellipse(x - s * 0.06, base - s * 0.15, s * 0.12, s * 0.05, 0, 0, Math.PI * 2);
      ctx.fill();
    },
  };

  // ─── Cảnh (rong, bọt) ─────────────────────────────────────────────────────
  /** Rong cố định theo cỡ bể (không nhảy chỗ khi đổi cỡ). */
  function makeScene(bounds) {
    const r = A.rng(11);
    const n = Math.max(3, Math.round(bounds.w / 70));
    const weeds = Array.from({ length: n }, (_, i) => ({
      x: ((i + 0.5) / n) * bounds.w + (r() - 0.5) * 30,
      h: bounds.h * (0.3 + r() * 0.35),
      phase: r() * Math.PI * 2,
      tone: r() < 0.5 ? 0 : 1,
    }));
    return { weeds, bubbles: [] };
  }

  /** Bọt nổi lên. Có máy sủi bọt thì bọt ra đều ở đó, không thì lác đác từ rong. */
  function stepBubbles(scene, dt, bounds, decor, rand, reduced) {
    if (reduced) {
      scene.bubbles = [];
      return;
    }
    const bubblers = (decor || []).map((d, i) => (d === 'bubbler' ? slotX(i, bounds.w) : null)).filter((x) => x != null);
    const rate = bubblers.length ? 6 : 1.5;
    if (scene.bubbles.length < (bubblers.length ? 24 : 12) && rand() < dt * rate) {
      const src = bubblers.length
        ? bubblers[Math.floor(rand() * bubblers.length)] + (rand() - 0.5) * 4
        : scene.weeds.length ? scene.weeds[Math.floor(rand() * scene.weeds.length)].x + 10 : rand() * bounds.w;
      scene.bubbles.push({ x: src, y: bounds.h - SAND - 2, r: 1.2 + rand() * 2, p: rand() * 6 });
    }
    for (const b of scene.bubbles) {
      b.y -= (14 + b.r * 6) * dt;
      b.p += dt * 3;
    }
    scene.bubbles = scene.bubbles.filter((b) => b.y > -4);
  }

  // ─── Vẽ ───────────────────────────────────────────────────────────────────
  function drawWater(ctx, bounds, bg, mode, clock) {
    const [a, b, c] = bg.water[mode];
    const g = ctx.createLinearGradient(0, 0, 0, bounds.h);
    g.addColorStop(0, a);
    g.addColorStop(0.55, b);
    g.addColorStop(1, c);
    ctx.fillStyle = g;
    ctx.fillRect(0, 0, bounds.w, bounds.h);
    if (bg.rays && mode !== 'night') {
      ctx.fillStyle = 'rgba(255,255,255,0.10)';
      for (let i = 0; i < 3; i++) {
        const x = ((clock * 6 + i * 97) % (bounds.w + 60)) - 30;
        ctx.beginPath();
        ctx.moveTo(x, 0);
        ctx.lineTo(x + 18, 0);
        ctx.lineTo(x + 50, bounds.h * 0.8);
        ctx.lineTo(x + 36, bounds.h * 0.8);
        ctx.closePath();
        ctx.fill();
      }
    }
  }

  /** Rạn san hô mờ phía sau (nền reef). */
  function drawBackdrop(ctx, bounds, bg) {
    if (bg.backdrop !== 'coral') return;
    const base = bounds.h - SAND + 2;
    ctx.save();
    ctx.globalAlpha = 0.35;
    const r = A.rng(5);
    for (let i = 0; i < 6; i++) {
      const x = (i + 0.3 + r() * 0.4) * (bounds.w / 6);
      const h = bounds.h * (0.25 + r() * 0.3);
      ctx.fillStyle = ['#ff9fc2', '#ffc28a', '#c9a0ff'][i % 3];
      ctx.beginPath();
      ctx.ellipse(x, base, 10 + r() * 10, h, 0, Math.PI, 0);
      ctx.fill();
    }
    ctx.restore();
  }

  function drawSand(ctx, bounds, bg, mode) {
    const top = bounds.h - SAND;
    ctx.fillStyle = bg.sand[mode];
    ctx.beginPath();
    ctx.moveTo(0, bounds.h);
    ctx.lineTo(0, top + 3);
    for (let x = 0; x <= bounds.w; x += 20) ctx.lineTo(x, top + Math.sin(x * 0.05) * 2.5);
    ctx.lineTo(bounds.w, bounds.h);
    ctx.closePath();
    ctx.fill();
  }

  function drawWeeds(ctx, bounds, scene, bg, mode, clock, reduced) {
    const sway = reduced ? 0.3 : 1;
    ctx.save();
    ctx.lineCap = 'round';
    if (bg.glow) {
      ctx.shadowBlur = 8;
    }
    for (const wd of scene.weeds) {
      const base = bounds.h - SAND + 3;
      const bend = Math.sin(clock * 0.9 + wd.phase) * 8 * sway;
      const color = mode === 'night' && !bg.glow ? bg.nightWeed : bg.weeds[wd.tone];
      ctx.strokeStyle = color;
      if (bg.glow) ctx.shadowColor = color;
      ctx.lineWidth = 4;
      ctx.beginPath();
      ctx.moveTo(wd.x, base);
      ctx.quadraticCurveTo(wd.x - bend, base - wd.h * 0.5, wd.x + bend, base - wd.h);
      ctx.stroke();
      ctx.lineWidth = 3;
      ctx.beginPath();
      ctx.moveTo(wd.x + 5, base);
      ctx.quadraticCurveTo(wd.x + 5 + bend, base - wd.h * 0.35, wd.x + 5 - bend * 0.6, base - wd.h * 0.7);
      ctx.stroke();
    }
    ctx.restore();
  }

  function drawDecor(ctx, bounds, decor, clock) {
    const s = decorSize(bounds);
    const base = bounds.h - SAND + 4;
    (decor || []).forEach((d, i) => {
      const fn = DECOR[d];
      if (fn) {
        ctx.save();
        fn(ctx, slotX(i, bounds.w), base, s, clock);
        ctx.restore();
      }
    });
  }

  function drawBubbles(ctx, scene) {
    ctx.strokeStyle = 'rgba(255,255,255,0.7)';
    ctx.lineWidth = 1;
    for (const b of scene.bubbles) {
      ctx.beginPath();
      ctx.arc(b.x + Math.sin(b.p) * 2, b.y, b.r, 0, Math.PI * 2);
      ctx.stroke();
    }
  }

  function drawFood(ctx, food) {
    ctx.fillStyle = '#c9783a';
    for (const p of food || []) {
      ctx.beginPath();
      ctx.arc(p.x, p.y, 1.8, 0, Math.PI * 2);
      ctx.fill();
    }
  }

  // ─── Ảnh cá (thay hình vẽ bằng code cho loài có ảnh) ──────────────────────
  // Khuôn ảnh: nhìn ngang, ĐẦU QUAY SANG PHẢI, nền trong suốt, cắt sát con cá,
  // rộng ~256 px, ≤ 80 KB (aquarium-draw.test.js kiểm). Loài chưa có ảnh vẫn vẽ
  // bằng code. joint = chỗ đuôi nối thân, tính từ mép trái (0..1): phần bên
  // trái lắc theo nhịp bơi, phần bên phải đứng yên. cy = tâm thân theo chiều dọc.
  // wag (tuỳ chọn) = biên độ lắc, mặc định 0.16: loài có vây lưng / bụng kéo dài qua
  // điểm nối (cá thần tiên) để nhỏ, không thì thấy vết cắt trên vây.
  //
  // Tuỳ chọn, cá có lớp rời (khuôn: docs/aquarium/fish-sprite-guide.png):
  // - parts: vây quạt riêng. Ảnh thân đã XOÁ các vây này; mỗi vây là một PNG cắt sát,
  //   cắt từ CHÍNH ảnh cá gốc (cùng tỉ lệ). x, y, w = hộp của vây; px, py = gốc vây (trục
  //   xoay); tất cả tính theo phần của ảnh thân (0..1). rot = biên độ xoay (rad), flap =
  //   co bề ngang tối đa (0..1), speed = nhịp so với nhịp đuôi, lag = lệch pha (rad).
  //   under = vây nằm SAU thân (thân che gốc vây): ảnh vây có thêm một đoạn gốc giấu dưới
  //   thân, quạt mạnh mà không hở khe; dùng cho vây gắn dọc mép thân / chui ra từ dưới váy.
  //   Vây được ghép lên thân trước, rồi cả con mới uốn dải / quay đầu như thường.
  // - front: ảnh nhìn thẳng (đối xứng), hoà vào giữa vòng quay đầu. h = cao so với ảnh
  //   thân, cy = tâm mặt theo chiều dọc của ảnh thẳng. Loài có front phải có turnSec trong
  //   aquarium-core SHAPES (quay chậm để kịp hoà ảnh; test kiểm).
  const SPRITES = {
    'fish.goldfish': { src: 'media/aquarium/fish/fish.goldfish.png', joint: 0.32, cy: 0.52 },
    'fish.guppy': {
      src: 'media/aquarium/fish/fish.guppy.png',
      joint: 0.366,
      cy: 0.525,
      parts: [
        { src: 'media/aquarium/fish/fish.guppy.anal.png', x: 0.4277, y: 0.6775, w: 0.2317, px: 0.6059, py: 0.6884, rot: 0.09, flap: 0, speed: 1, lag: 1.7 },
        { src: 'media/aquarium/fish/fish.guppy.pelvic.png', x: 0.604, y: 0.692, w: 0.1287, px: 0.705, py: 0.6993, rot: 0.13, flap: 0, speed: 1, lag: 0.9 },
        { src: 'media/aquarium/fish/fish.guppy.pectoral.png', x: 0.6673, y: 0.5036, w: 0.1386, px: 0.7941, py: 0.5688, rot: 0.22, flap: 0.28, speed: 1.7, lag: 0 },
      ],
      front: { src: 'media/aquarium/fish/fish.guppy.front.png', h: 1.1, cy: 0.6 },
    },
    'fish.betta': { src: 'media/aquarium/fish/fish.betta.png', joint: 0.44, cy: 0.45 },
    'fish.molly': { src: 'media/aquarium/fish/fish.molly.png', joint: 0.39, cy: 0.55 },
    'fish.angel': { src: 'media/aquarium/fish/fish.angel.png', joint: 0.38, cy: 0.47, wag: 0.05 },
    'fish.koi': { src: 'media/aquarium/fish/fish.koi.png', joint: 0.26, cy: 0.54, wag: 0.12 },
    // Nàng tiên cá: vây đuôi lắc nhẹ (tóc bắt đầu ngay sau điểm nối); ba vây nhỏ nằm sau
    // thân quạt riêng; giữa vòng quay hiện ảnh nhìn thẳng (cả người, đuôi buông xuống).
    'fish.miu': {
      src: 'media/aquarium/fish/fish.miu.png',
      joint: 0.269,
      cy: 0.566,
      wag: 0.1,
      parts: [
        { src: 'media/aquarium/fish/fish.miu.dorsal.png', x: 0.2968, y: 0.3387, w: 0.2144, px: 0.4847, py: 0.5037, rot: 0.09, flap: 0, speed: 1, lag: 0.8, under: true },
        { src: 'media/aquarium/fish/fish.miu.anal.png', x: 0.2138, y: 0.6215, w: 0.2049, px: 0.4071, py: 0.7334, rot: 0.08, flap: 0, speed: 1, lag: 1.6, under: true },
        { src: 'media/aquarium/fish/fish.miu.hip.png', x: 0.4364, y: 0.7025, w: 0.1736, px: 0.599, py: 0.7467, rot: 0.12, flap: 0.1, speed: 1, lag: 0, under: true },
      ],
      // Mắt ảnh thẳng ngang mắt ảnh nghiêng (không nhảy lúc hoà); đuôi buông thấp hơn thân.
      front: { src: 'media/aquarium/fish/fish.miu.front.png', h: 1.45, cy: 0.39 },
    },
  };
  const loaded = {};

  /** Ảnh theo đường dẫn (nạp một lần), hoặc null khi chưa xong. */
  function image(src, onReady) {
    let img = loaded[src];
    if (!img) {
      img = new Image();
      img.decoding = 'async';
      img.src = src;
      loaded[src] = img;
    }
    if (img.complete && img.naturalWidth > 0) return img;
    if (onReady) img.addEventListener('load', onReady, { once: true });
    return null;
  }

  /**
   * Ảnh đã nạp xong của loài, hoặc null (chưa có ảnh / đang nạp / không có Image).
   * Loài có vây rời: chỉ sẵn sàng khi thân và mọi vây đã nạp (thân không có vây trông
   * cụt). Ảnh thẳng thì không bắt buộc: chưa có thì quay đầu kiểu cũ.
   */
  function sprite(species, onReady) {
    const meta = SPRITES[species];
    if (!meta || typeof Image === 'undefined') return null;
    const img = image(meta.src, onReady);
    const parts = (meta.parts || []).map((p) => ({ p, img: image(p.src, onReady) }));
    if (!img || parts.some((x) => !x.img)) return null;
    const front = meta.front ? image(meta.front.src) : null;
    return { img, meta, parts, front };
  }

  /** Canvas ghép thân + vây, mỗi loài một cái (vẽ lần lượt từng con, không song song). */
  const finCanvas = {};

  /**
   * Thân với các vây ở tư thế lúc này (A.finPose), cùng cỡ ảnh thân, để uốn dải / quay
   * đầu dùng y như ảnh gốc. Không có vây rời, hoặc không có document (node test): ảnh thân.
   */
  function withFins(sp, f, reduced) {
    const { img, meta, parts } = sp;
    if (!parts || !parts.length || typeof document === 'undefined') return img;
    const iw = img.naturalWidth;
    const ih = img.naturalHeight;
    let c = finCanvas[meta.src];
    if (!c) {
      const canvas = document.createElement('canvas');
      c = finCanvas[meta.src] = { canvas, ctx: canvas.getContext('2d') };
    }
    if (c.canvas.width !== iw || c.canvas.height !== ih) {
      c.canvas.width = iw;
      c.canvas.height = ih;
    }
    const g = c.ctx;
    g.setTransform(1, 0, 0, 1, 0, 0);
    g.clearRect(0, 0, iw, ih);
    const fin = ({ p, img: pi }) => {
      const pose = A.finPose(p, f.wag, reduced);
      const px = p.px * iw;
      const py = p.py * ih;
      const w = p.w * iw;
      const h = (w * pi.naturalHeight) / pi.naturalWidth;
      g.save();
      g.translate(px, py);
      g.rotate(pose.angle);
      g.scale(pose.scaleX, 1);
      g.drawImage(pi, p.x * iw - px, p.y * ih - py, w, h);
      g.restore();
    };
    parts.filter((x) => x.p.under).forEach(fin); // sau thân: thân che gốc vây
    g.drawImage(img, 0, 0, iw, ih);
    parts.filter((x) => !x.p.under).forEach(fin);
    return c.canvas;
  }

  /**
   * Vẽ cá bằng ảnh. f.len = dài THÂN (miệng → gốc đuôi), giống hình vẽ bằng
   * code, để bơi / đớp mồi / bấm trúng không đổi; đuôi trong ảnh thò ra sau.
   */
  function drawSpriteFish(ctx, f, sp, opts) {
    const { img, meta } = sp;
    const iw = img.naturalWidth;
    const ih = img.naturalHeight;
    const W = f.len / (1 - meta.joint);
    const H = (W * ih) / iw;
    const top = -H * meta.cy;
    const amp = (meta.wag != null ? meta.wag : 0.16) * (opts.reduced ? 0.3 : 1);
    // Uốn thân: sóng chạy từ đầu (u = 1, đứng yên) xuống đuôi (u = 0, uốn mạnh nhất).
    // Biên độ ~10% chiều cao ảnh ở mức mặc định; vùng uốn kéo tới quá gốc đuôi một
    // đoạn để đuôi nối mềm vào thân (không còn "khớp" cứng).
    const A0 = (amp / 0.16) * H * 0.1;
    const reach = Math.min(1, meta.joint + 0.45);
    const phase = f.wag || 0;
    // Quay đầu: xoay quanh trục dọc, thân cong chữ C, phối cảnh (aquarium-core spine).
    // Loài có ảnh thẳng: quay chậm, hoà dần sang ảnh thẳng, dừng, hoà lại (frontTurn).
    const ft = sp.front ? A.frontTurn(f) : null;
    const phi = ft ? ft.phi : A.turnPhi(f);
    const turning = Math.sin(phi) > 0.03;
    const segs = A.spine(phi, W, STRIPS, f.len);
    const facingRight = Math.cos(phi) >= 0;
    const mix = ft ? ft.mix : 0;
    const alpha = A.depthAlpha(f);
    const body = mix < 0.99 ? withFins(sp, f, opts.reduced) : null;

    ctx.save();
    ctx.translate(f.x, f.y);
    ctx.rotate(-(f.pitch || 0) * (facingRight ? 1 : -1));
    // Hoà chéo không "ma": hình đang chiếm ưu thế giữ đặc, hình kia hiện dần lên trên.
    ctx.globalAlpha = alpha * Math.min(1, 2 * (1 - mix));
    if (opts.mode === 'night' && !opts.glow) ctx.filter = 'brightness(0.72)';
    if (opts.glow) {
      ctx.shadowBlur = 8;
      ctx.shadowColor = A.shape(f.species).fin;
    }

    // Đang quay: vẽ vào canvas phụ để phủ tối dải đang nghiêng (source-atop chỉ phủ lên
    // chính con cá), rồi mới đưa lên. Bơi thẳng: vẽ thẳng (không tốn canvas phụ).
    const scratch = turning && body ? scratchCanvas(W * 2.2, H * 2.2) : null;
    const c = scratch ? scratch.ctx : ctx;
    const ox = scratch ? scratch.w / 2 : 0;
    const oy = scratch ? scratch.h / 2 : 0;
    if (scratch) {
      c.setTransform(scratch.r, 0, 0, scratch.r, 0, 0);
      c.clearRect(0, 0, scratch.w, scratch.h);
    }
    const order = !body ? [] : turning ? [...segs].sort((a, b) => a.z - b.z) : segs; // xa trước
    for (const g of order) {
      const uc = (g.u0 + g.u1) / 2;
      const env = Math.pow(Math.max(0, 1 - uc / reach), 1.4);
      const off = A0 * env * Math.sin(phase - (1 - uc) * 2.4) * g.s;
      const left = Math.min(g.pa, g.pb);
      // Chồng 0.8 px (đích) / nửa pixel (nguồn) để không hở vạch giữa các dải.
      const w = Math.abs(g.pa - g.pb) + 0.8;
      const flip = g.pb > g.pa; // dải đang quay lưng (mặt sau ảnh = gương)
      const sx = g.u0 * iw;
      const sw = Math.min(iw - sx, (g.u1 - g.u0) * iw + 0.5);
      c.save();
      c.translate(ox + (flip ? left + w : left), oy);
      c.scale(flip ? -1 : 1, 1);
      c.drawImage(body, sx, 0, sw, ih, 0, top * g.s + off, w, H * g.s);
      if (scratch) {
        const shade = (1 - Math.abs(Math.cos(g.yaw))) * TURN_SHADE;
        if (shade > 0.01) {
          c.globalCompositeOperation = 'source-atop';
          c.fillStyle = `rgba(8, 10, 30, ${shade.toFixed(3)})`;
          c.fillRect(0, top * g.s + off, w, H * g.s);
          c.globalCompositeOperation = 'source-over';
        }
      }
      c.restore();
    }
    if (scratch) ctx.drawImage(scratch.canvas, 0, 0, scratch.canvas.width, scratch.canvas.height, -ox, -oy, scratch.w, scratch.h);
    if (mix > 0.01) {
      // Ảnh thẳng: tâm mặt đặt đúng tâm thân, cao theo ảnh thân (meta.front.h).
      const fi = sp.front;
      const fh = H * (meta.front.h || 0.8);
      const fw = (fh * fi.naturalWidth) / fi.naturalHeight;
      ctx.globalAlpha = alpha * Math.min(1, 2 * mix);
      ctx.drawImage(fi, -fw / 2, -fh * (meta.front.cy || 0.5), fw, fh);
    }
    ctx.restore();
  }

  /** Độ tối tối đa của dải đang nằm nghiêng lúc quay (bề mặt cong quay ra xa thì tối). */
  const TURN_SHADE = 0.55;

  /** Một canvas phụ dùng chung cho mọi con cá đang quay (vẽ lần lượt, không song song). */
  let scratchCache = null;
  function scratchCanvas(w, h) {
    if (typeof document === 'undefined') return null;
    const r = Math.min(2, (typeof window !== 'undefined' && window.devicePixelRatio) || 1);
    const W2 = Math.ceil(w);
    const H2 = Math.ceil(h);
    if (!scratchCache) {
      const canvas = document.createElement('canvas');
      scratchCache = { canvas, ctx: canvas.getContext('2d') };
    }
    const s = scratchCache;
    if (s.canvas.width < W2 * r || s.canvas.height < H2 * r) {
      s.canvas.width = Math.max(s.canvas.width, Math.ceil(W2 * r));
      s.canvas.height = Math.max(s.canvas.height, Math.ceil(H2 * r));
    }
    s.w = s.canvas.width / r;
    s.h = s.canvas.height / r;
    s.r = r;
    return s;
  }

  /**
   * Hướng cho cá vẽ bằng code (không có ảnh để chia dải): bóp ngang theo góc xoay,
   * cùng nhịp nhanh-ở-giữa với cá có ảnh. Cộng ngóc / chúi theo hướng bơi.
   */
  function orient(ctx, f) {
    const c = Math.cos(A.turnPhi(f));
    ctx.scale((c < 0 ? -1 : 1) * Math.max(0.12, Math.abs(c)), 1);
    ctx.rotate(-(f.pitch || 0));
    return 1 - Math.abs(c);
  }

  /** Số dải dọc khi vẽ cá bằng ảnh (uốn thân, phối cảnh lúc quay đầu). */
  const STRIPS = 18;

  /**
   * Bóng mờ dưới cá: widget đổ xuống cát (floorY, càng gần đáy càng đậm); bể full
   * không có đáy nên đổ ngay dưới thân, như cá nổi trên hình nền.
   */
  function drawShadow(ctx, f, floorY, h, mode) {
    const near = floorY != null ? 1 - Math.max(0, Math.min(1, (floorY - f.y) / Math.max(1, h))) : 1;
    const a = (mode === 'night' ? 0.06 : 0.13) * near * A.depthAlpha(f);
    if (a < 0.01) return;
    const y = floorY != null ? floorY : f.y + f.len * 0.75;
    const rx = f.len * 0.55 * (0.7 + 0.3 * near);
    const ry = Math.max(2, f.len * 0.1);
    ctx.save();
    ctx.translate(f.x, y);
    ctx.scale(1, ry / rx);
    const g = ctx.createRadialGradient(0, 0, 0, 0, 0, rx);
    g.addColorStop(0, `rgba(10, 20, 40, ${a.toFixed(3)})`);
    g.addColorStop(1, 'rgba(10, 20, 40, 0)');
    ctx.fillStyle = g;
    ctx.beginPath();
    ctx.arc(0, 0, rx, 0, Math.PI * 2);
    ctx.fill();
    ctx.restore();
  }

  /** Một con cá ở (f.x, f.y), quay theo f.dir. opts: { mode, reduced, glow, onReady }. */
  function drawFish(ctx, f, opts) {
    const sp = sprite(f.species, opts && opts.onReady);
    if (sp) return drawSpriteFish(ctx, f, sp, opts || {});
    const mode = (opts && opts.mode) || 'day';
    const reduced = !!(opts && opts.reduced);
    const s = A.shape(f.species);
    const L = f.len;
    const rx = L / 2;
    const ry = Math.max(3, (L * s.h) / 2);
    const wag = Math.sin(f.wag || 0) * (reduced ? 0.15 : 0.35);
    ctx.save();
    ctx.translate(f.x, f.y);
    orient(ctx, f);
    ctx.globalAlpha = A.depthAlpha(f) * (mode === 'night' ? 0.85 : 1);
    if (opts && opts.glow) {
      ctx.shadowBlur = 7;
      ctx.shadowColor = s.fin;
    }

    // Đuôi (vẽ trước thân).
    ctx.save();
    ctx.translate(-rx * 0.85, 0);
    ctx.rotate(wag);
    ctx.fillStyle = s.fin;
    ctx.beginPath();
    if (s.tail === 'long') {
      ctx.moveTo(0, 0);
      ctx.bezierCurveTo(-rx * 0.6, -ry * 1.6, -rx * 1.3, -ry * 1.2, -rx * 1.2, 0);
      ctx.bezierCurveTo(-rx * 1.3, ry * 1.2, -rx * 0.6, ry * 1.6, 0, 0);
    } else if (s.tail === 'fork') {
      ctx.moveTo(0, 0);
      ctx.lineTo(-rx * 0.6, -ry * 0.9);
      ctx.lineTo(-rx * 0.4, 0);
      ctx.lineTo(-rx * 0.6, ry * 0.9);
    } else if (s.tail === 'round') {
      ctx.ellipse(-rx * 0.3, 0, rx * 0.35, ry * 0.7, 0, 0, Math.PI * 2);
    } else {
      ctx.moveTo(0, 0);
      ctx.quadraticCurveTo(-rx * 0.7, -ry * 1.3, -rx * 0.75, -ry * 0.2);
      ctx.lineTo(-rx * 0.55, 0);
      ctx.lineTo(-rx * 0.75, ry * 0.2);
      ctx.quadraticCurveTo(-rx * 0.7, ry * 1.3, 0, 0);
    }
    ctx.closePath();
    ctx.fill();
    ctx.restore();

    // Vây lưng / vây bụng.
    ctx.fillStyle = s.fin;
    ctx.beginPath();
    if (f.species === 'fish.angel') {
      ctx.moveTo(-rx * 0.1, -ry * 0.8);
      ctx.lineTo(-rx * 0.6, -ry * 1.9);
      ctx.lineTo(rx * 0.2, -ry * 0.9);
      ctx.moveTo(-rx * 0.1, ry * 0.8);
      ctx.lineTo(-rx * 0.6, ry * 1.9);
      ctx.lineTo(rx * 0.2, ry * 0.9);
    } else {
      ctx.moveTo(-rx * 0.3, -ry * 0.85);
      ctx.quadraticCurveTo(0, -ry * 1.5, rx * 0.25, -ry * 0.9);
    }
    ctx.closePath();
    ctx.fill();

    // Thân: tô chuyển từ lưng xuống bụng.
    const g = ctx.createLinearGradient(0, -ry, 0, ry);
    g.addColorStop(0, s.body);
    g.addColorStop(1, s.belly);
    ctx.fillStyle = g;
    ctx.beginPath();
    if (f.species === 'fish.angel') {
      ctx.moveTo(rx, 0);
      ctx.quadraticCurveTo(rx * 0.2, -ry, -rx * 0.8, 0);
      ctx.quadraticCurveTo(rx * 0.2, ry, rx, 0);
    } else {
      ctx.ellipse(0, 0, rx, ry, 0, 0, Math.PI * 2);
    }
    ctx.fill();
    ctx.shadowBlur = 0;

    drawPattern(ctx, s, f, rx, ry);

    // Tai mèo + ria (cá mèo Miu).
    if (s.ears) {
      ctx.fillStyle = s.body;
      for (const ex of [rx * 0.15, rx * 0.5]) {
        ctx.beginPath();
        ctx.moveTo(ex - 4, -ry * 0.8);
        ctx.lineTo(ex, -ry * 1.45);
        ctx.lineTo(ex + 4, -ry * 0.75);
        ctx.closePath();
        ctx.fill();
      }
      ctx.strokeStyle = 'rgba(255,255,255,0.8)';
      ctx.lineWidth = 0.8;
      for (const dy of [-1.5, 1.5]) {
        ctx.beginPath();
        ctx.moveTo(rx * 0.85, dy);
        ctx.lineTo(rx * 1.25, dy * 2.2);
        ctx.stroke();
      }
    }

    // Mắt: ban đêm cá ngủ thì nhắm (một vạch).
    const ex = rx * 0.55;
    const ey = -ry * 0.2;
    const er = Math.max(1.4, L * 0.05);
    if (mode === 'night') {
      ctx.strokeStyle = '#1b1b2a';
      ctx.lineWidth = 1;
      ctx.beginPath();
      ctx.moveTo(ex - er, ey);
      ctx.lineTo(ex + er, ey);
      ctx.stroke();
    } else {
      ctx.fillStyle = '#fff';
      ctx.beginPath();
      ctx.arc(ex, ey, er * 1.2, 0, Math.PI * 2);
      ctx.fill();
      ctx.fillStyle = '#1b1b2a';
      ctx.beginPath();
      ctx.arc(ex + er * 0.25, ey, er * 0.7, 0, Math.PI * 2);
      ctx.fill();
    }
    ctx.restore();
  }

  function drawPattern(ctx, s, f, rx, ry) {
    ctx.save();
    ctx.beginPath();
    ctx.ellipse(0, 0, rx, ry, 0, 0, Math.PI * 2);
    ctx.clip();
    if (s.pattern === 'stripe') {
      ctx.fillStyle = f.species === 'fish.clown' ? '#ffffff' : 'rgba(60,50,40,0.55)';
      for (const x of [rx * 0.35, -rx * 0.15, -rx * 0.62]) ctx.fillRect(x - rx * 0.08, -ry, rx * 0.16, ry * 2);
    } else if (s.pattern === 'neon') {
      ctx.fillStyle = '#3df2ff';
      ctx.fillRect(-rx, -ry * 0.25, rx * 2, ry * 0.3);
      ctx.fillStyle = '#ff3b5c';
      ctx.fillRect(-rx, ry * 0.1, rx * 1.1, ry * 0.5);
    } else if (s.pattern === 'spots') {
      ctx.fillStyle = 'rgba(255,111,177,0.7)';
      for (const [x, y] of [[-0.4, -0.2], [0, 0.3], [0.3, -0.3]]) {
        ctx.beginPath();
        ctx.arc(rx * x, ry * y, ry * 0.25, 0, Math.PI * 2);
        ctx.fill();
      }
    } else if (s.pattern === 'koi') {
      ctx.fillStyle = '#ff5a2a';
      for (const [x, y, r] of [[0.45, -0.3, 0.5], [-0.25, -0.1, 0.6], [-0.75, 0.3, 0.35]]) {
        ctx.beginPath();
        ctx.ellipse(rx * x, ry * y, rx * r * 0.5, ry * r, 0, 0, Math.PI * 2);
        ctx.fill();
      }
    }
    ctx.restore();
  }

  /**
   * Vẽ cả bể một khung. st: { bounds, scene, bg, mode, clock, reduced, decor, food, sim }.
   * Gọi ctx.setTransform(dpr…) trước.
   */
  function drawTank(ctx, st) {
    const bg = bgOf(st.bg);
    const { bounds, scene, mode, clock = 0, reduced = false } = st;
    drawWater(ctx, bounds, bg, mode, clock);
    drawBackdrop(ctx, bounds, bg);
    drawWeeds(ctx, bounds, scene, bg, mode, clock, reduced);
    drawSand(ctx, bounds, bg, mode);
    drawDecor(ctx, bounds, st.decor, clock);
    drawFood(ctx, st.food);
    // Xa trước, gần sau; bóng đổ xuống cát (không vẽ ở nền phát sáng: đáy tối sẵn).
    const fish = A.byDepth(st.sim);
    if (!bg.glow) for (const f of fish) drawShadow(ctx, f, bounds.h - SAND + 2, bounds.h, mode);
    for (const f of fish) drawFish(ctx, f, { mode, reduced, glow: bg.glow, onReady: st.onReady });
    drawBubbles(ctx, scene);
    if (mode === 'night' && !bg.glow) {
      ctx.fillStyle = 'rgba(10, 12, 40, 0.18)';
      ctx.fillRect(0, 0, bounds.w, bounds.h);
    }
  }

  /**
   * Bể full trên hình nền: KHÔNG vẽ nước / cát / rong (nền trong suốt, hình nền
   * desktop lộ ra), chỉ cá, thức ăn, bọt. st: { bounds, scene, mode, reduced, food, sim, bg }.
   */
  function drawSwimmers(ctx, st) {
    const bg = bgOf(st.bg);
    ctx.clearRect(0, 0, st.bounds.w, st.bounds.h);
    drawFood(ctx, st.food);
    // Bể full không có đáy: bóng ngay dưới thân, như cá nổi trên hình nền.
    const fish = A.byDepth(st.sim);
    for (const f of fish) drawShadow(ctx, f, null, st.bounds.h, st.mode);
    for (const f of fish) drawFish(ctx, f, { mode: st.mode, reduced: st.reduced, glow: bg.glow, onReady: st.onReady });
    drawBubbles(ctx, st.scene);
  }

  /**
   * Ảnh tĩnh cho thẻ / danh sách: một con cá giữa nền nước, hoặc một bể nhỏ
   * có nền / vật trang trí. kind: 'fish' | 'bg' | 'decor'.
   */
  function drawIcon(canvas, kind, asset, opts) {
    // Ảnh cá chưa nạp xong: vẽ tạm bằng code, nạp xong thì vẽ lại một lần.
    const redraw = () => drawIcon(canvas, kind, asset, opts);
    const dpr = (typeof window !== 'undefined' && window.devicePixelRatio) || 1;
    const w = canvas.clientWidth || canvas.width || 120;
    const h = canvas.clientHeight || canvas.height || 80;
    canvas.width = Math.round(w * dpr);
    canvas.height = Math.round(h * dpr);
    const ctx = canvas.getContext('2d');
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    const bounds = { w, h, sand: SAND };
    const scene = makeScene(bounds);
    const bgId = kind === 'bg' ? asset : (opts && opts.bg) || 'fresh';
    const decor = kind === 'decor' ? ['', asset, '', ''] : [];
    let sim = [];
    if (kind === 'fish') {
      const len = Math.min(w * 0.55, h * 1.1, A.fishLen(asset, 2) * 1.8);
      sim = [{ species: asset, len, x: w / 2, y: h * 0.45, dir: 1, wag: 0.6 }];
    } else if (kind === 'bg') {
      sim = [
        { species: 'fish.goldfish', len: h * 0.3, x: w * 0.35, y: h * 0.4, dir: 1, wag: 0.4 },
        { species: 'fish.goldfish', len: h * 0.24, x: w * 0.68, y: h * 0.55, dir: -1, wag: 1.2 },
      ];
    }
    if (kind === 'decor') {
      // Vật trang trí phóng to giữa bể.
      ctx.save();
      drawTank(ctx, { bounds, scene, bg: bgId, mode: 'day', decor: [], sim: [], onReady: redraw });
      const s = Math.min(h * 0.7, w * 0.6);
      const fn = DECOR[asset];
      if (fn) fn(ctx, w / 2, h - SAND + 4, s, 0);
      ctx.restore();
      return;
    }
    drawTank(ctx, { bounds, scene, bg: bgId, mode: 'day', decor, sim, onReady: redraw });
  }

  root.AquariumDraw = { SAND, BG, DECOR, SPRITES, bgOf, slotX, decorSize, makeScene, stepBubbles, drawTank, drawSwimmers, drawFish, drawIcon };
})(typeof window !== 'undefined' ? window : globalThis);
