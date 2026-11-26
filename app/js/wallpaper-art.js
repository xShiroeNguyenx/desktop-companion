// 🖼 Bộ hình nền vẽ sẵn (Cài đặt → Hình nền): app tự vẽ bằng canvas ở đúng độ phân
// giải màn hình, nên không kèm file ảnh nào và không vướng bản quyền ảnh.
// scene(id, w, h) là hàm thuần (cùng id + kích thước → cùng một cảnh), trả danh sách
// lệnh vẽ; paint(ctx, scene) mới đụng canvas. Dùng trong cửa sổ (window.WallArt) và node --test.
(function (root) {
  // [id, vi, ja, en]
  const THEMES = [
    ['dawn', 'Bình minh', '夜明け', 'Dawn'],
    ['night', 'Đêm sao', '星空', 'Starry night'],
    ['sakura', 'Anh đào', '桜', 'Sakura'],
    ['ocean', 'Biển chiều', '夕暮れの海', 'Evening sea'],
    ['forest', 'Rừng sương', '霧の森', 'Misty forest'],
    ['aurora', 'Cực quang', 'オーロラ', 'Aurora'],
  ];
  const LANG_COL = { vi: 1, ja: 2, en: 3 };

  function themeName(id, lang) {
    const row = THEMES.find((r) => r[0] === id);
    return row ? row[LANG_COL[lang] || 1] : id;
  }

  // mulberry32: số ngẫu nhiên có hạt giống, để cảnh vẽ lại y hệt.
  function rng(seed) {
    let a = seed >>> 0;
    return function () {
      a = (a + 0x6d2b79f5) >>> 0;
      let t = a;
      t = Math.imul(t ^ (t >>> 15), t | 1);
      t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
      return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
    };
  }

  function seedOf(id) {
    let h = 2166136261;
    for (const c of id) h = Math.imul(h ^ c.charCodeAt(0), 16777619);
    return h >>> 0;
  }

  // Đường núi / sóng: tổng vài sóng sin + chút gồ ghề, đi hết bề ngang.
  function ridge(r, w, baseY, amp, jag, steps) {
    const p1 = r() * 6.28, p2 = r() * 6.28, p3 = r() * 6.28;
    const f1 = 1 + r() * 1.5, f2 = 3 + r() * 3, f3 = 9 + r() * 8;
    const pts = [];
    for (let i = 0; i <= steps; i++) {
      const x = (i / steps) * w;
      const u = i / steps;
      const y = baseY
        - amp * (0.6 * Math.sin(u * 6.28 * f1 + p1) + 0.3 * Math.sin(u * 6.28 * f2 + p2) + 0.1 * Math.sin(u * 6.28 * f3 + p3))
        - jag * (r() - 0.5);
      pts.push([x, y]);
    }
    return pts;
  }

  // Hàng thông: răng cưa nhọn trên một đường nền.
  function pines(r, w, baseY, height, count) {
    const pts = [[0, baseY]];
    const step = w / count;
    for (let i = 0; i < count; i++) {
      const x = i * step;
      const hgt = height * (0.55 + r() * 0.45);
      const by = baseY + (r() - 0.5) * height * 0.15;
      pts.push([x + step * 0.1, by], [x + step * 0.5, by - hgt], [x + step * 0.9, by]);
    }
    pts.push([w, baseY]);
    return pts;
  }

  function stars(r, w, h, count, maxY, size) {
    const out = [];
    for (let i = 0; i < count; i++) {
      out.push({ t: 'circle', x: r() * w, y: r() * h * maxY, r: size * (0.3 + r() * r() * 1.4), color: '#ffffff', alpha: 0.35 + r() * 0.65 });
    }
    return out;
  }

  function scene(id, w, h) {
    const r = rng(seedOf(id));
    const s = h / 1080; // cỡ chi tiết theo chiều cao màn hình
    const ops = [];
    const steps = 160;
    switch (id) {
      case 'dawn':
        ops.push({ t: 'grad', stops: [[0, '#2b1e4a'], [0.35, '#7a4d8c'], [0.65, '#f28c8c'], [1, '#ffd6a0']] });
        ops.push({ t: 'glow', x: w * 0.68, y: h * 0.62, r: h * 0.35, color: '#fff1c9', alpha: 0.9 });
        ops.push({ t: 'circle', x: w * 0.68, y: h * 0.62, r: h * 0.07, color: '#fff6dc', alpha: 1 });
        ops.push({ t: 'ridge', pts: ridge(r, w, h * 0.7, h * 0.08, 0, steps), color: '#9a5a8c', alpha: 0.85 });
        ops.push({ t: 'ridge', pts: ridge(r, w, h * 0.8, h * 0.07, 0, steps), color: '#6b3c6e', alpha: 0.95 });
        ops.push({ t: 'ridge', pts: ridge(r, w, h * 0.9, h * 0.05, 0, steps), color: '#3d2246', alpha: 1 });
        break;
      case 'night':
        ops.push({ t: 'grad', stops: [[0, '#050816'], [0.6, '#0f1d3d'], [1, '#1e3560']] });
        ops.push(...stars(r, w, h, 320, 0.8, 1.6 * s));
        ops.push({ t: 'glow', x: w * 0.8, y: h * 0.2, r: h * 0.2, color: '#c9d8ff', alpha: 0.35 });
        ops.push({ t: 'circle', x: w * 0.8, y: h * 0.2, r: h * 0.055, color: '#f6f1d5', alpha: 1 });
        ops.push({ t: 'ridge', pts: ridge(r, w, h * 0.86, h * 0.09, h * 0.006, steps), color: '#0a1226', alpha: 1 });
        break;
      case 'sakura':
        ops.push({ t: 'grad', stops: [[0, '#fff0f5'], [0.5, '#ffd0e0'], [1, '#f4a3c2']] });
        for (let i = 0; i < 20; i++) ops.push({ t: 'glow', x: r() * w, y: r() * h, r: h * (0.04 + r() * 0.1), color: '#ffffff', alpha: 0.25 + r() * 0.2 });
        for (let i = 0; i < 160; i++) {
          const rr = h * (0.006 + r() * 0.01);
          ops.push({ t: 'ellipse', x: r() * w, y: r() * h, rx: rr * 1.6, ry: rr, rot: r() * 3.14, color: r() < 0.5 ? '#ff8fb5' : '#ffc4d8', alpha: 0.55 + r() * 0.45 });
        }
        break;
      case 'ocean':
        ops.push({ t: 'grad', stops: [[0, '#3a3f78'], [0.35, '#c06c84'], [0.55, '#f8b195'], [0.56, '#3b5b8a'], [1, '#162a4a']] });
        ops.push({ t: 'glow', x: w * 0.5, y: h * 0.55, r: h * 0.3, color: '#ffd9a8', alpha: 0.8 });
        ops.push({ t: 'circle', x: w * 0.5, y: h * 0.55, r: h * 0.06, color: '#fff0d0', alpha: 0.95 });
        for (let i = 0; i < 7; i++) {
          const y = h * (0.58 + i * 0.065);
          const c = ['#4a6c9b', '#41618f', '#385683', '#2f4a76', '#273f69', '#20355c', '#1a2c50'][i];
          ops.push({ t: 'ridge', pts: ridge(r, w, y, h * 0.006 * (i + 1), 0, steps), color: c, alpha: 0.9 });
        }
        break;
      case 'forest':
        ops.push({ t: 'grad', stops: [[0, '#dfece3'], [0.55, '#a9c9b3'], [1, '#5f8f72']] });
        ops.push({ t: 'glow', x: w * 0.3, y: h * 0.3, r: h * 0.4, color: '#ffffff', alpha: 0.5 });
        ['#8fb39c', '#6f9a80', '#4f7d63', '#335c46', '#1f3d2e'].forEach((c, i) => {
          ops.push({ t: 'ridge', pts: pines(r, w, h * (0.6 + i * 0.09), h * (0.12 + i * 0.03), 26 - i * 3), color: c, alpha: 1 });
          if (i < 4) ops.push({ t: 'mist', y: h * (0.6 + i * 0.09), hgt: h * 0.08, color: '#e8f2ec', alpha: 0.45 });
        });
        break;
      case 'aurora':
      default:
        ops.push({ t: 'grad', stops: [[0, '#020611'], [0.7, '#06162a'], [1, '#0b2238']] });
        ops.push(...stars(r, w, h, 180, 0.7, 1.4 * s));
        [['#3dffb0', 0.3], ['#2bd4ff', 0.4], ['#8a5cff', 0.22]].forEach(([c, y]) => {
          // Dải cực quang cần mịn hơn núi: ~3 px một cột, không thì mép trên thành bậc thang.
          ops.push({ t: 'band', pts: ridge(r, w, h * y, h * 0.08, 0, Math.max(steps, Math.round(w / 3))), thick: h * 0.22, color: c, alpha: 0.4 });
        });
        ops.push({ t: 'ridge', pts: ridge(r, w, h * 0.88, h * 0.07, h * 0.01, steps), color: '#01040b', alpha: 1 });
        break;
    }
    return { id, w, h, ops };
  }

  function hexToRgba(hex, a) {
    const n = parseInt(hex.slice(1), 16);
    return `rgba(${(n >> 16) & 255},${(n >> 8) & 255},${n & 255},${a})`;
  }

  function paint(ctx, sc) {
    const { w, h } = sc;
    for (const op of sc.ops) {
      ctx.globalAlpha = 1;
      switch (op.t) {
        case 'grad': {
          const g = ctx.createLinearGradient(0, 0, 0, h);
          for (const [p, c] of op.stops) g.addColorStop(p, c);
          ctx.fillStyle = g;
          ctx.fillRect(0, 0, w, h);
          break;
        }
        case 'glow': {
          const g = ctx.createRadialGradient(op.x, op.y, 0, op.x, op.y, op.r);
          g.addColorStop(0, hexToRgba(op.color, op.alpha));
          g.addColorStop(1, hexToRgba(op.color, 0));
          ctx.fillStyle = g;
          ctx.fillRect(op.x - op.r, op.y - op.r, op.r * 2, op.r * 2);
          break;
        }
        case 'circle':
          ctx.globalAlpha = op.alpha;
          ctx.fillStyle = op.color;
          ctx.beginPath();
          ctx.arc(op.x, op.y, op.r, 0, Math.PI * 2);
          ctx.fill();
          break;
        case 'ellipse':
          ctx.globalAlpha = op.alpha;
          ctx.fillStyle = op.color;
          ctx.beginPath();
          ctx.ellipse(op.x, op.y, op.rx, op.ry, op.rot, 0, Math.PI * 2);
          ctx.fill();
          break;
        case 'ridge':
          ctx.globalAlpha = op.alpha;
          ctx.fillStyle = op.color;
          ctx.beginPath();
          ctx.moveTo(0, h);
          for (const [x, y] of op.pts) ctx.lineTo(x, y);
          ctx.lineTo(w, h);
          ctx.closePath();
          ctx.fill();
          break;
        case 'mist': {
          const g = ctx.createLinearGradient(0, op.y - op.hgt, 0, op.y + op.hgt * 0.2);
          g.addColorStop(0, hexToRgba(op.color, 0));
          g.addColorStop(1, hexToRgba(op.color, op.alpha));
          ctx.fillStyle = g;
          ctx.fillRect(0, op.y - op.hgt, w, op.hgt * 1.2);
          break;
        }
        case 'band':
          // Rèm cực quang: dải dọc mờ dần từ đường cong xuống dưới.
          // Cột biên nguyên pixel, khít nhau: chồng lên nhau là hiện sọc.
          for (let i = 0; i + 1 < op.pts.length; i++) {
            const [xa, ya] = op.pts[i];
            const x0 = Math.round(xa), x1 = Math.round(op.pts[i + 1][0]);
            if (x1 <= x0) continue;
            const y = (ya + op.pts[i + 1][1]) / 2;
            const g = ctx.createLinearGradient(0, y, 0, y + op.thick);
            g.addColorStop(0, hexToRgba(op.color, 0));
            g.addColorStop(0.08, hexToRgba(op.color, op.alpha));
            g.addColorStop(1, hexToRgba(op.color, 0));
            ctx.fillStyle = g;
            ctx.fillRect(x0, y, x1 - x0, op.thick);
          }
          break;
      }
    }
    ctx.globalAlpha = 1;
  }

  const api = { THEMES, themeName, rng, seedOf, scene, paint };
  if (typeof module !== 'undefined' && module.exports) module.exports = api;
  else root.WallArt = api;
})(typeof window !== 'undefined' ? window : globalThis);
