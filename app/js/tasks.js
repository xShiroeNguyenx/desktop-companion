// Tasks (TODO) window: list / add / toggle done / delete via the Rust store.
(function () {
  const { invoke, listen } = window.DC;
  const list = document.getElementById('list');

  async function render() {
    let tasks = [];
    try { tasks = await invoke('tasks_list'); } catch (e) { console.error(e); }
    list.innerHTML = '';
    if (!tasks.length) {
      list.innerHTML = '<div class="empty-hint">Chưa có công việc nào.<br/>Bôi đen văn bản → bông hoa → Lưu task, hoặc thêm ở trên.</div>';
      return;
    }
    for (const t of tasks) {
      const row = document.createElement('div');
      row.className = 'task' + (t.done ? ' done' : '');

      const cb = document.createElement('input');
      cb.type = 'checkbox';
      cb.checked = t.done;
      cb.style.cssText = 'flex:0 0 auto; margin-top:2px;';
      cb.addEventListener('change', async () => {
        await invoke('tasks_set_done', { id: t.id, done: cb.checked });
        render();
      });

      const txt = document.createElement('div');
      txt.className = 'task-text';
      txt.textContent = t.text + (t.note ? ' — ' + t.note : '');

      const del = document.createElement('button');
      del.className = 'task-del';
      del.textContent = '🗑';
      del.title = 'Xoá';
      del.addEventListener('click', async () => {
        await invoke('tasks_delete', { id: t.id });
        render();
      });

      row.append(cb, txt, del);
      list.appendChild(row);
    }
  }

  async function add() {
    const inp = document.getElementById('newTask');
    const v = inp.value.trim();
    if (!v) return;
    await invoke('tasks_add', { text: v, note: '', source: 'manual' });
    inp.value = '';
    render();
  }

  document.getElementById('addBtn').addEventListener('click', add);
  document.getElementById('newTask').addEventListener('keydown', (e) => {
    if (e.key === 'Enter') add();
  });

  // Reload the list each time the window is shown (it's pre-created once).
  listen('tasks-refresh', render);

  render();
})();
