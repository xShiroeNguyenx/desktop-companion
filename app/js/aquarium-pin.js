// Widget 🐠 Bể cá (aquarium-pin.html). Rust gửi cá + nền + trang trí + đã cho
// ăn chưa + skin (aquarium_get / "aquarium-data") và báo dừng vẽ
// ("aquarium-pause": có app toàn màn hình, máy khoá, người dùng rời máy).
// Trang chỉ mô phỏng (aquarium-core.js) và vẽ (aquarium-draw.js).
//
// Nhịp vẽ: ~30 fps khi chuột ở trên bể, ~10 fps lúc khác; dừng hẳn khi bị
// dừng / trang ẩn. Gỡ ghim thì Rust huỷ cửa sổ; pagehide thả canvas trước.
(function () {
  const { invoke, listen, log } = window.DC;
  const A = window.AquariumCore;
  const D = window.AquariumDraw;
  const $ = (id) => document.getElementById(id);
  const canvas = $('water');
  const ctx = canvas.getContext('2d');
  const reducedQuery = window.matchMedia ? window.matchMedia('(prefers-reduced-motion: reduce)') : null;
  const rand = A.rng((Date.now() ^ 0x9e3779b9) >>> 0);
  // Độ sâu (cảm giác 3D): con xa nhỏ / mờ / chậm hơn tối đa 15% (aquarium-core syncSim).
  const DEPTH = 0.15;

  let data = { pinTop: true, big: false, lang: 'vi', bg: 'fresh', tank: { fish: [], decor: [], fedToday: false }, paused: false };
  let sim = [];
  let food = [];
  let bounds = { w: 0, h: 0, sand: D.SAND };
  let scene = { weeds: [], bubbles: [] };
  let dpr = 1;
  let hover = false;
  let paused = false;
  let closed = false;
  let timer = 0;
  let raf = 0;
  let lastT = 0;
  let clock = 0;
  let toastTimer = 0;
  let feeding = false;
  let lastFedNote = 0;

  const reduced = () => !!(reducedQuery && reducedQuery.matches);
  const light = () => A.lightAt(new Date().getHours());
  const tank = () => data.tank || { fish: [], decor: [] };
  /** Tên asset của vật trang trí theo ô ("tdeco.castle" → "castle"). */
  const decorAssets = () => (tank().decor || []).map((id) => (id ? id.replace(/^tdeco\./, '') : ''));

  function resize() {
    const rect = $('tank').getBoundingClientRect();
    dpr = window.devicePixelRatio || 1;
    const w = Math.max(1, Math.round(rect.width));
    const h = Math.max(1, Math.round(rect.height));
    if (w === bounds.w && h === bounds.h && canvas.width === Math.round(w * dpr)) return;
    bounds = { w, h, sand: D.SAND };
    canvas.width = Math.round(w * dpr);
    canvas.height = Math.round(h * dpr);
    scene = D.makeScene(bounds);
    sim = A.syncSim(sim, tank().fish, bounds, rand, 1, DEPTH);
    draw();
  }

  function draw() {
    if (!bounds.w || closed) return;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    D.drawTank(ctx, { bounds, scene, bg: data.bg, mode: light(), clock, reduced: reduced(), decor: decorAssets(), food, sim });
  }

  // ─── Vòng vẽ ────────────────────────────────────────────────────────────────

  function tick(t) {
    raf = 0;
    timer = 0;
    if (!running()) return;
    const now = t || performance.now();
    const dt = lastT ? Math.min(0.1, (now - lastT) / 1000) : 1 / 30;
    lastT = now;
    clock += dt;
    A.step(sim, dt, { bounds, food, rand, night: light() === 'night', reduced: reduced(), hunger: tank().hunger });
    food = A.eat(sim, A.sinkFood(food, dt, bounds));
    D.stepBubbles(scene, dt, bounds, decorAssets(), rand, reduced());
    draw();
    schedule();
  }

  function running() {
    return !closed && !paused && !document.hidden && bounds.w > 0;
  }

  function schedule() {
    if (raf || timer || !running()) return;
    // Chuột ở trên bể: rAF (~30 fps nhờ bỏ khung). Không thì hẹn giờ 10 fps,
    // để không đánh thức trang 60 lần mỗi giây khi chẳng ai nhìn kỹ.
    if (hover) raf = requestAnimationFrame((t) => (t - lastT >= 30 ? tick(t) : ((raf = 0), schedule())));
    else timer = setTimeout(() => tick(performance.now()), 100);
  }

  function stop() {
    if (raf) cancelAnimationFrame(raf);
    if (timer) clearTimeout(timer);
    raf = 0;
    timer = 0;
    lastT = 0;
  }

  function setPaused(p) {
    paused = !!p;
    if (paused) stop();
    else schedule();
  }

  // ─── Dữ liệu ────────────────────────────────────────────────────────────────

  function toast(text) {
    const el = $('toast');
    el.textContent = text;
    el.classList.add('on');
    clearTimeout(toastTimer);
    toastTimer = setTimeout(() => el.classList.remove('on'), 1800);
  }

  function renderStatic() {
    const t = A.strings(data.lang);
    const tk = tank();
    $('title').textContent = '🐠 ' + t.title;
    $('fedState').textContent = '🍤 ' + A.fedText(tk, t);
    $('fedState').classList.toggle('hungry', !tk.fedToday);
    $('count').textContent = `${(tk.fish || []).length}/${tk.max || 8}`;
    $('empty').hidden = (tk.fish || []).length > 0;
    $('empty').textContent = t.empty;
    $('feedBtn').title = t.feed;
    $('manageBtn').title = t.manage;
    $('fullBtn').title = `${t.fullOn}\n${t.fullHint}`;
    $('sizeBtn').title = data.big ? t.smaller : t.bigger;
    $('topBtn').title = t.top;
    $('topBtn').classList.toggle('on', !!data.pinTop);
    $('unpinBtn').title = t.unpin;
  }

  function receive(v) {
    if (!v) return;
    data = v;
    document.body.dataset.skin = data.skin || 'lavender';
    window.WidgetDeco.mount($('wrap'), data.deco);
    renderStatic();
    sim = A.syncSim(sim, tank().fish || [], bounds, rand, 1, DEPTH);
    setPaused(data.paused);
    draw();
  }

  async function feed(x) {
    food = A.dropFood(food, x, bounds, rand, 3);
    if (!paused) schedule();
    if (tank().fedToday) {
      // Mỗi ngày chỉ tính (và Miu chỉ nói) một lần: nói rõ cho người bấm thêm.
      if (Date.now() - lastFedNote > 20000) toast(A.strings(data.lang).alreadyFed);
      lastFedNote = Date.now();
      return;
    }
    if (feeding) return;
    feeding = true;
    try {
      const r = await invoke('aquarium_feed');
      if (r && r.grew > 0) toast('✨ ' + A.strings(data.lang).grew);
    } catch (e) {
      log('[aquarium] feed: ' + e);
    } finally {
      feeding = false;
    }
  }

  const openManage = () => invoke('open_aquarium').catch((e) => log('[aquarium] open: ' + e));

  // ─── Sự kiện ────────────────────────────────────────────────────────────────

  function currentWindow() {
    const T = window.__TAURI__ || {};
    return (T.window && T.window.getCurrentWindow && T.window.getCurrentWindow()) || null;
  }
  // Kéo ở khung (đầu thẻ, viền). Lòng bể là để bấm cho ăn / gõ kính.
  $('card').addEventListener('mousedown', (e) => {
    if (e.button !== 0 || e.target.closest('button, .tank')) return;
    const w = currentWindow();
    if (w && w.startDragging) w.startDragging().catch(() => {});
  });

  function localPoint(e) {
    const r = canvas.getBoundingClientRect();
    return { x: e.clientX - r.left, y: e.clientY - r.top };
  }
  $('tank').addEventListener('click', (e) => {
    if (e.detail > 1) return; // bấm đúp: mở cửa sổ quản lý, không thả thức ăn thêm
    const p = localPoint(e);
    // Bấm trúng cá hoặc dưới đáy: gõ kính, cá giật mình. Còn lại: thả thức ăn.
    if (A.fishAt(sim, p.x, p.y) || p.y > bounds.h - D.SAND) {
      if (!reduced()) A.scatter(sim, p.x, p.y);
      schedule();
    } else {
      feed(p.x);
    }
  });
  $('tank').addEventListener('dblclick', openManage);
  $('tank').addEventListener('mousemove', (e) => {
    const p = localPoint(e);
    const f = A.fishAt(sim, p.x, p.y);
    const d = f && (tank().fish || []).find((x) => x.uid === f.uid);
    $('tank').title = d ? A.fishLabel(d, data.lang) : A.strings(data.lang).dblHint;
  });
  $('card').addEventListener('mouseenter', () => {
    hover = true;
    stop();
    schedule();
  });
  $('card').addEventListener('mouseleave', () => {
    hover = false;
    stop();
    schedule();
  });

  $('feedBtn').addEventListener('click', () => {
    for (let i = 0; i < 2; i++) food = A.dropFood(food, 20 + rand() * (bounds.w - 40), bounds, rand, 2);
    feed(bounds.w / 2);
  });
  $('manageBtn').addEventListener('click', openManage);
  $('fullBtn').addEventListener('click', () =>
    invoke('aquarium_full_set', { on: true }).catch((e) => toast(String(e))));
  $('sizeBtn').addEventListener('click', () =>
    invoke('aquarium_size', { big: !data.big }).catch((e) => log('[aquarium] size: ' + e)));
  $('topBtn').addEventListener('click', () =>
    invoke('widget_pin_top', { id: 'aquarium', top: !data.pinTop }).catch((e) => log('[aquarium] top: ' + e)));
  $('unpinBtn').addEventListener('click', () =>
    invoke('widget_pin_set', { id: 'aquarium', pinned: false }).catch((e) => log('[aquarium] unpin: ' + e)));

  document.addEventListener('visibilitychange', () => (document.hidden ? stop() : schedule()));
  // Gỡ ghim: dừng vòng vẽ và thả bộ nhớ canvas trước khi cửa sổ bị huỷ.
  window.addEventListener('pagehide', () => {
    closed = true;
    stop();
    canvas.width = 0;
    canvas.height = 0;
  });
  if (window.ResizeObserver) new ResizeObserver(resize).observe($('tank'));
  else window.addEventListener('resize', resize);

  listen('aquarium-data', (e) => receive(e && e.payload));
  listen('aquarium-pause', (e) => setPaused(e && e.payload));
  invoke('aquarium_get').then(receive).catch((e) => log('[aquarium] aquarium_get: ' + e));
  renderStatic();
  resize();
})();
