// Nhịp vẽ của pet Live2D (PIXI ticker). Hàm thuần, dùng trong pet.html
// (window.PetFps) và node --test.
//
// Vì sao: PIXI mặc định vẽ theo tần số quét màn hình. Laptop 120 Hz → pet vẽ
// 120 khung/giây (kèm MSAA) cả lúc chỉ đứng thở, ngốn ~35% engine 3D của GPU
// tích hợp (đo 2026-11-05). Đứng yên 30 khung/giây là đủ mượt; người dùng đang
// rê / kéo / xoay pet thì lên 60 vài giây cho tay cảm thấy nhạy.
(function (root) {
  const IDLE_FPS = 30;
  const ACTIVE_FPS = 60;
  /** Sau lần chạm chuột cuối cùng bao lâu thì về nhịp nghỉ (ms). */
  const ACTIVE_HOLD_MS = 3000;

  /** Số khung/giây tối đa lúc `now`, biết lần tương tác cuối ở `lastActiveAt` (ms). */
  function fpsAt(now, lastActiveAt) {
    if (!Number.isFinite(lastActiveAt)) return IDLE_FPS;
    const since = now - lastActiveAt;
    return since >= 0 && since < ACTIVE_HOLD_MS ? ACTIVE_FPS : IDLE_FPS;
  }

  /**
   * Hệ số trộn cho một khung dài `deltaMS`, sao cho kết quả theo THỜI GIAN giống
   * như trộn `perFrame60` mỗi khung ở 60 khung/giây (không nhanh / chậm theo FPS).
   */
  function blendAlpha(perFrame60, deltaMS) {
    const p = Math.min(1, Math.max(0, Number(perFrame60) || 0));
    const frames = Math.min(30, Math.max(0, Number(deltaMS) || 0) / (1000 / 60));
    return 1 - Math.pow(1 - p, frames);
  }

  const api = { IDLE_FPS, ACTIVE_FPS, ACTIVE_HOLD_MS, fpsAt, blendAlpha };
  if (typeof module !== 'undefined' && module.exports) module.exports = api;
  else root.PetFps = api;
})(typeof window !== 'undefined' ? window : globalThis);
