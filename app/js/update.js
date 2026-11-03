// Hộp thoại "có bản mới": hiện tự động khi app khởi động và tìm thấy bản mới
// (Rust gọi windows::show_update), hoặc khi người dùng tự kiểm tra trong Cài đặt.
// Cửa sổ được precreate nên script này chạy một lần lúc khởi động — mọi lần
// hiện sau đó đến qua event "update-refresh".
(function () {
  const { invoke, listen, log } = window.DC;
  const $ = (id) => document.getElementById(id);
  const T = (key, vars) => window.I18n.t(key, vars);

  let version = '';
  let installing = false;
  // Trạng thái đang hiện, để vẽ lại chữ khi đổi ngôn ngữ:
  // 'checking' | 'latest' | 'available' | 'failed' (+ info), và % tải (null = chưa tải).
  let view = { kind: 'checking', info: null };
  let percent = null;

  function paint() {
    const info = view.info || {};
    $('title').textContent = view.kind === 'available' ? T('update.titleV', { v: info.version }) : T('update.title');
    $('versions').textContent =
      view.kind === 'latest' ? T('update.latest', { v: info.current })
      : view.kind === 'available' ? T('update.current', { v: info.current })
      : view.kind === 'failed' ? T('update.checkFail')
      : T('update.checking');
    if (view.kind === 'available') {
      const notes = (info.notes || '').trim();
      $('notes').textContent = notes || T('update.noNotes');
      $('notes').classList.toggle('empty', !notes);
    } else if ($('notes').classList.contains('empty')) {
      $('notes').textContent = T('update.noNotes');
    }
    if (percent != null) {
      $('status').textContent = percent < 0 ? T('update.downloading') : T('update.downloadingPct', { p: percent });
    }
  }

  function setErr(msg) {
    const el = $('errStatus');
    el.className = 'status' + (msg ? ' err' : '');
    el.textContent = msg || '';
  }

  function setProgress(p) {
    percent = p;
    $('progressWrap').classList.add('show');
    const bar = $('bar');
    if (p < 0) {
      // Server không trả Content-Length → chạy sọc vô định.
      bar.classList.add('indeterminate');
    } else {
      bar.classList.remove('indeterminate');
      $('barFill').style.width = p + '%';
    }
    paint();
  }

  // Nạp thông tin bản mới mỗi lần hộp thoại được hiện. `known` là kết quả
  // check_update mà Rust vừa có sẵn (luồng kiểm tra lúc khởi động) — có thì
  // dùng luôn, khỏi gọi lại qua mạng.
  async function load(known) {
    if (installing) return; // đang tải dở thì đừng reset UI
    setErr('');
    $('progressWrap').classList.remove('show');
    $('installBtn').disabled = false;
    $('skipBtn').disabled = false;
    percent = null;
    view = { kind: 'checking', info: null };
    paint();
    try {
      const info = known || (await invoke('check_update'));
      if (!info.available) {
        // Bản mới vừa biến mất (hiếm) → không giữ hộp thoại vô nghĩa.
        view = { kind: 'latest', info };
        paint();
        $('installBtn').disabled = true;
        return;
      }
      version = info.version;
      view = { kind: 'available', info };
      paint();
    } catch (e) {
      view = { kind: 'failed', info: null };
      paint();
      setErr(String(e));
      $('installBtn').disabled = true;
    }
  }

  $('installBtn').addEventListener('click', async () => {
    if (installing) return;
    installing = true;
    setErr('');
    $('installBtn').disabled = true;
    $('skipBtn').disabled = true;
    setProgress(-1);
    try {
      // Thành công thì app tự khởi động lại — promise này không trả về.
      await invoke('install_update');
    } catch (e) {
      installing = false;
      percent = null;
      setErr(String(e));
      $('progressWrap').classList.remove('show');
      $('installBtn').disabled = false;
      $('skipBtn').disabled = false;
    }
  });

  $('laterBtn').addEventListener('click', () => {
    invoke('close_update_dialog').catch((e) => log('[update] close: ' + e));
  });

  $('skipBtn').addEventListener('click', async () => {
    try {
      if (version) await invoke('skip_update_version', { version });
    } catch (e) {
      log('[update] skip: ' + e);
    }
    invoke('close_update_dialog').catch(() => {});
  });

  window.I18n.onChange(paint);

  listen('update-progress', (event) => {
    if (installing) setProgress(Number(event.payload));
  });
  // Cửa sổ được precreate lúc khởi động (ẩn) nên KHÔNG tự load ở đây — gọi
  // check_update lúc đó là một lượt mạng thừa. Rust phát "update-refresh"
  // mỗi lần thật sự hiện hộp thoại, và phát lại một nhịp sau đề phòng
  // listener chưa kịp sẵn sàng — lần lặp đó bỏ qua.
  let lastLoadAt = 0;
  listen('update-refresh', (event) => {
    // Bỏ qua lần phát lại (cách lần nạp trước dưới 1.5s) để UI khỏi nháy.
    const now = Date.now();
    if (lastLoadAt && now - lastLoadAt < 1500) return;
    lastLoadAt = now;
    load(event && event.payload);
  });
  log('update.js ready');
})();
