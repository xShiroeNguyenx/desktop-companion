// 🐠 Bể cá full (aquarium-full.html): cá bơi trên hình nền desktop. Dữ liệu
// giống widget (aquarium_get / "aquarium-data"); Rust báo dừng vẽ
// ("aquarium-pause": cửa sổ che kín desktop, toàn màn hình, rời máy, khoá máy)
// và thả thức ăn khi cho ăn từ cửa sổ quản lý ("aquarium-food").
//
// GPU: ~15 khung/giây, chỉ cá + bọt (không tô nước), dừng hẳn khi bị che. Tắt
// chế độ full là Rust huỷ cửa sổ; pagehide thả canvas trước.
(function () {
  const { invoke, listen, log } = window.DC;
  const A = window.AquariumCore;
  const D = window.AquariumDraw;
  const canvas = document.getElementById('sea');
  const ctx = canvas.getContext('2d');
  const reducedQuery = window.matchMedia ? window.matchMedia('(prefers-reduced-motion: reduce)') : null;
  const reduced = () => !!(reducedQuery && reducedQuery.matches);
  const rand = A.rng((Date.now() ^ 0x2545f491) >>> 0);
  // Độ sâu (cảm giác 3D): con xa nhỏ / mờ / chậm hơn tối đa 30% (aquarium-core syncSim).
  const DEPTH = 0.3;
  const FRAME_MS = 66;

  let data = { bg: 'fresh', tank: { fish: [] } };
  let bounds = { w: 0, h: 0, sand: 0 };
  let scene = { weeds: [], bubbles: [] };
  let sim = [];
  let food = [];
  let dpr = 1;
  let scale = 2.5;
  let paused = false;
  let closed = false;
  let timer = 0;
  let lastT = 0;

  /** Cỡ cá theo chiều cao màn hình: ~2.7 lần bể widget trên màn 1080p. */
  function scaleFor(h) {
    return Math.max(1.6, Math.min(4, h / 400));
  }

  function resize() {
    dpr = window.devicePixelRatio || 1;
    const w = window.innerWidth;
    // Chừa taskbar: cá không lặn xuống dưới thanh tác vụ.
    const avail = window.screen && window.screen.availHeight ? Math.min(window.innerHeight, window.screen.availHeight) : window.innerHeight;
    canvas.width = Math.round(w * dpr);
    canvas.height = Math.round(window.innerHeight * dpr);
    bounds = { w, h: Math.max(200, avail - 8), sand: 0 };
    scale = scaleFor(bounds.h);
    sim = A.syncSim([], (data.tank && data.tank.fish) || [], bounds, rand, scale, DEPTH);
    draw();
  }

  function draw() {
    if (closed || !bounds.w) return;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    D.drawSwimmers(ctx, { bounds, scene, bg: data.bg, mode: A.lightAt(new Date().getHours()), reduced: reduced(), food, sim });
    drawLabel();
  }

  // ─── Tương tác (Rust gửi toạ độ physical px từ góc màn hình chính) ─────────
  let cursor = null; // { x, y } CSS px, hoặc null
  let selfFedAt = 0;

  const toCss = (p) => ({ x: p.x / dpr, y: p.y / dpr });

  /** Tên cá gần con trỏ: vẽ thẳng lên canvas (lớp này không có tooltip). */
  function drawLabel() {
    if (!cursor) return;
    const f = A.fishNear(sim, cursor.x, cursor.y, 40);
    if (!f) return;
    const d = ((data.tank && data.tank.fish) || []).find((x) => x.uid === f.uid);
    if (!d) return;
    const kind = (data.names && data.names[d.species]) || A.strings(data.lang).names[d.species] || '';
    const text = d.name ? `${d.name} · ${kind}` : kind;
    if (!text) return;
    ctx.save();
    ctx.font = '600 13px "Segoe UI", system-ui, sans-serif';
    const w = ctx.measureText(text).width + 18;
    const h = 24;
    const x = Math.max(4, Math.min(bounds.w - w - 4, f.x - w / 2));
    const y = Math.max(4, f.y - f.len * 0.55 - h - 6);
    ctx.fillStyle = 'rgba(30, 26, 60, 0.78)';
    ctx.beginPath();
    if (ctx.roundRect) ctx.roundRect(x, y, w, h, 12);
    else ctx.rect(x, y, w, h);
    ctx.fill();
    ctx.fillStyle = '#ffffff';
    ctx.textBaseline = 'middle';
    ctx.fillText(text, x + 9, y + h / 2 + 1);
    ctx.restore();
  }

  /** Bấm vào chỗ trống trên desktop: trúng cá thì cá giật mình, không thì thả thức ăn. */
  async function onClick(p) {
    const c = toCss(p);
    if (A.fishAt(sim, c.x, c.y) || A.fishNear(sim, c.x, c.y, 18)) {
      if (!reduced()) A.scatter(sim, c.x, c.y, 60 * scale);
      start();
      return;
    }
    food = A.dropFoodAt(food, c.x, c.y, bounds, rand, 4);
    start();
    // Lần đầu trong ngày: tính là cho ăn (Miu nói một câu, như widget). Rust sẽ phát
    // "aquarium-food" cho trang này — bỏ qua lần đó, thức ăn đã rơi đúng chỗ bấm.
    if (!(data.tank && data.tank.fedToday)) {
      selfFedAt = Date.now();
      invoke('aquarium_feed').catch((e) => log('[aquarium-full] feed: ' + e));
    }
  }

  function tick() {
    timer = 0;
    if (closed || paused || document.hidden) return;
    const now = performance.now();
    const dt = lastT ? Math.min(0.15, (now - lastT) / 1000) : FRAME_MS / 1000;
    lastT = now;
    const night = A.lightAt(new Date().getHours()) === 'night';
    A.step(sim, dt, { bounds, food, rand, night, reduced: reduced(), speedScale: scale * 0.8, hunger: data.tank && data.tank.hunger });
    food = A.eat(sim, A.sinkFood(food, dt * scale, bounds));
    D.stepBubbles(scene, dt, bounds, [], rand, reduced());
    draw();
    timer = setTimeout(tick, FRAME_MS);
  }

  function start() {
    if (!timer && !paused && !closed) {
      lastT = 0;
      timer = setTimeout(tick, FRAME_MS);
    }
  }

  function stop() {
    clearTimeout(timer);
    timer = 0;
  }

  function receive(v) {
    if (!v) return;
    data = v;
    sim = A.syncSim(sim, (data.tank && data.tank.fish) || [], bounds, rand, scale, DEPTH);
    draw();
  }

  listen('aquarium-data', (e) => receive(e && e.payload));
  listen('aquarium-pause', (e) => {
    paused = !!(e && e.payload);
    if (paused) stop();
    else start();
  });
  listen('aquarium-food', () => {
    if (Date.now() - selfFedAt < 2000) return; // do chính cú bấm trên desktop
    for (let i = 0; i < 4; i++) food = A.dropFood(food, bounds.w * (0.15 + rand() * 0.7), bounds, rand, 3);
    start();
  });
  listen('aquarium-click', (e) => e && e.payload && onClick(e.payload));
  listen('aquarium-cursor', (e) => {
    cursor = e && e.payload ? toCss(e.payload) : null;
  });
  document.addEventListener('visibilitychange', () => (document.hidden ? stop() : start()));
  window.addEventListener('resize', resize);
  window.addEventListener('pagehide', () => {
    closed = true;
    stop();
    canvas.width = 0;
    canvas.height = 0;
  });
  invoke('aquarium_get').then(receive).catch((e) => log('[aquarium-full] aquarium_get: ' + e));
  resize();
  start();
})();
