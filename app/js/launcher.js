// 🔎 Ô mở nhanh (launcher.html). Rust hiện cửa sổ khi bấm phím tắt và gửi
// "launcher-show"; trang nạp bookmark / bang (launcher_data), người dùng gõ,
// ↑↓ chọn, Tab điền tên bang, Enter mở, Esc / bấm ra ngoài là ẩn.
// Không lưu lịch sử gõ; chỉ nhớ câu cuối trong bộ nhớ cho phím ↑.
(function () {
  const { invoke, listen, log } = window.DC;
  const L = window.LauncherCore;
  const $ = (id) => document.getElementById(id);
  const ICON = { bang: '❗', hint: '❗', url: '🌐', bookmark: '🔖', search: '🔎' };

  let data = { bookmarks: [], engine: 'google', bangs: [] };
  let rows = [];
  let sel = 0;
  let lastQuery = '';

  function fit() {
    requestAnimationFrame(() => {
      const h = $('wrap').offsetHeight;
      invoke('launcher_resize', { h }).catch(() => {});
    });
  }

  function render() {
    rows = L.results($('q').value, data);
    sel = Math.min(sel, Math.max(rows.length - 1, 0));
    const list = $('list');
    list.textContent = '';
    rows.forEach((r, i) => {
      const row = document.createElement('div');
      row.className = 'row' + (i === sel ? ' sel' : '');
      row.setAttribute('role', 'option');
      row.dataset.i = String(i);
      const ic = document.createElement('span');
      ic.className = 'i';
      ic.textContent = ICON[r.type] || '•';
      const t = document.createElement('span');
      t.className = 't';
      const b = document.createElement('b');
      b.textContent = r.type === 'search' ? `Tìm “${r.title}”` : r.title;
      const small = document.createElement('small');
      small.textContent = r.type === 'search' ? `trên ${r.sub}` : r.sub;
      t.append(b, small);
      row.append(ic, t);
      list.append(row);
    });
    fit();
  }

  function msg(text) {
    $('msg').textContent = text || '';
    fit();
  }

  async function choose(i) {
    const r = rows[i];
    if (!r) return;
    if (r.type === 'hint') {
      $('q').value = r.insert;
      sel = 0;
      render();
      return;
    }
    try {
      await invoke('open_url', { url: r.url });
      lastQuery = $('q').value;
      invoke('launcher_hide').catch(() => {});
    } catch (e) {
      // Rust từ chối link (không phải http / https…): báo rõ, giữ ô mở.
      msg(String(e));
    }
  }

  $('q').addEventListener('input', () => {
    sel = 0;
    msg('');
    render();
  });
  $('q').addEventListener('keydown', (e) => {
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      if (rows.length) sel = (sel + 1) % rows.length;
      render();
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      if (!$('q').value && lastQuery) {
        $('q').value = lastQuery;
        sel = 0;
      } else if (rows.length) {
        sel = (sel - 1 + rows.length) % rows.length;
      }
      render();
    } else if (e.key === 'Tab') {
      e.preventDefault();
      const hint = rows[sel] && rows[sel].type === 'hint' ? rows[sel] : rows.find((r) => r.type === 'hint');
      if (hint) choose(rows.indexOf(hint));
    } else if (e.key === 'Enter') {
      e.preventDefault();
      choose(sel);
    } else if (e.key === 'Escape') {
      e.preventDefault();
      invoke('launcher_hide').catch(() => {});
    }
  });
  $('list').addEventListener('mousedown', (e) => {
    const row = e.target.closest('.row');
    if (!row) return;
    // mousedown để chọn trước khi ô gõ mất focus (blur sẽ ẩn cửa sổ).
    e.preventDefault();
    choose(Number(row.dataset.i));
  });
  // Bấm ra ngoài (cửa sổ mất focus) là ẩn, như Spotlight.
  window.addEventListener('blur', () => invoke('launcher_hide').catch(() => {}));

  async function onShow() {
    $('q').value = '';
    sel = 0;
    msg('');
    try {
      data = await invoke('launcher_data');
      if (data.error) msg('Không đọc được bookmark: ' + data.error);
    } catch (e) {
      log('[launcher] launcher_data: ' + e);
    }
    $('q').placeholder = 'Gõ để tìm bookmark, !yt …, !gh …, hoặc địa chỉ web';
    render();
    $('q').focus();
  }

  listen('launcher-show', onShow);
  onShow();
})();
