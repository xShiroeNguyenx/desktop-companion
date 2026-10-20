// Trang 🍬 Bộ sưu tập (collection.html). Rust: collection_get (kẹo, catalog,
// trạng thái) và các lệnh collection_buy / collection_select /
// collection_toggle_effect / collection_preview_fx; mọi thay đổi (kể cả kẹo
// vừa cộng khi xong việc) tới qua sự kiện collection-changed. Pet tự khoe món
// mới (Rust nói), trang chỉ hiện một toast ngắn.
(function () {
  const { invoke, listen, log } = window.DC;
  const C = window.CollectionCore;
  const $ = (id) => document.getElementById(id);

  let data = { candy: 0, today: 0, todayCap: 35, items: [], lang: 'vi', pet: '', you: '' };
  let tab = 'skin';
  // Chế độ: 'gacha' (🎰 Candy Capsule) · 'shop' (đổi thẳng) · 'history'.
  let mode = 'gacha';
  try {
    tab = localStorage.getItem('collectionTab') || tab;
    mode = localStorage.getItem('collectionMode') || mode;
  } catch (_) { /* không có storage cũng không sao */ }
  if (!C.MODES.includes(mode)) mode = 'gacha';
  let toastTimer = null;
  // Đang quay (chờ Rust + hoạt cảnh): chặn bấm thêm.
  let busy = false;
  // Lần quay cuối (1 hoặc 10) cho nút "Quay tiếp".
  let lastCount = 1;

  const S = () => C.strings(data.lang);
  const petName = () => (data.pet && String(data.pet).trim()) || 'Miu';

  function el(tag, cls, text) {
    const e = document.createElement(tag);
    if (cls) e.className = cls;
    if (text != null) e.textContent = text;
    return e;
  }

  function toast(text, err) {
    const t = $('toast');
    t.textContent = text;
    t.classList.toggle('err', !!err);
    t.hidden = false;
    clearTimeout(toastTimer);
    toastTimer = setTimeout(() => { t.hidden = true; }, 2600);
  }

  // fixed = ảnh của đúng một bộ (ảnh đại diện): miu-skin.js không đổi sang bộ đang dùng.
  function img(src, cls, fixed) {
    const i = el('img', cls);
    if (fixed) i.setAttribute('data-miu-fixed', '');
    i.src = src;
    i.alt = '';
    i.draggable = false;
    return i;
  }

  /** Widget thu nhỏ đúng màu skin: giờ, một hàng ô lịch (ô hôm nay tô màu nhấn), Miu mờ. */
  function miniWidget(asset) {
    const m = el('div', 'mini');
    m.style.background = C.swatch(asset);
    m.style.color = C.swatchInk(asset);
    m.append(el('div', 't', '12:34'));
    const cells = el('div', 'cells');
    for (let i = 0; i < 14; i++) {
      const c = el('i');
      c.style.background = i === 9 ? C.swatchAccent(asset) : 'rgba(255, 255, 255, 0.28)';
      cells.append(c);
    }
    m.append(cells, img((data.miuDir || 'media/game') + '/rps-wink.' + (data.miuExt || 'png')));
    return m;
  }

  function preview(item) {
    const pv = el('div', 'pv');
    if (item.cat === 'skin') {
      pv.append(miniWidget(item.asset));
    } else if (item.cat === 'effect') {
      pv.append(el('span', 'ic', C.effectIcon(item.asset)));
    } else if (item.cat === 'bubble') {
      // Bong bóng thoại thu nhỏ đúng kiểu (CSS .bub-* trong collection.html).
      pv.append(el('span', 'bub bub-' + item.asset, 'Xin chào~'));
    } else if (item.cat === 'flower') {
      pv.append(el('span', 'flw flw-' + item.asset, C.flowerGlyph(item.asset)));
    } else if (item.cat === 'side_set' || item.cat === 'miu_set') {
      if (item.cat === 'side_set') pv.classList.add('side');
      pv.append(img(C.thumb(item), 'full', true));
    } else if (item.cat === 'model') {
      // Model Live2D: ảnh preview khi có, chưa có thì biểu tượng trên nền cầu vồng.
      pv.classList.add('model');
      if (C.thumb(item)) pv.append(img(C.thumb(item), 'full', true));
      else pv.append(el('span', 'ic', '🎭'));
    } else {
      pv.append(el('span', 'ic', '💬'));
    }
    if (item.selected || item.on) pv.append(el('span', 'badge', '✔ ' + (item.on ? S().on : S().using)));
    if (!item.owned) pv.append(el('span', 'lock', '🔒'));
    return pv;
  }

  function card(item) {
    const c = el('article', 'item' + (item.selected || item.on ? ' selected' : ''));
    c.append(preview(item));
    const name = el('div', 'name', item.name);
    name.append(el('span', 'rar ' + C.rarityClass(item.rarity), S().rarity[item.rarity] || item.rarity));
    c.append(name, el('div', 'desc', item.desc || ''));
    const acts = el('div', 'acts');
    const st = C.buttonState(item, data.candy, data.lang);
    const b = el('button', 'act ' + st.kind, st.label);
    b.type = 'button';
    b.disabled = st.disabled;
    b.addEventListener('click', () => act(item, st.kind));
    acts.append(b);
    if (item.cat === 'model' && item.owned && item.update && item.progress == null) {
      const u = el('button', 'act ghost', C.fill(S().update, { v: item.update }));
      u.type = 'button';
      u.addEventListener('click', () => act(item, 'download'));
      acts.append(u);
    }
    if (item.cat === 'effect') {
      const p = el('button', 'act ghost', S().preview);
      p.type = 'button';
      p.addEventListener('click', () => invoke('collection_preview_fx', { kind: item.asset }).catch((e) => toast(C.errorText(String(e), data.lang), true)));
      acts.append(p);
    }
    c.append(acts);
    return c;
  }

  async function act(item, kind) {
    if (kind === 'download') {
      // Tải có thể lâu: không chờ ở đây, tiến độ tới qua sự kiện model-install.
      item.progress = 0;
      renderShop();
      invoke('model_install', { id: item.asset })
        .then(() => toast(C.fill(S().installed, { item: item.name })))
        .catch((e) => {
          toast(C.errorText(String(e), data.lang), true);
          log('[collection] model_install ' + item.asset + ': ' + e);
        })
        .finally(refresh);
      return;
    }
    try {
      if (kind === 'buy') {
        await invoke('collection_buy', { id: item.id });
        toast(C.fill(S().bought, { item: item.name, you: data.you || '' }));
      } else if (kind === 'use' && item.cat === 'model') {
        // Đổi model của pet: Rust lưu pet_model và báo cửa sổ pet nạp lại.
        await invoke('pet_set_model', { id: item.asset });
      } else if (kind === 'use') {
        await invoke('collection_select', { category: item.cat, id: item.id });
      } else if (kind === 'on' || kind === 'off') {
        await invoke('collection_toggle_effect', { id: item.id, on: kind === 'off' });
      }
    } catch (e) {
      // Mã lỗi từ Rust (not_enough:12…) → câu đọc được; lỗi lạ hiện nguyên văn.
      toast(C.errorText(String(e), data.lang), true);
      log('[collection] ' + kind + ' ' + item.id + ': ' + e);
    }
    // Rust phát collection-changed; gọi lại cho chắc (vd. lỗi giữa chừng).
    refresh();
  }

  function renderHead() {
    const s = S();
    const pet = petName();
    document.documentElement.lang = data.lang;
    $('title').textContent = s.title;
    $('subtitle').textContent = C.fill(s.subtitle, { pet });
    $('bubble').textContent = C.fill(mode === 'gacha' ? s.g.bubble : s.bubble, { pet, Pet: pet });
    $('candyLabel').textContent = s.candyLabel;
    $('candy').textContent = String(data.candy || 0);
    $('candyWord').textContent = s.candy;
    $('todayLabel').textContent = s.todayLabel;
    const got = data.today || 0;
    const cap = data.todayCap || 0;
    $('todayNums').textContent = C.fill(s.today, { n: got, cap });
    $('todayBar').style.width = cap ? Math.min(100, Math.round((got * 100) / cap)) + '%' : '0';
    $('closeBtn').title = s.close;
    $('howTitle').textContent = s.howTitle;
    // Chi tiết (trần mỗi ngày, +200 mỗi cột mốc) để trong tooltip, dải dưới gọn.
    $('howTitle').title = s.howTo;
    const chips = $('chips');
    chips.textContent = '';
    for (const e of s.earn) chips.append(el('span', 'chip', e));
    $('noMoney').textContent = s.noMoney;
    $('devTools').hidden = !data.debug;
  }

  function setMode(m) {
    mode = m;
    try { localStorage.setItem('collectionMode', mode); } catch (_) { /* bỏ qua */ }
    render();
  }

  function renderModes() {
    const s = S();
    const nav = $('modes');
    nav.textContent = '';
    const icons = { gacha: '🎰', shop: '🛍️', history: '📜' };
    for (const m of C.MODES) {
      const b = el('button', 'mode' + (m === mode ? ' on' : ''));
      b.type = 'button';
      b.append(el('span', null, icons[m]), el('span', null, s.modes[m]));
      b.addEventListener('click', () => setMode(m));
      nav.append(b);
    }
    $('shopView').hidden = mode !== 'shop';
    $('gachaView').hidden = mode !== 'gacha';
    $('histView').hidden = mode !== 'history';
  }

  function render() {
    renderHead();
    renderModes();
    if (mode === 'gacha') renderGacha();
    else if (mode === 'history') renderHistory();
    else renderShop();
  }

  const gacha = () => data.gacha || { enabled: false, cost1: 100, cost10: 1000, rates: [], refund: [], history: [] };
  const findItem = (id) => (data.items || []).find((i) => i.id === id);

  /** Màn máy kẹo: chỉ cập nhật chữ/số; máy (DOM tĩnh) không bị dựng lại giữa hoạt cảnh. */
  function renderGacha() {
    const s = S();
    const g = gacha();
    $('gSign').textContent = s.g.sign;
    $('gSignSub').textContent = s.g.signSub;
    for (const [n, cost] of [[1, g.cost1], [10, g.cost10]]) {
      const st = C.pullState(cost, data.candy, g.enabled, data.lang);
      $('pull' + n + 'Label').textContent = st.ok ? s.g['pull' + n] : st.label;
      $('pull' + n + 'Cost').textContent = String(cost);
      $('pull' + n).disabled = !st.ok || busy;
    }
    $('gLocked').hidden = !!g.enabled;
    $('gLocked').textContent = C.fill(s.g.locked, { n: g.minPool || 15 });

    $('ratesTitle').textContent = s.g.ratesTitle;
    const rates = $('rates');
    rates.textContent = '';
    const star = { common: '🌸', rare: '💜', epic: '🌟', legendary: '🌈' };
    for (const r of g.rates || []) {
      const row = el('div', 'rate ' + r.rarity);
      const txt = el('div');
      txt.append(el('b', null, s.rarity[r.rarity] || r.rarity), el('small', null, s.g.rateDesc[r.rarity] || ''));
      row.append(el('span', 'st', star[r.rarity] || '⭐'), txt, el('em', null, r.pct + '%'));
      rates.append(row);
    }
    // Huyền thoại: mỗi model một tỉ lệ riêng, ghi rõ từng cái.
    for (const l of g.legendary || []) {
      rates.append(el('div', 'rate-item', C.fill(s.g.legendItem, { name: l.name, pct: l.pct, c: l.refund })));
    }
    const pity = $('pity');
    pity.textContent = '';
    pity.append(el('span', null, C.fill(s.g.pityRare, { n: g.rareIn })), el('span', null, C.fill(s.g.pityEpic, { n: g.epicIn })));
    const ref = Object.fromEntries((g.refund || []).map((x) => [x.rarity, x.candy]));
    $('dupNote').textContent = C.fill(s.g.dupNote, { c: ref.common, r: ref.rare, e: ref.epic });

    $('featTitle').textContent = '⭐ ' + s.g.featured;
    $('seeAll').textContent = s.g.seeAll;
    const row = $('featRow');
    row.textContent = '';
    for (const it of C.featured(data.items, 6)) {
      const b = el('button', 'fc' + (it.owned ? ' owned' : ''));
      b.type = 'button';
      b.title = it.desc || '';
      b.append(preview(it), el('span', 'rar solid ' + C.rarityClass(it.rarity), s.rarity[it.rarity] || it.rarity), el('span', 'fn', it.name));
      // Bấm món → mở đúng nhóm trong cửa hàng.
      b.addEventListener('click', () => {
        tab = it.cat;
        try { localStorage.setItem('collectionTab', tab); } catch (_) { /* bỏ qua */ }
        setMode('shop');
      });
      row.append(b);
    }
  }

  function renderHistory() {
    const s = S();
    const g = gacha();
    $('histTitle').textContent = s.g.histTitle;
    $('histTotal').textContent = C.fill(s.g.histTotal, { n: g.pulls || 0 });
    const list = $('histList');
    list.textContent = '';
    const hist = g.history || [];
    if (!hist.length) list.append(el('div', 'empty', s.g.histEmpty));
    for (const h of hist) {
      const r = el('div', 'hrow');
      r.append(
        el('time', null, C.formatTime(h.t, data.lang)),
        el('span', 'rar ' + C.rarityClass(h.rarity), s.rarity[h.rarity] || h.rarity),
        el('span', 'hn', h.name),
        el('span', 'hs ' + (h.dup ? 'dup' : 'new'), h.dup ? s.g.histDup : s.g.histNew),
      );
      list.append(r);
    }
  }

  const FX = window.GachaFx;

  async function pull(n) {
    if (busy) return;
    busy = true;
    lastCount = n;
    $('pull1').disabled = $('pull10').disabled = true;
    let res;
    try {
      res = await invoke('collection_pull', { count: n });
    } catch (e) {
      busy = false;
      toast(C.errorText(String(e), data.lang), true);
      log('[collection] pull ' + n + ': ' + e);
      refresh();
      return;
    }
    const sum = C.summarize(res.results);
    // Hoạt cảnh theo hạng cao nhất: máy quay rồi mở capsule (bấm để bỏ qua).
    const m = $('machine');
    try {
      await FX.spin(m, sum.best);
      // Món hiếm nhất (ưu tiên món mới) hiện ra lúc capsule bung.
      const rank = C.RARITY_RANK;
      const best = [...res.results].sort((x, y) => (rank[y.rarity] || 0) - (rank[x.rarity] || 0) || Number(x.dup) - Number(y.dup))[0];
      const bestItem = best && findItem(best.id);
      await FX.reveal($('reveal'), sum.best, S().g, bestItem ? preview(bestItem) : null);
    } catch (e) {
      log('[collection] fx: ' + e);
    }
    m.classList.remove('spin', 'spin-rare', 'spin-epic', 'spin-legendary');
    busy = false;
    receive(res.view);
    showResults(res.results, sum);
  }

  function showResults(results, sum) {
    const s = S();
    $('gresTitle').textContent = '🎉 ' + s.g.resultTitle;
    const grid = $('gresGrid');
    grid.textContent = '';
    grid.classList.toggle('one', results.length === 1);
    results.forEach((r, i) => {
      const it = findItem(r.id) || { id: r.id, name: r.id, cat: '', rarity: r.rarity };
      const c = el('div', 'rc ' + r.rarity);
      c.style.setProperty('--i', String(i));
      const tags = el('div', 'tags');
      tags.append(el('span', 'rar ' + C.rarityClass(r.rarity), s.rarity[r.rarity] || r.rarity));
      tags.append(r.dup ? el('span', 'tg dup', C.fill(s.g.dupTag, { n: r.refund })) : el('span', 'tg new', s.g.newTag));
      if (r.pity) tags.append(el('span', 'tg pity', s.g.pityTag));
      c.append(preview(it), el('div', 'rn', it.name), tags);
      grid.append(c);
    });
    // Không có món trùng thì không nhắc chuyện hoàn kẹo.
    const parts = [C.fill(s.g.sumNew, { n: sum.fresh })];
    if (sum.refund > 0) parts.push(C.fill(s.g.sumRefund, { c: sum.refund }));
    $('gresSum').textContent = parts.join(' · ');
    const g = gacha();
    const cost = lastCount === 10 ? g.cost10 : g.cost1;
    const again = $('gresAgain');
    again.textContent = C.fill(s.g.again, { c: cost });
    again.disabled = !C.pullState(cost, data.candy, g.enabled, data.lang).ok;
    $('gresOk').textContent = s.g.ok;
    $('gres').hidden = false;
  }

  function closeResults() {
    $('gres').hidden = true;
  }

  function renderShop() {
    const s = S();
    const groups = C.groupByCategory(data.items);
    if (!groups.some(([c]) => c === tab)) tab = groups.length ? groups[0][0] : 'skin';
    const tabs = $('tabs');
    tabs.textContent = '';
    for (const [cat, list] of groups) {
      const b = el('button', 'tab' + (cat === tab ? ' on' : ''));
      b.type = 'button';
      b.append(el('span', 'ti', C.TAB_ICONS[cat] || '•'), el('span', null, s.tabs[cat] || cat));
      b.append(el('span', 'n', `${list.filter((i) => i.owned).length}/${list.length}`));
      b.addEventListener('click', () => {
        tab = cat;
        try { localStorage.setItem('collectionTab', tab); } catch (_) { /* bỏ qua */ }
        renderShop();
      });
      tabs.append(b);
    }
    const grid = $('grid');
    grid.textContent = '';
    if (tab === 'side_set' && data.sideGirls === false) grid.append(el('div', 'note', s.sideOff));
    const list = (groups.find(([c]) => c === tab) || [tab, []])[1];
    if (!list.length) grid.append(el('div', 'empty', s.empty));
    for (const it of C.sortItems(list)) grid.append(card(it));
  }

  function receive(v) {
    if (!v || !Array.isArray(v.items)) return;
    data = v;
    render();
  }

  function refresh() {
    invoke('collection_get').then(receive).catch((e) => log('[collection] collection_get: ' + e));
  }

  function hide() {
    invoke('collection_close').catch(() => {
      const w = window.__TAURI__ && window.__TAURI__.window && window.__TAURI__.window.getCurrentWindow();
      if (w) w.hide().catch(() => {});
    });
  }

  // Chỉ bản dev (Rust gửi debug: true): thử nhanh mà không cần chờ đủ kẹo.
  function dev(action) {
    invoke('collection_debug', { action }).then(receive).catch((e) => toast(String(e), true));
  }
  $('devCandy').addEventListener('click', () => dev('candy'));
  $('devUnlock').addEventListener('click', () => dev('unlock'));
  $('pull1').addEventListener('click', () => pull(1));
  $('pull10').addEventListener('click', () => pull(10));
  $('gresOk').addEventListener('click', closeResults);
  $('gresAgain').addEventListener('click', () => {
    closeResults();
    pull(lastCount);
  });
  $('seeAll').addEventListener('click', () => setMode('shop'));
  $('closeBtn').addEventListener('click', hide);
  window.addEventListener('keydown', (e) => {
    if (e.key !== 'Escape') return;
    // Esc đóng bảng kết quả trước, lần nữa mới ẩn cửa sổ.
    if (!$('gres').hidden) closeResults();
    else hide();
  });
  listen('collection-changed', (e) => receive(e && e.payload));
  listen('collection-refresh', refresh);
  // Tiến độ tải model (model_dl.rs): cập nhật % trên thẻ, không đọc lại cả trang.
  listen('model-install', (e) => {
    const p = (e && e.payload) || {};
    const it = (data.items || []).find((i) => i.cat === 'model' && i.asset === p.id);
    if (!it || p.state !== 'progress') return;
    it.progress = p.pct;
    if (mode === 'shop') renderShop();
  });
  // Bấm model còn khoá ở menu Diện mạo của pet → mở thẳng tab của nó trong cửa hàng.
  listen('collection-focus', (e) => {
    const t = e && e.payload && e.payload.tab;
    if (!t || !C.CATS.includes(t)) return;
    tab = t;
    try { localStorage.setItem('collectionTab', tab); } catch (_) { /* bỏ qua */ }
    closeResults();
    setMode('shop');
  });
  FX.buildMachine($('machine'));
  renderHead();
  refresh();
})();
