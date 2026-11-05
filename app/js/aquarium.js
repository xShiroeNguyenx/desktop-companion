// 🐠 Cửa sổ quản lý Bể cá (aquarium.html). Rust giữ mọi thứ (aquarium_pin.rs):
// trang chỉ hiện và gọi lệnh, mỗi lệnh trả lại toàn bộ dữ liệu mới. Lỗi là mã
// aq_* → AquariumCore.errorText. Bể ở cột trái là bản xem trực tiếp (cùng cách
// vẽ với widget), chạy ~20 fps và dừng khi cửa sổ ẩn; đóng là huỷ cửa sổ.
(function () {
  const { invoke, listen, log } = window.DC;
  const A = window.AquariumCore;
  const D = window.AquariumDraw;
  const $ = (id) => document.getElementById(id);
  const el = (tag, cls, text) => {
    const e = document.createElement(tag);
    if (cls) e.className = cls;
    if (text != null) e.textContent = text;
    return e;
  };
  const reducedQuery = window.matchMedia ? window.matchMedia('(prefers-reduced-motion: reduce)') : null;
  const reduced = () => !!(reducedQuery && reducedQuery.matches);
  const rand = A.rng((Date.now() ^ 0x51ed270b) >>> 0);
  // Độ sâu (cảm giác 3D): con xa nhỏ / mờ / chậm hơn tối đa 15% (aquarium-core syncSim).
  const DEPTH = 0.15;

  let data = { lang: 'vi', bg: 'fresh', tank: { fish: [], stored: [], decor: [] }, ownedFish: [], ownedDecor: [], names: {} };
  let tab = 'tank';
  let editing = null; // uid đang đổi tên
  let toastTimer = 0;
  let lastFedNote = 0;

  const S = () => A.strings(data.lang);
  const tank = () => data.tank || { fish: [], stored: [], decor: [] };
  const nameOf = (id) => (data.names && data.names[id]) || (S().names && S().names[id]) || id;

  // ─── Bể xem trực tiếp ───────────────────────────────────────────────────────
  const canvas = $('water');
  const ctx = canvas.getContext('2d');
  let bounds = { w: 0, h: 0, sand: D.SAND };
  let scene = { weeds: [], bubbles: [] };
  let sim = [];
  let food = [];
  let dpr = 1;
  let clock = 0;
  let lastT = 0;
  let timer = 0;
  const decorAssets = () => (tank().decor || []).map((id) => (id ? id.replace(/^tdeco\./, '') : ''));

  function resize() {
    const r = $('tank').getBoundingClientRect();
    // html { zoom } làm rect nhỏ đi: vẽ theo khổ CSS (clientWidth) để hình khớp toạ độ chuột.
    const w = Math.max(1, $('tank').clientWidth || Math.round(r.width));
    const h = Math.max(1, $('tank').clientHeight || Math.round(r.height));
    dpr = (window.devicePixelRatio || 1);
    bounds = { w, h, sand: D.SAND };
    canvas.width = Math.round(w * dpr);
    canvas.height = Math.round(h * dpr);
    scene = D.makeScene(bounds);
    sim = A.syncSim(sim, tank().fish, bounds, rand, 1, DEPTH);
    draw();
  }

  function draw() {
    if (!bounds.w) return;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    D.drawTank(ctx, { bounds, scene, bg: data.bg, mode: A.lightAt(new Date().getHours()), clock, reduced: reduced(), decor: decorAssets(), food, sim });
  }

  function tick() {
    timer = 0;
    if (document.hidden) return;
    const now = performance.now();
    const dt = lastT ? Math.min(0.1, (now - lastT) / 1000) : 0.05;
    lastT = now;
    clock += dt;
    A.step(sim, dt, { bounds, food, rand, night: A.lightAt(new Date().getHours()) === 'night', reduced: reduced() });
    food = A.eat(sim, A.sinkFood(food, dt, bounds));
    D.stepBubbles(scene, dt, bounds, decorAssets(), rand, reduced());
    draw();
    timer = setTimeout(tick, 50);
  }

  function start() {
    if (!timer && !document.hidden) {
      lastT = 0;
      timer = setTimeout(tick, 50);
    }
  }

  function stop() {
    clearTimeout(timer);
    timer = 0;
  }

  // ─── Hiển thị ───────────────────────────────────────────────────────────────

  function toast(text, err) {
    const t = $('toast');
    t.textContent = text;
    t.classList.toggle('err', !!err);
    t.classList.add('on');
    clearTimeout(toastTimer);
    toastTimer = setTimeout(() => t.classList.remove('on'), 2400);
  }

  async function run(cmd, args) {
    try {
      const v = await invoke(cmd, args);
      receive(v);
      return true;
    } catch (e) {
      toast(A.errorText(e, data.lang), true);
      log('[aquarium] ' + cmd + ': ' + e);
      return false;
    }
  }

  function icon(kind, asset, opts) {
    const c = el('canvas');
    // Vẽ sau khi gắn vào DOM (cần clientWidth).
    requestAnimationFrame(() => D.drawIcon(c, kind, asset, opts));
    return c;
  }

  function fishRow(f, where) {
    const s = S();
    const row = el('div', 'row');
    row.append(icon('fish', f.species, { bg: data.bg }));
    const info = el('div', 'info');
    if (editing === f.uid) {
      const inp = el('input');
      inp.maxLength = 20;
      inp.value = f.name || '';
      inp.placeholder = s.namePh;
      inp.addEventListener('keydown', (e) => {
        if (e.key === 'Enter') save();
        if (e.key === 'Escape') {
          e.stopPropagation();
          editing = null;
          render();
        }
      });
      const save = async () => {
        editing = null;
        if (await run('aquarium_rename', { uid: f.uid, name: inp.value })) toast(s.saved);
      };
      info.append(inp);
      requestAnimationFrame(() => inp.focus());
      const ok = el('button', 'act main', s.save);
      ok.type = 'button';
      ok.addEventListener('click', save);
      row.append(info, ok);
      return row;
    }
    info.append(el('div', 'nm', f.name || nameOf(f.species)));
    const age = f.age > 0 ? A.fill(s.age, { n: f.age }) : s.newborn;
    const kind = f.name ? nameOf(f.species) + ' · ' : '';
    info.append(el('div', 'sub', `${kind}${s.stages[f.stage] || s.stages[0]} · ${age} · ${A.fill(s.fedDays, { n: f.fedDays || 0 })}`));
    row.append(info);
    const ren = el('button', 'act', '✏️ ' + s.rename);
    ren.type = 'button';
    ren.addEventListener('click', () => {
      editing = f.uid;
      render();
    });
    row.append(ren);
    const mv = el('button', 'act' + (where === 'stored' ? ' main' : ''), where === 'tank' ? s.store : s.putBack);
    mv.type = 'button';
    if (where === 'stored') {
      const room = (tank().fish || []).length < (tank().max || 8);
      mv.disabled = !room;
      if (!room) mv.title = s.errors.aq_tank_full;
    }
    mv.addEventListener('click', () => run(where === 'tank' ? 'aquarium_store' : 'aquarium_return', { uid: f.uid }));
    row.append(mv);
    return row;
  }

  function renderTank(list) {
    const fish = tank().fish || [];
    if (!fish.length) list.append(el('div', 'empty', S().emptyTank));
    for (const f of fish) list.append(fishRow(f, 'tank'));
  }

  function renderStored(list) {
    const s = S();
    const stored = tank().stored || [];
    if (!stored.length) list.append(el('div', 'note', s.emptyStored));
    for (const f of stored) list.append(fishRow(f, 'stored'));
    list.append(el('div', 'sec', s.ownedTitle));
    for (const sp of data.ownedFish || []) {
      const row = el('div', 'row');
      row.append(icon('fish', sp, { bg: data.bg }));
      const info = el('div', 'info');
      const n = [...(tank().fish || []), ...stored].filter((f) => f.species === sp).length;
      info.append(el('div', 'nm', nameOf(sp)), el('div', 'sub', `${n}/${tank().perSpecies || 3}`));
      row.append(info);
      const can = A.canAdd(tank(), sp);
      const b = el('button', 'act main', '＋ ' + s.add);
      b.type = 'button';
      b.disabled = !can.ok;
      if (!can.ok) b.title = s.errors[can.why] || '';
      b.addEventListener('click', () => run('aquarium_add', { species: sp }));
      row.append(b);
      list.append(row);
    }
    list.append(el('div', 'note', A.fill(s.perSpecies, { n: tank().perSpecies || 3 }) + ' · ' + s.shopHint));
  }

  function renderDecor(list) {
    const s = S();
    // Nền bể: chọn ở Bộ sưu tập (nhóm tank_bg, như skin).
    const bgRow = el('div', 'row');
    bgRow.append(icon('bg', data.bg));
    const info = el('div', 'info');
    info.append(el('div', 'nm', nameOf('tank.' + data.bg)), el('div', 'sub', s.bg));
    bgRow.append(info);
    const ch = el('button', 'act', '🍬 ' + s.changeBg);
    ch.type = 'button';
    ch.addEventListener('click', () => invoke('open_collection_tab', { tab: 'tank_bg' }).catch((e) => log('[aquarium] open bg: ' + e)));
    bgRow.append(ch);
    list.append(bgRow);

    const owned = data.ownedDecor || [];
    if (!owned.length) {
      list.append(el('div', 'note', s.noDecor + ' ' + s.shopHint));
      return;
    }
    const grid = el('div', 'slots');
    const decor = tank().decor || [];
    for (let i = 0; i < 4; i++) {
      const box = el('div', 'row slot');
      const top = el('div', 'top');
      const cur = decor[i] || '';
      top.append(icon(cur ? 'decor' : 'bg', cur ? cur.replace(/^tdeco\./, '') : data.bg, { bg: data.bg }));
      top.append(el('div', 'nm', A.fill(s.slot, { n: i + 1 })));
      box.append(top);
      const sel = el('select');
      const none = el('option', null, s.none);
      none.value = '';
      sel.append(none);
      for (const id of owned) {
        const o = el('option', null, nameOf(id));
        o.value = id;
        o.selected = id === cur;
        sel.append(o);
      }
      sel.addEventListener('change', () => run('aquarium_set_decor', { slot: i, id: sel.value }));
      box.append(sel);
      grid.append(box);
    }
    list.append(grid);
  }

  function render() {
    const s = S();
    const tk = tank();
    document.documentElement.lang = data.lang;
    $('title').textContent = s.title;
    $('subtitle').textContent = s.manage;
    $('shopBtn').textContent = s.openShop;
    $('fullBtn').textContent = data.full ? s.fullOff : s.fullOn;
    $('fullBtn').title = s.fullHint;
    $('closeBtn').title = s.close;
    $('feedBtn').textContent = '🍤 ' + s.feed;
    $('fedState').textContent = tk.fedToday ? '✔ ' + s.fed : s.notFed;
    $('fedState').classList.toggle('hungry', !tk.fedToday);
    $('tank').title = s.dblHint.split('·')[0].trim();
    $('tabTank').textContent = `🐠 ${s.tabTank}`;
    $('tabStored').textContent = `🪣 ${s.tabStored}`;
    $('tabDecor').textContent = `🏰 ${s.tabDecor}`;
    for (const b of $('tabs').querySelectorAll('button')) b.classList.toggle('on', b.dataset.tab === tab);
    $('count').textContent = A.fill(s.count, { n: (tk.fish || []).length, max: tk.max || 8 });
    const list = $('list');
    list.textContent = '';
    if (tab === 'tank') renderTank(list);
    else if (tab === 'stored') renderStored(list);
    else renderDecor(list);
  }

  function receive(v) {
    if (!v) return;
    data = v;
    sim = A.syncSim(sim, tank().fish || [], bounds, rand, 1, DEPTH);
    render();
    draw();
  }

  // ─── Sự kiện ────────────────────────────────────────────────────────────────

  for (const b of $('tabs').querySelectorAll('button')) {
    b.addEventListener('click', () => {
      tab = b.dataset.tab;
      editing = null;
      render();
    });
  }
  const feed = (x) => {
    food = A.dropFood(food, x, bounds, rand, 4);
    // Trả { fed, grew } chứ không phải cả dữ liệu; Rust tự phát "aquarium-data" sau đó.
    if (tank().fedToday) {
      if (Date.now() - lastFedNote > 20000) toast(S().alreadyFed);
      lastFedNote = Date.now();
      return;
    }
    invoke('aquarium_feed').catch((e) => log('[aquarium] feed: ' + e));
  };
  $('feedBtn').addEventListener('click', () => feed(bounds.w * (0.3 + rand() * 0.4)));
  $('tank').addEventListener('click', (e) => {
    const r = canvas.getBoundingClientRect();
    // Toạ độ chuột (đã zoom) → toạ độ vẽ.
    const x = ((e.clientX - r.left) / r.width) * bounds.w;
    const y = ((e.clientY - r.top) / r.height) * bounds.h;
    if (A.fishAt(sim, x, y)) {
      if (!reduced()) A.scatter(sim, x, y);
    } else feed(x);
  });
  $('shopBtn').addEventListener('click', () => invoke('open_collection_tab', { tab: 'fish' }).catch((e) => log('[aquarium] open shop: ' + e)));
  $('fullBtn').addEventListener('click', async () => {
    const want = !data.full;
    try {
      await invoke('aquarium_full_set', { on: want });
      invoke('aquarium_get').then(receive).catch(() => {});
    } catch (e) {
      toast(String(e), true);
      log('[aquarium] full: ' + e);
    }
  });
  const hide = () => invoke('aquarium_close').catch(() => {});
  $('closeBtn').addEventListener('click', hide);
  window.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') hide();
  });
  document.addEventListener('visibilitychange', () => (document.hidden ? stop() : start()));
  window.addEventListener('pagehide', () => {
    stop();
    canvas.width = 0;
    canvas.height = 0;
  });
  if (window.ResizeObserver) new ResizeObserver(resize).observe($('tank'));

  listen('aquarium-data', (e) => receive(e && e.payload));
  invoke('aquarium_get').then(receive).catch((e) => log('[aquarium] aquarium_get: ' + e));
  render();
  resize();
  start();
})();
