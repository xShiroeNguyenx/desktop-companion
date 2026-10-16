// Thẻ dự báo thời tiết (weather.html). Rust gửi toàn bộ dữ liệu qua
// weather_get / sự kiện "weather-data" (weather_net::view): thành phố, đơn vị,
// dự báo (nhiệt độ luôn bằng °C), lỗi, chế độ hiệu ứng, ghim. Thẻ không tự gọi
// mạng; tìm thành phố đi qua lệnh weather_search (Rust gọi Open-Meteo).
(function () {
  const { invoke, listen, log } = window.DC;
  const W = window.WeatherCore;
  const $ = (id) => document.getElementById(id);
  const SVG = 'http://www.w3.org/2000/svg';

  // Câu Miu nói ở góc thẻ, theo nhóm thời tiết. {you} = cách gọi người dùng.
  const SAY = {
    vi: { sun: 'Nắng đẹp ghê, {you} nhớ uống nước nha~', night: 'Tối rồi, {you} nghỉ sớm nha 🌙', cloud: 'Trời hơi âm u, {me} vẫn ở đây với {you}~',
      rain: 'Mưa rồi á, {you} nhớ mang ô nha ☔', snow: 'Lạnh lắm, {you} mặc ấm vào nha ⛄', storm: 'Có dông, {you} ở trong nhà cho an toàn nha!',
      fog: 'Sương mù đó, {you} đi chậm thôi nha~' },
    ja: { sun: 'いいお天気！お水も飲んでね～', night: '夜だね、早めに休んでね🌙', cloud: 'ちょっとくもり。わたしはそばにいるよ～',
      rain: '雨だよ、傘を忘れないでね☔', snow: '寒いね、あったかくしてね⛄', storm: '雷雨だよ、おうちにいてね！', fog: '霧が出てるよ、ゆっくりね～' },
    en: { sun: 'Lovely sunshine~ remember to drink water!', night: "It's night, get some rest soon 🌙", cloud: "A bit cloudy, but I'm right here~",
      rain: 'Rain! Take an umbrella ☔', snow: 'So cold, stay warm ⛄', storm: 'Thunderstorm — stay inside, okay?', fog: "It's foggy, go slowly~" },
  };

  let data = null;
  let receivedAt = 0;
  let pickerForced = false;
  let note = null; // { text, until } — thông báo ngắn (vừa ghim, vừa gỡ ghim)
  let refreshing = false;

  const lang = () => (data && W.T[data.lang] ? data.lang : 'vi');
  const L = () => W.strings(lang());
  const words = () => {
    const you = (data && data.you) || (lang() === 'vi' ? 'bạn' : '');
    const me = (data && data.me) || (lang() === 'vi' ? 'em' : '');
    return { you, me };
  };
  const unit = () => (data && data.units === 'f' ? 'f' : 'c');
  const deg = () => (unit() === 'f' ? '°F' : '°C');
  const nowSecs = () => (data ? data.now + (Date.now() - receivedAt) / 1000 : Date.now() / 1000);

  function sayKind(code, isDay) {
    const g = W.group(code);
    if (g === 'storm') return 'storm';
    if (g.startsWith('snow')) return 'snow';
    if (g === 'fog') return 'fog';
    if (['drizzle', 'rainL', 'rain', 'rainH', 'freezing', 'showers', 'showersH'].includes(g)) return 'rain';
    if (!isDay) return 'night';
    return g === 'cloudy' || g === 'partly' ? 'cloud' : 'sun';
  }

  // Ảnh Miu theo trời (W.face); chỉ đổi khi khác để khỏi nháy ảnh.
  function setMiu(face) {
    const src = 'media/game/rps-' + face + '.png';
    if ($('miuImg').getAttribute('src') !== src) $('miuImg').setAttribute('src', src);
  }

  function el(tag, cls, text) {
    const e = document.createElement(tag);
    if (cls) e.className = cls;
    if (text != null) e.textContent = text;
    return e;
  }

  function svg(tag, attrs, text) {
    const e = document.createElementNS(SVG, tag);
    for (const [k, v] of Object.entries(attrs || {})) e.setAttribute(k, v);
    if (text != null) e.textContent = text;
    return e;
  }

  // ─── Biểu đồ 24 giờ ───────────────────────────────────────────────────────
  function drawChart(hours) {
    const box = $('chart');
    box.textContent = '';
    const w = box.clientWidth || 600;
    const h = box.clientHeight || 180;
    box.setAttribute('viewBox', `0 0 ${w} ${h}`);
    const pad = 16;
    const c = W.chart(hours, w - pad * 2, h, { top: 30, bottom: 50, every: 5 });
    if (!c.points.length) return;
    const g = svg('g', { transform: `translate(${pad},0)` });
    const defs = svg('defs');
    const grad = svg('linearGradient', { id: 'heat', x1: '0', y1: '0', x2: '0', y2: '1' });
    grad.append(svg('stop', { offset: '0', 'stop-color': '#ff7a8f', 'stop-opacity': '0.55' }));
    grad.append(svg('stop', { offset: '1', 'stop-color': '#ff7a8f', 'stop-opacity': '0.04' }));
    defs.append(grad);
    g.append(defs);
    g.append(svg('path', { d: c.area, fill: 'url(#heat)' }));
    g.append(svg('path', { d: c.line, fill: 'none', stroke: '#ff9db0', 'stroke-width': '2.5', 'stroke-linejoin': 'round' }));
    const base = h - 50;
    g.append(svg('line', { x1: 0, y1: base + 22, x2: w - pad * 2, y2: base + 22, stroke: 'rgba(255,255,255,0.2)' }));
    for (const m of c.marks) {
      g.append(svg('circle', { cx: m.x, cy: m.y, r: 3.5, fill: '#fff' }));
      g.append(svg('text', { x: m.x, y: m.y - 9, 'text-anchor': 'middle', class: 'tl' }, `${W.temp(m.hour.temp, unit())}°`));
      if (m.hour.rain != null) {
        g.append(svg('text', { x: m.x, y: base + 14, 'text-anchor': 'middle', class: 'rl' }, `💧${m.hour.rain}%`));
      }
      g.append(svg('line', { x1: m.x, y1: base + 18, x2: m.x, y2: base + 26, stroke: 'rgba(255,255,255,0.35)' }));
      g.append(svg('text', { x: m.x, y: base + 42, 'text-anchor': 'middle', class: 'hl' }, W.hourLabel(m.hour.time, lang())));
    }
    box.append(g);
  }

  // ─── Vẽ cả thẻ ────────────────────────────────────────────────────────────
  function render() {
    if (!data) return;
    const S = L();
    document.documentElement.lang = lang();
    const f = data.forecast;

    $('cityBtn').textContent = data.hasCity ? data.city : S.pick;
    $('cityBtn').title = S.change;
    $('cityTime').textContent = f ? W.clock(f.current.time) : '';
    $('ago').textContent = data.hasCity ? W.ago(data.fetchedAt, nowSecs(), lang()) : '';
    for (const b of $('units').querySelectorAll('button')) b.classList.toggle('on', b.dataset.u === unit());
    $('refreshBtn').title = S.refresh;
    $('pinBtn').title = data.pinned ? S.unpin : S.pin;
    $('pinBtn').classList.toggle('on', !!data.pinned);
    $('pinBtn').disabled = !data.hasCity;
    $('closeBtn').title = S.close;
    $('credit').textContent = S.credit;

    const live = data.mode === 'live';
    $('liveBtn').textContent = live ? S.liveOn : S.useLive;
    $('liveBtn').disabled = live || !data.hasCity;
    $('liveBtn').className = live ? 'pill ghost' : 'pill';

    let warn = '';
    if (data.hasCity && data.error) warn = f ? S.stale : `${S.offline}: ${data.error}`;
    else if (data.hasCity && f && data.stale) warn = S.stale;
    const info = note && Date.now() < note.until ? note.text : '';
    $('banner').hidden = !warn && !info;
    $('banner').textContent = warn || info;
    $('banner').className = warn ? 'banner' : 'banner info';
    $('refreshBtn').classList.toggle('spin', refreshing);
    if (refreshing) $('ago').textContent = S.refreshing;

    const win = $('win');
    win.classList.remove('sky-sun', 'sky-rain', 'sky-night');
    if (f) {
      const c = f.current;
      const today = f.daily[0];
      const kind = sayKind(c.code, c.is_day);
      win.classList.add(kind === 'rain' || kind === 'storm' ? 'sky-rain' : !c.is_day ? 'sky-night' : kind === 'sun' ? 'sky-sun' : 'x');
      $('nowIcon').textContent = W.icon(c.code, c.is_day);
      $('nowTemp').innerHTML = '';
      $('nowTemp').append(String(W.temp(c.temp, unit())), el('sup', null, deg()));
      $('nowDesc').textContent = W.describe(c.code, lang());
      $('nowHL').textContent = today ? `${S.hi} ${W.temp(today.max, unit())}° · ${S.lo} ${W.temp(today.min, unit())}°` : '';
      $('miuSay').textContent = W.fill(SAY[lang()][kind], words());
      setMiu(W.face(c.code, c.is_day, !!warn));
      const days = $('days');
      days.textContent = '';
      f.daily.slice(0, 7).forEach((d, i) => {
        const cell = el('div', 'day' + (i === 0 ? ' first' : ''));
        const r = el('div', 'r', `${W.temp(d.max, unit())}° `);
        r.append(el('span', null, `${W.temp(d.min, unit())}°`));
        cell.append(el('div', 'n', W.dayName(d.date, i, lang())), el('div', 'i', W.icon(d.code, true)), r);
        days.append(cell);
      });
      drawChart(f.hourly);
    } else {
      $('nowIcon').textContent = '🌡️';
      $('nowTemp').textContent = '--';
      $('nowDesc').textContent = data.hasCity ? S.never : S.noCity;
      $('nowHL').textContent = '';
      $('miuSay').textContent = S.noCity;
      // Có thành phố mà chưa lấy được số liệu: ngại; chưa chọn thành phố: nháy mắt mời chọn.
      setMiu(data.hasCity ? 'shy' : 'wink');
      $('days').textContent = '';
      $('chart').textContent = '';
    }

    // Chưa có thành phố: thẻ chỉ còn hàng tìm kiếm + lời Miu.
    win.classList.toggle('nocity', !data.hasCity);
    if (!data.hasCity) openPicker(true);
    else if (!pickerForced) $('search').hidden = true;
  }

  // ─── Chọn thành phố ───────────────────────────────────────────────────────
  function openPicker(auto) {
    const S = L();
    const p = $('search');
    const wasHidden = p.hidden;
    pickerForced = !auto;
    $('pickHint').textContent = S.privacy;
    $('pickInput').placeholder = S.search;
    $('pickGo').textContent = S.find;
    $('pickClose').hidden = !data || !data.hasCity;
    $('pickClose').title = S.close;
    p.hidden = false;
    if (wasHidden) {
      $('results').textContent = '';
      setTimeout(() => $('pickInput').focus(), 50);
    }
  }
  function closePicker() {
    pickerForced = false;
    if (data && data.hasCity) $('search').hidden = true;
  }

  async function search(e) {
    if (e) e.preventDefault();
    const q = $('pickInput').value.trim();
    const box = $('results');
    if (q.length < 2) return;
    box.textContent = '';
    box.append(el('div', 'msg', '…'));
    try {
      const places = await invoke('weather_search', { query: q });
      box.textContent = '';
      if (!places.length) box.append(el('div', 'msg', L().none));
      for (const p of places) {
        const b = el('button', null, p.name);
        b.type = 'button';
        b.append(el('small', null, [p.admin, p.country].filter(Boolean).join(', ')));
        b.addEventListener('click', () => choose(p));
        box.append(b);
      }
    } catch (err) {
      box.textContent = '';
      box.append(el('div', 'msg', String(err)));
    }
  }

  async function choose(p) {
    const name = [p.name, p.admin].filter(Boolean).join(', ');
    try {
      await invoke('weather_set_city', { name, lat: p.lat, lon: p.lon });
      pickerForced = false;
      $('search').hidden = true;
    } catch (err) {
      $('results').textContent = '';
      $('results').append(el('div', 'msg', String(err)));
    }
  }

  function currentWindow() {
    const T = window.__TAURI__ || {};
    return (T.window && T.window.getCurrentWindow && T.window.getCurrentWindow()) || null;
  }

  // Ẩn bằng lệnh Rust (chắc ăn), lỗi thì mới dùng API cửa sổ của JS.
  function hideWindow() {
    invoke('weather_close').catch(() => {
      const w = currentWindow();
      if (w) w.hide().catch((e) => log('[weather] hide: ' + e));
    });
  }

  function showNote(text) {
    note = { text, until: Date.now() + 4000 };
    render();
    setTimeout(render, 4100);
  }

  // Kéo thẻ ở bất cứ đâu (trừ nút, ô nhập, danh sách kết quả, chữ chọn được).
  $('win').addEventListener('mousedown', (e) => {
    if (e.button !== 0 || e.target.closest('button, input, a, .results')) return;
    const w = currentWindow();
    if (w && w.startDragging) w.startDragging().catch((err) => log('[weather] drag: ' + err));
  });

  // ─── Sự kiện ─────────────────────────────────────────────────────────────
  function receive(v) {
    if (!v) return;
    // Dữ liệu mới tới (hoặc lỗi mới) thì thôi quay nút làm mới.
    if (refreshing && data && (v.fetchedAt !== data.fetchedAt || v.error !== data.error)) refreshing = false;
    data = v;
    receivedAt = Date.now();
    render();
  }

  // Bấm tên thành phố: mở / đóng hàng tìm kiếm.
  $('cityBtn').addEventListener('click', () => ($('search').hidden ? openPicker(false) : closePicker()));
  $('pickClose').addEventListener('click', closePicker);
  $('pickForm').addEventListener('submit', search);
  $('closeBtn').addEventListener('click', hideWindow);
  $('refreshBtn').addEventListener('click', async () => {
    refreshing = true;
    render();
    // Mạng chậm / không đổi gì: quá 12 giây thì thôi quay.
    setTimeout(() => { refreshing = false; render(); }, 12000);
    try {
      await invoke('weather_refresh');
    } catch (e) {
      refreshing = false;
      log('[weather] ' + e);
      render();
    }
  });
  $('pinBtn').addEventListener('click', async () => {
    const next = !(data && data.pinned);
    try {
      await invoke('weather_pin_set', { pinned: next });
      showNote(next ? L().pinnedNote : L().unpinnedNote);
    } catch (e) {
      showNote(String(e));
    }
  });
  $('liveBtn').addEventListener('click', () => invoke('weather_use_live').catch((e) => log('[weather] ' + e)));
  for (const b of $('units').querySelectorAll('button')) {
    b.addEventListener('click', () => invoke('weather_set_units', { units: b.dataset.u }).catch((e) => log('[weather] ' + e)));
  }
  window.addEventListener('keydown', (e) => {
    if (e.key !== 'Escape') return;
    if (!$('search').hidden && data && data.hasCity) closePicker();
    else hideWindow();
  });
  window.addEventListener('resize', () => data && data.forecast && drawChart(data.forecast.hourly));
  // "Cập nhật … phút trước" tự đổi theo thời gian.
  setInterval(() => {
    if (data && data.hasCity) $('ago').textContent = W.ago(data.fetchedAt, nowSecs(), lang());
  }, 30000);

  listen('weather-data', (e) => receive(e && e.payload));
  invoke('weather_get').then(receive).catch((e) => log('[weather] weather_get: ' + e));
})();
