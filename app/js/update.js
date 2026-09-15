// Hộp thoại "có bản mới": hiện tự động khi app khởi động và tìm thấy bản mới
// (Rust gọi windows::show_update), hoặc khi người dùng tự kiểm tra trong Cài đặt.
// Cửa sổ được precreate nên script này chạy một lần lúc khởi động — mọi lần
// hiện sau đó đến qua event "update-refresh".
(function () {
  const { invoke, listen, log } = window.DC;
  const $ = (id) => document.getElementById(id);

  let version = '';
  let installing = false;

  function setErr(msg) {
    const el = $('errStatus');
    el.className = 'status' + (msg ? ' err' : '');
    el.textContent = msg || '';
  }

  function setProgress(percent) {
    $('progressWrap').classList.add('show');
    const bar = $('bar');
    if (percent < 0) {
      // Server không trả Content-Length → chạy sọc vô định.
      bar.classList.add('indeterminate');
      $('status').textContent = 'Đang tải bản mới...';
    } else {
      bar.classList.remove('indeterminate');
      $('barFill').style.width = percent + '%';
      $('status').textContent = 'Đang tải bản mới... ' + percent + '%';
    }
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
    $('versions').textContent = 'Đang kiểm tra...';
    try {
      const info = known || (await invoke('check_update'));
      if (!info.available) {
        // Bản mới vừa biến mất (hiếm) → không giữ hộp thoại vô nghĩa.
        $('versions').textContent = 'Bạn đang dùng bản mới nhất (v' + info.current + ').';
        $('installBtn').disabled = true;
        return;
      }
      version = info.version;
      $('title').textContent = 'Có bản mới v' + info.version + '!';
      $('versions').textContent = 'Bạn đang dùng v' + info.current + ' — bấm tải về để cập nhật.';
      const notes = (info.notes || '').trim();
      $('notes').textContent = notes || '(không có ghi chú phát hành)';
      $('notes').classList.toggle('empty', !notes);
    } catch (e) {
      $('versions').textContent = 'Không kiểm tra được bản cập nhật.';
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
