// Hiệu ứng thưởng trên pet, mở trong Bộ sưu tập (collection.rs → pet-message
// {command: 'fx', kind}): 'confetti' khi xong phiên Tập trung, 'fireworks' khi
// xong hết việc trong ngày. Vẽ bằng span trong #particles như createSparkle
// (ui.js), tự xoá sau khi chạy; prefers-reduced-motion thì chỉ vài hạt.

const COLORS = ['#ff73ab', '#ffd27a', '#8da2ff', '#57b894', '#ff9ec7', '#ffffff'];

function host() {
  return document.getElementById('particles');
}

function reduced() {
  return !!(window.matchMedia && window.matchMedia('(prefers-reduced-motion: reduce)').matches);
}

function drop(el, ms) {
  setTimeout(() => el.remove(), ms);
}

/** Pháo giấy rơi từ mép trên xuống hết cửa sổ pet. */
function confetti(root) {
  const n = reduced() ? 8 : 40;
  for (let i = 0; i < n; i++) {
    const s = document.createElement('span');
    s.className = 'fx-confetti';
    s.style.left = (5 + Math.random() * 90) + '%';
    s.style.background = COLORS[i % COLORS.length];
    s.style.animationDelay = (Math.random() * 0.5).toFixed(2) + 's';
    s.style.animationDuration = (1.4 + Math.random() * 0.8).toFixed(2) + 's';
    s.style.setProperty('--dx', (Math.random() * 60 - 30).toFixed(0) + 'px');
    s.style.setProperty('--rot', (Math.random() * 720 - 360).toFixed(0) + 'deg');
    root.appendChild(s);
    drop(s, 2800);
  }
}

/** Ba chùm pháo hoa nổ lệch nhau, mỗi chùm 14 tia bay toả tròn. */
function fireworks(root) {
  const bursts = reduced() ? 1 : 3;
  for (let b = 0; b < bursts; b++) {
    setTimeout(() => {
      const cx = 20 + Math.random() * 60;
      const cy = 15 + Math.random() * 40;
      const color = COLORS[b % COLORS.length];
      for (let i = 0; i < 14; i++) {
        const s = document.createElement('span');
        s.className = 'fx-spark';
        s.style.left = cx + '%';
        s.style.top = cy + '%';
        s.style.background = color;
        s.style.color = color;
        const a = (i / 14) * Math.PI * 2;
        const r = 40 + Math.random() * 30;
        s.style.setProperty('--dx', (Math.cos(a) * r).toFixed(0) + 'px');
        s.style.setProperty('--dy', (Math.sin(a) * r).toFixed(0) + 'px');
        root.appendChild(s);
        drop(s, 1300);
      }
    }, b * 350);
  }
}

/** Cánh hoa anh đào chao lượn rơi xuống chậm. */
function petals(root) {
  const n = reduced() ? 6 : 26;
  for (let i = 0; i < n; i++) {
    const s = document.createElement('span');
    s.className = 'fx-petal';
    s.style.left = (Math.random() * 100) + '%';
    s.style.animationDelay = (Math.random() * 1.2).toFixed(2) + 's';
    s.style.animationDuration = (2.4 + Math.random() * 1.4).toFixed(2) + 's';
    s.style.setProperty('--dx', (Math.random() * 120 - 60).toFixed(0) + 'px');
    s.style.setProperty('--rot', (Math.random() * 540 - 270).toFixed(0) + 'deg');
    s.style.transform = 'scale(' + (0.7 + Math.random() * 0.6).toFixed(2) + ')';
    root.appendChild(s);
    drop(s, 4000);
  }
}

/** @param {'confetti'|'fireworks'|'petals'} kind */
export function playFx(kind) {
  const root = host();
  if (!root) return;
  if (kind === 'confetti') confetti(root);
  else if (kind === 'fireworks') fireworks(root);
  else if (kind === 'petals') petals(root);
}
