// The selection "flower" overlay.
//   - Short click  → dịch ngay (mở popup ở tab Dịch và tự chạy).
//   - Nhấn giữ     → mở popup đầy đủ 3 tab để chọn (Dịch / Trả lời / Lưu task).
// Auto-hides after a short idle so a stale flower doesn't linger.
(function () {
  const { invoke, listen, log } = window.DC;
  const btn = document.getElementById('flower');
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
    if (longPressed) return; // đã xử lý ở long-press
    log('flower click → translate');
    invoke('flower_action', { mode: 'translate' }).catch((e) => log('flower_action translate: ' + e));
  });

  btn.addEventListener('mouseenter', () => clearTimeout(hideTimer));
  btn.addEventListener('mouseleave', () => scheduleHide(2500));

  listen('flower-shown', () => scheduleHide(5000));
  scheduleHide(5000);
  log('flower.js ready');
})();
