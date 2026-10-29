// Một tờ note ghim trên desktop (pin.html). Id do Rust tiêm vào
// window.__PIN_ID__ lúc dựng cửa sổ. Hai loại tờ dùng chung trang này:
//   - task ghim (pins.rs, mặc định): nút ✓ Xong, hạn chót;
//   - ghi chú ghim (note_pins.rs, window.__PIN_KIND__ = "note"): nút 📋 Copy.
(function () {
  const { invoke, listen, log } = window.DC;
  const $ = (id) => document.getElementById(id);
  const id = String(window.__PIN_ID__ || '');
  const isNote = window.__PIN_KIND__ === 'note';
  const FALLBACK_AVATAR = '/media/character.png';
  let item = null;
  let busy = false;

  // Lệnh Rust theo loại tờ.
  const api = isNote
    ? {
        get: 'note_pin_get',
        resize: 'note_pin_resize',
        setTop: 'note_pin_set_top',
        unpin: () => invoke('notes_set_desk', { id, on: false }),
        open: 'open_notes',
        top: (n) => !!n.desk_top,
      }
    : {
        get: 'pin_get',
        resize: 'pin_resize',
        setTop: 'pin_set_top',
        unpin: () => invoke('tasks_set_pinned', { id, pinned: false }),
        open: 'open_tasks',
        top: (t) => !!t.pin_top,
      };

  function currentWindow() {
    const T = window.__TAURI__ || {};
    return (T.window && T.window.getCurrentWindow && T.window.getCurrentWindow()) || null;
  }

  // "hôm nay 09:30" / "12/10" — ngày ghim, đọc lướt là đủ.
  function fmtWhen(millis) {
    const n = Number(millis);
    if (!n) return '';
    const d = new Date(n);
    const p2 = (x) => String(x).padStart(2, '0');
    const now = new Date();
    if (d.toDateString() === now.toDateString()) return `hôm nay ${p2(d.getHours())}:${p2(d.getMinutes())}`;
    return `${p2(d.getDate())}/${p2(d.getMonth() + 1)}`;
  }

  // Co cửa sổ vừa khít tờ note: phần trong suốt thừa vẫn chặn click xuống desktop.
  function fitWindow() {
    requestAnimationFrame(() => {
      const wrap = $('wrap');
      invoke(api.resize, { id, w: wrap.offsetWidth, h: wrap.offsetHeight }).catch(() => {});
    });
  }

  function showError(msg) {
    $('err').textContent = msg || '';
    fitWindow();
  }

  function renderTask(task) {
    window.Linkify.renderLinks($('text'), task.text, openLink);
    $('note').title = task.text;
    window.Linkify.renderLinks($('memo'), task.note || '', openLink);
    const due = window.DueFmt.describeDue(task.due_at, Date.now());
    $('due').textContent = due ? due.text : '';
    $('due').className = 'due' + (due ? ' ' + due.level : '');
    $('when').textContent = fmtWhen(task.created_at);
  }

  function renderNote(note) {
    window.Linkify.renderLinks($('text'), note.title || '', openLink);
    window.Linkify.renderLinks($('memo'), note.body || '', openLink);
    $('note').title = [note.title, note.body].filter(Boolean).join('\n');
    $('when').textContent = fmtWhen(note.updated_at || note.created_at);
  }

  async function render() {
    try {
      item = await invoke(api.get, { id });
    } catch (e) {
      log('[pin] ' + api.get + ' lỗi: ' + e);
    }
    if (!item) return; // đã bị xoá: Rust sẽ đóng cửa sổ này
    if (isNote) renderNote(item);
    else renderTask(item);
    const top = api.top(item);
    $('topBtn').classList.toggle('on', top);
    $('topBtn').title = top
      ? 'Đang luôn nổi. Bấm để tờ note nằm lại trên desktop'
      : 'Luôn nổi trên các cửa sổ khác';
    fitWindow();
  }

  async function loadAvatar() {
    try {
      const url = await invoke('pin_avatar');
      if (url) $('avatar').src = url;
    } catch (_) { /* giữ ảnh mặc định */ }
  }
  $('avatar').addEventListener('error', () => {
    if (!$('avatar').src.endsWith(FALLBACK_AVATAR)) $('avatar').src = FALLBACK_AVATAR;
  });

  // Link trong nội dung / ghi chú: mở bằng trình duyệt (Rust chỉ nhận http/https).
  async function openLink(url) {
    try {
      await invoke('open_url', { url });
      showError('');
    } catch (e) {
      showError(String(e));
    }
  }

  // Kéo tờ note: nhấn giữ ở bất cứ đâu trừ các nút và link.
  function startDrag(e) {
    if (e.button !== 0 || e.target.closest('button, a')) return;
    const w = currentWindow();
    if (w && w.startDragging) w.startDragging().catch(() => {});
  }
  $('note').addEventListener('mousedown', startDrag);
  $('pin').addEventListener('mousedown', startDrag);

  // Nhấp đúp mở bảng Công việc / Ghi chú để sửa đầy đủ.
  $('note').addEventListener('dblclick', (e) => {
    if (!e.target.closest('button, a')) invoke(api.open).catch(() => {});
  });

  if (isNote) {
    $('wrap').classList.add('kind-note');
    $('headLabel').textContent = '🗒️ Ghi chú';
    document.title = 'Ghi chú ghim';
    $('unpinBtn').title = 'Gỡ khỏi desktop (ghi chú vẫn còn trong bảng Ghi chú)';
    // Nút chính của tờ ghi chú là Copy nội dung, không có "xong".
    const copy = $('doneBtn');
    copy.textContent = '📋 Copy';
    copy.title = 'Copy nội dung ghi chú';
    copy.addEventListener('click', async () => {
      if (!item) return;
      try {
        await navigator.clipboard.writeText(item.body || item.title || '');
        copy.textContent = '✓ Đã copy';
        setTimeout(() => { copy.textContent = '📋 Copy'; }, 1500);
        showError('');
      } catch (e) {
        // Clipboard từ chối (vd tờ giấy chưa có focus): báo rõ, đừng giả vờ đã copy.
        showError('Không copy được: ' + (e && e.message ? e.message : e));
      }
    });
  } else {
    $('doneBtn').addEventListener('click', async () => {
      if (busy) return;
      busy = true;
      showError('');
      $('wrap').classList.add('finished');
      // Cho người dùng thấy con dấu "XONG!" rồi mới gỡ tờ note.
      await new Promise((r) => setTimeout(r, 900));
      try {
        await invoke('pin_done', { id });
      } catch (e) {
        $('wrap').classList.remove('finished');
        showError('Không lưu được: ' + e);
        busy = false;
      }
    });
  }

  $('unpinBtn').addEventListener('click', async () => {
    try {
      await api.unpin();
    } catch (e) {
      showError('Không gỡ được: ' + e);
    }
  });

  $('topBtn').addEventListener('click', async () => {
    const next = !(item && api.top(item));
    try {
      await invoke(api.setTop, { id, top: next });
      render();
    } catch (e) {
      showError('Không đổi được: ' + e);
    }
  });

  listen('pin-refresh', render);
  // Nhãn hạn chót ("còn 20 phút") tự đếm xuống.
  setInterval(() => { if (!isNote && item && item.due_at && !busy) render(); }, 60_000);
  listen('pin-avatar', (ev) => {
    if (typeof ev.payload === 'string' && ev.payload) $('avatar').src = ev.payload;
  });

  render();
  loadAvatar();
})();
