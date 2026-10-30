// Khung 🎁 quay miễn phí mỗi ngày, nhỏ ở giữa trên đầu nhân vật (vòng 🍅
// nằm góc phải). Rust (companion.rs offer_free_spin) gửi gacha:offer khi hôm nay
// còn lượt; bấm vào thì gửi gacha:free, Rust quay (gacha::pull_free) và trả
// gacha:result để khung mở capsule ra món trúng.
//
// Cùng khung đó nhắc 🎁 lượt quay quà (gacha::grant_gift, 20 lượt một lần trong
// đời) khi còn lượt và hôm nay không còn lượt free: bấm vào thì mở máy quay trong
// Bộ sưu tập (gacha:gift) — quay quà theo lô 1 / 10 ở đó, không quay trên đầu pet.
// Rust gửi gacha:gift {gift} sau mỗi lần quay quà để đổi số / ẩn khung.
import { vscode, debugLog, state } from './core.js';

const LABELS = {
  vi: {
    free: 'Quay free', title: 'Lượt quay miễn phí hôm nay, bấm để quay!', hide: 'Ẩn',
    fresh: 'Mới!', dup: 'Trùng · +{n} 🍬', view: '🍬 Xem', error: 'Chưa quay được, thử lại sau nha',
    gift: 'Quà hôm nay',
    giftTag: 'Quà · còn {n} lượt', giftTitle: 'Bạn còn {n} lượt quay quà miễn phí, bấm để mở máy quay!',
    rarity: { common: 'Thường', rare: 'Hiếm', epic: 'Sử thi', legendary: 'Huyền thoại' },
  },
  ja: {
    free: '無料ガチャ', title: '今日の無料ガチャ、クリックでまわす！', hide: '閉じる',
    fresh: 'NEW!', dup: 'ダブり · +{n} 🍬', view: '🍬 見る', error: 'まわせませんでした。あとでもう一度',
    gift: '今日のごほうび',
    giftTag: 'プレゼント · 残り{n}回', giftTitle: '無料のプレゼントガチャが{n}回残っているよ、クリックでマシンを開く！',
    rarity: { common: 'ノーマル', rare: 'レア', epic: 'エピック', legendary: 'レジェンド' },
  },
  en: {
    free: 'Free spin', title: "Today's free spin, click to spin!", hide: 'Hide',
    fresh: 'New!', dup: 'Duplicate · +{n} 🍬', view: '🍬 View', error: "Couldn't spin, try again later",
    gift: "Today's gift",
    giftTag: 'Gift · {n} left', giftTitle: 'You have {n} free gift spins, click to open the capsule machine!',
    rarity: { common: 'Common', rare: 'Rare', epic: 'Epic', legendary: 'Legendary' },
  },
};
// Như TAB_ICONS của Bộ sưu tập (js/collection-core.js).
const CAT_ICONS = { skin: '🎨', widget_deco: '🎀', effect: '✨', side_set: '🖼️', miu_set: '🐰', bubble: '💬', flower: '🌸', line_pack: '📜', model: '🎭' };
// Capsule lắc ít nhất chừng này rồi mới mở, kể cả khi Rust trả lời ngay.
const SHAKE_MS = 900;
// Rust không trả lời (lỗi lạ) thì thôi chờ, báo lỗi.
const REPLY_TIMEOUT_MS = 8000;
const RESULT_CLOSE_MS = 8000;
// Số sao trên nhãn hạng và số tia pháo lấp lánh lúc mở thẻ.
const STARS = { common: 1, rare: 2, epic: 3, legendary: 4 };
const BURST = { common: 8, rare: 12, epic: 16, legendary: 22 };
// Tìm đầu: quét mỗi HEAD_STEP điểm ảnh; dải HEAD_BAND (phần chiều cao canvas) dưới
// đỉnh tóc để lấy tâm ngang; đáy khung đè lên tóc HEAD_OVERLAP px; đặt lại mỗi PLACE_MS.
const HEAD_STEP = 2;
const HEAD_BAND = 0.08;
const HEAD_OVERLAP = 6;
const PLACE_MS = 1500;
const EDGE_GAP = 8;
const BURST_GLYPHS = ['✦', '•', '♥', '✧'];

