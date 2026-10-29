// Widget 🛰 Theo dõi website (monitor-pin.html). Rust gửi phần ít đổi
// (monitor_get / "monitor-data": ngôn ngữ, luôn nổi, skin, chu kỳ) và kết quả
// mỗi lượt kiểm ("monitor-stats"). Bấm một site để mở trong trình duyệt.
(function () {
  const { invoke, listen, log } = window.DC;
  const M = window.MonitorCore;
  const $ = (id) => document.getElementById(id);

  let data = { pinTop: true, lang: 'vi', interval: 5 };
  let sites = [];

  function el(tag, cls, text) {
    const e = document.createElement(tag);
    if (cls) e.className = cls;
    if (text != null) e.textContent = text;
    return e;
  }

  function render() {
    const t = M.strings(data.lang);
    $('title').textContent = '🛰 ' + t.title;
    $('topBtn').title = t.top;
    $('topBtn').classList.toggle('on', !!data.pinTop);
    $('unpinBtn').title = t.unpin;
    const sum = $('sum');
    sum.textContent = sites.length ? M.summary(sites, data.lang) : '';
    sum.classList.toggle('bad', sites.some((s) => M.dot(s) === 'down'));
    sum.title = M.fill(t.every, { m: data.interval || 5 });

    const list = $('list');
    list.textContent = '';
    if (!sites.length) {
      list.append(el('div', 'empty', t.empty));
      return;
    }
    for (const s of sites) {
      const state = M.dot(s);
      const row = el('div', 'site ' + state);
      row.dataset.url = s.url;
      row.title = `${s.url}\n${t.open}`;
      row.append(el('span', 'dot ' + state), el('span', 'n', s.name), el('span', 'v', M.resultText(s.last, data.lang)));
      list.append(row);
    }
  }

  function receive(v) {
    if (!v) return;
    data = v;
    document.body.dataset.skin = data.skin || 'lavender';
    window.WidgetDeco.mount($('wrap'), data.deco);
    render();
  }

  function receiveStats(v) {
    sites = (v && v.sites) || [];
    render();
  }

  function currentWindow() {
    const T = window.__TAURI__ || {};
    return (T.window && T.window.getCurrentWindow && T.window.getCurrentWindow()) || null;
  }
  $('card').addEventListener('mousedown', (e) => {
    if (e.button !== 0 || e.target.closest('button, .site')) return;
    const w = currentWindow();
    if (w && w.startDragging) w.startDragging().catch(() => {});
  });
  $('list').addEventListener('click', (e) => {
    const row = e.target.closest('.site');
    if (row) invoke('open_url', { url: row.dataset.url }).catch((err) => log('[monitor] open: ' + err));
  });
  $('topBtn').addEventListener('click', () =>
    invoke('widget_pin_top', { id: 'monitor', top: !data.pinTop }).catch((e) => log('[monitor] top: ' + e)));
  $('unpinBtn').addEventListener('click', () =>
    invoke('widget_pin_set', { id: 'monitor', pinned: false }).catch((e) => log('[monitor] unpin: ' + e)));

  listen('monitor-data', (e) => receive(e && e.payload));
  listen('monitor-stats', (e) => receiveStats(e && e.payload));
  invoke('monitor_get').then(receive).catch((e) => log('[monitor] monitor_get: ' + e));
  invoke('monitor_stats').then(receiveStats).catch((e) => log('[monitor] monitor_stats: ' + e));
  render();
})();
