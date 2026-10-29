// Lọc + sắp xếp ghi chú cho bảng 🗒️ Ghi chú (notes.js).
// Hàm thuần, dùng được cả trong trình duyệt (window.NotesCore) lẫn node --test.
(function (root) {
  // Tìm không dấu, không phân biệt hoa thường: gõ "mat khau" ra "Mật khẩu".
  function fold(s) {
    return String(s || '')
      .normalize('NFD')
      .replace(/[̀-ͯ]/g, '')
      .replace(/đ/g, 'd')
      .replace(/Đ/g, 'D')
      .toLowerCase();
  }

  /** Mọi từ trong `query` đều phải có trong tiêu đề hoặc nội dung. */
  function matches(note, query) {
    const words = fold(query).split(/\s+/).filter(Boolean);
    if (!words.length) return true;
    const hay = fold(note.title) + '\n' + fold(note.body);
    return words.every((w) => hay.includes(w));
  }

  /** Ghim lên trước, rồi sửa gần nhất lên trước. Không đổi mảng gốc. */
  function sortNotes(notes) {
    const t = (n) => Number(n.updated_at) || Number(n.created_at) || 0;
    return [...notes].sort((a, b) => {
      if (!!a.pinned !== !!b.pinned) return a.pinned ? -1 : 1;
      return t(b) - t(a);
    });
  }

  function visibleNotes(notes, query) {
    return sortNotes((notes || []).filter((n) => matches(n, query)));
  }

  const api = { fold, matches, sortNotes, visibleNotes };
  if (typeof module !== 'undefined' && module.exports) module.exports = api;
  else root.NotesCore = api;
})(typeof window !== 'undefined' ? window : globalThis);