let box = null;
let lang = 'vi';
let spinAt = 0;
let replyTimer = 0;
let closeTimer = 0;
let placeTimer = 0;
// Khung đang nhắc lượt quay quà (không phải lượt free hôm nay).
let giftMode = false;
// Người dùng bấm × trên khung quà: không nhắc lại tới lần mở app sau.
let giftHidden = false;

function L() {
  return LABELS[lang] || LABELS.vi;
}

function injectStyle() {
  if (document.getElementById('daily-gacha-style')) return;
  const style = document.createElement('style');
  style.id = 'daily-gacha-style';
  style.textContent = `
    /* Chỗ mặc định khi chưa tìm được đầu (place() đặt lại): giữa, sát mép trên. "translate" riêng để các
       animation dùng "transform" không làm lệch tâm. */
    .daily-gacha {
      position: absolute;
      top: 4px;
      left: 50%;
      translate: -50% 0;
      z-index: 30;
      display: flex;
      flex-direction: column;
      align-items: center;
      gap: 3px;
      font: 600 10.5px/1.2 "Segoe UI", system-ui, sans-serif;
      color: #5b2f47;
      animation: dg-in 0.4s cubic-bezier(0.34, 1.56, 0.64, 1);
    }
    @keyframes dg-in { from { opacity: 0; transform: scale(0.6); } }
    .daily-gacha .dg-btn {
      position: relative;
      width: 46px;
      height: 46px;
      padding: 0;
      border: 2px solid rgba(255, 150, 190, 0.75);
      border-radius: 50%;
      background: radial-gradient(circle at 35% 30%, #fff, #ffe3ef 60%, #ffc6dd);
      box-shadow: 0 4px 12px rgba(255, 120, 170, 0.4);
      cursor: pointer;
      animation: dg-bob 2.4s ease-in-out infinite;
    }
    .daily-gacha .dg-btn:hover { filter: brightness(1.05); transform: scale(1.06); }
    .daily-gacha .dg-btn:disabled { cursor: default; }
    @keyframes dg-bob { 50% { transform: translateY(-3px); } }
    /* Capsule hai nửa: trên hồng, dưới trắng. */
    .daily-gacha .dg-cap {
      position: absolute;
      left: 50%;
      top: 50%;
      width: 24px;
      height: 24px;
      border-radius: 50%;
      transform: translate(-50%, -50%);
      background: linear-gradient(#ff73ab 0 50%, #fff 50% 100%);
      border: 2px solid #e8558f;
      box-sizing: border-box;
    }
    .daily-gacha .dg-cap::after {
      content: "";
      position: absolute;
      left: 3px;
      top: 3px;
      width: 6px;
      height: 4px;
      border-radius: 50%;
      background: rgba(255, 255, 255, 0.8);
    }
    .daily-gacha .dg-spark {
      position: absolute;
      right: -2px;
      top: -4px;
      font-size: 12px;
      animation: dg-twinkle 1.6s ease-in-out infinite;
    }
    @keyframes dg-twinkle { 50% { opacity: 0.3; transform: scale(0.7); } }
    .daily-gacha .dg-tag {
      padding: 1px 7px;
      border-radius: 999px;
      background: #ff73ab;
      color: #fff;
      white-space: nowrap;
      box-shadow: 0 2px 6px rgba(255, 115, 171, 0.4);
    }
    .daily-gacha .dg-x {
      position: absolute;
      top: -6px;
      left: -6px;
      width: 16px;
      height: 16px;
      padding: 0;
      border: none;
      border-radius: 50%;
      background: rgba(91, 47, 71, 0.7);
      color: #fff;
      font-size: 10px;
      line-height: 16px;
      cursor: pointer;
      opacity: 0;
      transition: opacity 0.15s;
    }
    .daily-gacha:hover .dg-x { opacity: 1; }
    .daily-gacha.spinning .dg-x, .daily-gacha.open .dg-x { display: none; }
    .daily-gacha.spinning .dg-btn { animation: dg-shake 0.3s ease-in-out infinite; }
    @keyframes dg-shake {
      25% { transform: rotate(-14deg) scale(1.05); }
      75% { transform: rotate(14deg) scale(1.05); }
    }
    /* Kết quả: thẻ quà thay cho nút. Màu theo hạng: --dg-c (viền, nhãn), --dg-c2
       (nền nhạt), --dg-glow (quầng sáng, tia sau huy chương). */
    .daily-gacha { --dg-c: #ff7fb3; --dg-c2: #ffe3ef; --dg-glow: rgba(255, 127, 179, 0.45); }
    .daily-gacha.r-rare { --dg-c: #9b6bff; --dg-c2: #ece3ff; --dg-glow: rgba(155, 107, 255, 0.55); }
    .daily-gacha.r-epic { --dg-c: #eea200; --dg-c2: #fff1c9; --dg-glow: rgba(255, 196, 40, 0.65); }
    .daily-gacha.r-legendary { --dg-c: #ff5fa8; --dg-c2: #ffe6f4; --dg-glow: rgba(255, 140, 220, 0.8); }
    .daily-gacha .dg-card {
      display: none;
      position: relative;
      width: 140px;
      margin-top: 8px;
      padding: 15px 10px 9px;
      border-radius: 18px;
      border: 2px solid var(--dg-c);
      background:
        radial-gradient(circle, color-mix(in srgb, var(--dg-c) 16%, transparent) 1px, transparent 1.6px) 0 0 / 9px 9px,
        linear-gradient(165deg, #fff 0%, var(--dg-c2) 60%, #fff 100%);
      box-shadow: 0 8px 22px rgba(120, 40, 90, 0.28), 0 0 18px var(--dg-glow);
      text-align: center;
      cursor: pointer;
      /* Tia sáng (z-index -1) nằm trên nền thẻ, dưới huy chương. */
      isolation: isolate;
      animation: dg-pop 0.5s cubic-bezier(0.34, 1.56, 0.64, 1);
    }
    /* Đường chỉ khâu bên trong viền. */
    .daily-gacha .dg-card::before {
      content: "";
      position: absolute;
      inset: 4px;
      border: 1.5px dashed color-mix(in srgb, var(--dg-c) 45%, transparent);
      border-radius: 14px;
      pointer-events: none;
    }
    @keyframes dg-pop { from { opacity: 0; transform: scale(0.4) rotate(-8deg); } }
    .daily-gacha.open .dg-btn, .daily-gacha.open .dg-tag { display: none; }
    .daily-gacha.open .dg-card { display: block; }
    .daily-gacha .dg-bow {
      position: absolute;
      left: 50%;
      top: -15px;
      transform: translateX(-50%);
      font-size: 22px;
      line-height: 1;
      filter: drop-shadow(0 2px 3px rgba(120, 40, 90, 0.3));
      animation: dg-wiggle 2.6s ease-in-out infinite;
    }
    @keyframes dg-wiggle { 50% { transform: translateX(-50%) rotate(-8deg); } }
    .daily-gacha .dg-corner {
      position: absolute;
      color: var(--dg-c);
      font-size: 11px;
      line-height: 1;
      text-shadow: 0 0 6px var(--dg-glow);
      animation: dg-twinkle 1.8s ease-in-out infinite;
    }
    .daily-gacha .dg-corner.tl { left: -6px; top: -5px; }
    .daily-gacha .dg-corner.tr { right: -6px; top: -5px; animation-delay: -0.45s; }
    .daily-gacha .dg-corner.bl { left: -6px; bottom: -5px; animation-delay: -0.9s; }
    .daily-gacha .dg-corner.br { right: -6px; bottom: -5px; animation-delay: -1.35s; }
    .daily-gacha .dg-head {
      color: var(--dg-c);
      font-size: 9.5px;
      letter-spacing: 0.04em;
      white-space: nowrap;
    }
    /* Huy chương tròn giữ biểu tượng món, tia sáng xoay phía sau. */
    .daily-gacha .dg-medal {
      position: relative;
      width: 48px;
      height: 48px;
      margin: 7px auto 6px;
      border-radius: 50%;
      border: 2px solid var(--dg-c);
      background: radial-gradient(circle at 35% 30%, #fff, var(--dg-c2) 75%);
      box-shadow: 0 0 12px var(--dg-glow), inset 0 -3px 6px color-mix(in srgb, var(--dg-c) 18%, transparent);
    }
    .daily-gacha .dg-rays {
      position: absolute;
      inset: -14px;
      z-index: -1;
      border-radius: 50%;
      background: repeating-conic-gradient(var(--dg-glow) 0 9deg, transparent 9deg 30deg);
      -webkit-mask: radial-gradient(circle, #000 35%, transparent 70%);
      mask: radial-gradient(circle, #000 35%, transparent 70%);
      animation: dg-spin 9s linear infinite;
    }
    @keyframes dg-spin { to { transform: rotate(360deg); } }
    .daily-gacha .dg-icon {
      position: relative;
      display: block;
      font-size: 25px;
      line-height: 44px;
      animation: dg-float 2.2s ease-in-out infinite;
    }
    @keyframes dg-float { 50% { transform: translateY(-2px) scale(1.05); } }
    .daily-gacha .dg-rarity {
      display: inline-block;
      padding: 1px 8px;
      border-radius: 999px;
      background: var(--dg-c);
      color: #fff;
      font-size: 8.5px;
      letter-spacing: 0.03em;
      text-transform: uppercase;
      white-space: nowrap;
      box-shadow: 0 2px 6px var(--dg-glow);
    }
    .daily-gacha .dg-name { margin: 5px 0 1px; font-size: 13px; font-weight: 700; color: #5b2f47; word-break: break-word; }
    .daily-gacha .dg-note { font-weight: 500; color: #9a6a84; }
    .daily-gacha .dg-note.fresh { color: var(--dg-c); font-weight: 800; letter-spacing: 0.04em; }
    .daily-gacha .dg-view {
      position: relative;
      overflow: hidden;
      margin-top: 6px;
      padding: 3px 12px;
      border: none;
      border-radius: 999px;
      background: linear-gradient(135deg, #ff9ac8, #ff5fa2);
      color: #fff;
      font: inherit;
      cursor: pointer;
      box-shadow: 0 3px 8px rgba(255, 95, 162, 0.4), inset 0 1px 0 rgba(255, 255, 255, 0.5);
    }
    .daily-gacha .dg-view:hover { filter: brightness(1.07); }
    .daily-gacha .dg-view::after {
      content: "";
      position: absolute;
      top: 0;
      bottom: 0;
      left: -40%;
      width: 30%;
      background: linear-gradient(90deg, transparent, rgba(255, 255, 255, 0.65), transparent);
      transform: skewX(-20deg);
      animation: dg-glint 2.6s ease-in-out 0.8s infinite;
    }
    @keyframes dg-glint { 0%, 60% { left: -40%; } 100% { left: 130%; } }
    /* Ánh sáng quét ngang thẻ một lần lúc mở. */
    .daily-gacha .dg-shine {
      position: absolute;
      inset: 0;
      overflow: hidden;
      border-radius: inherit;
      pointer-events: none;
    }
    .daily-gacha .dg-shine::after {
      content: "";
      position: absolute;
      top: -10%;
      bottom: -10%;
      left: -60%;
      width: 45%;
      background: linear-gradient(90deg, transparent, rgba(255, 255, 255, 0.8), transparent);
      transform: skewX(-20deg);
      animation: dg-sweep 1.1s ease-out 0.35s both;
    }
    @keyframes dg-sweep { to { left: 130%; } }
    /* Pháo lấp lánh bung ra từ huy chương. */
    .daily-gacha .dg-burst {
      position: absolute;
      left: 50%;
      top: 62px;
      z-index: 2;
      color: var(--dg-c);
      font-size: 11px;
      line-height: 1;
      pointer-events: none;
      text-shadow: 0 0 5px var(--dg-glow);
      animation: dg-fly 0.95s cubic-bezier(0.2, 0.7, 0.3, 1) both;
    }
    @keyframes dg-fly {
      from { opacity: 1; transform: translate(-50%, -50%) scale(0.4); }
      70% { opacity: 1; }
      to { opacity: 0; transform: translate(calc(-50% + var(--dx)), calc(-50% + var(--dy))) scale(1.1) rotate(90deg); }
    }
    .daily-gacha .dg-err { max-width: 120px; text-align: center; font-weight: 500; color: #c43d5c; }
    .daily-gacha .dg-err:empty { display: none; }
    /* Huyền thoại: viền và tia cầu vồng chạy, pháo nhiều màu. */
    .daily-gacha.r-legendary .dg-card {
      border-color: transparent;
      background:
        radial-gradient(circle, rgba(255, 95, 168, 0.16) 1px, transparent 1.6px) 0 0 / 9px 9px padding-box,
        linear-gradient(165deg, #fff 0%, #ffe6f4 45%, #e9f2ff 75%, #fff 100%) padding-box,
        linear-gradient(90deg, #ff6fb2, #ffcf5a, #6fe3a0, #6fb8ff, #b77bff, #ff6fb2) 0 0 / 300% 100% border-box;
      animation: dg-pop 0.5s cubic-bezier(0.34, 1.56, 0.64, 1), dg-rainbow 3s linear infinite;
    }
    @keyframes dg-rainbow { to { background-position: 0 0, 0 0, 300% 0; } }
    .daily-gacha.r-legendary .dg-rays {
      background: repeating-conic-gradient(#ff8cc6 0 8deg, transparent 8deg 20deg, #ffd66e 20deg 28deg, transparent 28deg 40deg, #8fd0ff 40deg 48deg, transparent 48deg 60deg);
    }
    .daily-gacha.r-legendary .dg-rarity {
      background: linear-gradient(90deg, #ff6fb2, #f0a800, #2fb58a, #4f8dff, #9b6bff);
    }
    .daily-gacha.r-legendary .dg-burst:nth-of-type(4n + 1) { color: #ffb800; }
    .daily-gacha.r-legendary .dg-burst:nth-of-type(4n + 2) { color: #4fb8ff; }
    .daily-gacha.r-legendary .dg-burst:nth-of-type(4n + 3) { color: #2fc58a; }
    @media (prefers-reduced-motion: reduce) {
      .daily-gacha, .daily-gacha * { animation: none !important; }
      .daily-gacha .dg-burst, .daily-gacha .dg-shine { display: none; }
    }
    @media (prefers-color-scheme: dark) {
      .daily-gacha { color: #ffd4e8; }
      .daily-gacha .dg-card {
        background:
          radial-gradient(circle, color-mix(in srgb, var(--dg-c) 22%, transparent) 1px, transparent 1.6px) 0 0 / 9px 9px,
          linear-gradient(165deg, #4b2a41 0%, color-mix(in srgb, var(--dg-c) 28%, #3a2033) 60%, #4b2a41 100%);
      }
      .daily-gacha.r-legendary .dg-card {
        background:
          radial-gradient(circle, rgba(255, 95, 168, 0.22) 1px, transparent 1.6px) 0 0 / 9px 9px padding-box,
          linear-gradient(165deg, #4b2a41 0%, #5c2f4f 45%, #34365a 75%, #4b2a41 100%) padding-box,
          linear-gradient(90deg, #ff6fb2, #ffcf5a, #6fe3a0, #6fb8ff, #b77bff, #ff6fb2) 0 0 / 300% 100% border-box;
      }
      .daily-gacha .dg-medal { background: radial-gradient(circle at 35% 30%, #fff6fb, var(--dg-c2) 80%); }
      .daily-gacha .dg-head { color: color-mix(in srgb, var(--dg-c) 60%, #fff); }
      .daily-gacha .dg-name { color: #ffe6f2; }
      .daily-gacha .dg-note { color: #d9a9c3; }
      .daily-gacha .dg-note.fresh { color: color-mix(in srgb, var(--dg-c) 55%, #fff); }
      .daily-gacha .dg-shine::after { background: linear-gradient(90deg, transparent, rgba(255, 255, 255, 0.35), transparent); }
      .daily-gacha .dg-err { color: #ff9db5; }
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

// Khung nằm trên pet: chặn để bấm vào không bị tính là chọc / kéo pet.
function isolate(node) {
  for (const type of ['pointerdown', 'mousedown', 'mouseup', 'click', 'dblclick', 'contextmenu', 'wheel']) {
    node.addEventListener(type, (e) => e.stopPropagation());
  }
}

function close() {
  clearTimeout(replyTimer);
  clearTimeout(closeTimer);
  clearInterval(placeTimer);
  window.removeEventListener('resize', place);
  if (box) box.remove();
  box = null;
}

// Đỉnh đầu nhân vật trong `host` (px CSS), tìm như capturePinAvatar
// (main.js): hàng trên cùng có điểm ảnh không trong suốt trên canvas Live2D (ảnh
// model có lề trong suốt nên khung bao của model không dùng được), tâm ngang là
// trung bình các điểm có màu ở dải ngay dưới đó. Chưa vẽ xong model thì null.
function headAnchor(host) {
  const canvas = document.getElementById('live2dCanvas');
  if (!state.isLive2DReady || !canvas || !canvas.width || !canvas.height || !canvas.clientWidth) return null;
  if (state.app && typeof state.app.render === 'function') state.app.render();
  const w = canvas.width;
  const h = canvas.height;
  const tmp = document.createElement('canvas');
  tmp.width = w;
  tmp.height = h;
  const ctx = tmp.getContext('2d', { willReadFrequently: true });
  if (!ctx) return null;
  ctx.drawImage(canvas, 0, 0);
  const data = ctx.getImageData(0, 0, w, h).data;
  const opaque = (x, y) => data[(y * w + x) * 4 + 3] > 8;
  let top = -1;
  for (let y = 0; y < h && top < 0; y += HEAD_STEP) {
    for (let x = 0; x < w; x += HEAD_STEP) {
      if (opaque(x, y)) { top = y; break; }
    }
  }
  if (top < 0) return null;
  let sum = 0;
  let n = 0;
  for (let y = top; y < Math.min(h, top + Math.round(h * HEAD_BAND)); y += HEAD_STEP) {
    for (let x = 0; x < w; x += HEAD_STEP) {
      if (opaque(x, y)) { sum += x; n++; }
    }
  }
  const cr = canvas.getBoundingClientRect();
  const hr = host.getBoundingClientRect();
  return {
    x: cr.left - hr.left + (n ? sum / n : w / 2) * (cr.width / w),
    y: cr.top - hr.top + top * (cr.height / h),
  };
}

// Đặt khung nằm trên đầu: đáy khung chạm đỉnh tóc, giữa theo tâm đầu. Không đủ
// chỗ phía trên (cửa sổ thấp, thẻ quà cao) thì dừng ở mép trên. Chưa tìm được
// đầu thì giữ chỗ mặc định trong CSS (giữa, sát mép trên).
function place() {
  if (!box || !box.parentElement) return;
  const host = box.parentElement;
  const head = headAnchor(host);
  if (!head) return;
  // Chừa lề cho ✦ ở góc thẻ và quầng sáng.
  const half = box.offsetWidth / 2 + EDGE_GAP;
  const x = Math.max(half, Math.min(host.clientWidth - half, head.x));
  const y = Math.max(0, head.y + HEAD_OVERLAP - box.offsetHeight);
  box.style.left = `${Math.round(x)}px`;
  box.style.top = `${Math.round(y)}px`;
}

function showError(text) {
  if (!box) return;
  box.classList.remove('spinning');
  box.querySelector('.dg-btn').disabled = false;
  box.querySelector('.dg-err').textContent = text;
}

function spin() {
  if (!box || box.classList.contains('spinning')) return;
  box.querySelector('.dg-err').textContent = '';
  box.classList.add('spinning');
  box.querySelector('.dg-btn').disabled = true;
  spinAt = Date.now();
  clearTimeout(replyTimer);
  replyTimer = setTimeout(() => showError(L().error), REPLY_TIMEOUT_MS);
  vscode.postMessage({ command: 'gacha:free' });
}

// Khung quà: bấm là mở máy quay trong Bộ sưu tập (Rust: pet_command gacha:gift).
function openGiftMachine() {
  vscode.postMessage({ command: 'gacha:gift' });
}

/** `data.gift` > 0 và không phải lượt free hôm nay (`data.giftMode`) → khung nhắc lượt quay quà. */
function offer(data) {
  injectStyle();
  close();
  if (['vi', 'ja', 'en'].includes(data.lang)) lang = data.lang;
  const S = L();
  giftMode = !!data.giftMode;
  const n = String(data.gift || 0);
  const title = giftMode ? S.giftTitle.replace('{n}', n) : S.title;
  box = el('div', 'daily-gacha');
  const btn = el('button', 'dg-btn');
  btn.type = 'button';
  btn.title = title;
  btn.setAttribute('aria-label', title);
  btn.append(el('span', 'dg-cap'), el('span', 'dg-spark', '✨'));
  btn.addEventListener('click', giftMode ? openGiftMachine : spin);
  // Ẩn tới lần mở app sau (lượt vẫn còn, tick không đưa lại trong ngày).
  const x = el('button', 'dg-x', '×');
  x.type = 'button';
  x.title = S.hide;
  x.setAttribute('aria-label', S.hide);
  x.addEventListener('click', () => {
    if (giftMode) giftHidden = true;
    close();
  });
  const tag = el('span', 'dg-tag', giftMode ? S.giftTag.replace('{n}', n) : S.free);
  box.append(x, btn, tag, el('div', 'dg-card'), el('div', 'dg-err'));
  isolate(box);
  // Cả cửa sổ pet, không chỉ vùng nhân vật: thẻ quà cao dùng được cả khoảng trống
  // phía trên đầu (chỗ bong bóng thoại) để khỏi che mặt.
  const host = document.querySelector('.companion-container') || document.getElementById('characterWrapper') || document.body;
  host.appendChild(box);
  place();
  // Model có thể đổi cỡ, đổi model, vào giữa bảng Cài đặt: đặt lại thường xuyên.
  placeTimer = setInterval(place, PLACE_MS);
  window.addEventListener('resize', place);
}

// Pháo lấp lánh bung từ huy chương ra mọi hướng; hạng càng cao càng nhiều.
function burst(rarity) {
  const n = BURST[rarity] || BURST.common;
  for (let i = 0; i < n; i++) {
    const p = el('span', 'dg-burst', BURST_GLYPHS[i % BURST_GLYPHS.length]);
    const angle = (Math.PI * 2 * i) / n + Math.random() * 0.4;
    const dist = 48 + Math.random() * 36;
    p.style.setProperty('--dx', `${Math.round(Math.cos(angle) * dist)}px`);
    p.style.setProperty('--dy', `${Math.round(Math.sin(angle) * dist)}px`);
    p.style.animationDelay = `${Math.round(Math.random() * 140)}ms`;
    box.appendChild(p);
    setTimeout(() => p.remove(), 1300);
  }
}

function reveal(data) {
  if (!box) return;
  const S = L();
  const rarity = S.rarity[data.rarity] ? data.rarity : 'common';
  box.classList.remove('spinning');
  box.classList.add('open', 'r-' + rarity);
  const card = box.querySelector('.dg-card');
  const medal = el('div', 'dg-medal');
  medal.append(el('span', 'dg-rays'), el('span', 'dg-icon', CAT_ICONS[data.cat] || '🎁'));
  card.replaceChildren(
    el('span', 'dg-shine'),
    el('span', 'dg-bow', '🎀'),
    ...['tl', 'tr', 'bl', 'br'].map((c) => el('span', 'dg-corner ' + c, '✦')),
    el('div', 'dg-head', `✿ ${S.gift} ✿`),
    medal,
    el('div', 'dg-rarity', `${'★'.repeat(STARS[rarity])} ${S.rarity[rarity]}`),
    el('div', 'dg-name', data.name || ''),
    data.dup ? el('div', 'dg-note', S.dup.replace('{n}', String(data.refund || 0))) : el('div', 'dg-note fresh', S.fresh),
  );
  burst(rarity);
  const view = el('button', 'dg-view', S.view);
  view.type = 'button';
  view.addEventListener('click', (e) => {
    e.stopPropagation();
    vscode.postMessage({ command: 'gacha:view', id: data.id || '' });
    closeThenRecheck();
  });
  card.appendChild(view);
  card.addEventListener('click', closeThenRecheck);
  place();
  closeTimer = setTimeout(closeThenRecheck, RESULT_CLOSE_MS);
}

// Đóng thẻ kết quả lượt free, rồi hỏi lại: còn lượt quay quà thì khung quà hiện tiếp.
function closeThenRecheck() {
  close();
  setTimeout(initDailyGacha, 400);
}

function result(data) {
  clearTimeout(replyTimer);
  if (!box) return;
  if (data.error === 'used_today') {
    // Đã quay ở nơi khác (hay đồng hồ đổi ngày): hết lượt free — còn quà thì nhắc quà.
    closeThenRecheck();
    return;
  }
  if (data.error) {
    showError(L().error);
    return;
  }
  // Cho capsule lắc đủ lâu rồi mới mở.
  setTimeout(() => reveal(data), Math.max(0, SHAKE_MS - (Date.now() - spinAt)));
}

/** Xử lý lệnh gacha:* từ Rust. Trả true nếu đã xử lý. */
export function handleDailyGachaMessage(data) {
  try {
    switch (data && data.command) {
      case 'gacha:offer':
        // Đang mở kết quả thì để yên (lệnh tới từ tick ngày mới hiếm khi trùng lúc).
        if (!box || !box.classList.contains('open')) offer(data);
        return true;
      case 'gacha:gift':
        // Vừa quay quà trong Bộ sưu tập: đổi số lượt trên khung quà, hết thì ẩn.
        if (box && giftMode) {
          if ((data.gift || 0) > 0) offer({ lang: data.lang, giftMode: true, gift: data.gift });
          else close();
        }
        return true;
      case 'gacha:result':
        result(data);
        return true;
      default:
        return false;
    }
  } catch (err) {
    debugLog('daily gacha failed: ' + (err && err.stack ? err.stack : err));
    return true;
  }
}

/** Trang pet vừa nạp (lại): hỏi Rust hôm nay còn lượt free không (khung cũ mất theo trang). */
export function initDailyGacha() {
  const T = window.__TAURI__ || {};
  const invoke = (T.core && T.core.invoke) || T.invoke;
  if (!invoke) return;
  invoke('gacha_free_status')
    .then((s) => {
      if (!s) return;
      // Lượt free hôm nay trước (quay ngay trên đầu pet); hết thì nhắc lượt quay quà.
      if (s.ready) handleDailyGachaMessage({ command: 'gacha:offer', lang: s.lang });
      else if (s.gift > 0 && !giftHidden) handleDailyGachaMessage({ command: 'gacha:offer', lang: s.lang, giftMode: true, gift: s.gift });
    })
    .catch((err) => debugLog('gacha_free_status failed: ' + err));
}
