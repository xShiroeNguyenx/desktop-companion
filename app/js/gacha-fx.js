// 🎰 Candy Capsule (collection.html): vẽ máy kẹo bằng SVG và diễn hoạt cảnh quay.
//   GachaFx.buildMachine(el)        dựng máy một lần (không dựng lại giữa hoạt cảnh)
//   GachaFx.spin(machine, rarity)   lắc máy, capsule nhảy, núm xoay, viên kẹo rơi ra
//   GachaFx.reveal(layer, rarity, strings)  phủ cửa sổ: capsule to rung rồi bung ra;
//       Thường: lóe hồng · Hiếm: tia tím xoay + sao · Sử thi: chớp sáng, tia vàng,
//       mưa sao + confetti, chữ "SỬ THI!" · Huyền thoại (model Live2D): tia cầu vồng,
//       rung lâu hơn, confetti dày, chữ "HUYỀN THOẠI!". Bấm vào lớp phủ để bỏ qua.
// Tôn trọng prefers-reduced-motion: không chạy hoạt cảnh, trả về ngay.
(function () {
  const NS = 'http://www.w3.org/2000/svg';
  const reduced = () => !!(window.matchMedia && window.matchMedia('(prefers-reduced-motion: reduce)').matches);

  /** Màu viên capsule [sáng, đậm]. */
  const CAP_COLORS = [
    ['#ffc4df', '#ff7fb6'],
    ['#e2d2ff', '#a98bff'],
    ['#c9ebff', '#6fbcff'],
    ['#fff0bf', '#ffc94d'],
    ['#d3f5df', '#6fd39a'],
  ];
  /** Màu theo hạng cho viên kẹo rơi ra, capsule lớn, tia sáng. */
  const RARITY = {
    common: { cap: '#ff8fc0', glow: 'rgba(255, 143, 192, 0.85)', parts: ['💗', '✦', '🌸', '✧'], count: 16 },
    rare: { cap: '#9b7bff', glow: 'rgba(155, 123, 255, 0.9)', parts: ['✦', '⭐', '💜', '✧', '💠'], count: 26 },
    epic: { cap: '#ffc53d', glow: 'rgba(255, 197, 61, 0.95)', parts: ['🌟', '⭐', '✦', '💛', '✨'], count: 34, confetti: 36 },
    legendary: { cap: '#ff6fb2', glow: 'rgba(255, 140, 220, 0.95)', parts: ['🌈', '🌟', '💖', '✦', '💎', '✨'], count: 44, confetti: 70 },
  };
  /** Độ dài từng nhịp theo hạng [rung, mở] (ms) và độ toả hạt. */
  const TIMING = { common: [650, 800, 0.38], rare: [1050, 1200, 0.5], epic: [1500, 1700, 0.62], legendary: [2000, 2300, 0.7] };
  const timing = (rarity) => TIMING[rarity] || TIMING.common;
  const LABEL = { common: 'rvCommon', rare: 'rvRare', epic: 'rvEpic', legendary: 'rvLegendary' };

  // ─── Máy kẹo ───────────────────────────────────────────────────────────────

  /** Một viên capsule trong vòm: cầu trong suốt có bé thỏ bên trong. */
  function capsule(x, y, r, ci, i) {
    const k = (n) => (n * r).toFixed(1);
    return `<g transform="translate(${x} ${y})"><g class="cap" style="--d:${(i * 0.06).toFixed(2)}s">
      <circle r="${r}" fill="url(#gcap${ci})" stroke="#fff" stroke-opacity=".95" stroke-width="1.6"/>
      <g fill="#fff" opacity=".95">
        <ellipse cx="${k(-0.2)}" cy="${k(-0.36)}" rx="${k(0.12)}" ry="${k(0.3)}" transform="rotate(-14 ${k(-0.2)} ${k(-0.36)})"/>
        <ellipse cx="${k(0.2)}" cy="${k(-0.36)}" rx="${k(0.12)}" ry="${k(0.3)}" transform="rotate(14 ${k(0.2)} ${k(-0.36)})"/>
        <ellipse cx="0" cy="${k(0.1)}" rx="${k(0.42)}" ry="${k(0.33)}"/>
      </g>
      <g fill="#ffb3d1" opacity=".9">
        <ellipse cx="${k(-0.2)}" cy="${k(-0.34)}" rx="${k(0.05)}" ry="${k(0.17)}" transform="rotate(-14 ${k(-0.2)} ${k(-0.34)})"/>
        <ellipse cx="${k(0.2)}" cy="${k(-0.34)}" rx="${k(0.05)}" ry="${k(0.17)}" transform="rotate(14 ${k(0.2)} ${k(-0.34)})"/>
        <ellipse cx="${k(-0.26)}" cy="${k(0.2)}" rx="${k(0.08)}" ry="${k(0.05)}"/>
        <ellipse cx="${k(0.26)}" cy="${k(0.2)}" rx="${k(0.08)}" ry="${k(0.05)}"/>
      </g>
      <g fill="#5a3550"><circle cx="${k(-0.14)}" cy="${k(0.06)}" r="${k(0.05)}"/><circle cx="${k(0.14)}" cy="${k(0.06)}" r="${k(0.05)}"/></g>
      <ellipse cx="${k(-0.42)}" cy="${k(-0.46)}" rx="${k(0.22)}" ry="${k(0.12)}" fill="#fff" opacity=".8" transform="rotate(-38 ${k(-0.42)} ${k(-0.46)})"/>
    </g></g>`;
  }

  /** SVG của cả máy (viewBox 220×244). */
  function machineSvg() {
    const grads = CAP_COLORS.map(([a, b], i) => `
      <radialGradient id="gcap${i}" cx=".38" cy=".32" r=".75">
        <stop offset="0" stop-color="#fff" stop-opacity=".95"/>
        <stop offset=".45" stop-color="${a}" stop-opacity=".88"/>
        <stop offset="1" stop-color="${b}" stop-opacity=".92"/>
      </radialGradient>`).join('');
    // Sau → trước (hàng trên vẽ trước để hàng dưới đè lên). Vòm tâm (118,122) r 78.
    const caps = [
      [96, 90, 18, 2], [140, 88, 18, 4],
      [66, 120, 19, 3], [106, 122, 21, 0], [150, 120, 20, 1], [186, 130, 15, 4],
      [54, 160, 17, 1], [86, 162, 22, 2], [130, 166, 23, 3], [174, 160, 21, 0],
    ].map(([x, y, r, c], i) => capsule(x, y, r, c, i)).join('');
    return `<svg viewBox="0 0 236 250" xmlns="${NS}">
  <defs>${grads}
    <radialGradient id="gdome" cx=".4" cy=".32" r=".8">
      <stop offset="0" stop-color="#fff" stop-opacity=".95"/>
      <stop offset=".6" stop-color="#ffe6f3" stop-opacity=".6"/>
      <stop offset="1" stop-color="#dccbff" stop-opacity=".75"/>
    </radialGradient>
    <linearGradient id="gbase" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color="#ffc8e0"/><stop offset="1" stop-color="#ff86b8"/>
    </linearGradient>
    <linearGradient id="gsign" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color="#ffb0d2"/><stop offset="1" stop-color="#ff5f9e"/>
    </linearGradient>
    <radialGradient id="goutlet" cx=".5" cy=".4" r=".7">
      <stop offset="0" stop-color="#ff9cc8"/><stop offset=".55" stop-color="#b8306f"/><stop offset="1" stop-color="#6e1d45"/>
    </radialGradient>
    <clipPath id="cdome"><circle cx="118" cy="122" r="74"/></clipPath>
    <filter id="fsoft" x="-20%" y="-20%" width="140%" height="140%"><feDropShadow dx="0" dy="3" stdDeviation="3" flood-color="#b0507f" flood-opacity=".3"/></filter>
  </defs>
  <!-- Đế -->
  <g class="base" filter="url(#fsoft)">
    <rect x="56" y="192" width="124" height="16" rx="8" fill="#ff9cc8" stroke="#fff" stroke-width="2.5"/>
    <path d="M44 206 H192 Q201 206 203 214 L212 238 Q214 248 204 248 H32 Q22 248 24 238 L33 214 Q35 206 44 206Z" fill="url(#gbase)" stroke="#fff" stroke-width="3"/>
    <path class="outlet" d="M118 244 C 101 234 97 222 106 216 C 112 212 118 216 118 220 C 118 216 124 212 130 216 C 139 222 135 234 118 244Z" fill="url(#goutlet)" stroke="#fff" stroke-width="2.5"/>
    <path d="M58 216 l3 6.2 6.8 1 -4.9 4.7 1.2 6.8 -6.1 -3.2 -6.1 3.2 1.2 -6.8 -4.9 -4.7 6.8 -1z" fill="#ffd45c" stroke="#fff" stroke-width="1.8" stroke-linejoin="round"/>
    <g class="knob">
      <circle cx="178" cy="228" r="11" fill="#ff7fb6" stroke="#fff" stroke-width="2.5"/>
      <rect x="171" y="226" width="14" height="4.4" rx="2.2" fill="#fff"/>
      <rect x="175.8" y="221" width="4.4" height="14" rx="2.2" fill="#fff"/>
    </g>
  </g>
  <!-- Vòm kính + capsule -->
  <g class="dome">
    <circle cx="118" cy="122" r="78" fill="url(#gdome)" stroke="#ffc2dc" stroke-width="8"/>
    <g clip-path="url(#cdome)">${caps}</g>
    <circle cx="118" cy="122" r="74" fill="none" stroke="#fff" stroke-width="2.5" opacity=".9"/>
    <path d="M66 100 Q78 72 104 62" fill="none" stroke="#fff" stroke-width="8" stroke-linecap="round" opacity=".55"/>
    <circle cx="62" cy="116" r="4" fill="#fff" opacity=".8"/>
    <path d="M176 84 l2 5 5 2 -5 2 -2 5 -2 -5 -5 -2 5 -2z" fill="#fff" opacity=".9"/>
  </g>
  <!-- Bảng tên -->
  <g class="sign" filter="url(#fsoft)">
    <rect x="50" y="56" width="136" height="30" rx="15" fill="url(#gsign)" stroke="#fff" stroke-width="3"/>
    <text x="118" y="76.5" text-anchor="middle" font-family="Segoe UI, sans-serif" font-size="16" font-weight="900" fill="#fff" stroke="#d94585" stroke-width="2.4" paint-order="stroke" letter-spacing=".3">Candy Capsule</text>
    <path d="M44 62 c-6 -7 -15 -1 -10 7 l10 9 10 -9 c5 -8 -4 -14 -10 -7z" fill="#ff4f93" stroke="#fff" stroke-width="2.2"/>
    <path d="M192 62 c-6 -7 -15 -1 -10 7 l10 9 10 -9 c5 -8 -4 -14 -10 -7z" fill="#ff4f93" stroke="#fff" stroke-width="2.2"/>
  </g>
  <!-- Đầu thỏ đội nơ -->
  <g class="topper" filter="url(#fsoft)">
    <g class="ear-l"><ellipse cx="105" cy="14" rx="7" ry="15" fill="#fff" transform="rotate(-14 105 14)"/><ellipse cx="105" cy="16" rx="3" ry="10" fill="#ffb3d1" transform="rotate(-14 105 16)"/></g>
    <g class="ear-r"><ellipse cx="131" cy="14" rx="7" ry="15" fill="#fff" transform="rotate(14 131 14)"/><ellipse cx="131" cy="16" rx="3" ry="10" fill="#ffb3d1" transform="rotate(14 131 16)"/></g>
    <ellipse cx="118" cy="36" rx="21" ry="16" fill="#fff"/>
    <circle cx="110" cy="35" r="2.4" fill="#5a3550"/><circle cx="126" cy="35" r="2.4" fill="#5a3550"/>
    <ellipse cx="105" cy="41" rx="3.6" ry="2.2" fill="#ffb3d1"/><ellipse cx="131" cy="41" rx="3.6" ry="2.2" fill="#ffb3d1"/>
    <path d="M115 40 q3 3 6 0" fill="none" stroke="#5a3550" stroke-width="1.3" stroke-linecap="round"/>
    <path d="M118 52 L103 45 L104 59 Z M118 52 L133 45 L132 59 Z" fill="#e8344f" stroke="#fff" stroke-width="1.5" stroke-linejoin="round"/>
    <circle cx="118" cy="52" r="4.2" fill="#c71f3c" stroke="#fff" stroke-width="1.5"/>
  </g>
</svg>`;
  }

  function buildMachine(el) {
    if (!el || el.dataset.built) return;
    el.dataset.built = '1';
    el.innerHTML = machineSvg();
    const drop = document.createElement('span');
    drop.className = 'm-drop';
    el.append(drop);
    // Lấp lánh quanh máy (trang trí, chạy chậm).
    [[-6, 30], [96, 18], [4, 62], [92, 70], [-2, 88]].forEach(([x, y], i) => {
      const s = document.createElement('span');
      s.className = 'm-sp';
      s.textContent = i % 2 ? '✦' : '♡';
      s.style.left = x + '%';
      s.style.top = y + '%';
      s.style.animationDelay = (i * 0.5) + 's';
      el.append(s);
    });
  }

  // ─── Đợi có thể bỏ qua ────────────────────────────────────────────────────

  let skipNow = null;
  function wait(ms) {
    return new Promise((res) => {
      const t = setTimeout(done, ms);
      function done() {
        clearTimeout(t);
        if (skipNow === done) skipNow = null;
        res();
      }
      skipNow = done;
    });
  }
  let skipping = false;
  function skip() {
    skipping = true;
    if (skipNow) skipNow();
  }
  const pause = (ms) => (skipping ? Promise.resolve() : wait(ms));

  /** Máy lắc, capsule nhảy, núm xoay, viên kẹo màu hạng rơi ra khe trái tim. */
  async function spin(machine, rarity) {
    if (reduced()) return;
    skipping = false;
    const R = RARITY[rarity] || RARITY.common;
    machine.style.setProperty('--dc', R.cap);
    machine.style.setProperty('--dg', R.glow);
    machine.classList.remove('spin', 'spin-rare', 'spin-epic', 'spin-legendary');
    void machine.offsetWidth; // chạy lại hoạt cảnh khi quay liên tiếp
    machine.classList.add('spin');
    if (rarity !== 'common') machine.classList.add('spin-' + rarity);
    await pause(1500);
  }

  // ─── Mở capsule ───────────────────────────────────────────────────────────

  function particles(box, R, rarity) {
    box.textContent = '';
    const W = box.clientWidth || 900;
    const H = box.clientHeight || 600;
    const spread = Math.min(W, H) * timing(rarity)[2];
    for (let i = 0; i < R.count; i++) {
      const a = (Math.PI * 2 * i) / R.count + Math.random() * 0.4;
      const d = spread * (0.55 + Math.random() * 0.5);
      const p = document.createElement('span');
      p.className = 'pt';
      p.textContent = R.parts[i % R.parts.length];
      p.style.setProperty('--dx', Math.round(Math.cos(a) * d) + 'px');
      p.style.setProperty('--dy', Math.round(Math.sin(a) * d) + 'px');
      p.style.setProperty('--s', (0.8 + Math.random() * 0.9).toFixed(2));
      p.style.setProperty('--r', Math.round(Math.random() * 360 - 180) + 'deg');
      p.style.animationDelay = Math.round(Math.random() * 120) + 'ms';
      box.append(p);
    }
    // Sử thi, Huyền thoại: confetti rơi từ trên xuống phủ cả cửa sổ.
    const colors = ['#ffc53d', '#ff7fb6', '#9b7bff', '#6fd3ff', '#7fe0b0', '#fff'];
    for (let i = 0; i < (R.confetti || 0); i++) {
      const c = document.createElement('i');
      c.className = 'cf';
      c.style.left = Math.round(Math.random() * 100) + '%';
      c.style.background = colors[i % colors.length];
      c.style.setProperty('--x', Math.round(Math.random() * 160 - 80) + 'px');
      c.style.setProperty('--r', Math.round(Math.random() * 720 - 360) + 'deg');
      c.style.animationDelay = Math.round(Math.random() * 600) + 'ms';
      c.style.animationDuration = (1.6 + Math.random() * 1.2).toFixed(2) + 's';
      box.append(c);
    }
  }

  /**
   * Lớp phủ mở capsule. `layer` là #reveal trong collection.html.
   * @param {{rvCommon:string, rvRare:string, rvEpic:string, rvLegendary:string, rvSkip:string}} s
   * @param {Node} [prize] ô xem trước món trúng (hiếm nhất), hiện ra lúc capsule bung.
   */
  async function reveal(layer, rarity, s, prize) {
    if (reduced() || skipping) return;
    const R = RARITY[rarity] || RARITY.common;
    layer.className = 'reveal ' + (RARITY[rarity] ? rarity : 'common');
    layer.style.setProperty('--rc', R.cap);
    layer.style.setProperty('--rg', R.glow);
    layer.querySelector('.rv-label').textContent = s[LABEL[rarity] || 'rvCommon'] || s.rvCommon;
    layer.querySelector('.rv-skip').textContent = s.rvSkip;
    layer.querySelector('.rv-parts').textContent = '';
    const slot = layer.querySelector('.rv-item');
    slot.textContent = '';
    if (prize) slot.append(prize);
    layer.hidden = false;
    const onClick = () => skip();
    layer.addEventListener('click', onClick);
    try {
      layer.classList.add('in');
      await pause(450);
      layer.classList.add('shake');
      await pause(timing(rarity)[0]);
      layer.classList.remove('shake');
      layer.classList.add('open');
      particles(layer.querySelector('.rv-parts'), R, rarity);
      await pause(timing(rarity)[1]);
    } finally {
      layer.removeEventListener('click', onClick);
      layer.hidden = true;
      layer.className = 'reveal';
      layer.querySelector('.rv-parts').textContent = '';
      slot.textContent = '';
    }
  }

  window.GachaFx = { buildMachine, spin, reveal, skip, RARITY };
})();
