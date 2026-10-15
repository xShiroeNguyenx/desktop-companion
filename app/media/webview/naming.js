// Thẻ "đặt tên cho em" và "em nên gọi bạn là gì" hiện ngay trên pet
// (Rust: src-tauri/src/naming.rs). Một thẻ tại một thời điểm; Rust trả
// naming:done để đóng thẻ, hoặc naming:error để hiện lỗi mà vẫn giữ thẻ.
import { vscode } from './core.js';

const LABELS = {
  vi: { name: 'Tên của em…', ok: 'Đặt tên', later: 'Để sau', you: 'Tên của bạn…', save: 'OK', more: 'Cách khác: mở Cài đặt' },
  ja: { name: '名前を入力…', ok: '決定', later: 'あとで', you: 'あなたの名前…', save: 'OK', more: 'ほかの呼び方は設定で' },
  en: { name: 'Type a name…', ok: 'Name me', later: 'Later', you: 'Your name…', save: 'OK', more: 'More options in Settings' },
};
const NAME_MAX = 20;
// Không ai bấm gì thì thẻ tự đóng (coi như bỏ qua, không nói gì thêm).
const IDLE_CLOSE_MS = 90_000;

let card = null;
let idleTimer = 0;

function injectStyle() {
  if (document.getElementById('naming-style')) return;
  const style = document.createElement('style');
  style.id = 'naming-style';
  style.textContent = `
    .naming-card {
      position: absolute;
      left: 50%;
      bottom: 34px;
      width: min(92%, 260px);
      transform: translateX(-50%);
      z-index: 1200;
      padding: 10px 12px;
      border-radius: 14px;
      border: 2px solid rgba(255, 150, 190, 0.6);
      background: linear-gradient(135deg, rgba(255, 228, 240, 0.97), rgba(255, 210, 228, 0.97));
      box-shadow: 0 6px 18px rgba(255, 120, 170, 0.35);
      color: #5b2f47;
      font: 12.5px/1.4 "Segoe UI", system-ui, sans-serif;
      animation: naming-in 0.35s cubic-bezier(0.34, 1.56, 0.64, 1);
    }
    @keyframes naming-in { from { opacity: 0; transform: translate(-50%, 10px) scale(0.92); } }
    .naming-card .nc-text { margin-bottom: 8px; text-align: center; font-weight: 600; }
    .naming-card .nc-row { display: flex; gap: 6px; }
    .naming-card input {
      flex: 1; min-width: 0;
      padding: 5px 8px;
      border-radius: 8px;
      border: 1px solid rgba(232, 110, 160, 0.5);
      background: #fff;
      color: #5b2f47;
      font: inherit;
    }
    .naming-card input:focus { outline: 2px solid rgba(255, 115, 171, 0.45); outline-offset: 0; }
    .naming-card button {
      border: none; border-radius: 8px; padding: 5px 10px;
      font: inherit; font-weight: 600; cursor: pointer;
    }
    .naming-card .nc-ok { background: #ff73ab; color: #fff; }
    .naming-card .nc-ok:hover { background: #f05a97; }
    .naming-card .nc-pairs { display: flex; flex-wrap: wrap; gap: 5px; justify-content: center; }
    .naming-card .nc-pair { background: #fff; color: #5b2f47; border: 1px solid rgba(232, 110, 160, 0.45); }
    .naming-card .nc-pair:hover { background: #ffe3ef; }
    .naming-card .nc-foot { display: flex; justify-content: space-between; align-items: center; margin-top: 6px; gap: 6px; }
    .naming-card .nc-later { background: none; color: #9a6a84; padding: 2px 4px; font-weight: 500; }
    .naming-card .nc-later:hover { color: #5b2f47; }
    .naming-card .nc-more { font-size: 10.5px; color: #9a6a84; }
    .naming-card .nc-err { margin-top: 5px; font-size: 11px; color: #c43d5c; }
    .naming-card .nc-err:empty { display: none; }
    @media (prefers-color-scheme: dark) {
      .naming-card {
        background: linear-gradient(135deg, rgba(90, 50, 75, 0.97), rgba(120, 60, 95, 0.97));
        border-color: rgba(200, 120, 170, 0.45);
        color: #ffd4e8;
      }
      .naming-card input, .naming-card .nc-pair { background: rgba(40, 20, 32, 0.8); color: #ffd4e8; }
      .naming-card .nc-pair:hover { background: rgba(70, 35, 55, 0.9); }
      .naming-card .nc-later, .naming-card .nc-more { color: #d9a9c3; }
      .naming-card .nc-err { color: #ff9db5; }
    }
  `;
  document.head.appendChild(style);
}

