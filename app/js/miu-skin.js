// Bộ mặt Miu chọn trong 🍬 Bộ sưu tập, áp cho các cửa sổ game + trang Mini Games.
// Các trang đó gắn ảnh cứng "media/game/rps-<face>.png" (HTML và JS); script này
// đổi mọi <img> như vậy sang bộ đang chọn (game_info.miuDir / miuExt), kể cả
// ảnh game đổi src về sau (MutationObserver). Nạp SAU tauri-api.js.
(function () {
  const CLASSIC = /(?:^|\/)media\/game\/rps-([a-z]+)\.png$/;
  let dir = 'media/game';
  let ext = 'png';

  const want = (face) => `${dir}/rps-${face}.${ext}`;

  function fix(img) {
    // Ảnh cố ý của một bộ cụ thể (vd. ảnh đại diện từng bộ trong Bộ sưu tập): giữ nguyên.
    if (img.hasAttribute('data-miu-fixed')) return;
    const src = img.getAttribute('src') || '';
    const m = CLASSIC.exec(src);
    if (m) img.dataset.miuFace = m[1];
    else if (img.dataset.miuFace && src !== want(img.dataset.miuFace)) {
      // Game đổi sang ảnh khác hẳn (vd. chọn nhân vật khác làm nền): thôi theo dõi.
      delete img.dataset.miuFace;
    }
    const face = img.dataset.miuFace;
    if (face && src !== want(face)) img.setAttribute('src', want(face));
  }

  function fixAll() {
    document.querySelectorAll('img').forEach(fix);
  }

  new MutationObserver((list) => {
    for (const r of list) {
      if (r.type === 'attributes' && r.target.tagName === 'IMG') fix(r.target);
      else r.addedNodes.forEach((n) => {
        if (n.tagName === 'IMG') fix(n);
        else if (n.querySelectorAll) n.querySelectorAll('img').forEach(fix);
      });
    }
  }).observe(document.documentElement, { subtree: true, childList: true, attributes: true, attributeFilter: ['src'] });

  function apply(info) {
    if (!info) return;
    dir = info.miuDir || 'media/game';
    ext = info.miuExt || 'png';
    fixAll();
  }

  const DC = window.DC;
  if (!DC) return;
  DC.invoke('game_info').then(apply).catch((e) => DC.log('[miu-skin] game_info: ' + e));
  DC.listen('collection-changed', (e) => apply(e && e.payload));
  fixAll();
})();
