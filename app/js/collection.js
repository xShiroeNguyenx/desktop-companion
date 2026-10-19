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
  try {
    tab = localStorage.getItem('collectionTab') || tab;
  } catch (_) { /* không có storage cũng không sao */ }
  let toastTimer = null;

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
    } else if (item.cat === 'side_set' || item.cat === 'miu_set') {
      if (item.cat === 'side_set') pv.classList.add('side');
      pv.append(img(C.thumb(item), 'full', true));
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
    try {
      if (kind === 'buy') {
        await invoke('collection_buy', { id: item.id });
        toast(C.fill(S().bought, { item: item.name, you: data.you || '' }));
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
    $('bubble').textContent = C.fill(s.bubble, { pet, Pet: pet });
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
    // Chi tiết (trần mỗi ngày, +20 mỗi cột mốc) để trong tooltip, dải dưới gọn.
    $('howTitle').title = s.howTo;
    const chips = $('chips');
    chips.textContent = '';
    for (const e of s.earn) chips.append(el('span', 'chip', e));
    $('noMoney').textContent = s.noMoney;
    $('devTools').hidden = !data.debug;
  }

  function render() {
    const s = S();
    renderHead();
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
        render();
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
  $('closeBtn').addEventListener('click', hide);
  window.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') hide();
  });
  listen('collection-changed', (e) => receive(e && e.payload));
  listen('collection-refresh', refresh);
  renderHead();
  refresh();
})();
