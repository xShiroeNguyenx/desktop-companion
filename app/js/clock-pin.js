// Đồng hồ ghim trên desktop (clock-pin.html). Rust gửi múi giờ, luôn nổi, pha
// Tập trung (clock_get / "clock-data"); trang tự chạy kim mỗi giây và đếm lùi,
// mỗi phút hỏi lại Rust để khỏi lệch với vòng đếm thật bên companion.rs.
(function () {
  const { invoke, listen, log } = window.DC;
  const K = window.ClockCore;
  const C = window.CalendarCore;
  const $ = (id) => document.getElementById(id);

  let data = { pinTop: true, lang: 'vi', zones: [], focus: { phase: 'idle', remaining: 0 }, focusMin: 25 };
  let receivedAt = Date.now();

  function el(tag, cls, text) {
    const e = document.createElement(tag);
    if (cls) e.className = cls;
    if (text != null) e.textContent = text;
    return e;
  }

  // Phần đổi theo dữ liệu Rust (múi giờ, nhãn nút): vẽ lại khi nhận dữ liệu.
  function renderStatic() {
    const s = K.strings(data.lang);
    $('topBtn').title = s.top;
    $('topBtn').classList.toggle('on', !!data.pinTop);
    $('unpinBtn').title = s.unpin;
    $('focusBtn').title = s.focusTitle;
    const zones = $('zones');
    zones.textContent = '';
    for (const z of data.zones || []) {
      const row = el('div', 'zone');
      row.dataset.zone = z;
      row.append(el('span', 'n', K.zoneName(z, data.lang)), el('span', 't'), el('span', 'd'));
      zones.append(row);
    }
  }

  // Phần chạy theo giây: giờ, ngày, giờ các múi, đếm lùi.
  function tick() {
    const s = K.strings(data.lang);
    const now = Date.now();
    const local = K.localParts(now);
    $('time').textContent = K.hhmm(local);
    const wd = new Date(now).getDay();
    const l = C.toLunar(local.day, local.month, local.year);
    const p2 = (x) => String(x).padStart(2, '0');
    $('date').textContent = `${s.weekdays[wd]}, ${p2(local.day)}/${p2(local.month)} · ${K.fill(s.lunar, { d: l.day, m: l.month })}`;

    for (const row of $('zones').children) {
      const c = K.compare(now, row.dataset.zone, local);
      const t = row.querySelector('.t');
      const d = row.querySelector('.d');
      if (!c) {
        t.textContent = '--:--';
        d.textContent = '';
        continue;
      }
      t.textContent = K.hhmm(c.parts);
      const shift = c.dayShift < 0 ? s.yesterday : c.dayShift > 0 ? s.tomorrow : '';
      d.textContent = [K.diffLabel(c.diffMin) || s.same, shift].filter(Boolean).join(' · ');
      d.classList.toggle('shift', !!shift);
    }

    const f = data.focus || { phase: 'idle' };
    const face = K.face(f.phase, local.hour);
    const src = `${data.miuDir || 'media/game'}/rps-${face}.${data.miuExt || 'png'}`;
    if ($('miu').getAttribute('src') !== src) $('miu').setAttribute('src', src);
    const btn = $('focusBtn');
    const left = K.countdown((f.remaining || 0) - (now - receivedAt) / 1000);
    btn.classList.toggle('on', f.phase === 'focus');
    btn.classList.toggle('brk', f.phase === 'break');
    btn.textContent = f.phase === 'focus'
      ? K.fill(s.focusOn, { left })
      : f.phase === 'break'
        ? K.fill(s.breakOn, { left })
        : K.fill(s.focusIdle, { min: data.focusMin || 25 });
  }

  function receive(v) {
    if (!v) return;
    data = v;
    receivedAt = Date.now();
    // Skin màu chọn trong Bộ sưu tập (app/skins.css).
    document.body.dataset.skin = data.skin || 'lavender';
    renderStatic();
    tick();
  }

  function refresh() {
    invoke('clock_get').then(receive).catch((e) => log('[clock] clock_get: ' + e));
  }

  function currentWindow() {
    const T = window.__TAURI__ || {};
    return (T.window && T.window.getCurrentWindow && T.window.getCurrentWindow()) || null;
  }
  $('card').addEventListener('mousedown', (e) => {
    if (e.button !== 0 || e.target.closest('button')) return;
    const w = currentWindow();
    if (w && w.startDragging) w.startDragging().catch(() => {});
  });
  // Bấm 🍅: như menu tròn (pet nói, vào chế độ tập trung). Rust gửi "clock-data" khi đổi pha.
  $('focusBtn').addEventListener('click', () => invoke('focus_toggle').catch((e) => log('[clock] focus_toggle: ' + e)));
  $('topBtn').addEventListener('click', () =>
    invoke('widget_pin_top', { id: 'clock', top: !data.pinTop }).catch((e) => log('[clock] top: ' + e)));
  $('unpinBtn').addEventListener('click', () =>
    invoke('widget_pin_set', { id: 'clock', pinned: false }).catch((e) => log('[clock] unpin: ' + e)));

  setInterval(tick, 1000);
  setInterval(refresh, 60000);
  listen('clock-data', (e) => receive(e && e.payload));
  renderStatic();
  tick();
  refresh();
})();
