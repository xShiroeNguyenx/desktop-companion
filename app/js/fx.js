// Vẽ hiệu ứng thời tiết trong cửa sổ phủ (fx.html). Tham số hạt: FxCore
// (fx-core.js, có test). Rust (fx.rs) gửi "fx-set":
//   { active, kind, intensity, ground, holes: [{x, y, w, h}] }  — CSS px
// active = false (tắt, đang toàn màn hình, rời máy, chạy pin…) thì dừng hẳn
// vòng vẽ và xoá canvas, không chỉ ẩn đi.
(function () {
  const { listen, log } = window.DC;
  const F = window.FxCore;
  const canvas = document.getElementById('fx');
  const ctx = canvas.getContext('2d');
  const reduced = window.matchMedia && window.matchMedia('(prefers-reduced-motion: reduce)').matches;

  let cfg = { active: false, kind: 'rain', intensity: 'medium', lightning: false, ground: 0, holes: [] };
  let flashAt = 0; // lần chớp gần nhất (đồng hồ chung)
  let flashNext = 0; // lần chớp kế tiếp
  let parts = [];
  let snowGround = [];
  let splashes = [];
  let raf = 0;
  let last = 0;
  let clock = 0;
  let W = 0;
  let H = 0;

  function resize() {
    const dpr = window.devicePixelRatio || 1;
    W = window.innerWidth;
    H = window.innerHeight;
    canvas.width = Math.round(W * dpr);
    canvas.height = Math.round(H * dpr);
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    snowGround = F.groundNew(W);
  }

  const ground = () => (cfg.ground > 0 && cfg.ground <= H ? cfg.ground : H);

  function reset() {
    let n = F.particleCount(cfg.kind, cfg.intensity, W, H);
    if (reduced) n = Math.ceil(n / 3);
    parts = [];
    for (let i = 0; i < n; i++) parts.push(F.spawn(cfg.kind, W, H, Math.random, true));
    splashes = [];
    snowGround = F.groundNew(W);
  }

  // ─── Vẽ từng loại ───────────────────────────────────────────────────────
  function drawRain(p) {
    ctx.globalAlpha = 0.25 + 0.45 * p.depth;
    ctx.lineWidth = 0.6 + 0.9 * p.depth;
    const k = p.len / p.vy;
    ctx.beginPath();
    ctx.moveTo(p.x, p.y);
    ctx.lineTo(p.x - p.vx * k, p.y - p.len);
    ctx.stroke();
  }
  function drawSnow(p) {
    ctx.globalAlpha = 0.5 + 0.5 * p.depth;
    ctx.beginPath();
    ctx.arc(p.x, p.y, p.size, 0, Math.PI * 2);
    ctx.fill();
  }
  function drawPetal(p) {
    ctx.save();
    ctx.translate(p.x, p.y);
    ctx.rotate(p.rot);
    // Lật theo thời gian cho giống cánh hoa đang xoay trong gió.
    ctx.scale(1, 0.55 + 0.45 * Math.abs(Math.sin(clock * 2 + p.phase)));
    ctx.globalAlpha = 0.65 + 0.35 * p.depth;
    ctx.fillStyle = p.color;
    ctx.beginPath();
    ctx.moveTo(0, -p.size);
    ctx.bezierCurveTo(p.size * 0.9, -p.size * 0.6, p.size * 0.7, p.size * 0.7, 0, p.size);
    ctx.bezierCurveTo(-p.size * 0.7, p.size * 0.7, -p.size * 0.9, -p.size * 0.6, 0, -p.size);
    ctx.fill();
    ctx.restore();
  }
  function drawLeaf(p) {
    ctx.save();
    ctx.translate(p.x, p.y);
    ctx.rotate(p.rot);
    ctx.scale(1, 0.6 + 0.4 * Math.abs(Math.cos(clock * 1.7 + p.phase)));
    ctx.globalAlpha = 0.75 + 0.25 * p.depth;
    ctx.fillStyle = p.color;
    const s = p.size;
    ctx.beginPath();
    ctx.moveTo(-s, 0);
    ctx.quadraticCurveTo(0, -s * 0.7, s, 0);
    ctx.quadraticCurveTo(0, s * 0.7, -s, 0);
    ctx.fill();
    ctx.globalAlpha *= 0.6;
    ctx.strokeStyle = 'rgba(90, 40, 10, 0.6)';
    ctx.lineWidth = 0.8;
    ctx.beginPath();
    ctx.moveTo(-s, 0);
    ctx.lineTo(s, 0);
    ctx.stroke();
    ctx.restore();
  }

  // Độ đậm của sương / nắng theo cường độ.
  const strength = () => ({ light: 0.6, medium: 1, heavy: 1.4 })[cfg.intensity] || 1;

  function drawFog(p) {
    const g = ctx.createRadialGradient(p.x, p.y, 0, p.x, p.y, p.size);
    const a = Math.min(0.32, 0.18 * p.depth * strength());
    g.addColorStop(0, 'rgba(235, 238, 248, ' + a + ')');
    g.addColorStop(1, 'rgba(235, 238, 248, 0)');
    ctx.globalAlpha = 1;
    ctx.fillStyle = g;
    ctx.fillRect(p.x - p.size, p.y - p.size, p.size * 2, p.size * 2);
  }
  function drawSun(p) {
    const k = strength();
    const glow = ctx.createRadialGradient(p.x, p.y, 0, p.x, p.y, p.size);
    glow.addColorStop(0, 'rgba(255, 236, 170, ' + 0.38 * k + ')');
    glow.addColorStop(0.45, 'rgba(255, 214, 140, ' + 0.12 * k + ')');
    glow.addColorStop(1, 'rgba(255, 214, 140, 0)');
    ctx.globalAlpha = 1;
    ctx.fillStyle = glow;
    ctx.fillRect(p.x - p.size, p.y, p.size, p.size);
    // Tia nắng từ góc, toả xuống bên trái (góc 90°–180°), lắc rất chậm.
    const len = p.size * 1.6;
    const spread = 0.05;
    for (let i = 0; i < 7; i++) {
      const a = Math.PI / 2 + ((i + 0.5) / 7) * (Math.PI / 2) + Math.sin(clock * 0.05 + i) * 0.04;
      const ray = ctx.createLinearGradient(p.x, p.y, p.x + Math.cos(a) * len, p.y + Math.sin(a) * len);
      ray.addColorStop(0, 'rgba(255, 240, 190, ' + 0.1 * k + ')');
      ray.addColorStop(1, 'rgba(255, 240, 190, 0)');
      ctx.fillStyle = ray;
      ctx.beginPath();
      ctx.moveTo(p.x, p.y);
      ctx.lineTo(p.x + Math.cos(a - spread) * len, p.y + Math.sin(a - spread) * len);
      ctx.lineTo(p.x + Math.cos(a + spread) * len, p.y + Math.sin(a + spread) * len);
      ctx.closePath();
      ctx.fill();
    }
  }
  function drawStar(p) {
    ctx.globalAlpha = F.twinkle(p, clock) * (0.5 + 0.5 * p.depth);
    ctx.fillStyle = '#fff8e1';
    ctx.beginPath();
    ctx.arc(p.x, p.y, p.size, 0, Math.PI * 2);
    ctx.fill();
    if (p.size > 2) {
      // Sao to có thêm 4 cánh sáng.
      ctx.strokeStyle = '#fff8e1';
      ctx.lineWidth = 0.7;
      const r = p.size * 3;
      ctx.beginPath();
      ctx.moveTo(p.x - r, p.y);
      ctx.lineTo(p.x + r, p.y);
      ctx.moveTo(p.x, p.y - r);
      ctx.lineTo(p.x, p.y + r);
      ctx.stroke();
    }
  }

  function frame(now) {
    raf = requestAnimationFrame(frame);
    if (now - last < F.FRAME_MS) return;
    const dt = Math.min(0.1, (now - last) / 1000 || 0.033);
    last = now;
    clock += dt;
    const g = ground();

    ctx.clearRect(0, 0, W, H);
    if (cfg.kind === 'rain') {
      ctx.strokeStyle = 'rgba(190, 215, 255, 1)';
      ctx.lineCap = 'round';
    } else if (cfg.kind === 'snow') {
      ctx.fillStyle = '#ffffff';
    }

    for (let i = 0; i < parts.length; i++) {
      const p = F.step(parts[i], dt, clock);
      if (F.gone(p, W, g)) {
        if (p.y > g && p.x >= 0 && p.x <= W && !F.inHoles(cfg.holes, p.x, g - 2)) {
          if (cfg.kind === 'rain' && p.depth > 0.6 && splashes.length < 60) splashes.push({ x: p.x, y: g, t: 0 });
          if (cfg.kind === 'snow') F.groundAdd(snowGround, p.x, 0.7 * p.depth);
        }
        parts[i] = F.spawn(cfg.kind, W, H, Math.random, false);
        continue;
      }
      if (cfg.kind === 'rain') drawRain(p);
      else if (cfg.kind === 'snow') drawSnow(p);
      else if (cfg.kind === 'sakura') drawPetal(p);
      else if (cfg.kind === 'fog') drawFog(p);
      else if (cfg.kind === 'sun') drawSun(p);
      else if (cfg.kind === 'stars') drawStar(p);
      else drawLeaf(p);
    }

    // Mưa bắn tóe ở mặt đất (mép trên taskbar).
    if (splashes.length) {
      ctx.strokeStyle = 'rgba(200, 225, 255, 1)';
      ctx.lineWidth = 1;
      splashes = splashes.filter((s) => {
        s.t += dt;
        if (s.t > 0.3) return false;
        ctx.globalAlpha = 0.6 * (1 - s.t / 0.3);
        const r = 2 + 10 * s.t;
        ctx.beginPath();
        ctx.ellipse(s.x, s.y - 1, r, r * 0.35, 0, Math.PI, Math.PI * 2);
        ctx.stroke();
        return true;
      });
    }

    // Tuyết đọng thành dải mỏng, tan dần.
    if (cfg.kind === 'snow') {
      F.groundMelt(snowGround, dt * 0.15);
      ctx.globalAlpha = 0.92;
      ctx.fillStyle = '#ffffff';
      ctx.beginPath();
      ctx.moveTo(0, g);
      for (let i = 0; i < snowGround.length; i++) ctx.lineTo(i * F.GROUND_BIN, g - snowGround[i]);
      ctx.lineTo(W, g);
      ctx.closePath();
      ctx.fill();
    }

    // Dông: chớp sáng nhẹ cả màn hình, thưa (≥ 8 giây), không có khi giảm chuyển động.
    if (cfg.lightning && !reduced) {
      if (!flashNext) flashNext = F.nextFlash(clock, Math.random);
      if (clock >= flashNext) {
        flashAt = clock;
        flashNext = F.nextFlash(clock, Math.random);
      }
      const a = F.flashAlpha(clock - flashAt);
      if (a > 0) {
        ctx.globalAlpha = 1;
        ctx.fillStyle = 'rgba(235, 240, 255, ' + a + ')';
        ctx.fillRect(0, 0, W, H);
      }
    } else {
      flashNext = 0;
    }

    // Chừa chỗ cho bảng của app (Cài đặt, Công việc, game…): không vẽ đè lên.
    ctx.globalAlpha = 1;
    for (const r of cfg.holes || []) ctx.clearRect(r.x, r.y, r.w, r.h);
  }

  function start() {
    if (raf) return;
    last = 0;
    raf = requestAnimationFrame(frame);
  }
  function stop() {
    if (raf) cancelAnimationFrame(raf);
    raf = 0;
    ctx.clearRect(0, 0, W, H);
  }

  function apply(next) {
    const prev = cfg;
    cfg = { ...cfg, ...next, holes: Array.isArray(next.holes) ? next.holes : cfg.holes };
    if (!F.KINDS.includes(cfg.kind)) cfg.kind = 'rain';
    if (!cfg.active) return stop();
    if (!raf || prev.kind !== cfg.kind || prev.intensity !== cfg.intensity || !prev.active) reset();
    start();
  }

  window.addEventListener('resize', () => {
    resize();
    if (cfg.active) reset();
  });
  resize();
  listen('fx-set', (e) => {
    try {
      apply((e && e.payload) || {});
    } catch (err) {
      log('[fx] ' + err);
    }
  });
})();
