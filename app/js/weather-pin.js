// Widget thời tiết ghim trên desktop (weather-pin.html). Không tự gọi mạng:
// Rust gửi dữ liệu qua weather_get / "weather-data" mỗi lần có số liệu mới.
// Kéo ở bất cứ đâu (trừ nút); nhấp đúp mở thẻ dự báo đầy đủ.
(function () {
  const { invoke, listen, log } = window.DC;
  const W = window.WeatherCore;
  const $ = (id) => document.getElementById(id);
  let data = null;
  let receivedAt = 0;

  const lang = () => (data && W.T[data.lang] ? data.lang : 'vi');
  const unit = () => (data && data.units === 'f' ? 'f' : 'c');

  function el(tag, cls, text) {
    const e = document.createElement(tag);
    if (cls) e.className = cls;
    if (text != null) e.textContent = text;
    return e;
  }

  // Đổi ảnh Miu chỉ khi khác, để khỏi nháy ảnh mỗi phút.
  function setFace(face) {
    const src = ((data && data.miuDir) || 'media/game') + '/rps-' + face + '.' + ((data && data.miuExt) || 'png');
    if ($('miu').getAttribute('src') !== src) $('miu').setAttribute('src', src);
  }

  function render() {
    if (!data) return;
    const S = W.strings(lang());
    const f = data.forecast;
    const card = $('card');
    card.classList.toggle('small', data.pinSize !== 'large');
    card.classList.toggle('large', data.pinSize === 'large');
    card.classList.toggle('old', !!(f && data.stale));
    card.classList.remove('sky-sun', 'sky-rain', 'sky-night');
    $('topBtn').title = S.top;
    $('topBtn').classList.toggle('on', !!data.pinTop);
    $('sizeBtn').title = S.size;
    $('unpinBtn').title = S.unpin;
    $('city').textContent = data.city || '';
    const now = data.now + (Date.now() - receivedAt) / 1000;
    $('ago').textContent = f && data.stale ? S.stale : W.ago(data.fetchedAt, now, lang());
    const meta = $('meta');
    const days = $('days');
    meta.textContent = '';
    days.textContent = '';
    if (!f) {
      $('ic').textContent = '🌡️';
      $('t').textContent = '--';
      meta.append(el('div', 'empty', data.error ? S.offline : S.never));
      setFace('shy');
      return;
    }
    const c = f.current;
    const g = W.group(c.code);
    const wet = /rain|drizzle|showers|freezing|storm/i.test(g);
    card.classList.add(wet ? 'sky-rain' : !c.is_day ? 'sky-night' : g === 'clear' || g === 'mostly' ? 'sky-sun' : 'x');
    setFace(W.face(c.code, c.is_day, !!(data.stale || data.error)));
    $('ic').textContent = W.icon(c.code, c.is_day);
    $('t').textContent = '';
    $('t').append(String(W.temp(c.temp, unit())), el('sup', null, unit() === 'f' ? '°F' : '°C'));
    const today = f.daily[0];
    const next = f.hourly[1] || f.hourly[0];
    meta.append(el('div', null, W.describe(c.code, lang())));
    if (today) {
      const hl = el('div');
      hl.append(`${S.hi} `, el('b', null, `${W.temp(today.max, unit())}°`), ` · ${S.lo} `, el('b', null, `${W.temp(today.min, unit())}°`));
      meta.append(hl);
    }
    if (next && next.rain != null) meta.append(el('div', 'rain', `💧 ${S.rainNext}: ${next.rain}%`));
    f.daily.slice(1, 4).forEach((d, i) => {
      const row = el('div');
      row.append(el('span', 'i', W.icon(d.code, true)), el('span', 'n', W.dayName(d.date, i + 1, lang())),
        el('span', null, `${W.temp(d.max, unit())}°`), el('span', 'lo', `${W.temp(d.min, unit())}°`));
      days.append(row);
    });
  }

  function receive(v) {
    if (!v) return;
    data = v;
    receivedAt = Date.now();
    // Skin màu chọn trong Bộ sưu tập (app/skins.css).
    document.body.dataset.skin = data.skin || 'lavender';
    render();
  }

  function currentWindow() {
    const T = window.__TAURI__ || {};
    return (T.window && T.window.getCurrentWindow && T.window.getCurrentWindow()) || null;
  }
  function startDrag(e) {
    if (e.button !== 0 || e.target.closest('button')) return;
    const w = currentWindow();
    if (w && w.startDragging) w.startDragging().catch(() => {});
  }
  $('card').addEventListener('mousedown', startDrag);
  $('card').addEventListener('dblclick', (e) => {
    if (!e.target.closest('button')) invoke('open_weather').catch(() => {});
  });
  $('topBtn').addEventListener('click', () => invoke('weather_pin_top', { top: !(data && data.pinTop) }).catch((e) => log('[wpin] ' + e)));
  $('sizeBtn').addEventListener('click', () =>
    invoke('weather_pin_size', { size: data && data.pinSize === 'large' ? 'small' : 'large' }).catch((e) => log('[wpin] ' + e)));
  $('unpinBtn').addEventListener('click', () => invoke('weather_pin_set', { pinned: false }).catch((e) => log('[wpin] ' + e)));

  setInterval(render, 60000);
  listen('weather-data', (e) => receive(e && e.payload));
  invoke('weather_get').then(receive).catch((e) => log('[wpin] weather_get: ' + e));
})();
