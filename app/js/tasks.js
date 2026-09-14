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
      await invoke('tasks_set_done', { id: t.id, done: cb.checked });
      render();
    });

    const body = document.createElement('div');
    body.className = 'tk-body';

    const txt = document.createElement('div');
    txt.className = 'tk-text';
    txt.textContent = t.text;

    // Ghi chú: bấm vào biến thành textarea, Enter hoặc rời chuột thì lưu.
    const note = document.createElement('div');
    note.className = 'tk-note' + (t.note ? '' : ' empty');
    note.textContent = t.note || '+ Thêm ghi chú';
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

    row.append(cb, body, del);
    return row;
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

    const todo = tasks.filter((t) => !t.done);
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

  render();
})();
