// ❓ Hướng dẫn nhanh (Rust: src-tauri/src/tour.rs): thẻ vài bước ngay trên pet, tự
// mở một lần cho người mới cài, mở lại từ tray → ❓. Thẻ báo hộp của mình cho Rust
// (`tour:rect`) để bấm được cả lúc click-through, như khung 🎁 (daily-gacha.js).
import { vscode } from './core.js';

export const LABELS = {
  vi: {
    title: 'Hướng dẫn nhanh', skip: 'Bỏ qua', prev: '← Trước', next: 'Tiếp →', done: 'Xong ✓',
    settings: '⚙️ Mở Cài đặt', pet: 'em',
    steps: [
      ['🌸 Bôi đen là có hoa', 'Bôi đen chữ ở bất kỳ app nào (hoặc nhấn {hotkey}) → bông hoa hiện cạnh con trỏ. Bấm để dịch, nhấn giữ để mở menu: trả lời, hỏi {pet}, lưu task, ghi chú.'],
      ['💞 Chuột phải vào {pet}', 'Mở menu tròn: Chat nhanh, Công việc, Ghi chú, widget, mini game, Bộ sưu tập 🍬… Kéo {pet} đi đâu cũng được, giữ Ctrl + lăn chuột để đổi cỡ.'],
      ['🖱️ Bấm xuyên qua {pet}', 'Bật click-through ở biểu tượng khay: chuột đi xuyên qua {pet} xuống app phía sau. Lúc đó giữ Ctrl + chuột phải vào {pet} vẫn mở được menu.'],
      ['🔊 {pet} nói được thành tiếng', 'Lời chào, lời nhắc nghỉ, chuyện thời tiết… {pet} nói bằng tiếng Việt. Đổi giọng (hai giọng Việt, Nhật, Anh) hay tắt tiếng: chuột phải vào {pet} → Âm thanh.'],
      ['🔑 Trò chuyện, dịch bằng AI', 'Nhập API key của Anthropic (Claude) hoặc Google (Gemini) ở Cài đặt → 🤖 Trí tuệ nhân tạo. Key cất trong két bảo mật của Windows, chỉ gửi tới nhà cung cấp bạn chọn.', true],
      ['✨ Còn nhiều thứ đang tắt sẵn', 'Hiệu ứng thời tiết, Bunny Lounge, cảnh báo CPU / RAM, lịch, đồng hồ, bể cá… bật trong Cài đặt khi bạn muốn. Mở lại hướng dẫn này ở biểu tượng khay → ❓.', true],
    ],
  },
  ja: {
    title: 'かんたんガイド', skip: 'スキップ', prev: '← 前へ', next: '次へ →', done: '完了 ✓',
    settings: '⚙️ 設定を開く', pet: 'ペット',
    steps: [
      ['🌸 選択するとお花', 'どのアプリでもテキストを選択（または {hotkey}）→ カーソルの横にお花が出ます。クリックで翻訳、長押しでメニュー：返信・{pet}に質問・タスク保存・メモ。'],
      ['💞 {pet}を右クリック', '円形メニュー：クイックチャット・タスク・メモ・ウィジェット・ミニゲーム・コレクション🍬… {pet}はドラッグで好きな場所へ、Ctrl + ホイールで大きさを変えられます。'],
      ['🖱️ {pet}を素通りクリック', 'トレイからクリック透過をオンにすると、マウスが{pet}を通り抜けて後ろのアプリに届きます。そのときも Ctrl + 右クリックでメニューを開けます。'],
      ['🔊 {pet}は声でも話します', 'あいさつ・休憩のお知らせ・天気のひとことを声に出します。いまはベトナム語だけなので、聞くには文字の言語と声をベトナム語にしてください。声の切り替えとミュートは、{pet}を右クリック → サウンド。'],
      ['🔑 AI でおしゃべり・翻訳', '設定 → 🤖 AI で Anthropic（Claude）か Google（Gemini）の API キーを入力してください。キーは Windows の資格情報マネージャーに保存され、選んだプロバイダーにだけ送られます。', true],
      ['✨ オフになっている機能も', '天気エフェクト・Bunny Lounge・CPU／RAM の警告・カレンダー・時計・水槽… 使いたくなったら設定でオンにしてね。このガイドはトレイ → ❓ でまた開けます。', true],
    ],
  },
  en: {
    title: 'Quick tour', skip: 'Skip', prev: '← Back', next: 'Next →', done: 'Done ✓',
    settings: '⚙️ Open Settings', pet: 'me',
    steps: [
      ['🌸 Select text, get a flower', 'Select text in any app (or press {hotkey}) and a flower appears by the cursor. Click it to translate, press and hold for more: reply, ask {pet}, save as task or note.'],
      ['💞 Right-click {pet}', 'Opens the ring menu: quick chat, tasks, notes, widgets, mini games, the 🍬 Collection… Drag {pet} anywhere, and hold Ctrl + scroll to resize.'],
      ['🖱️ Click right past {pet}', 'Turn on click-through from the tray icon and the mouse goes through {pet} to the app behind. Ctrl + right-click on {pet} still opens the menu.'],
      ['🔊 Spoken lines', 'Greetings, break reminders and weather remarks can be said out loud. They are in Vietnamese only for now: set both the text language and the voice to Vietnamese to hear them. Change the voice or mute: right-click {pet} → Sound.'],
      ['🔑 Chat and translate with AI', 'Add an Anthropic (Claude) or Google (Gemini) API key in Settings → 🤖 AI. The key is kept in the Windows credential store and only sent to the provider you pick.', true],
      ['✨ More is switched off for now', 'Weather effects, Bunny Lounge, CPU / RAM warnings, the calendar, clock, aquarium… turn them on in Settings whenever you like. Reopen this tour from the tray icon → ❓.', true],
    ],
  },
};
// Thẻ mời đổi sang giọng tiếng Việt (openVoiceOffer). Chỉ hiện khi chữ đang là
// tiếng Việt (voice-core.js offerDue), nên chỉ cần chữ tiếng Việt.
export const VOICE_OFFER = {
  title: '🔊 Em nói được tiếng Việt rồi nè',
  body: 'Lời chào, lời nhắc nghỉ, chuyện thời tiết… giờ em nói thành tiếng bằng tiếng Việt được rồi. Đổi sang giọng tiếng Việt nha? Muốn đổi lại: chuột phải vào em → Âm thanh → Giọng nói.',
  yes: '🔊 Nói tiếng Việt',
  later: 'Để sau',
};
// Không ai bấm gì thì thẻ tự đóng (lâu hơn thẻ đặt tên vì có chữ để đọc).
const IDLE_CLOSE_MS = 180_000;
// Nới hộp báo cho Rust một chút: con trỏ sát mép thẻ không làm click-through bật / tắt liên tục.
const RECT_PAD = 6;

