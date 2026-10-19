// Giao diện theo bộ nhân vật đang dùng (🍬 Bộ sưu tập), dùng chung cho Cài đặt,
// Công việc và Trợ lý văn bản: Xứ Tuyết → theme-snow.css, Neko Lounge →
// theme-gothic.css. Chỉ khi Bunny Lounge đang bật (tắt thì không thấy nhân vật
// nào, giữ giao diện mặc định). Đổi bộ / bật tắt → sự kiện collection-changed.
// Nạp SAU tauri-api.js. Thêm giao diện mới: thêm một dòng vào THEMES + file CSS.
(function () {
  const THEMES = { snow: 'snow', neko: 'gothic' };
  const DC = window.DC;
  if (!DC) return;

  function apply(v) {
    if (!v || !document.body) return;
    document.body.dataset.theme = (v.sideGirls && THEMES[v.sideSet]) || '';
  }

  function refresh() {
    DC.invoke('collection_get').then(apply).catch((e) => DC.log('[theme] collection_get: ' + e));
  }

  window.ThemeApply = { refresh };
  DC.listen('collection-changed', (e) => apply(e && e.payload));
  if (document.body) refresh();
  else document.addEventListener('DOMContentLoaded', refresh);
})();
