// Lịch tháng ghim trên desktop (calendar-pin.html). Rust chỉ gửi luôn nổi /
// ngôn ngữ (calendar_get / "calendar-data"); task đọc qua tasks_list, vẽ lại khi
// có "tasks-refresh". Bấm một ngày để xem âm lịch, lễ, việc tới hạn và thêm việc.
(function () {
  const { invoke, listen, log } = window.DC;
  const C = window.CalendarCore;
  const $ = (id) => document.getElementById(id);

  let data = { pinTop: true, lang: 'vi' };
  let tasks = [];
  const now0 = new Date();
  let view = { year: now0.getFullYear(), month: now0.getMonth() + 1 };
  let selected = C.keyOfDate(now0);
  // Hôm nay lần vẽ trước: qua nửa đêm thì đổi ô "hôm nay" (và sang tháng mới
  // nếu đang xem tháng cũ của hôm qua).
  let todayKey = selected;

  const S = () => C.strings(data.lang);
  const p2 = (x) => String(x).padStart(2, '0');

  function el(tag, cls, text) {
    const e = document.createElement(tag);
    if (cls) e.className = cls;
    if (text != null) e.textContent = text;
    return e;
  }

  function parseKey(key) {
    const [y, m, d] = key.split('-').map(Number);
    return { y, m, d };
  }

  function renderHead() {
    const s = S();
    $('title').textContent = C.monthTitle(view.year, view.month, data.lang);
    // Âm lịch của ngày giữa tháng: đại diện cho phần lớn lưới.
    const mid = C.toLunar(15, view.month, view.year);
    $('sub').textContent = C.fill(s.lunarMonth, { m: mid.month, cc: C.canChi(mid.year, data.lang) });
    const now = new Date();
    const onToday = view.year === now.getFullYear() && view.month === now.getMonth() + 1;
    $('todayBtn').hidden = onToday;
    $('todayBtn').textContent = s.today;
    $('prevBtn').title = s.prev;
    $('nextBtn').title = s.next;
    $('topBtn').title = s.top;
    $('topBtn').classList.toggle('on', !!data.pinTop);
    $('unpinBtn').title = s.unpin;
    $('addBtn').textContent = s.add;
    $('addBtn').title = s.addTitle;
    $('addText').placeholder = s.placeholder;
    $('addSave').textContent = s.save;
    const week = $('week');
    week.textContent = '';
    s.weekdays.forEach((w, i) => week.append(el('div', i === 6 ? 'sun' : null, w)));
  }

  function renderGrid(byDay) {
    const grid = $('grid');
    grid.textContent = '';
    for (const c of C.monthGrid(view.year, view.month)) {
      const key = C.dayKey(c.year, c.month, c.day);
      const lunar = C.toLunar(c.day, c.month, c.year);
      const hol = C.holidays(c.year, c.month, c.day, lunar);
      const b = el('button', 'day');
      b.type = 'button';
      b.dataset.key = key;
      b.classList.toggle('out', !c.inMonth);
      b.classList.toggle('sun', c.weekday === 6);
      b.classList.toggle('off', hol.some((h) => h.off));
      const bday = C.isBirthday(data.birthday, c.month, c.day);
      b.classList.toggle('fest', hol.length > 0 || bday);
      b.classList.toggle('bday', bday);
      b.classList.toggle('today', key === todayKey);
      b.classList.toggle('sel', key === selected && key !== todayKey);
      b.append(el('span', 'n', String(c.day)), el('span', lunar.day === 1 ? 'l first' : 'l', C.lunarLabel(lunar, data.lang)));
      const due = byDay[key];
      if (due) {
        b.append(el('span', due.some((t) => t.over) ? 'dot over' : 'dot'));
      }
      const tips = hol.map((h) => S().h[h.id]);
      if (bday) tips.unshift(C.fill(S().bday, { you: data.you || '' }));
      if (due) tips.push(`${due.length} ✓`);
      if (tips.length) b.title = tips.join(' · ');
      grid.append(b);
    }
  }

  function renderInfo(byDay) {
    const s = S();
    const { y, m, d } = parseKey(selected);
    const weekday = (new Date(y, m - 1, d).getDay() + 6) % 7;
    $('infoDate').textContent = `${s.weekdaysLong[weekday]}, ${p2(d)}/${p2(m)}/${y}`;
    const l = C.toLunar(d, m, y);
    $('infoLunar').textContent = C.fill(s.lunarFull, { d: l.day, m: l.month, leap: l.leap ? s.leap : '', cc: C.canChi(l.year, data.lang) });
    const hol = C.holidays(y, m, d, l);
    const bday = C.isBirthday(data.birthday, m, d);
    const fest = hol.map((h) => '🎉 ' + s.h[h.id]);
    if (bday) fest.unshift(C.fill(s.bday, { you: data.you || '' }));
    $('infoFest').hidden = fest.length === 0;
    $('infoFest').textContent = fest.join(' · ');

    const list = $('infoTasks');
    list.textContent = '';
    const due = byDay[selected] || [];
    // Miu mini: ngày lễ / sinh nhật thì reo, có việc quá hạn thì giật mình,
    // có việc tới hạn thì "cố lên", còn lại nháy mắt.
    const face = fest.length ? 'cheer' : due.some((t) => t.over) ? 'surprised' : due.length ? 'fight' : 'wink';
    const src = `${data.miuDir || 'media/game'}/rps-${face}.${data.miuExt || 'png'}`;
    if ($('miu').getAttribute('src') !== src) $('miu').setAttribute('src', src);
    // Chừa chỗ cho dòng lễ và ô thêm việc: tối đa 2 việc, còn lại gộp "+n".
    const room = fest.length ? 1 : 2;
    if (!due.length) list.append(el('div', 'muted', s.noTasks));
    for (const t of due.slice(0, room)) {
      const b = el('button', t.over ? 'task over' : 'task');
      b.type = 'button';
      const at = new Date(t.due);
      b.append(el('span', 't', `${p2(at.getHours())}:${p2(at.getMinutes())}`), el('span', 'x', t.text));
      b.title = t.over ? `${t.text} (${s.overdue})` : t.text;
      list.append(b);
    }
    if (due.length > room) list.append(el('div', 'muted', C.fill(s.more, { n: due.length - room })));
  }

  function render() {
    const byDay = C.tasksByDay(tasks, Date.now());
    renderHead();
    renderGrid(byDay);
    renderInfo(byDay);
  }

  async function loadTasks() {
    try {
      tasks = (await invoke('tasks_list')) || [];
    } catch (e) {
      log('[cal] tasks_list: ' + e);
    }
    render();
  }

  function receive(v) {
    if (!v) return;
    data = v;
    // Skin màu chọn trong Bộ sưu tập (app/skins.css).
    document.body.dataset.skin = data.skin || 'lavender';
    render();
  }

  function goTo(year, month) {
    view = { year, month };
    render();
  }

  function showMsg(text, err) {
    const m = $('msg');
    m.textContent = text || '';
    m.classList.toggle('err', !!err);
    clearTimeout(showMsg.timer);
    if (text) showMsg.timer = setTimeout(() => { m.textContent = ''; }, 4000);
  }

  function openAdd(on) {
    $('addRow').hidden = !on;
    $('addBtn').textContent = on ? S().cancel : S().add;
    if (on) {
      $('addText').value = '';
      $('addText').focus();
    }
  }

  async function saveTask() {
    const text = $('addText').value.trim();
    if (!text) {
      $('addText').focus();
      return;
    }
    const due = C.dueAt(selected, $('addTime').value);
    if (due == null) {
      showMsg('⚠️ ' + $('addTime').value, true);
      return;
    }
    $('addSave').disabled = true;
    try {
      const t = await invoke('tasks_add', { text, note: '', source: 'manual' });
      await invoke('tasks_set_due', { id: t.id, dueAt: String(due) });
      openAdd(false);
      showMsg(S().saved);
    } catch (e) {
      // Báo nguyên văn lỗi từ Rust (vd. hạn chót không hợp lệ), đừng im lặng.
      showMsg(String(e), true);
      log('[cal] thêm việc: ' + e);
    } finally {
      $('addSave').disabled = false;
    }
    // tasks.json đổi → Rust gửi "tasks-refresh"; vẽ lại luôn cho chắc.
    loadTasks();
  }

  // ─── Kéo / bấm ─────────────────────────────────────────────────────────────

  function currentWindow() {
    const T = window.__TAURI__ || {};
    return (T.window && T.window.getCurrentWindow && T.window.getCurrentWindow()) || null;
  }
  // Kéo ở nền / tiêu đề. Ô ngày, nút, ô nhập là để bấm (startDragging nuốt click).
  $('card').addEventListener('mousedown', (e) => {
    if (e.button !== 0 || e.target.closest('button, input, .info')) return;
    const w = currentWindow();
    if (w && w.startDragging) w.startDragging().catch(() => {});
  });
  $('grid').addEventListener('click', (e) => {
    const b = e.target.closest('.day');
    if (!b) return;
    selected = b.dataset.key;
    const { y, m } = parseKey(selected);
    // Bấm ô của tháng trước / sau thì lật sang tháng đó.
    if (y !== view.year || m !== view.month) view = { year: y, month: m };
    render();
  });
  $('infoTasks').addEventListener('click', (e) => {
    if (e.target.closest('.task')) invoke('open_tasks').catch((err) => log('[cal] open_tasks: ' + err));
  });
  $('prevBtn').addEventListener('click', () => {
    const p = C.addMonths(view.year, view.month, -1);
    goTo(p.year, p.month);
  });
  $('nextBtn').addEventListener('click', () => {
    const n = C.addMonths(view.year, view.month, 1);
    goTo(n.year, n.month);
  });
  $('todayBtn').addEventListener('click', () => {
    const now = new Date();
    selected = C.keyOfDate(now);
    goTo(now.getFullYear(), now.getMonth() + 1);
  });
  $('card').addEventListener('wheel', (e) => {
    if (e.target.closest('.info')) return;
    const n = C.addMonths(view.year, view.month, e.deltaY > 0 ? 1 : -1);
    goTo(n.year, n.month);
  }, { passive: true });
  $('addBtn').addEventListener('click', () => openAdd($('addRow').hidden));
  $('addSave').addEventListener('click', saveTask);
  $('addText').addEventListener('keydown', (e) => {
    if (e.key === 'Enter') saveTask();
    if (e.key === 'Escape') openAdd(false);
  });
  $('topBtn').addEventListener('click', () =>
    invoke('widget_pin_top', { id: 'calendar', top: !data.pinTop }).catch((e) => showMsg(String(e), true)));
  $('unpinBtn').addEventListener('click', () =>
    invoke('widget_pin_set', { id: 'calendar', pinned: false }).catch((e) => showMsg(String(e), true)));

  // Qua nửa đêm: đổi ô hôm nay; đang xem tháng của hôm qua thì đi theo sang tháng mới,
  // còn đang chọn hôm qua thì chọn sang hôm nay. Đang xem tháng khác thì giữ nguyên.
  setInterval(() => {
    const now = new Date();
    const key = C.keyOfDate(now);
    if (key === todayKey) return;
    const prev = parseKey(todayKey);
    if (view.year === prev.y && view.month === prev.m) view = { year: now.getFullYear(), month: now.getMonth() + 1 };
    if (selected === todayKey) selected = key;
    todayKey = key;
    render();
  }, 30000);

  listen('calendar-data', (e) => receive(e && e.payload));
  listen('tasks-refresh', loadTasks);
  invoke('calendar_get').then(receive).catch((e) => log('[cal] calendar_get: ' + e));
  loadTasks();
})();