let card = null;
let idleTimer = 0;
let step = 0;
let ctx = null;
let reportedRect = '';

function labels(lang) {
  return LABELS[lang] || LABELS.vi;
}

/** Thay {pet}, {hotkey} trong chữ của một bước. */
export function fillStep(text, vars) {
  return String(text).replace(/\{(pet|hotkey)\}/g, (_, k) => vars[k] || '');
}

function injectStyle() {
  if (document.getElementById('tour-style')) return;
  const style = document.createElement('style');
  style.id = 'tour-style';
  style.textContent = `
    .tour-card {
      position: absolute;
      left: 50%;
      bottom: 34px;
      width: min(94%, 280px);
      transform: translateX(-50%);
      z-index: 1250;
      padding: 10px 12px 9px;
      border-radius: 14px;
      border: 2px solid rgba(255, 150, 190, 0.6);
      background: linear-gradient(135deg, rgba(255, 236, 245, 0.97), rgba(255, 214, 231, 0.97));
      box-shadow: 0 6px 18px rgba(255, 120, 170, 0.35);
      color: #5b2f47;
      font: 12.5px/1.45 "Segoe UI", system-ui, sans-serif;
      animation: tour-in 0.3s ease-out;
    }
    @keyframes tour-in { from { opacity: 0; transform: translate(-50%, 8px); } }
    @media (prefers-reduced-motion: reduce) { .tour-card { animation: none; } }
    .tour-card .tc-kicker { font-size: 10.5px; color: #9a6a84; display: flex; justify-content: space-between; align-items: center; gap: 6px; }
    .tour-card .tc-progress { display: flex; align-items: center; gap: 6px; flex: 0 0 auto; }
    .tour-card .tc-title { font-weight: 700; font-size: 13.5px; margin: 2px 0 4px; }
    .tour-card .tc-body { margin: 0 0 7px; }
    .tour-card .tc-dots { display: flex; gap: 4px; }
    .tour-card .tc-dots i { width: 6px; height: 6px; border-radius: 50%; background: rgba(232, 110, 160, 0.3); }
    .tour-card .tc-dots i.on { background: #ff73ab; }
    /* Ba nút luôn nằm một hàng: chấm tiến độ nằm ở dòng tiêu đề nhỏ phía trên. */
    .tour-card .tc-foot { display: flex; align-items: center; gap: 6px; flex-wrap: nowrap; }
    .tour-card .tc-foot .tc-sp { flex: 1; min-width: 0; }
    .tour-card button {
      flex: 0 0 auto; white-space: nowrap;
      border: none; border-radius: 8px; padding: 4px 9px;
      font: inherit; font-weight: 600; cursor: pointer;
      background: #fff; color: #5b2f47; border: 1px solid rgba(232, 110, 160, 0.45);
    }
    .tour-card button:hover { background: #ffe3ef; }
    .tour-card button.tc-main { background: #ff73ab; color: #fff; border-color: transparent; }
    .tour-card button.tc-main:hover { background: #f05a97; }
    .tour-card button.tc-skip { background: none; border: none; color: #9a6a84; font-weight: 500; padding: 2px 4px; }
    .tour-card button.tc-skip:hover { color: #5b2f47; background: none; }
    .tour-card .tc-extra { margin: -2px 0 7px; }
    @media (prefers-color-scheme: dark) {
      .tour-card {
        background: linear-gradient(135deg, rgba(90, 50, 75, 0.97), rgba(120, 60, 95, 0.97));
        border-color: rgba(200, 120, 170, 0.45);
        color: #ffd4e8;
      }
      .tour-card .tc-kicker, .tour-card button.tc-skip { color: #d9a9c3; }
      .tour-card button { background: rgba(40, 20, 32, 0.8); color: #ffd4e8; }
      .tour-card button:hover { background: rgba(70, 35, 55, 0.9); }
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

function button(cls, text, onClick) {
  const b = el('button', cls, text);
  b.type = 'button';
  b.addEventListener('click', () => {
    touch();
    onClick();
  });
  return b;
}

/** Báo hộp thẻ (px màn hình) cho Rust; null khi đóng. Chỉ gửi khi đổi. */
function reportRect() {
  const S = window.PetSeeThrough;
  let rect = null;
  if (card && card.isConnected && S) {
    const r = card.getBoundingClientRect();
    const p = RECT_PAD;
    rect = S.toScreenRect({ left: r.left - p, top: r.top - p, width: r.width + 2 * p, height: r.height + 2 * p }, window);
  }
  const key = JSON.stringify(rect);
  if (key === reportedRect) return;
  reportedRect = key;
  vscode.postMessage({ command: 'tour:rect', rect });
}

function touch() {
  clearTimeout(idleTimer);
  idleTimer = setTimeout(close, IDLE_CLOSE_MS);
}

function close() {
  clearTimeout(idleTimer);
  if (card) card.remove();
  card = null;
  window.removeEventListener('keydown', onKey, true);
  reportRect();
}

function onKey(e) {
  if (!card) return;
  if (e.key === 'Escape') close();
  else if (!ctx) return; // thẻ mời đổi giọng: không có bước để chuyển
  else if (e.key === 'ArrowRight') go(step + 1);
  else if (e.key === 'ArrowLeft') go(step - 1);
}

function go(i) {
  const L = labels(ctx.lang);
  if (i >= L.steps.length) {
    close();
    return;
  }
  step = Math.max(0, i);
  render();
}

function render() {
  const L = labels(ctx.lang);
  const vars = { pet: ctx.pet || L.pet, hotkey: ctx.hotkey || 'Ctrl+Shift+Space' };
  const [title, body, withSettings] = L.steps[step];
  const last = step === L.steps.length - 1;
  card.textContent = '';

  const kicker = el('div', 'tc-kicker');
  const dots = el('div', 'tc-dots');
  L.steps.forEach((_, i) => dots.appendChild(el('i', i === step ? 'on' : '')));
  const progress = el('div', 'tc-progress');
  progress.append(dots, el('span', null, `${step + 1} / ${L.steps.length}`));
  kicker.append(el('span', null, `❓ ${L.title}`), progress);
  card.append(kicker, el('div', 'tc-title', fillStep(title, vars)), el('p', 'tc-body', fillStep(body, vars)));
  if (withSettings) {
    const extra = el('div', 'tc-extra');
    extra.appendChild(button('', L.settings, () => vscode.postMessage({ command: 'openSettings' })));
    card.appendChild(extra);
  }

  const foot = el('div', 'tc-foot');
  foot.append(el('span', 'tc-sp'));
  if (!last) foot.appendChild(button('tc-skip', L.skip, close));
  if (step > 0) foot.appendChild(button('', L.prev, () => go(step - 1)));
  foot.appendChild(button('tc-main', last ? L.done : L.next, () => go(step + 1)));
  card.appendChild(foot);
  // Cao thấp đổi theo chữ: báo lại hộp sau khi trình duyệt dàn trang.
  requestAnimationFrame(reportRect);
}

/** Dựng khung thẻ rỗng trên pet (dùng chung cho hướng dẫn và thẻ mời đổi giọng). */
function mountCard(ariaLabel) {
  injectStyle();
  close();
  card = el('div', 'tour-card');
  card.setAttribute('role', 'dialog');
  card.setAttribute('aria-label', ariaLabel);
  // Bấm trong thẻ không bị tính là chọc / kéo pet.
  for (const type of ['pointerdown', 'mousedown', 'mouseup', 'click', 'dblclick', 'contextmenu', 'wheel']) {
    card.addEventListener(type, (e) => e.stopPropagation());
  }
  card.addEventListener('animationend', reportRect);
  (document.querySelector('.companion-container') || document.body).appendChild(card);
  window.addEventListener('keydown', onKey, true);
}

function open(data) {
  mountCard(labels(data.lang).title);
  ctx = { lang: data.lang, pet: data.pet, hotkey: data.hotkey };
  step = 0;
  render();
  touch();
}

/**
 * Thẻ mời người dùng cũ đổi sang giọng tiếng Việt (hỏi một lần, main.js lo điều
 * kiện). Trả false nếu đang có thẻ khác trên pet (hướng dẫn, đặt tên) — nơi gọi
 * thử lại sau. `onAccept` chạy khi bấm đồng ý.
 */
export function openVoiceOffer(onAccept) {
  if (card || document.querySelector('.naming-card')) return false;
  mountCard(VOICE_OFFER.title);
  ctx = null;
  const foot = el('div', 'tc-foot');
  foot.append(
    el('span', 'tc-sp'),
    button('tc-skip', VOICE_OFFER.later, close),
    button('tc-main', VOICE_OFFER.yes, () => {
      close();
      onAccept();
    }),
  );
  card.append(el('div', 'tc-title', VOICE_OFFER.title), el('p', 'tc-body', VOICE_OFFER.body), foot);
  requestAnimationFrame(reportRect);
  touch();
  return true;
}

/** Xử lý lệnh tour:* từ Rust. Trả true nếu đã xử lý. */
export function handleTourMessage(data) {
  if (data && data.command === 'tour:open') {
    open(data);
    return true;
  }
  return false;
}
