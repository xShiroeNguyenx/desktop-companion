// 📌 Ảnh chụp màn hình ghim lên desktop (shot-pin.html): lấy PNG của chính cửa sổ này từ
// Rust (shot.rs giữ trong RAM), hiện lên, cho kéo đi / chép lại / gỡ.
(function () {
  const { invoke, log } = window.DC;
  const $ = (id) => document.getElementById(id);
  const TAURI = window.__TAURI__ || {};
  const currentWindow = () => {
    const get = (TAURI.window && TAURI.window.getCurrentWindow) ||
      (TAURI.webviewWindow && TAURI.webviewWindow.getCurrentWebviewWindow);
    return get ? get() : null;
  };

  const close = () => invoke('shotpin_close');

  // Kéo ở bất kỳ đâu trên ảnh để dời (trừ các nút).
  document.addEventListener('mousedown', (e) => {
    if (e.button !== 0 || e.target.closest('button')) return;
    const w = currentWindow();
    if (w && w.startDragging) w.startDragging().catch(() => {});
  });
  document.addEventListener('dblclick', (e) => { if (!e.target.closest('button')) close(); });
  document.addEventListener('contextmenu', (e) => e.preventDefault());
  window.addEventListener('keydown', (e) => { if (e.key === 'Escape') close(); });
  $('closeBtn').addEventListener('click', close);

  $('copyBtn').addEventListener('click', async () => {
    const btn = $('copyBtn');
    try {
      await invoke('shotpin_copy');
      btn.textContent = '✓';
    } catch (e) {
      log('[shotpin] copy: ' + e);
      btn.textContent = '⚠';
    }
    setTimeout(() => { btn.textContent = '📋'; }, 1200);
  });

  (async function boot() {
    try {
      const buf = await invoke('shotpin_image');
      const img = $('pic');
      img.onload = () => setTimeout(() => invoke('shotpin_ready'), 30);
      img.onerror = () => { log('[shotpin] ảnh hỏng'); close(); };
      img.src = URL.createObjectURL(new Blob([buf], { type: 'image/png' }));
    } catch (e) {
      log('[shotpin] không lấy được ảnh: ' + e);
      close();
    }
  })();
})();
