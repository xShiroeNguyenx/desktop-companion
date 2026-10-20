// Đồ trang trí chọn trong 🍬 Bộ sưu tập cho cửa sổ pet và bông hoa khi bôi chữ:
//   - pet.html: body[data-bubble] → kiểu bong bóng thoại (media/companion.css)
//   - flower.html: biểu tượng của bông hoa (🌸 ⭐ 🍀 🐾 ❄️) + màu nền
// Không phụ thuộc tauri-api.js (pet.html không nạp nó): gọi thẳng window.__TAURI__.
// Đổi món → sự kiện collection-changed, áp dụng ngay.
(function () {
  const FLOWERS = { sakura: '🌸', star: '⭐', clover: '🍀', paw: '🐾', snow: '❄️' };
  const T = () => window.__TAURI__ || {};
  const invoke = (cmd) => {
    const fn = T().core && T().core.invoke;
    return fn ? fn(cmd) : Promise.reject(new Error('Tauri chưa sẵn sàng'));
  };

  /** Áp biểu tượng + màu nền bông hoa. flower.js gọi lại mỗi lần bông hoa hiện ra. */
  function applyFlower(id) {
    if (!document.body) return;
    const key = FLOWERS[id] ? id : 'sakura';
    document.body.dataset.flower = key;
    window.__FLOWER_GLYPH__ = FLOWERS[key];
    const btn = document.getElementById('flower');
    if (btn) btn.textContent = window.__FLOWER_GLYPH__;
  }
  window.__applyFlower__ = applyFlower;

  function apply(v) {
    if (!v || !document.body) return;
    document.body.dataset.bubble = v.bubble || 'classic';
    applyFlower(v.flower);
  }

  function refresh() {
    invoke('collection_get').then(apply).catch(() => {});
  }

  window.__FLOWER_GLYPH__ = FLOWERS.sakura;
  if (document.body) refresh();
  else document.addEventListener('DOMContentLoaded', refresh);
  const listen = T().event && T().event.listen;
  if (listen) listen('collection-changed', (e) => apply(e && e.payload));
})();
