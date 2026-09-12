// The selection "flower" overlay — một nút duy nhất.
//   - Click 🌸    → dịch nhanh (cửa sổ dịch đơn giản).
//   - Nhấn giữ 🌸 → mở menu xoay tròn (dịch / copy / trả lời / lưu task / mở đầy đủ).
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

  // Nháy một ký tự lên bông hoa để báo kết quả, rồi trả lại 🌸.
  function flash(glyph) {
    btn.textContent = glyph;
    setTimeout(() => { btn.textContent = '🌸'; }, 600);
  }

  btn.addEventListener('mousedown', () => {
    clearTimeout(hideTimer);
    longPressed = false;
    pressTimer = setTimeout(() => {
      longPressed = true;
      log('flower long-press → radial menu');
      invoke('flower_action', { mode: 'radial' }).catch((e) => log('flower_action radial: ' + e));
    }, HOLD_MS);
  });

  btn.addEventListener('mouseup', () => {
    clearTimeout(pressTimer);
    if (longPressed) return; // giữ lâu → đã mở ring
    log('flower click → translate');
    invoke('flower_action', { mode: 'translate' }).catch((e) => log('flower_action translate: ' + e));
  });

  // Keep alive while the cursor is anywhere over the flower window.
  document.body.addEventListener('mouseenter', () => clearTimeout(hideTimer));
  document.body.addEventListener('mouseleave', () => scheduleHide(2500));

  // Ctrl+C (handled in Rust while the flower is shown) copied the selection —
  // flash the result on the flower itself, then close.
  listen('flower-copied', (event) => {
    clearTimeout(hideTimer);
    flash((event && event.payload) ? '✓' : '✕');
    setTimeout(() => invoke('close_flower').catch((e) => log('close_flower: ' + e)), 650);
  });

  // Reset the glyph on each fresh show.
  listen('flower-shown', () => { btn.textContent = '🌸'; scheduleHide(5000); });
  scheduleHide(5000);
  log('flower.js ready');
})();
