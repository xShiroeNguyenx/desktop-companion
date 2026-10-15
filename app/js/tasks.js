// Tasks (TODO) window: chia 2 cột chưa làm / đã làm, kèm mốc thời gian tạo và
// hoàn thành, ghi chú sửa được tại chỗ.
(function () {
  const { invoke, listen } = window.DC;
  const $ = (id) => document.getElementById(id);

  // created_at/done_at được lưu dạng chuỗi millis. Task cũ (trước khi có 2
  // trường này) để rỗng — khi đó không hiện dòng thời gian nào.
  function fmtTime(millis) {
    const n = Number(millis);
    if (!millis || !Number.isFinite(n) || n <= 0) return '';
    const d = new Date(n);
    if (Number.isNaN(d.getTime())) return '';
    const p2 = (x) => String(x).padStart(2, '0');
    const today = new Date();
    const sameDay =
      d.getFullYear() === today.getFullYear() &&
      d.getMonth() === today.getMonth() &&
      d.getDate() === today.getDate();
    const hhmm = `${p2(d.getHours())}:${p2(d.getMinutes())}`;
    // Việc trong ngày chỉ cần giờ; cũ hơn thì kèm ngày/tháng.
    return sameDay ? hhmm : `${p2(d.getDate())}/${p2(d.getMonth() + 1)}/${d.getFullYear()} ${hhmm}`;
  }

  function makeItem(t) {
    const row = document.createElement('div');
    row.className = 'tk-item' + (t.done ? ' done' : '');

    const cb = document.createElement('input');
    cb.type = 'checkbox';
    cb.checked = !!t.done;
    cb.title = t.done ? 'Bỏ đánh dấu hoàn thành' : 'Đánh dấu đã làm';
    cb.addEventListener('change', async () => {
      try {
        await invoke('tasks_set_done', { id: t.id, done: cb.checked });
      } catch (e) {
        showStatus(String(e));
      }
      render();
    });

    const body = document.createElement('div');
    body.className = 'tk-body';

    const txt = document.createElement('div');
    txt.className = 'tk-text';
    window.Linkify.renderLinks(txt, t.text, openLink);

    // Ghi chú: bấm vào biến thành textarea, Enter hoặc rời chuột thì lưu.
    // Bấm vào link trong ghi chú thì mở link (renderLinks chặn sự kiện lại).
    const note = document.createElement('div');
    note.className = 'tk-note' + (t.note ? '' : ' empty');
    if (t.note) window.Linkify.renderLinks(note, t.note, openLink);
    else note.textContent = '+ Thêm ghi chú';
    note.title = 'Bấm để sửa ghi chú';
    note.addEventListener('click', () => editNote(t, note));

    const meta = document.createElement('div');
    meta.className = 'tk-meta';
    const created = fmtTime(t.created_at);
    if (created) {
      const span = document.createElement('span');
      span.textContent = '🕒 Tạo: ' + created;
      meta.appendChild(span);
    }
    const doneAt = fmtTime(t.done_at);
    if (t.done && doneAt) {
      const span = document.createElement('span');
      span.className = 'ok';
      span.textContent = '✓ Xong: ' + doneAt;
      meta.appendChild(span);
    }
    // Hạn chót: việc chưa xong thì hiện mức gấp; bấm vào để sửa.
    const due = !t.done && window.DueFmt.describeDue(t.due_at, Date.now());
    if (due) {
      const span = document.createElement('span');
      span.className = 'tk-due ' + due.level;
      span.textContent = due.text;
      span.title = 'Bấm để đổi hạn chót';
      span.addEventListener('click', () => editDue(t, body));
      meta.appendChild(span);
    }

    body.append(txt, note);
    if (meta.childElementCount) body.appendChild(meta);

    const del = document.createElement('button');
    del.className = 'tk-del';
    del.textContent = '🗑';
    del.title = 'Xoá';
    del.type = 'button';
    del.addEventListener('click', async () => {
      await invoke('tasks_delete', { id: t.id });
      render();
    });

    // Ghim lên desktop thành tờ note (chỉ việc chưa làm).
    if (!t.done) {
      const cal = document.createElement('button');
      cal.className = 'tk-cal' + (t.due_at ? ' on' : '');
      cal.textContent = '📅';
      cal.type = 'button';
      cal.title = t.due_at ? 'Đổi hạn chót' : 'Đặt hạn chót';
      cal.addEventListener('click', () => editDue(t, body));

      const pin = document.createElement('button');
      pin.className = 'tk-pin' + (t.pinned ? ' on' : '');
      pin.textContent = '📌';
      pin.type = 'button';
      pin.title = t.pinned ? 'Đang ghim trên desktop. Bấm để gỡ' : 'Ghim lên màn hình desktop';
      pin.addEventListener('click', async () => {
        try {
          await invoke('tasks_set_pinned', { id: t.id, pinned: !t.pinned });
          showStatus('');
        } catch (e) {
          showStatus(String(e));
        }
        render();
      });
      row.append(cb, body, cal, pin, del);
    } else {
      row.append(cb, body, del);
    }
    return row;
  }

  // Link trong nội dung / ghi chú: mở bằng trình duyệt (Rust chỉ nhận http/https).
  async function openLink(url) {
    try {
      await invoke('open_url', { url });
    } catch (e) {
      showStatus(String(e));
    }
  }

  // Ô chọn hạn chót hiện ngay dưới việc đó: Lưu / Bỏ hạn / Esc để huỷ.
  function editDue(task, body) {
    if (body.querySelector('.tk-due-edit')) return;
    const box = document.createElement('div');
    box.className = 'tk-due-edit';
    const input = document.createElement('input');
    input.type = 'datetime-local';
    input.value = window.DueFmt.toLocalInput(task.due_at);
    input.setAttribute('aria-label', 'Hạn chót');
    const save = document.createElement('button');
    save.type = 'button';
    save.className = 'action';
    save.textContent = 'Lưu';
    const clear = document.createElement('button');
    clear.type = 'button';
    clear.className = 'ghost';
    clear.textContent = 'Bỏ hạn';

    const apply = async (dueAt) => {
      try {
        await invoke('tasks_set_due', { id: task.id, dueAt });
        showStatus('');
      } catch (e) {
        showStatus(String(e));
      }
      render();
    };
    save.addEventListener('click', () => {
      const ms = window.DueFmt.fromLocalInput(input.value);
      if (ms == null) {
        showStatus('Chọn ngày giờ trước đã nha.');
        input.focus();
        return;
      }
      apply(String(ms));
    });
    clear.addEventListener('click', () => apply(''));
    input.addEventListener('keydown', (e) => {
      if (e.key === 'Enter') save.click();
      else if (e.key === 'Escape') render();
    });
    box.append(input, save);
    if (task.due_at) box.append(clear);
    body.appendChild(box);
    input.focus();
  }

  // Dòng báo lỗi dưới ô thêm việc (vd: đã ghim đủ số tờ). Tự ẩn sau vài giây.
  let statusTimer = 0;
  function showStatus(msg) {
    const el = $('tkStatus');
    el.textContent = msg;
    clearTimeout(statusTimer);
    if (msg) statusTimer = setTimeout(() => { el.textContent = ''; }, 5000);
  }

  // Đổi dòng ghi chú thành ô nhập, lưu khi Enter hoặc rời ô. Esc thì huỷ.
  function editNote(task, noteEl) {
    if (noteEl.dataset.editing === '1') return;
    noteEl.dataset.editing = '1';

    const input = document.createElement('textarea');
    input.className = 'tk-note-input';
    input.value = task.note || '';
    input.placeholder = 'Ghi chú cho công việc...';
    noteEl.replaceWith(input);
    input.focus();
    input.setSelectionRange(input.value.length, input.value.length);

    let settled = false;
    const finish = async (save) => {
      if (settled) return;
      settled = true;
      const value = input.value.trim();
      if (save && value !== (task.note || '')) {
        await invoke('tasks_set_note', { id: task.id, note: value });
      }
      render();
    };

    input.addEventListener('keydown', (e) => {
      // Enter lưu, Shift+Enter xuống dòng, Esc huỷ.
      if (e.key === 'Enter' && !e.shiftKey) {
        e.preventDefault();
        finish(true);
      } else if (e.key === 'Escape') {
        e.preventDefault();
        finish(false);
      }
    });
    input.addEventListener('blur', () => finish(true));
  }

  function fillList(el, tasks, emptyHtml) {
    el.innerHTML = '';
    if (!tasks.length) {
      el.innerHTML = `<div class="tk-empty">${emptyHtml}</div>`;
      return;
    }
    for (const t of tasks) el.appendChild(makeItem(t));
  }

  async function render() {
    let tasks = [];
    try {
      tasks = await invoke('tasks_list');
    } catch (e) {
      console.error(e);
    }

    // Việc có hạn gần nhất lên đầu; việc không hạn giữ thứ tự cũ ở dưới.
    const todo = tasks.filter((t) => !t.done).sort(window.DueFmt.byDue);
    // Việc xong gần nhất lên đầu; task cũ chưa có done_at thì xuống cuối.
    const done = tasks
      .filter((t) => t.done)
      .sort((a, b) => (Number(b.done_at) || 0) - (Number(a.done_at) || 0));

    $('todoCount').textContent = todo.length;
    $('doneCount').textContent = done.length;

    fillList(
      $('todoList'),
      todo,
      'Chưa có việc nào.<br/>Bôi đen văn bản → bông hoa → Lưu task,<br/>hoặc thêm ở ô trên.'
    );
    fillList($('doneList'), done, 'Chưa hoàn thành việc nào.');
  }

  async function add() {
    const inp = $('newTask');
    const v = inp.value.trim();
    if (!v) return;
    await invoke('tasks_add', { text: v, note: '', source: 'manual' });
    inp.value = '';
    render();
  }

  $('addBtn').addEventListener('click', add);
  $('newTask').addEventListener('keydown', (e) => {
    if (e.key === 'Enter') add();
  });

  // Reload the list each time the window is shown (it's pre-created once).
  listen('tasks-refresh', render);
  // "Còn 20 phút" phải tự đếm xuống khi bảng đang mở. Bỏ qua lúc đang sửa
  // hạn / ghi chú, để không xoá mất ô người dùng đang nhập.
  setInterval(() => {
    if (!document.hidden && !document.querySelector('.tk-due-edit, .tk-note-input')) render();
  }, 60_000);

  render();
})();
