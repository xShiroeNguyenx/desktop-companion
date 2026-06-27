// Pet resize: Ctrl + mouse wheel scales the window (model auto-fits via the
// runtime's ResizeObserver). Keeps the original aspect ratio, clamps to a
// sensible range, and the new size persists via the Rust Resized handler.
(function () {
  const TAURI = window.__TAURI__ || {};
  const getWin =
    (TAURI.window && TAURI.window.getCurrentWindow) ||
    (TAURI.webviewWindow && TAURI.webviewWindow.getCurrentWebviewWindow);
  if (!getWin) return;

  const ASPECT = 300 / 420; // width / height of the default pet window
  const MIN_H = 160;
  const MAX_H = 900;
  const STEP = 0.1; // 10% per wheel notch

  let busy = false;

  window.addEventListener(
    'wheel',
    async (e) => {
      if (!e.ctrlKey) return; // only Ctrl+wheel resizes; plain wheel is left alone
      e.preventDefault();
      if (busy) return;
      busy = true;
      try {
        const win = getWin();
        const PhysicalSize =
          (TAURI.window && TAURI.window.PhysicalSize) ||
          (TAURI.dpi && TAURI.dpi.PhysicalSize);
        const size = await win.innerSize();
        const factor = e.deltaY < 0 ? 1 + STEP : 1 - STEP; // up = bigger
        let h = Math.round(size.height * factor);
        h = Math.max(MIN_H, Math.min(MAX_H, h));
        const w = Math.round(h * ASPECT);
        if (PhysicalSize) {
          await win.setSize(new PhysicalSize(w, h));
        } else {
          await win.setSize({ type: 'Physical', width: w, height: h });
        }
      } catch (err) {
        console.error('[pet-zoom]', err);
      } finally {
        busy = false;
      }
    },
    { passive: false }
  );
})();
