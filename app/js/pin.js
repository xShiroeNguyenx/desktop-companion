// Một tờ note ghim trên desktop (pin.html). Id task do Rust tiêm vào
// window.__PIN_ID__ lúc dựng cửa sổ (pins.rs → open_on_main).
(function () {
  const { invoke, listen, log } = window.DC;
  const $ = (id) => document.getElementById(id);
  const id = String(window.__PIN_ID__ || '');
  const FALLBACK_AVATAR = '/media/character.png';
  let task = null;
  let busy = false;

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
      invoke('pin_resize', { id, w: wrap.offsetWidth, h: wrap.offsetHeight }).catch(() => {});
    });
  }

  function showError(msg) {
    $('err').textContent = msg || '';
    fitWindow();
  }

  async function render() {
    try {
      task = await invoke('pin_get', { id });
    } catch (e) {
      log('[pin] pin_get lỗi: ' + e);
    }
    if (!task) return; // task đã bị xoá: Rust sẽ đóng cửa sổ này
    window.Linkify.renderLinks($('text'), task.text, openLink);
    $('note').title = task.text;
    window.Linkify.renderLinks($('memo'), task.note || '', openLink);
    const due = window.DueFmt.describeDue(task.due_at, Date.now());
    $('due').textContent = due ? due.text : '';
    $('due').className = 'due' + (due ? ' ' + due.level : '');
    $('when').textContent = fmtWhen(task.created_at);
    $('topBtn').classList.toggle('on', !!task.pin_top);
    $('topBtn').title = task.pin_top
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

  // Nhấp đúp mở bảng Công việc để sửa đầy đủ.
  $('note').addEventListener('dblclick', (e) => {
    if (!e.target.closest('button, a')) invoke('open_tasks').catch(() => {});
  });

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

  $('unpinBtn').addEventListener('click', async () => {
    try {
      await invoke('tasks_set_pinned', { id, pinned: false });
    } catch (e) {
      showError('Không gỡ được: ' + e);
    }
  });

  $('topBtn').addEventListener('click', async () => {
    const next = !(task && task.pin_top);
    try {
      await invoke('pin_set_top', { id, top: next });
      render();
    } catch (e) {
      showError('Không đổi được: ' + e);
    }
  });

  listen('pin-refresh', render);
  // Nhãn hạn chót ("còn 20 phút") tự đếm xuống.
  setInterval(() => { if (task && task.due_at && !busy) render(); }, 60_000);
  listen('pin-avatar', (ev) => {
    if (typeof ev.payload === 'string' && ev.payload) $('avatar').src = ev.payload;
  });

  render();
  loadAvatar();
})();
