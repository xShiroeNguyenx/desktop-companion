// Trang trí widget (Bộ sưu tập → nhóm "Trang trí"). Gắn [data-deco] lên phần tử
// chủ và một lớp .deco gồm 8 ô <i class="d1…d8">; hình vẽ nằm hết trong
// widget-deco.css. Dùng ở 3 widget ghim (chủ = #wrap) và thẻ xem trước.
(function () {
  const SLOTS = 8;

  /** Áp trang trí `asset` lên `host` ("none" hoặc rỗng = không trang trí). */
  function mount(host, asset) {
    if (!host) return null;
    host.dataset.deco = asset || 'none';
    let layer = host.querySelector(':scope > .deco');
    if (!layer) {
      layer = document.createElement('div');
      layer.className = 'deco';
      layer.setAttribute('aria-hidden', 'true');
      for (let i = 1; i <= SLOTS; i++) {
        const slot = document.createElement('i');
        slot.className = 'd' + i;
        layer.append(slot);
      }
      host.append(layer);
    }
    return layer;
  }

  window.WidgetDeco = { mount };
})();
