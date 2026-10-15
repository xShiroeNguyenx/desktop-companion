// Biến link http(s) trong nội dung / ghi chú task thành chỗ bấm được (tờ note
// ghim, bảng Công việc). Dựng bằng DOM, không innerHTML, nên nội dung task
// không chèn được HTML. Mở link qua Rust (commands::open_url), Rust chỉ nhận
// http/https.
(function (root) {
  const URL_RE = /https?:\/\/[^\s<>"'`]+/gi;
  // Dấu câu dính ở cuối link khi viết trong câu: "xem https://example.com." → bỏ ".".
  const TRAILING = /[.,;:!?…'"\]}>]+$/;

  // Ngoặc đóng ở cuối chỉ thuộc link khi trong link có ngoặc mở tương ứng
  // (vd link Wikipedia "..._(film)").
  function trimLink(url) {
    let u = url.replace(TRAILING, '');
    while (u.endsWith(')') && (u.match(/\(/g) || []).length < (u.match(/\)/g) || []).length) {
      u = u.slice(0, -1).replace(TRAILING, '');
    }
    return u;
  }

  /**
   * Tách `text` thành các đoạn chữ thường và link.
   * @returns {Array<{type: 'text'|'link', value: string}>}
   */
  function splitLinks(text) {
    const s = String(text || '');
    const out = [];
    let last = 0;
    for (const m of s.matchAll(URL_RE)) {
      const url = trimLink(m[0]);
      if (!/^https?:\/\/[^/?#]+/i.test(url)) continue; // "https://" trơn
      if (m.index > last) out.push({ type: 'text', value: s.slice(last, m.index) });
      out.push({ type: 'link', value: url });
      last = m.index + url.length;
    }
    if (last < s.length) out.push({ type: 'text', value: s.slice(last) });
    return out;
  }

  /** Ghi `text` vào `el`, link thành <a>; bấm link gọi onOpen(url). */
  function renderLinks(el, text, onOpen) {
    el.textContent = '';
    for (const part of splitLinks(text)) {
      if (part.type === 'text') {
        el.appendChild(document.createTextNode(part.value));
        continue;
      }
      const a = document.createElement('a');
      a.className = 'lnk';
      a.href = part.value;
      a.textContent = part.value;
      a.title = 'Mở trong trình duyệt: ' + part.value;
      a.addEventListener('click', (e) => {
        e.preventDefault();
        e.stopPropagation();
        onOpen(part.value);
      });
      el.appendChild(a);
    }
  }

  const api = { splitLinks, renderLinks };
  if (typeof module !== 'undefined' && module.exports) module.exports = api;
  else root.Linkify = api;
})(typeof window !== 'undefined' ? window : globalThis);
