// Ghim mục hay dùng ra vòng ngoài của menu tròn trên pet (PLAN.md 15.2 B6):
// chuột phải vào một mục trong nhóm để ghim, chuột phải vào mục đã ghim để gỡ.
// Lưu trong localStorage của cửa sổ pet (tiện ích riêng máy này, mất cũng không sao).
// Hàm thuần, dùng được cả trong trình duyệt (window.RadialPins) lẫn node --test.
(function (root) {
  const MAX_PINS = 2;
  const KEY = 'radialPins';

  /** Ghim / gỡ `action`. Trả { pins, result: 'pinned' | 'unpinned' | 'full' }. */
  function toggle(pins, action, max = MAX_PINS) {
    if (pins.includes(action)) return { pins: pins.filter((p) => p !== action), result: 'unpinned' };
    if (pins.length >= max) return { pins, result: 'full' };
    return { pins: [...pins, action], result: 'pinned' };
  }

  /** Mục của từng ghim, tìm trong các nhóm (mỗi nhóm có `items`). Ghim không còn mục nào thì bỏ. */
  function resolve(pins, groups) {
    const out = [];
    for (const action of pins) {
      for (const g of groups) {
        const hit = (g.items || []).find((it) => it.action === action);
        if (hit) {
          out.push(hit);
          break;
        }
      }
    }
    return out;
  }

  /** Đọc danh sách ghim; dữ liệu hỏng / storage bị chặn thì về rỗng. */
  function load(storage) {
    try {
      const v = JSON.parse(storage.getItem(KEY) || '[]');
      if (!Array.isArray(v)) return [];
      return [...new Set(v.filter((x) => typeof x === 'string' && x))].slice(0, MAX_PINS);
    } catch (_) {
      return [];
    }
  }

  function save(storage, pins) {
    try {
      storage.setItem(KEY, JSON.stringify(pins));
    } catch (_) {
      /* storage bị chặn: ghim chỉ sống tới khi đóng app */
    }
  }

  const api = { MAX_PINS, KEY, toggle, resolve, load, save };
  if (typeof module !== 'undefined' && module.exports) module.exports = api;
  else root.RadialPins = api;
})(typeof window !== 'undefined' ? window : globalThis);
