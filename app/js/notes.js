// Bảng 🗒️ Ghi chú: thông tin hay dùng hằng ngày (không phải việc cần làm).
// Tìm không dấu, đưa lên đầu, ghim lên desktop, copy nhanh, sửa tại chỗ.
// Tab 🎗 Sự kiện: các sự kiện lịch (events.json) theo lần xảy ra kế tiếp; bấm
// một dòng để mở lịch ghim ở đúng ngày đó.
(function () {
  const { invoke, listen } = window.DC;
  const { visibleNotes, matches } = window.NotesCore;
  const Cal = window.CalendarCore;
  const T = (key, vars) => window.I18n.t(key, vars);
  const $ = (id) => document.getElementById(id);

  let notes = [];
  let events = [];
  let bookmarks = [];
  let tab = 'notes';
  // Id ghi chú đang mở rộng nội dung (giữ qua các lần vẽ lại).
  const expanded = new Set();
  const p2 = (x) => String(x).padStart(2, '0');

  function fmtTime(millis) {
    const n = Number(millis);
    if (!Number.isFinite(n) || n <= 0) return '';
    const d = new Date(n);
    const now = new Date();
    const hhmm = `${p2(d.getHours())}:${p2(d.getMinutes())}`;
    const sameDay =
      d.getFullYear() === now.getFullYear() && d.getMonth() === now.getMonth() && d.getDate() === now.getDate();
    return sameDay ? hhmm : `${p2(d.getDate())}/${p2(d.getMonth() + 1)}/${d.getFullYear()} ${hhmm}`;
  }

  // Dòng trạng thái dưới ô tìm kiếm. Tự ẩn sau vài giây.
  let statusTimer = 0;
  function showStatus(msg, ok = false, ms) {
    const el = $('ntStatus');
    el.textContent = msg;
    el.classList.toggle('ok', ok);
    clearTimeout(statusTimer);
    if (msg) statusTimer = setTimeout(() => { el.textContent = ''; }, ms || (ok ? 2000 : 6000));
  }

  async function openLink(url) {
    try {
      await invoke('open_url', { url });
    } catch (e) {
      showStatus(String(e));
    }
  }

  // Gọi Rust, lỗi thì hiện lên dòng trạng thái. Trả về true nếu thành công.
  async function call(cmd, args) {
    try {
      await invoke(cmd, args);
      return true;
    } catch (e) {
      showStatus(String(e), false, 10000);
      return false;
    }
  }

  // Copy nội dung (không có nội dung thì copy tiêu đề). WebView từ chối ghi
  // clipboard (vd cửa sổ mất focus) thì promise bị reject: báo rõ, đừng báo "đã copy".
  async function copyNote(n) {
    const text = n.body || n.title;
    try {
      await navigator.clipboard.writeText(text);
      showStatus(T('notes.copied'), true);
    } catch (e) {
      showStatus(T('notes.copyFail', { err: e && e.message ? e.message : e }));
    }
  }

  function btn(cls, label, title, onClick) {
    const b = document.createElement('button');
    b.type = 'button';
    b.className = 'nt-btn ' + cls;
    b.textContent = label;
    b.title = title;
    b.addEventListener('click', (e) => {
      e.stopPropagation();
      onClick(b);
    });
    return b;
  }

  // 🗑 bấm lần đầu chỉ hỏi lại ("Xoá?"), bấm lần nữa mới gọi `onDelete`.
  function deleteBtn(title, onDelete) {
    let timer = 0;
    return btn('del', '🗑', title, async (b) => {
      if (!b.classList.contains('confirm')) {
        b.classList.add('confirm');
        b.textContent = T('notes.delConfirm');
        b.title = T('notes.delConfirmTitle');
        timer = setTimeout(() => {
          b.classList.remove('confirm');
          b.textContent = '🗑';
          b.title = title;
        }, 3000);
        return;
      }
      clearTimeout(timer);
      await onDelete();
    });
  }

  // ─── Tab Ghi chú ────────────────────────────────────────────────────────────

  function makeNote(n) {
    const row = document.createElement('div');
    row.className = 'nt-item tk-item' + (n.pinned ? ' pinned' : '');

    const top = document.createElement('div');
    top.className = 'nt-top';
    const title = document.createElement('div');
    title.className = 'nt-title' + (n.title ? '' : ' untitled');
    title.textContent = n.title || T('notes.untitled');

    const star = btn('pin' + (n.pinned ? ' on' : ''), '⭐', n.pinned ? T('notes.unstar') : T('notes.star'), async () => {
      await call('notes_set_pinned', { id: n.id, pinned: !n.pinned });
      render();
    });
    const desk = btn('pin' + (n.desk ? ' on' : ''), '📌', n.desk ? T('notes.undesk') : T('notes.desk'), async () => {
      await call('notes_set_desk', { id: n.id, on: !n.desk });
      render();
    });
    const copy = btn('copy', '📋', T('notes.copyBody'), () => copyNote(n));
    const edit = btn('edit', '✏️', T('notes.edit'), () => openEditor(n, row));
    const del = deleteBtn(T('notes.del'), async () => {
      expanded.delete(n.id);
      await call('notes_delete', { id: n.id });
      render();
    });
    top.append(title, star, desk, copy, edit, del);
    row.appendChild(top);

    const body = document.createElement('div');
    body.className = 'nt-body' + (expanded.has(n.id) ? ' open' : '');
    if (n.body) window.Linkify.renderLinks(body, n.body, openLink);
    body.title = T('notes.bodyToggle');
    body.addEventListener('click', () => {
      body.classList.toggle('open');
      if (body.classList.contains('open')) expanded.add(n.id);
      else expanded.delete(n.id);
    });
    row.appendChild(body);

    const when = fmtTime(n.updated_at || n.created_at);
    if (when) {
      const meta = document.createElement('div');
      meta.className = 'nt-meta';
      meta.textContent = '🕒 ' + when + (n.desk ? ' · ' + T('notes.onDesk') : '');
      row.appendChild(meta);
    }

    row.addEventListener('dblclick', (e) => {
      if (!e.target.closest('button, a')) openEditor(n, row);
    });
    return row;
  }

  // Ô soạn dùng chung cho thêm mới (n = null, đặt ở đầu cột) và sửa (thay chỗ thẻ).
  // `prefill` = nội dung điền sẵn (đoạn bôi đen gửi từ bông hoa). Ctrl+Enter lưu, Esc huỷ.
  function openEditor(n, replaceEl, prefill) {
    const open = document.querySelector('.nt-editor');
    if (open) {
      // Đang soạn dở mà có đoạn mới gửi tới: nối vào cuối, đừng nuốt mất.
      // (Ô đang mở có thể là ô bookmark, không có textarea.)
      const ta = open.querySelector('textarea');
      if (prefill && ta) ta.value = ta.value ? ta.value.replace(/\s*$/, '') + '\n\n' + prefill : prefill;
      ((prefill || !ta) ? open.querySelector('input') : ta).focus();
      return;
    }
    const box = document.createElement('div');
    box.className = 'nt-editor';
    const title = document.createElement('input');
    title.type = 'text';
    title.placeholder = T('notes.titlePh');
    title.maxLength = 120;
    title.value = n ? n.title : '';
    title.setAttribute('aria-label', T('notes.titleLabel'));
    const body = document.createElement('textarea');
    body.setAttribute('aria-label', T('notes.bodyLabel'));
    body.placeholder = T('notes.bodyPh');
    body.value = n ? n.body : prefill || '';

    const bar = document.createElement('div');
    bar.className = 'nt-editor-bar';
    const hint = document.createElement('span');
    hint.className = 'hint';
    hint.textContent = T('notes.hint');
    const cancel = document.createElement('button');
    cancel.type = 'button';
    cancel.className = 'ghost';
    cancel.textContent = T('notes.cancel');
    const save = document.createElement('button');
    save.type = 'button';
    save.className = 'action';
    save.textContent = T('notes.save');
    bar.append(hint, cancel, save);
    box.append(title, body, bar);

    const close = () => {
      box.remove();
      render();
    };
    save.addEventListener('click', async () => {
      if (!title.value.trim() && !body.value.trim()) {
        showStatus(T('notes.needText'));
        title.focus();
        return;
      }
      const ok = n
        ? await call('notes_update', { id: n.id, title: title.value, body: body.value })
        : await call('notes_add', { title: title.value, body: body.value });
      // Lưu hỏng thì giữ ô soạn lại để không mất chữ đã gõ.
      if (ok) close();
    });
    cancel.addEventListener('click', close);
    box.addEventListener('keydown', (e) => {
      if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) {
        e.preventDefault();
        save.click();
      } else if (e.key === 'Escape') {
        e.preventDefault();
        close();
      } else if (e.key === 'Enter' && e.target === title) {
        e.preventDefault();
        body.focus();
      }
    });

    if (replaceEl) replaceEl.replaceWith(box);
    else $('ntComposer').appendChild(box);
    // Sửa: vào thẳng nội dung. Mới (kể cả có sẵn nội dung): gõ tiêu đề trước.
    (n ? body : title).focus();
  }

  // ─── Tab Bookmark (bookmarks.json; ô mở nhanh tìm trong đây) ────────────────

  function makeBookmark(b) {
    const row = document.createElement('div');
    row.className = 'nt-item tk-item ev';
    row.title = T('notes.bm.open');
    const top = document.createElement('div');
    top.className = 'nt-top';
    const title = document.createElement('div');
    title.className = 'nt-title';
    title.textContent = '🔖 ' + b.title;
    top.appendChild(title);
    if (b.group) {
      const g = document.createElement('span');
      g.className = 'nt-badge';
      g.textContent = b.group;
      top.appendChild(g);
    }
    const copy = btn('copy', '📋', T('notes.bm.copy'), async () => {
      try {
        await navigator.clipboard.writeText(b.url);
        showStatus(T('notes.bm.copied'), true);
      } catch (e) {
        showStatus(T('notes.copyFail', { err: e && e.message ? e.message : e }));
      }
    });
    const edit = btn('edit', '✏️', T('notes.edit'), () => openBmEditor(b, row));
    const del = deleteBtn(T('notes.bm.del'), async () => {
      await call('bookmarks_delete', { id: b.id });
      render();
    });
    top.append(copy, edit, del);
    const url = document.createElement('div');
    url.className = 'nt-when';
    url.textContent = b.url;
    row.append(top, url);
    row.addEventListener('click', (e) => {
      if (!e.target.closest('button')) openLink(b.url);
    });
    return row;
  }

  // Ô soạn bookmark (thêm mới: b = null). Enter lưu, Esc huỷ.
  function openBmEditor(b, replaceEl) {
    const open = document.querySelector('.nt-editor');
    if (open) {
      open.querySelector('input').focus();
      return;
    }
    const box = document.createElement('div');
    box.className = 'nt-editor';
    const input = (ph, value, label, max) => {
      const i = document.createElement('input');
      i.type = 'text';
      i.placeholder = ph;
      i.value = value || '';
      i.maxLength = max;
      i.setAttribute('aria-label', label);
      return i;
    };
    const url = input(T('notes.bm.urlPh'), b && b.url, T('notes.bm.urlLabel'), 2000);
    const title = input(T('notes.bm.namePh'), b && b.title, T('notes.bm.nameLabel'), 80);
    const group = input(T('notes.bm.groupPh'), b && b.group, T('notes.bm.groupLabel'), 30);
    const bar = document.createElement('div');
    bar.className = 'nt-editor-bar';
    const hint = document.createElement('span');
    hint.className = 'hint';
    hint.textContent = T('notes.bm.hint');
    const cancel = document.createElement('button');
    cancel.type = 'button';
    cancel.className = 'ghost';
    cancel.textContent = T('notes.cancel');
    const save = document.createElement('button');
    save.type = 'button';
    save.className = 'action';
    save.textContent = T('notes.save');
    bar.append(hint, cancel, save);
    box.append(url, title, group, bar);

    const close = () => {
      box.remove();
      render();
    };
    save.addEventListener('click', async () => {
      const args = { title: title.value, url: url.value, group: group.value };
      const ok = b ? await call('bookmarks_update', { id: b.id, ...args }) : await call('bookmarks_add', args);
      // Rust từ chối (URL không phải http / https…): giữ ô lại, lỗi đã hiện ở dòng trạng thái.
      if (ok) close();
      else url.focus();
    });
    cancel.addEventListener('click', close);
    box.addEventListener('keydown', (e) => {
      if (e.key === 'Enter') {
        e.preventDefault();
        save.click();
      } else if (e.key === 'Escape') {
        e.preventDefault();
        close();
      }
    });
    if (replaceEl) replaceEl.replaceWith(box);
    else $('ntComposer').appendChild(box);
    url.focus();
  }

  // ─── Tab Sự kiện ────────────────────────────────────────────────────────────

  // Tên thứ lấy từ bảng của lịch (calendar-core.js, bắt đầu từ Thứ Hai).
  function fmtDay(key) {
    const [y, m, d] = key.split('-').map(Number);
    const wd = Cal.strings(window.I18n.lang()).weekdaysLong[(new Date(y, m - 1, d).getDay() + 6) % 7];
    return T('notes.ev.day', { wd, d: p2(d), m: p2(m), y });
  }

  /** Nhãn đếm ngược và mức tô màu. */
  function countdown(days) {
    if (days == null) return { text: T('notes.ev.past'), cls: '' };
    if (days === 0) return { text: T('notes.ev.today'), cls: 'today' };
    if (days === 1) return { text: T('notes.ev.tomorrow'), cls: 'soon' };
    return { text: T('notes.ev.left', { n: days }), cls: days <= 7 ? 'soon' : '' };
  }

  function repeatText(ev) {
    if (ev.repeat === 'yearly') return T('notes.ev.yearly');
    if (ev.repeat === 'lunar') {
      const [y, m, d] = ev.date.split('-').map(Number);
      const l = Cal.toLunar(d, m, y);
      return T('notes.ev.lunar', { d: l.day, m: l.month });
    }
    return '';
  }

  function makeEvent(r) {
    const ev = r.ev;
    const row = document.createElement('div');
    row.className = 'nt-item tk-item ev' + (r.next ? '' : ' past');
    row.title = T('notes.ev.open');

    const top = document.createElement('div');
    top.className = 'nt-top';
    const title = document.createElement('div');
    title.className = 'nt-title';
    title.textContent = '🎗 ' + ev.title;
    const cd = countdown(r.days);
    const badge = document.createElement('span');
    badge.className = 'nt-badge ' + cd.cls;
    badge.textContent = cd.text;
    const del = deleteBtn(T('notes.ev.del'), async () => {
      await call('events_delete', { id: ev.id });
      render();
    });
    top.append(title, badge, del);

    const when = document.createElement('div');
    when.className = 'nt-when';
    const parts = [fmtDay(r.next || ev.date), ev.time, repeatText(ev)].filter(Boolean);
    if (ev.remind_days && ev.remind_days.length) parts.push('🔔 ' + Cal.remindTitle(ev.remind_days, window.I18n.lang()));
    when.textContent = parts.join(' · ');
    row.append(top, when);

    row.addEventListener('click', (e) => {
      if (e.target.closest('button')) return;
      openOnCalendar(r.next || ev.date);
    });
    return row;
  }

  async function openOnCalendar(date) {
    if (await call('calendar_open_day', { date })) showStatus(T('notes.ev.opened', { day: fmtDay(date) }), true, 2500);
  }

  // ─── Vẽ ─────────────────────────────────────────────────────────────────────

  function emptyRow(html) {
    const empty = document.createElement('div');
    empty.className = 'nt-empty';
    empty.innerHTML = html;
    return empty;
  }

  function fillList() {
    const list = $('ntList');
    list.innerHTML = '';
    const q = $('ntSearch').value;
    const searching = !!q.trim();

    const shownNotes = visibleNotes(notes, q);
    $('ntCount').textContent = searching ? `${shownNotes.length}/${notes.length}` : String(notes.length);
    const rows = Cal.upcomingEvents(events, Cal.keyOfDate(new Date())).filter((r) => matches({ title: r.ev.title, body: '' }, q));
    $('evCount').textContent = searching ? `${rows.length}/${events.length}` : String(events.length);

    const shownBm = (bookmarks || [])
      .filter((b) => matches({ title: b.title, body: `${b.group} ${b.url}` }, q))
      .sort((a, b) => (a.group || '￿').localeCompare(b.group || '￿') || a.title.localeCompare(b.title));
    $('bmCount').textContent = searching ? `${shownBm.length}/${bookmarks.length}` : String(bookmarks.length);

    if (tab === 'bookmarks') {
      if (!shownBm.length) {
        list.appendChild(emptyRow(bookmarks.length ? T('notes.bm.none') : T('notes.bm.empty')));
        return;
      }
      for (const b of shownBm) list.appendChild(makeBookmark(b));
      return;
    }
    if (tab === 'events') {
      if (!rows.length) {
        list.appendChild(emptyRow(events.length ? T('notes.ev.none') : T('notes.ev.empty')));
        return;
      }
      for (const r of rows) list.appendChild(makeEvent(r));
      return;
    }
    if (!shownNotes.length) {
      list.appendChild(emptyRow(notes.length ? T('notes.none') : T('notes.empty')));
      return;
    }
    for (const n of shownNotes) list.appendChild(makeNote(n));
  }

  async function render() {
    // Đang soạn / sửa thì không vẽ lại, kẻo mất chữ đang gõ.
    if (document.querySelector('.nt-editor')) return;
    // Ba file độc lập: file này hỏng thì vẫn hiện được file kia.
    const [n, e, b] = await Promise.allSettled([invoke('notes_list'), invoke('events_list'), invoke('bookmarks_list')]);
    notes = n.status === 'fulfilled' ? n.value || [] : [];
    events = e.status === 'fulfilled' ? e.value || [] : [];
    bookmarks = b.status === 'fulfilled' ? b.value || [] : [];
    const errs = [n, e, b].filter((r) => r.status === 'rejected').map((r) => String(r.reason));
    if (errs.length) showStatus(errs.join(' · '), false, 20000);
    fillList();
  }

  // Chữ ô tìm + nút "mới" theo tab đang mở (vẽ lại cả khi đổi ngôn ngữ).
  function paintHead() {
    $('ntSearch').placeholder = T('notes.search.' + tab);
    $('ntNew').textContent = T('notes.new.' + tab);
    $('ntNew').title = tab === 'events' ? T('notes.new.eventsTitle') : 'Ctrl+N';
  }

  function setTab(next) {
    tab = next;
    for (const b of document.querySelectorAll('.nt-tabs .tab')) b.classList.toggle('active', b.dataset.tab === tab);
    const isEvents = tab === 'events';
    paintHead();
    $('ntComposer').hidden = isEvents;
    fillList();
  }

  document.querySelector('.nt-tabs').addEventListener('click', (e) => {
    const b = e.target.closest('.tab');
    if (b) setTab(b.dataset.tab);
  });
  $('ntNew').addEventListener('click', () => {
    if (tab === 'events') openOnCalendar(Cal.keyOfDate(new Date()));
    else if (tab === 'bookmarks') openBmEditor(null, null);
    else openEditor(null, null);
  });
  $('ntSearch').addEventListener('input', () => {
    if (!document.querySelector('.nt-editor')) fillList();
  });
  $('ntSearch').addEventListener('keydown', (e) => {
    if (e.key === 'Escape') {
      e.currentTarget.value = '';
      fillList();
    }
  });
  // Ctrl+N: ghi chú mới; Ctrl+F: nhảy vào ô tìm.
  document.addEventListener('keydown', (e) => {
    if (!(e.ctrlKey || e.metaKey)) return;
    const k = e.key.toLowerCase();
    if (k === 'n') {
      e.preventDefault();
      if (tab === 'bookmarks') {
        openBmEditor(null, null);
        return;
      }
      if (tab !== 'notes') setTab('notes');
      openEditor(null, null);
    } else if (k === 'f') {
      e.preventDefault();
      $('ntSearch').focus();
      $('ntSearch').select();
    }
  });

  // Cửa sổ dựng sẵn một lần: mỗi lần hiện lại thì nạp lại danh sách.
  listen('notes-refresh', render);
  listen('events-refresh', render);
  // Bông hoa → 🗒️ Lưu ghi chú: ô soạn mới điền sẵn đoạn vừa bôi đen.
  listen('notes-compose', (e) => {
    const text = String((e && e.payload) || '').trim();
    if (tab !== 'notes') setTab('notes');
    openEditor(null, null, text);
    if (!text) showStatus(T('notes.composeFail'));
  });
  // Đổi ngôn ngữ: vẽ lại đầu bảng và danh sách (trừ khi đang soạn, kẻo mất chữ).
  window.I18n.onChange(() => {
    paintHead();
    if (!document.querySelector('.nt-editor')) fillList();
  });
  paintHead();
  render();
})();
