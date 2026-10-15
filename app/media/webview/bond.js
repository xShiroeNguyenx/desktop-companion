// Bảng "💗 Thân thiết" trên pet (Rust: companion.rs → show_bond_card). Chỉ
// hiển thị số liệu Rust gửi sang; đóng bằng ×, Esc, bấm ra ngoài, hoặc tự đóng.
const LABELS = {
  vi: { title: 'Thân thiết', next: 'Lên', max: 'Cấp cao nhất rồi 💕', days: 'ngày quen nhau', streak: 'ngày liên tiếp', marks: 'cột mốc', close: 'Đóng' },
  ja: { title: 'なかよし度', next: '次は', max: '最高レベル 💕', days: '日目', streak: '日連続', marks: 'の記念', close: '閉じる' },
  en: { title: 'Bond', next: 'Next:', max: 'Max level 💕', days: 'days together', streak: 'day streak', marks: 'milestones', close: 'Close' },
};
const AUTO_CLOSE_MS = 20_000;

let card = null;
let timer = 0;

function injectStyle() {
  if (document.getElementById('bond-style')) return;
  const style = document.createElement('style');
  style.id = 'bond-style';
  style.textContent = `
    .bond-card {
      position: absolute;
      left: 50%;
      bottom: 34px;
      width: min(92%, 250px);
      transform: translateX(-50%);
      z-index: 1200;
      padding: 10px 12px 12px;
      border-radius: 14px;
      border: 2px solid rgba(255, 150, 190, 0.6);
      background: linear-gradient(135deg, rgba(255, 228, 240, 0.97), rgba(255, 210, 228, 0.97));
      box-shadow: 0 6px 18px rgba(255, 120, 170, 0.35);
      color: #5b2f47;
      font: 12px/1.4 "Segoe UI", system-ui, sans-serif;
      animation: bond-in 0.35s cubic-bezier(0.34, 1.56, 0.64, 1);
    }
    @keyframes bond-in { from { opacity: 0; transform: translate(-50%, 10px) scale(0.92); } }
    .bond-card .bc-head { display: flex; align-items: center; gap: 6px; font-size: 10.5px; font-weight: 700;
      letter-spacing: 0.05em; text-transform: uppercase; color: #e0458f; }
    .bond-card .bc-close { margin-left: auto; border: none; background: none; color: #9a6a84; cursor: pointer;
      font-size: 14px; line-height: 1; padding: 0 2px; }
    .bond-card .bc-level { margin: 6px 0 2px; text-align: center; font-size: 17px; font-weight: 800; }
    .bond-card .bc-bar { height: 8px; border-radius: 4px; background: rgba(255, 255, 255, 0.7); overflow: hidden; }
    .bond-card .bc-fill { height: 100%; border-radius: 4px; background: linear-gradient(90deg, #ff8fc0, #ff5fa2);
      transition: width 0.6s ease; }
    .bond-card .bc-next { margin-top: 3px; text-align: right; font-size: 10.5px; color: #9a6a84; }
    .bond-card .bc-stats { display: flex; justify-content: space-between; gap: 6px; margin-top: 8px; text-align: center; }
    .bond-card .bc-stat b { display: block; font-size: 15px; }
    .bond-card .bc-stat span { font-size: 10px; color: #9a6a84; }
    @media (prefers-color-scheme: dark) {
      .bond-card { background: linear-gradient(135deg, rgba(90, 50, 75, 0.97), rgba(120, 60, 95, 0.97));
        border-color: rgba(200, 120, 170, 0.45); color: #ffd4e8; }
      .bond-card .bc-bar { background: rgba(40, 20, 32, 0.8); }
      .bond-card .bc-next, .bond-card .bc-stat span, .bond-card .bc-close { color: #d9a9c3; }
    }
  `;
  document.head.appendChild(style);
}

function el(tag, cls, text) {
  const e = document.createElement(tag);
  if (cls) e.className = cls;
  if (text != null) e.textContent = text;
  return e;
}

function close() {
  clearTimeout(timer);
  if (card) card.remove();
  card = null;
  window.removeEventListener('keydown', onKey, true);
  window.removeEventListener('pointerdown', onOutside, true);
}

function onKey(e) {
  if (e.key === 'Escape') close();
}

function onOutside(e) {
  if (card && !card.contains(e.target)) close();
}

function stat(value, label) {
  const s = el('div', 'bc-stat');
  s.append(el('b', '', String(value)), el('span', '', label));
  return s;
}

function show(data) {
  injectStyle();
  close();
  const L = LABELS[data.lang] || LABELS.vi;
  card = el('div', 'bond-card');
  card.setAttribute('role', 'dialog');
  // Bấm trong bảng không bị tính là chọc / kéo pet.
  for (const type of ['pointerdown', 'mousedown', 'click', 'contextmenu']) {
    card.addEventListener(type, (e) => e.stopPropagation());
  }

  const head = el('div', 'bc-head', `💗 ${L.title}${data.pet ? ' · ' + data.pet : ''}`);
  const x = el('button', 'bc-close', '×');
  x.type = 'button';
  x.title = L.close;
  x.addEventListener('click', close);
  head.appendChild(x);

  const level = el('div', 'bc-level', data.levelName || '');
  const bar = el('div', 'bc-bar');
  const fill = el('div', 'bc-fill');
  const need = Number(data.need) || 0;
  const pct = need > 0 ? Math.min(100, Math.round((Number(data.have) || 0) * 100 / need)) : 100;
  fill.style.width = '0%';
  bar.appendChild(fill);
  const next = el('div', 'bc-next', data.nextName ? `${L.next} ${data.nextName} (${data.have}/${need})` : L.max);

  const stats = el('div', 'bc-stats');
  stats.append(stat(data.days, L.days), stat(data.streak, L.streak), stat(data.milestones, L.marks));

  card.append(head, level, bar, next, stats);
  (document.querySelector('.companion-container') || document.body).appendChild(card);
  requestAnimationFrame(() => { fill.style.width = pct + '%'; });

  window.addEventListener('keydown', onKey, true);
  // Đợi qua cú bấm vừa mở bảng rồi mới bắt "bấm ra ngoài".
  setTimeout(() => window.addEventListener('pointerdown', onOutside, true), 0);
  timer = setTimeout(close, AUTO_CLOSE_MS);
}

/** Xử lý lệnh bond:* từ Rust. Trả true nếu đã xử lý. */
export function handleBondMessage(data) {
  if (data && data.command === 'bond:card') {
    show(data);
    return true;
  }
  return false;
}
