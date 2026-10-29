// Lịch tháng ghim trên desktop (calendar-pin.html). Rust chỉ gửi luôn nổi /
// ngôn ngữ (calendar_get / "calendar-data"); task đọc qua tasks_list, sự kiện
// lịch qua events_list, vẽ lại khi có "tasks-refresh" / "events-refresh".
// Bấm một ngày để xem âm lịch, lễ, việc tới hạn, sự kiện và thêm việc / sự kiện.
(function () {
  const { invoke, listen, log } = window.DC;
  const C = window.CalendarCore;
  const $ = (id) => document.getElementById(id);

  let data = { pinTop: true, lang: 'vi' };
  let tasks = [];
  // Sự kiện lịch (events.json): giỗ, sinh nhật… có thể lặp theo dương / âm lịch.
  let events = [];
  const now0 = new Date();
  let view = { year: now0.getFullYear(), month: now0.getMonth() + 1 };
  let selected = C.keyOfDate(now0);
  // Hôm nay lần vẽ trước: qua nửa đêm thì đổi ô "hôm nay" (và sang tháng mới
  // nếu đang xem tháng cũ của hôm qua).
  let todayKey = selected;

  const S = () => C.strings(data.lang);
  // Mốc "nhắc trước" đang chọn ở ô thêm việc; mở ô thì về mặc định 1 ngày.
  let remindPick = [1];
  // Đang thêm "task" hay "event"; lần thêm sau nhớ loại vừa dùng.
  let kind = 'event';
  let lastKind = 'event';
  // Kiểu lặp của sự kiện: "once" | "yearly" | "lunar".
  let repeatPick = 'once';
  // Đang sửa: {type: "task" | "event", item} (null = ô đang thêm mới).
  let editing = null;
  // Việc / sự kiện đang hiện trong khung ngày, theo "t:<id>" / "e:<id>" (bấm vào thì sửa).
  let shown = {};
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
    // Vẽ lại (task / sự kiện đổi) khi ô thêm đang mở thì giữ nguyên trạng thái ô.
    const adding = !$('addRow').hidden;
    $('addBtn').textContent = adding ? s.cancel : s.add;
    $('addBtn').title = s.addTitle;
    $('addSave').textContent = s.save;
    $('remindLbl').textContent = '🔔 ' + s.remind;
    renderChips();
    renderKind();
    setDelArmed($('addDel').classList.contains('confirm'));
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
      const evs = C.eventsOn(events, c.year, c.month, c.day, lunar);
      if (evs.length) b.append(el('span', 'edot'));
      const tips = hol.map((h) => S().h[h.id]);
      if (bday) tips.unshift(C.fill(S().bday, { you: data.you || '' }));
      for (const e of evs) tips.push('🎗 ' + e.title);
      if (due) tips.push(`${due.length} ✓`);
      if (tips.length) b.title = tips.join(' · ');
      grid.append(b);
    }
  }

  // Tiêu đề nút kiểu lặp theo ngày gốc: "Lặp hằng năm vào 28/9 âm lịch…".
  function repeatTitle(rep, dateKey) {
    const s = S();
    if (rep === 'once') return s.onceTitle;
    const { y, m, d } = parseKey(dateKey);
    if (rep === 'yearly') return C.fill(s.yearlyTitle, { d, m });
    const l = C.toLunar(d, m, y);
    return C.fill(s.lunarTitle, { d: l.day, m: l.month });
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
    const evs = C.eventsOn(events, y, m, d, l);
    shown = {};
    // Miu mini: ngày lễ / sinh nhật thì reo, có việc quá hạn thì giật mình,
    // có việc tới hạn thì "cố lên", chỉ có sự kiện thì nghĩ ngợi, còn lại nháy mắt.
    const face = fest.length ? 'cheer' : due.some((t) => t.over) ? 'surprised' : due.length ? 'fight' : evs.length ? 'think' : 'wink';
    const src = `${data.miuDir || 'media/game'}/rps-${face}.${data.miuExt || 'png'}`;
    if ($('miu').getAttribute('src') !== src) $('miu').setAttribute('src', src);
    if (!due.length && !evs.length) list.append(el('div', 'muted', s.noTasks));
    // Sự kiện trước, rồi tới việc; danh sách dài thì cuộn trong khung.
    for (const e of evs) {
      const b = el('button', 'task ev');
      b.type = 'button';
      b.dataset.ref = 'e:' + e.id;
      shown['e:' + e.id] = { type: 'event', item: e };
      b.append(el('span', 't', e.time), el('span', 'x', '🎗 ' + e.title));
      b.title = e.title + '\n✏️ ' + s.edit;
      if (e.repeat !== 'once') {
        b.append(el('span', 'rep', e.repeat === 'lunar' ? '↻☾' : '↻'));
        b.title += '\n' + repeatTitle(e.repeat, e.ev.date);
      }
      if (e.remind.length) {
        b.append(el('span', 'bell', '🔔'));
        b.title += '\n🔔 ' + C.remindTitle(e.remind, data.lang);
      }
      list.append(b);
    }
    for (const t of due) {
      const b = el('button', t.over ? 'task over' : 'task');
      b.type = 'button';
      b.dataset.ref = 't:' + t.id;
      shown['t:' + t.id] = { type: 'task', item: t };
      const at = new Date(t.due);
      b.append(el('span', 't', `${p2(at.getHours())}:${p2(at.getMinutes())}`), el('span', 'x', t.text));
      b.title = (t.over ? `${t.text} (${s.overdue})` : t.text) + '\n✏️ ' + s.edit;
      if (t.remind.length && !t.over) {
        b.append(el('span', 'bell', '🔔'));
        b.title += '\n🔔 ' + C.remindTitle(t.remind, data.lang);
      }
      list.append(b);
    }
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

  // events.json hỏng thì Rust từ chối đọc (để không ghi đè mất dữ liệu): báo
  // thẳng lên khung, đừng hiện lịch trống như không có gì.
  async function loadEvents() {
    try {
      events = (await invoke('events_list')) || [];
    } catch (e) {
      events = [];
      showMsg(String(e), true, 15000);
      log('[cal] events_list: ' + e);
    }
    render();
  }

  function receive(v) {
    if (!v) return;
    data = v;
    // Skin màu chọn trong Bộ sưu tập (app/skins.css).
    document.body.dataset.skin = data.skin || 'lavender';
    // Trang trí trên mép thẻ (widget-deco.css), cũng chọn trong Bộ sưu tập.
    window.WidgetDeco.mount($('wrap'), data.deco);
    render();
  }

  function goTo(year, month) {
    view = { year, month };
    render();
  }

  // Bảng Ghi chú → tab Sự kiện → bấm một sự kiện: lật tới tháng đó, chọn ngày đó.
  function goToDay(key) {
    if (!/^\d{4}-\d{2}-\d{2}$/.test(key || '')) return;
    openAdd(false);
    selected = key;
    const { y, m } = parseKey(key);
    goTo(y, m);
  }

  function showMsg(text, err, ms = 4000) {
    const m = $('msg');
    m.textContent = text || '';
    m.classList.toggle('err', !!err);
    m.title = text || '';
    clearTimeout(showMsg.timer);
    if (text) showMsg.timer = setTimeout(() => { m.textContent = ''; m.title = ''; }, ms);
  }

  function renderChips() {
    const box = $('remindChips');
    box.textContent = '';
    for (const d of C.REMIND_CHOICES) {
      const b = el('button', remindPick.includes(d) ? 'chip on' : 'chip', C.remindLabel(d, data.lang));
      b.type = 'button';
      b.dataset.d = String(d);
      box.append(b);
    }
  }

  // Hàng chọn ✓ Việc / 🎗 Sự kiện, và (với sự kiện) kiểu lặp.
  function renderKind() {
    const s = S();
    const kinds = $('kindChips');
    kinds.textContent = '';
    for (const [k, label, title] of [['task', s.kindTask, s.kindTaskTitle], ['event', s.kindEvent, s.kindEventTitle]]) {
      const b = el('button', kind === k ? 'chip on' : 'chip', label);
      b.type = 'button';
      b.dataset.kind = k;
      b.title = title;
      kinds.append(b);
    }
    const reps = $('repeatChips');
    reps.textContent = '';
    const isEvent = kind === 'event';
    $('kindSep').hidden = !isEvent;
    reps.hidden = !isEvent;
    // Sửa sự kiện lặp: kiểu lặp tính theo ngày gốc của nó, không phải ngày đang xem.
    const origin = editing && editing.type === 'event' ? editing.item.ev.date : selected;
    for (const [r, label] of [['once', s.once], ['yearly', s.yearly], ['lunar', s.lunarYearly]]) {
      const b = el('button', repeatPick === r ? 'chip on' : 'chip', label);
      b.type = 'button';
      b.dataset.rep = r;
      b.title = repeatTitle(r, origin);
      reps.append(b);
    }
    $('addText').placeholder = isEvent ? s.evPlaceholder : s.placeholder;
  }

  function setDelArmed(on) {
    const b = $('addDel');
    b.classList.toggle('confirm', on);
    b.textContent = on ? S().delConfirm : S().del;
    b.title = S().delTitle;
  }

  function openAdd(on) {
    editing = null;
    $('info').classList.toggle('adding', on);
    $('kindRow').hidden = !on;
    $('addRow').hidden = !on;
    $('remindRow').hidden = !on;
    $('addDel').hidden = true;
    setDelArmed(false);
    if (on) {
      kind = lastKind;
      repeatPick = 'once';
      remindPick = [1];
      renderChips();
      renderKind();
    }
    $('addBtn').textContent = on ? S().cancel : S().add;
    if (on) {
      $('addText').value = '';
      $('addText').focus();
    }
  }

  /** Mở ô sửa một việc / sự kiện đã có: điền sẵn tên, giờ, nhắc trước, kiểu lặp. */
  function openEdit(ref) {
    openAdd(true);
    editing = ref;
    $('addDel').hidden = false;
    if (ref.type === 'event') {
      const e = ref.item;
      kind = 'event';
      repeatPick = e.repeat;
      $('addText').value = e.title;
      $('addTime').value = e.time;
      remindPick = [...e.remind];
    } else {
      const t = ref.item;
      kind = 'task';
      repeatPick = 'once';
      $('addText').value = t.text;
      $('addTime').value = C.hhmm(t.due);
      remindPick = [...t.remind];
    }
    renderChips();
    renderKind();
  }

  async function saveTaskEdit(task, text, due) {
    const ch = C.editChanges(task, text, due, remindPick);
    if (ch.text != null) await invoke('tasks_set_text', { id: task.id, text: ch.text });
    if (ch.due != null) await invoke('tasks_set_due', { id: task.id, dueAt: String(ch.due) });
    // Đổi giờ thì Rust xoá các mốc đã nhắc; gửi lại mốc nhắc trước cho chắc.
    if (ch.remind || ch.due != null) await invoke('tasks_set_remind', { id: task.id, days: remindPick });
  }

  async function addTask(text, due) {
    const t = await invoke('tasks_add', { text, note: '', source: 'manual' });
    await invoke('tasks_set_due', { id: t.id, dueAt: String(due) });
    if (remindPick.length) await invoke('tasks_set_remind', { id: t.id, days: remindPick });
  }

  // Lưu theo loại đang chọn. Đổi loại khi sửa (việc ↔ sự kiện) = tạo bản mới
  // ở loại kia rồi mới xoá bản cũ, để lỗi giữa chừng không làm mất gì.
  async function save() {
    const text = $('addText').value.trim();
    if (!text) {
      $('addText').focus();
      return;
    }
    const time = $('addTime').value;
    const due = C.dueAt(selected, time);
    if (due == null) {
      showMsg('⚠️ ' + time, true);
      return;
    }
    const ed = editing;
    $('addSave').disabled = true;
    try {
      if (kind === 'event') {
        const args = { title: text, time, repeat: repeatPick, remindDays: remindPick };
        if (ed && ed.type === 'event') {
          await invoke('events_update', { id: ed.item.id, date: ed.item.ev.date, ...args });
        } else {
          await invoke('events_add', { date: selected, ...args });
          if (ed) await invoke('tasks_delete', { id: ed.item.id });
        }
      } else if (ed && ed.type === 'task') {
        await saveTaskEdit(ed.item, text, due);
      } else {
        await addTask(text, due);
        if (ed) await invoke('events_delete', { id: ed.item.id });
      }
      lastKind = kind;
      openAdd(false);
      showMsg(ed ? S().edited : kind === 'event' ? S().evSaved : S().saved);
    } catch (e) {
      // Báo nguyên văn lỗi từ Rust (vd. hạn chót không hợp lệ), đừng im lặng.
      showMsg(String(e), true, 8000);
      log('[cal] lưu: ' + e);
    } finally {
      $('addSave').disabled = false;
      // Rust gửi "tasks-refresh" / "events-refresh"; vẽ lại luôn cho chắc.
      loadTasks();
      loadEvents();
    }
  }

  // 🗑 bấm lần đầu chỉ hỏi lại ("Xoá?"), bấm lần nữa mới xoá.
  async function removeEditing() {
    const ed = editing;
    if (!ed) return;
    const b = $('addDel');
    if (!b.classList.contains('confirm')) {
      setDelArmed(true);
      clearTimeout(removeEditing.timer);
      removeEditing.timer = setTimeout(() => setDelArmed(false), 3000);
      return;
    }
    clearTimeout(removeEditing.timer);
    try {
      await invoke(ed.type === 'event' ? 'events_delete' : 'tasks_delete', { id: ed.item.id });
      openAdd(false);
      showMsg(S().deleted);
    } catch (e) {
      showMsg(String(e), true, 8000);
      log('[cal] xoá: ' + e);
    } finally {
      loadTasks();
      loadEvents();
    }
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
    // Bấm một việc / sự kiện: sửa ngay trong khung (tên, giờ, nhắc trước, kiểu lặp).
    const b = e.target.closest('.task');
    if (b && shown[b.dataset.ref]) openEdit(shown[b.dataset.ref]);
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
  $('addSave').addEventListener('click', save);
  $('addDel').addEventListener('click', removeEditing);
  $('kindRow').addEventListener('click', (e) => {
    const k = e.target.closest('[data-kind]');
    const r = e.target.closest('[data-rep]');
    if (k) kind = k.dataset.kind;
    if (r) repeatPick = r.dataset.rep;
    if (k || r) renderKind();
  });
  $('remindChips').addEventListener('click', (e) => {
    const b = e.target.closest('.chip');
    if (!b) return;
    const d = Number(b.dataset.d);
    remindPick = remindPick.includes(d) ? remindPick.filter((x) => x !== d) : [...remindPick, d];
    renderChips();
  });
  $('addText').addEventListener('keydown', (e) => {
    if (e.key === 'Enter') save();
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
  listen('events-refresh', loadEvents);
  // Ngày chờ mở: nhận qua sự kiện khi cửa sổ đã có, hoặc tự lấy lúc vừa dựng.
  // Lấy xong thì Rust xoá, để lần mở lịch sau không nhảy về ngày cũ.
  listen('calendar-goto', (e) => {
    goToDay(e && e.payload);
    invoke('calendar_take_goto').catch(() => {});
  });
  invoke('calendar_take_goto').then(goToDay).catch((e) => log('[cal] calendar_take_goto: ' + e));
  invoke('calendar_get').then(receive).catch((e) => log('[cal] calendar_get: ' + e));
  loadTasks();
  loadEvents();
})();
