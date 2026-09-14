// Quick-translate window: fetch the selection, translate with the default
// context, show the result + a Copy button. Nothing else.
(function () {
  const { invoke, listen, log } = window.DC;
  const $ = (id) => document.getElementById(id);
  let lastResult = '';
  let lastSource = '';

  // --- Cửa sổ tự co/giãn chiều cao theo nội dung -------------------------
  const TAURI = window.__TAURI__ || {};
  const getWin =
    (TAURI.window && TAURI.window.getCurrentWindow) ||
    (TAURI.webviewWindow && TAURI.webviewWindow.getCurrentWebviewWindow);
  const MIN_H = 90;   // logical px — đủ cho 1 dòng + icon
  const MAX_H = 620;

  let fitting = false;
  let refit = false; // có yêu cầu fit đến trong lúc đang fit → chạy bù 1 lần
  async function fitHeight() {
    if (!getWin) return;
    if (fitting) { refit = true; return; }
    fitting = true;
    try {
      const win = getWin();
      const LogicalSize =
        (TAURI.window && TAURI.window.LogicalSize) || (TAURI.dpi && TAURI.dpi.LogicalSize);
      // Chiều cao nội dung = #wrap + padding trên/dưới của body.
      const wrap = $('wrap');
      const pad = parseFloat(getComputedStyle(document.body).paddingTop) || 0;
      const want = Math.round(
        Math.max(MIN_H, Math.min(MAX_H, wrap.getBoundingClientRect().height + pad * 2))
      );
      // Bề rộng giữ nguyên (đổi theo tỉ lệ DPI về logical px).
      const dpr = window.devicePixelRatio || 1;
      const cur = await win.innerSize();
      const curH = Math.round(cur.height / dpr);
      if (Math.abs(curH - want) <= 2) return; // đã vừa, khỏi nhấp nháy
      const w = Math.round(cur.width / dpr);
      if (LogicalSize) await win.setSize(new LogicalSize(w, want));
      else await win.setSize({ type: 'Logical', width: w, height: want });
    } catch (e) {
      log('[quick] fitHeight: ' + e);
    } finally {
      fitting = false;
      if (refit) { refit = false; requestAnimationFrame(fitHeight); }
    }
  }
  // Bám mọi thay đổi chiều cao nội dung (dịch xong, bật/tắt nội dung gốc,
  // hiện status, người dùng kéo rộng cửa sổ làm chữ xuống dòng lại).
  if (window.ResizeObserver) {
    new ResizeObserver(() => fitHeight()).observe($('wrap'));
  }

  function setStatus(msg, kind) {
    const el = $('status');
    el.className = 'status' + (kind ? ' ' + kind : '');
    el.textContent = msg || '';
  }

  async function run() {
    lastResult = '';
    lastSource = '';
    $('source').textContent = '...';
    $('result').textContent = 'Đang dịch...';
    $('result').classList.add('empty');
    setSourceVisible(false); // mỗi lần dịch mới lại thu gọn về chỉ bản dịch
    setStatus('');
    let sel;
    try {
      sel = await invoke('get_selection');
    } catch (e) {
      setStatus(String(e), 'err');
      return;
    }
    if (!sel || !sel.text) {
      $('source').textContent = '(chưa có văn bản)';
      $('result').textContent = '(chưa có văn bản được chọn)';
      return;
    }
    $('source').textContent = sel.text;
    lastSource = sel.text;
    try {
      // Empty context → Rust falls back to the default context from Settings.
      const out = await invoke('llm_translate', { text: sel.text, context: '', target: '' });
      lastResult = out;
      $('result').textContent = out;
      $('result').classList.remove('empty');
    } catch (e) {
      $('result').textContent = '';
      setStatus(String(e), 'err');
    }
  }

  // Copy + nháy dấu ✓ ngay trên icon (gọn hơn dòng status).
  async function copyWithFeedback(btn, text) {
    if (!text) return;
    try {
      await navigator.clipboard.writeText(text);
      btn.textContent = '✓';
      setTimeout(() => { btn.textContent = '📋'; }, 1000);
    } catch (e) {
      setStatus('Không sao chép được.', 'err');
    }
  }

  $('copyBtn').addEventListener('click', (e) => copyWithFeedback(e.currentTarget, lastResult));
  $('copySrcBtn').addEventListener('click', (e) => copyWithFeedback(e.currentTarget, lastSource));

  // Nội dung gốc ẩn mặc định; nút 👁 bật/tắt.
  function setSourceVisible(on) {
    $('srcBox').classList.toggle('show', on);
    // .dual bật nhãn "Nội dung gốc" / "Bản dịch" để phân biệt 2 khung.
    document.body.classList.toggle('dual', on);
    const btn = $('toggleSrcBtn');
    btn.classList.toggle('on', on);
    btn.title = on ? 'Ẩn nội dung gốc' : 'Hiện nội dung gốc';
  }
  $('toggleSrcBtn').addEventListener('click', () => {
    setSourceVisible(!$('srcBox').classList.contains('show'));
  });

  listen('quick-refresh', run);
  run();
  log('quick.js ready');
})();
