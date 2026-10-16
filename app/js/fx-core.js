// Tham số hiệu ứng thời tiết toàn màn hình (fx.html, PLAN.md 13). Hàm thuần,
// không đụng DOM / canvas; `rand` trả số trong [0, 1) được truyền vào để test
// được. Đơn vị: CSS px và giây. Dùng trong cửa sổ phủ (window.FxCore) và node --test.
(function (root) {
  // ĐỒNG BỘ với weather::KINDS (src-tauri/src/weather.rs).
  const KINDS = ['rain', 'snow', 'sakura', 'leaves', 'fog', 'sun', 'stars'];
  // Loại có hạt rơi (chạm mặt đất); sương trôi ngang, nắng / sao đứng yên.
  const FALLING = ['rain', 'snow', 'sakura', 'leaves'];
  const INTENSITY = { light: 0.5, medium: 1, heavy: 1.75 };
  // Số hạt ở mức Vừa trên màn 1920×1080; màn khác cỡ thì nhân theo diện tích.
  const BASE = { rain: 160, snow: 110, sakura: 40, leaves: 30, fog: 6, sun: 1, stars: 70 };
  // Trần cứng để màn 4K / mức Dày không làm nặng máy.
  const MAX = { rain: 420, snow: 280, sakura: 100, leaves: 80, fog: 12, sun: 1, stars: 160 };
  const REF_AREA = 1920 * 1080;
  // ~30 khung hình / giây là đủ mượt cho hạt rơi, đỡ tốn GPU hơn 60.
  const FRAME_MS = 33;
  // Tuyết đọng: mỗi ô rộng GROUND_BIN px, cao tối đa GROUND_MAX px, tan dần.
  const GROUND_BIN = 8;
  const GROUND_MAX = 14;
  const GROUND_MELT = 0.6; // px / giây
  // Chớp (dông, chỉ khi người dùng bật): cách nhau tối thiểu 8 giây, sáng nhẹ,
  // tắt dần trong FLASH_SECS — an toàn cho người nhạy cảm ánh sáng.
  const FLASH_GAP_MIN = 8;
  const FLASH_GAP_MAX = 20;
  const FLASH_PEAK = 0.18;
  const FLASH_SECS = 0.35;

  const SAKURA = ['#ffc1d9', '#ffd6e6', '#ffb0cf', '#ffe4ef'];
  const LEAVES = ['#e8893a', '#d9622b', '#f0b13c', '#b8642f', '#c9482d'];

  function pick(list, rand) {
    return list[Math.min(list.length - 1, Math.floor(rand() * list.length))];
  }

  /** Số hạt cho một màn `w`×`h` (CSS px). */
  function particleCount(kind, intensity, w, h) {
    const base = BASE[kind];
    if (!base || !(w > 0) || !(h > 0)) return 0;
    if (kind === 'sun') return 1;
    const mult = INTENSITY[intensity] || INTENSITY.medium;
    const n = Math.round(base * mult * ((w * h) / REF_AREA));
    return Math.max(1, Math.min(MAX[kind], n));
  }

  /**
   * Một hạt mới. `initial` = lúc vừa bật (rải khắp màn cho khỏi trống), còn
   * lại sinh ngay trên mép trên (lá bay sinh ở mép trái hoặc trên).
   */
  function spawn(kind, w, h, rand, initial) {
    const depth = 0.4 + rand() * 0.6; // xa (mờ, nhỏ, chậm) → gần
    const p = { kind, depth, x: rand() * w, y: initial ? rand() * h : -20 - rand() * 60, rot: 0, vr: 0, phase: rand() * Math.PI * 2 };
    if (kind === 'rain') {
      p.len = 10 + 16 * depth;
      p.vy = 700 + 600 * depth;
      p.vx = -90 * depth;
    } else if (kind === 'snow') {
      p.size = 1.5 + 3.5 * depth;
      p.vy = 30 + 60 * depth;
      p.vx = 0;
      p.sway = 12 + 18 * rand();
    } else if (kind === 'sakura') {
      p.size = 5 + 6 * depth;
      p.vy = 35 + 45 * depth;
      p.vx = 20 + 30 * rand();
      p.sway = 18 + 22 * rand();
      p.rot = rand() * Math.PI * 2;
      p.vr = (rand() - 0.5) * 3;
      p.color = pick(SAKURA, rand);
    } else if (kind === 'fog') {
      // Một mảng sương lớn trôi chậm sang phải ở nửa dưới màn hình.
      p.size = 180 + 260 * depth;
      p.x = initial ? rand() * w : -p.size;
      p.y = h * (0.3 + 0.7 * rand());
      p.vy = 0;
      p.vx = 8 + 14 * depth;
    } else if (kind === 'sun') {
      // Một nguồn sáng ở góc trên bên phải.
      p.x = w;
      p.y = 0;
      p.size = Math.min(w, h) * 0.6;
      p.vx = 0;
      p.vy = 0;
    } else if (kind === 'stars') {
      // Sao đứng yên ở nửa trên màn hình, chỉ lấp lánh.
      p.x = rand() * w;
      p.y = rand() * h * 0.45;
      p.size = 0.8 + 1.8 * depth;
      p.vx = 0;
      p.vy = 0;
      p.tw = 0.8 + 2 * rand();
    } else {
      p.size = 8 + 8 * depth;
      p.vy = 40 + 50 * depth;
      p.vx = 60 + 80 * rand();
      p.sway = 25 + 25 * rand();
      p.rot = rand() * Math.PI * 2;
      p.vr = (rand() - 0.5) * 4;
      p.color = pick(LEAVES, rand);
      if (!initial && rand() < 0.5) {
        p.x = -30;
        p.y = rand() * h * 0.7;
      }
    }
    return p;
  }

  /** Dời hạt sau `dt` giây; `t` là đồng hồ chung (giây) để lắc lư. Sửa tại chỗ. */
  function step(p, dt, t) {
    p.y += p.vy * dt;
    if (p.sway) p.x += p.vx * dt + Math.sin(t * 1.6 + p.phase) * p.sway * dt;
    else p.x += p.vx * dt;
    p.rot += p.vr * dt;
    return p;
  }

  /** Hạt đã ra khỏi màn (hoặc chạm "mặt đất" `ground`) chưa. */
  function gone(p, w, ground) {
    if (p.kind === 'sun' || p.kind === 'stars') return false;
    const m = Math.max(60, p.size || 0);
    return p.y > ground || p.x < -m || p.x > w + m;
  }

  /** Độ sáng lấp lánh của một ngôi sao lúc `t` (0.25–1). */
  function twinkle(p, t) {
    return 0.25 + 0.75 * Math.abs(Math.sin(t * (p.tw || 1) + p.phase));
  }

  /** Dải tuyết đọng: một mảng chiều cao theo từng ô ngang. */
  function groundNew(w) {
    return new Array(Math.max(1, Math.ceil(w / GROUND_BIN))).fill(0);
  }
  function groundAdd(bins, x, amount) {
    const i = Math.floor(x / GROUND_BIN);
    if (i < 0 || i >= bins.length) return bins;
    bins[i] = Math.min(GROUND_MAX, bins[i] + amount);
    return bins;
  }
  function groundMelt(bins, dt) {
    for (let i = 0; i < bins.length; i++) bins[i] = Math.max(0, bins[i] - GROUND_MELT * dt);
    return bins;
  }

  /** Lần chớp kế tiếp (giây, theo đồng hồ chung `t`). */
  function nextFlash(t, rand) {
    return t + FLASH_GAP_MIN + rand() * (FLASH_GAP_MAX - FLASH_GAP_MIN);
  }
  /** Độ sáng lớp chớp `dt` giây sau khi chớp (0 khi đã tắt hẳn). */
  function flashAlpha(dt) {
    if (!(dt >= 0) || dt >= FLASH_SECS) return 0;
    return FLASH_PEAK * (1 - dt / FLASH_SECS);
  }

  /** Điểm (x, y) có nằm trong một "lỗ" (bảng của app) không. */
  function inHoles(holes, x, y) {
    if (!holes) return false;
    for (const r of holes) if (x >= r.x && x < r.x + r.w && y >= r.y && y < r.y + r.h) return true;
    return false;
  }

  const api = {
    KINDS, FALLING, INTENSITY, BASE, MAX, FRAME_MS, GROUND_BIN, GROUND_MAX,
    FLASH_GAP_MIN, FLASH_PEAK,
    particleCount, spawn, step, gone, twinkle, groundNew, groundAdd, groundMelt, inHoles, nextFlash, flashAlpha,
  };
  if (typeof module !== 'undefined' && module.exports) module.exports = api;
  else root.FxCore = api;
})(typeof window !== 'undefined' ? window : globalThis);
