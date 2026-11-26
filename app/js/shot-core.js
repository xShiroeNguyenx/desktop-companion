// 📸 Chụp màn hình (shot.html, PLAN.md 20): hình học của vùng chọn, tay nắm, chỗ đặt
// thanh công cụ, đầu mũi tên, tên file. Hàm thuần, dùng trong cửa sổ (window.ShotCore)
// và node --test. Mọi toạ độ là pixel ảnh (pixel thật của màn hình), gốc ở góc trên-trái.
(function (root) {
  /** 8 tay nắm, theo chiều kim đồng hồ từ góc trên-trái. */
  const HANDLES = ['nw', 'n', 'ne', 'e', 'se', 's', 'sw', 'w'];

  /** Vùng chọn nhỏ hơn cỡ này (mỗi chiều) coi như bấm nhầm, không phải khoanh vùng. */
  const MIN_SIZE = 4;

  const clamp = (v, lo, hi) => Math.min(Math.max(v, lo), Math.max(lo, hi));

  /** Hình chữ nhật từ hai góc bất kỳ → { x, y, w, h } số nguyên, w / h không âm. */
  function normRect(x1, y1, x2, y2) {
    const x = Math.round(Math.min(x1, x2));
    const y = Math.round(Math.min(y1, y2));
    return { x, y, w: Math.round(Math.max(x1, x2)) - x, h: Math.round(Math.max(y1, y2)) - y };
  }

  /** Cắt hình chữ nhật cho nằm gọn trong ảnh W × H (dùng khi đang kéo khoanh / đổi cỡ). */
  function cropRect(r, W, H) {
    const x1 = clamp(r.x, 0, W);
    const y1 = clamp(r.y, 0, H);
    const x2 = clamp(r.x + r.w, 0, W);
    const y2 = clamp(r.y + r.h, 0, H);
    return { x: x1, y: y1, w: x2 - x1, h: y2 - y1 };
  }

  /** Đủ lớn để coi là một vùng chọn thật. */
  function usable(r) {
    return !!r && r.w >= MIN_SIZE && r.h >= MIN_SIZE;
  }

  /** Toạ độ tâm của từng tay nắm. */
  function handlePoints(r) {
    const mx = r.x + r.w / 2;
    const my = r.y + r.h / 2;
    const rx = r.x + r.w;
    const by = r.y + r.h;
    return {
      nw: [r.x, r.y], n: [mx, r.y], ne: [rx, r.y], e: [rx, my],
      se: [rx, by], s: [mx, by], sw: [r.x, by], w: [r.x, my],
    };
  }

  /**
   * Con trỏ đang ở đâu so với vùng chọn: tên tay nắm ('nw'…), 'move' (bên trong),
   * hay 'new' (bên ngoài / chưa có vùng chọn). `tol` = bán kính bắt tay nắm.
   */
  function hitTest(r, px, py, tol) {
    if (!usable(r)) return 'new';
    const pts = handlePoints(r);
    for (const name of HANDLES) {
      const [hx, hy] = pts[name];
      if (Math.abs(px - hx) <= tol && Math.abs(py - hy) <= tol) return name;
    }
    if (px >= r.x && px <= r.x + r.w && py >= r.y && py <= r.y + r.h) return 'move';
    return 'new';
  }

  /**
   * Vùng chọn sau khi kéo: `start` là vùng lúc nhấn chuột, `mode` là kết quả hitTest
   * ('move' hoặc tên tay nắm), (dx, dy) là quãng đã kéo. Dời thì giữ nguyên cỡ và kẹp
   * trong ảnh; đổi cỡ thì kéo qua cạnh đối diện sẽ lật lại (không ra cỡ âm).
   */
  function dragRect(start, mode, dx, dy, W, H) {
    if (mode === 'move') {
      return {
        x: Math.round(clamp(start.x + dx, 0, W - start.w)),
        y: Math.round(clamp(start.y + dy, 0, H - start.h)),
        w: start.w,
        h: start.h,
      };
    }
    let x1 = start.x;
    let y1 = start.y;
    let x2 = start.x + start.w;
    let y2 = start.y + start.h;
    if (mode.includes('w')) x1 += dx;
    if (mode.includes('e')) x2 += dx;
    if (mode.includes('n')) y1 += dy;
    if (mode.includes('s')) y2 += dy;
    return cropRect(normRect(x1, y1, x2, y2), W, H);
  }

  /** Con trỏ chuột (CSS cursor) cho kết quả hitTest. */
  function cursorFor(mode) {
    switch (mode) {
      case 'nw': case 'se': return 'nwse-resize';
      case 'ne': case 'sw': return 'nesw-resize';
      case 'n': case 's': return 'ns-resize';
      case 'e': case 'w': return 'ew-resize';
      case 'move': return 'move';
      default: return 'crosshair';
    }
  }

  /**
   * Góc trên-trái của thanh công cụ cỡ tb.w × tb.h cho vùng chọn `r` trong khung W × H:
   * dưới vùng chọn (canh mép phải); hết chỗ thì lên trên; hết nữa thì nằm trong, sát đáy.
   * Cùng đơn vị với `r` (người gọi tự đổi sang CSS px).
   */
  function toolbarPos(r, tb, W, H, gap) {
    const x = clamp(r.x + r.w - tb.w, 0, W - tb.w);
    const below = r.y + r.h + gap;
    if (below + tb.h <= H) return { x, y: below };
    const above = r.y - gap - tb.h;
    if (above >= 0) return { x, y: above };
    return { x, y: clamp(r.y + r.h - gap - tb.h, 0, H - tb.h) };
  }

  /**
   * Hai đỉnh cánh của đầu mũi tên tại (x2, y2), mũi tên đi từ (x1, y1). `size` = độ dài
   * cánh. Đoạn quá ngắn (hai điểm trùng nhau) thì trả null.
   */
  function arrowHead(x1, y1, x2, y2, size) {
    const len = Math.hypot(x2 - x1, y2 - y1);
    if (len < 1) return null;
    const ang = Math.atan2(y2 - y1, x2 - x1);
    const spread = Math.PI / 7;
    return [
      [x2 - size * Math.cos(ang - spread), y2 - size * Math.sin(ang - spread)],
      [x2 - size * Math.cos(ang + spread), y2 - size * Math.sin(ang + spread)],
    ];
  }

  /**
   * Giữ Shift khi kéo đường thẳng / mũi tên: bẻ điểm cuối về góc gần nhất trong các
   * góc 0°, 45°, 90°… (giữ nguyên độ dài), để kẻ được đường ngang, dọc, chéo thật thẳng.
   */
  function snapEnd(x1, y1, x2, y2) {
    const len = Math.hypot(x2 - x1, y2 - y1);
    if (len < 1) return [x2, y2];
    const step = Math.PI / 4;
    const ang = Math.round(Math.atan2(y2 - y1, x2 - x1) / step) * step;
    // "+ 0": Math.round có thể ra -0, đổi về 0 cho gọn.
    return [Math.round(x1 + len * Math.cos(ang)) + 0, Math.round(y1 + len * Math.sin(ang)) + 0];
  }

  /** Các nấc độ dày nét (CSS px), lăn chuột để đổi. */
  const WIDTHS = [1, 2, 3, 4, 6, 8, 12, 16];
  const DEFAULT_WIDTH = 3;

  /** Nấc kế tiếp khi lăn chuột: dir > 0 dày lên, dir < 0 mỏng đi; hết nấc thì đứng yên. */
  function stepWidth(w, dir) {
    let i = WIDTHS.indexOf(w);
    if (i < 0) i = WIDTHS.indexOf(DEFAULT_WIDTH);
    return WIDTHS[clamp(i + (dir > 0 ? 1 : -1), 0, WIDTHS.length - 1)];
  }

  /**
   * Cỡ thật (CSS px) của từng công cụ ở nấc `w`: bút dạ quang to gấp ~5 lần nét thường,
   * chữ thì là cỡ chữ (nấc mặc định 3 → 18px).
   */
  function strokeFor(tool, w) {
    if (tool === 'marker') return w * 5 + 1;
    if (tool === 'text') return 12 + w * 2;
    return w;
  }

  /**
   * Hệ số phóng ảnh trước khi đọc chữ: chữ giao diện ở 100% DPI chỉ cao ~10px, phóng gấp
   * đôi thì bộ OCR đọc đúng hơn hẳn. Vùng đã lớn thì giữ nguyên (ảnh gửi đi không quá
   * 4000px mỗi chiều).
   */
  function ocrScale(w, h) {
    return Math.max(w, h) * 2 <= 4000 ? 2 : 1;
  }

  const pad2 = (n) => String(n).padStart(2, '0');

  /** Tên file gợi ý theo giờ máy: Screenshot_2026-11-26_14-03-09.png. */
  function fileName(d) {
    return `Screenshot_${d.getFullYear()}-${pad2(d.getMonth() + 1)}-${pad2(d.getDate())}` +
      `_${pad2(d.getHours())}-${pad2(d.getMinutes())}-${pad2(d.getSeconds())}.png`;
  }

  const api = {
    HANDLES, MIN_SIZE, normRect, cropRect, usable, handlePoints, hitTest, dragRect,
    cursorFor, toolbarPos, arrowHead, snapEnd, WIDTHS, DEFAULT_WIDTH, stepWidth, strokeFor, ocrScale, fileName,
  };
  if (typeof module !== 'undefined' && module.exports) module.exports = api;
  else root.ShotCore = api;
})(typeof window !== 'undefined' ? window : globalThis);
