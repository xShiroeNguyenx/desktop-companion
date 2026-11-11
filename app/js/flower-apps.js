// 🌸 Ô "Cách chép theo từng app" trong Cài đặt: mỗi dòng "tên-app shift|skip|ctrl"
// (Rust kiểm lại khi lưu: selection::parse_app_rules). Hàm thuần, dùng trong cửa sổ
// (window.FlowerApps) và node --test.
(function (root) {
  const MODES = ['shift', 'skip', 'ctrl'];

  /** Tên so khớp như Rust: viết thường, bỏ ".exe", bỏ khoảng trắng hai đầu. */
  function stem(name) {
    const lower = String(name || '').trim().toLowerCase();
    return (lower.endsWith('.exe') ? lower.slice(0, -4) : lower).trim();
  }

  /** Tên app của một dòng (mọi thứ trước từ cuối); null với dòng trống / ghi chú / thiếu cách chép. */
  function lineApp(line) {
    const s = line.trim();
    if (!s || s.startsWith('#')) return null;
    const cut = s.search(/\s+\S+$/);
    return cut > 0 ? s.slice(0, cut).trim() : null;
  }

  /**
   * Thêm luật cho `app` (bấm nút "+ Ctrl+Shift+C" / "+ Không tự chép"). App đã có dòng
   * thì đổi cách chép ngay trên dòng đó (khỏi tạo dòng trùng, Rust sẽ từ chối), chưa có
   * thì thêm dòng cuối. Giữ nguyên các dòng khác, kể cả ghi chú.
   */
  function addRule(text, app, mode) {
    const name = String(app || '').trim();
    if (!name || !MODES.includes(mode)) return String(text || '');
    const line = `${name} ${mode}`;
    const lines = String(text || '').split(/\r?\n/);
    const key = stem(name);
    const i = lines.findIndex((l) => {
      const a = lineApp(l);
      return a !== null && stem(a) === key;
    });
    if (i >= 0) {
      lines[i] = line;
      return lines.join('\n');
    }
    while (lines.length && lines[lines.length - 1].trim() === '') lines.pop();
    lines.push(line);
    return lines.join('\n');
  }

  const api = { MODES, stem, lineApp, addRule };
  if (typeof module !== 'undefined' && module.exports) module.exports = api;
  else root.FlowerApps = api;
})(typeof window !== 'undefined' ? window : globalThis);
