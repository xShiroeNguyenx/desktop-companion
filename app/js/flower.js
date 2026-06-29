// The selection "flower" overlay. The copy icon (📋) shows above the flower as
// soon as text is captured.
//   - 📋          → sao chép nội dung gốc (làm ở Rust, khỏi vướng focus webview).
//   - Click 🌸    → dịch nhanh (cửa sổ dịch đơn giản).
//   - Nhấn giữ 🌸 → mở popup đầy đủ 3 tab (Dịch / Trả lời / Lưu task).
// Auto-hides after a short idle so a stale flower doesn't linger.
(function () {
  const { invoke, listen, log } = window.DC;
  const btn = document.getElementById('flower');
  const qaCopy = document.getElementById('qaCopy');
  let hideTimer = null;
  let pressTimer = null;
  let longPressed = false;

  const HOLD_MS = 450; // giữ lâu hơn mức này = "nhấn giữ"

  function scheduleHide(ms) {
    clearTimeout(hideTimer);
    hideTimer = setTimeout(() => invoke('close_flower').catch((e) => log('close_flower: ' + e)), ms);
  }

  btn.addEventListener('mousedown', () => {
    clearTimeout(hideTimer);
    longPressed = false;
    pressTimer = setTimeout(() => {
      longPressed = true;
      log('flower long-press → full popup');
      invoke('flower_action', { mode: 'menu' }).catch((e) => log('flower_action menu: ' + e));
    }, HOLD_MS);
  });

  btn.addEventListener('mouseup', () => {
    clearTimeout(pressTimer);
    if (longPressed) return; // giữ lâu → đã mở popup
    log('flower click → translate');
    invoke('flower_action', { mode: 'translate' }).catch((e) => log('flower_action translate: ' + e));
  });

  // 📋 Sao chép nội dung gốc.
  qaCopy.addEventListener('click', async () => {
    clearTimeout(hideTimer);
    try {
      await invoke('copy_selection');
      qaCopy.textContent = '✓';
      setTimeout(() => invoke('close_flower').catch((e) => log('close_flower: ' + e)), 650);
    } catch (e) {
      log('copy_selection: ' + e);
      qaCopy.textContent = '✕';
      setTimeout(() => invoke('close_flower').catch(() => {}), 900);
    }
  });

  // Keep alive while the cursor is anywhere over the flower window.
  document.body.addEventListener('mouseenter', () => clearTimeout(hideTimer));
  document.body.addEventListener('mouseleave', () => scheduleHide(2500));

  // Ctrl+C (handled in Rust while the flower is shown) copied the selection —
  // flash the result on the copy icon, then close.
  listen('flower-copied', (event) => {
    clearTimeout(hideTimer);
    qaCopy.textContent = (event && event.payload) ? '✓' : '✕';
    setTimeout(() => invoke('close_flower').catch((e) => log('close_flower: ' + e)), 650);
  });

  // Reset the copy icon glyph on each fresh show.
  listen('flower-shown', () => { qaCopy.textContent = '📋'; scheduleHide(5000); });
  scheduleHide(5000);
  log('flower.js ready');
})();