function labels(lang) {
  return LABELS[lang] || LABELS.vi;
}

function close() {
  clearTimeout(idleTimer);
  if (card) card.remove();
  card = null;
}

function el(tag, cls, text) {
  const e = document.createElement(tag);
  if (cls) e.className = cls;
  if (text != null) e.textContent = text;
  return e;
}

// Thẻ nằm trên pet: chặn để bấm/gõ trong thẻ không bị tính là chọc, kéo pet.
function isolate(node) {
  for (const type of ['pointerdown', 'mousedown', 'click', 'dblclick', 'contextmenu', 'wheel']) {
    node.addEventListener(type, (e) => e.stopPropagation());
  }
}

function open(text) {
  injectStyle();
  close();
  card = el('div', 'naming-card');
  card.setAttribute('role', 'dialog');
  card.appendChild(el('div', 'nc-text', text || ''));
  isolate(card);
  const host = document.querySelector('.companion-container') || document.body;
  host.appendChild(card);
  idleTimer = setTimeout(close, IDLE_CLOSE_MS);
  return card;
}

function showError(message) {
  if (!card) return;
  let err = card.querySelector('.nc-err');
  if (!err) {
    err = el('div', 'nc-err');
    card.appendChild(err);
  }
  err.textContent = message || '';
}

// Ô nhập + nút xác nhận; Enter cũng gửi, Esc đóng.
function inputRow(placeholder, okLabel, initial, onSubmit) {
  const row = el('div', 'nc-row');
  const input = el('input');
  input.type = 'text';
  input.maxLength = NAME_MAX;
  input.placeholder = placeholder;
  input.value = initial || '';
  input.setAttribute('aria-label', placeholder);
  const ok = el('button', 'nc-ok', okLabel);
  ok.type = 'button';
  const submit = () => {
    const v = input.value.trim();
    if (!v) { input.focus(); return; }
    showError('');
    onSubmit(v);
  };
  ok.addEventListener('click', submit);
  input.addEventListener('keydown', (e) => {
    if (e.key === 'Enter') { e.preventDefault(); submit(); }
    else if (e.key === 'Escape') { e.preventDefault(); close(); }
  });
  input.addEventListener('input', () => {
    clearTimeout(idleTimer);
    idleTimer = setTimeout(close, IDLE_CLOSE_MS);
  });
  row.append(input, ok);
  setTimeout(() => input.focus(), 60);
  return row;
}

function askName(data) {
  const L = labels(data.lang);
  const c = open(data.text);
  c.appendChild(inputRow(L.name, L.ok, '', (name) => {
    vscode.postMessage({ command: 'naming:setName', name });
  }));
  const foot = el('div', 'nc-foot');
  const later = el('button', 'nc-later', L.later);
  later.type = 'button';
  later.addEventListener('click', () => {
    vscode.postMessage({ command: 'naming:later' });
    close();
  });
  foot.appendChild(later);
  c.appendChild(foot);
}

function askAddress(data) {
  const L = labels(data.lang);
  const c = open(data.text);
  const pairs = Array.isArray(data.pairs) ? data.pairs : [];
  if (pairs.length) {
    // Tiếng Việt: chọn một cặp xưng hô.
    const box = el('div', 'nc-pairs');
    for (const p of pairs) {
      const b = el('button', 'nc-pair', `${p.me} – ${p.you}`);
      b.type = 'button';
      b.addEventListener('click', () => {
        vscode.postMessage({ command: 'naming:setAddress', me: p.me, you: p.you });
      });
      box.appendChild(b);
    }
    c.appendChild(box);
  } else {
    // Tiếng Nhật / Anh: hỏi tên người dùng.
    c.appendChild(inputRow(L.you, L.save, data.userName || '', (userName) => {
      vscode.postMessage({ command: 'naming:setAddress', userName });
    }));
  }
  const foot = el('div', 'nc-foot');
  foot.appendChild(el('span', 'nc-more', L.more));
  const later = el('button', 'nc-later', L.later);
  later.type = 'button';
  later.addEventListener('click', close);
  foot.appendChild(later);
  c.appendChild(foot);
}

/** Xử lý lệnh naming:* từ Rust. Trả true nếu đã xử lý. */
export function handleNamingMessage(data) {
  switch (data && data.command) {
    case 'naming:askName':
      askName(data);
      return true;
    case 'naming:askAddress':
      askAddress(data);
      return true;
    case 'naming:done':
      close();
      return true;
    case 'naming:error':
      showError(data.message);
      return true;
    default:
      return false;
  }
}
