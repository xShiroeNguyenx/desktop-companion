// Bảng mini game trên pet (PLAN.md 12.8): oẳn tù tì và đoán số. Luật nằm ở
// app/js/games-core.js (window.GamesCore, có test). Rust mở bảng bằng
// games:open {lang, pet}; chơi xong một ván thì báo game:result để cộng điểm
// thân thiết. Một ván dưới 1–2 phút, đóng lúc nào cũng được, không lưu ván dở.
import { setExpression } from './expression.js';

const T = {
  vi: {
    title: 'Chơi cùng', me: 'Em', you: 'Bạn', rps: '✊✌️✋ Oẳn tù tì', guess: '🔢 Đoán số',
    moves: { rock: '✊ Búa', scissors: '✌️ Kéo', paper: '✋ Bao' },
    picked: '{pet} ra {move}', win: 'Bạn thắng lượt này!', lose: '{pet} thắng lượt này~', draw: 'Hoà rồi!',
    gameWin: 'Bạn thắng cả ván! 🎉', gameLose: '{pet} thắng ván này~ hihi', firstTo: 'Ai thắng 3 trận trước thắng ván',
    guessPrompt: '{pet} đang nghĩ một số từ 1 đến 100…', guessBtn: 'Đoán', higher: 'Cao hơn nữa ⬆️', lower: 'Thấp hơn ⬇️',
    invalid: 'Nhập số nguyên từ 1 đến 100 nha', correct: 'Đúng rồi! Là số {n} 🎉', out: 'Hết lượt rồi~ số là {n}',
    turnsLeft: 'Còn {n} lượt', again: 'Chơi lại', back: 'Đổi trò', close: 'Đóng',
  },
  ja: {
    title: 'あそぼう', me: 'わたし', you: 'あなた', rps: '✊✌️✋ じゃんけん', guess: '🔢 数当て',
    moves: { rock: '✊ グー', scissors: '✌️ チョキ', paper: '✋ パー' },
    picked: '{pet}は{move}', win: 'あなたの勝ち！', lose: '{pet}の勝ち～', draw: 'あいこ！',
    gameWin: 'あなたの優勝！🎉', gameLose: '{pet}の優勝～えへへ', firstTo: '先に3勝した方の勝ち',
    guessPrompt: '{pet}は1〜100の数を思い浮かべてるよ…', guessBtn: '当てる', higher: 'もっと大きい ⬆️', lower: 'もっと小さい ⬇️',
    invalid: '1〜100の整数を入れてね', correct: '正解！{n}だよ🎉', out: '残念～答えは{n}',
    turnsLeft: 'あと{n}回', again: 'もう一回', back: 'ゲームを変える', close: '閉じる',
  },
  en: {
    title: 'Play with', me: 'Me', you: 'You', rps: '✊✌️✋ Rock paper scissors', guess: '🔢 Guess the number',
    moves: { rock: '✊ Rock', scissors: '✌️ Scissors', paper: '✋ Paper' },
    picked: '{pet} played {move}', win: 'You win this round!', lose: '{pet} wins this round~', draw: "It's a draw!",
    gameWin: 'You won the game! 🎉', gameLose: '{pet} won this game~ hehe', firstTo: 'First to 3 wins',
    guessPrompt: '{pet} is thinking of a number from 1 to 100…', guessBtn: 'Guess', higher: 'Higher ⬆️', lower: 'Lower ⬇️',
    invalid: 'Type a whole number from 1 to 100', correct: 'Correct! It was {n} 🎉', out: 'Out of turns~ it was {n}',
    turnsLeft: '{n} turns left', again: 'Play again', back: 'Other game', close: 'Close',
  },
};
const EXPR_MS = 4000;

let card = null;
let L = T.vi;
let petName = 'Em';

function fill(s, vars) {
  return String(s).replace(/\{(\w+)\}/g, (m, k) => (k in vars ? vars[k] : m));
}

function injectStyle() {
  if (document.getElementById('games-style')) return;
  const style = document.createElement('style');
  style.id = 'games-style';
  style.textContent = `
    .games-card {
      position: absolute; left: 50%; bottom: 34px; width: min(94%, 262px);
      transform: translateX(-50%); z-index: 1200;
      padding: 10px 12px 12px; border-radius: 14px;
      border: 2px solid rgba(255, 150, 190, 0.6);
      background: linear-gradient(135deg, rgba(255, 228, 240, 0.97), rgba(255, 210, 228, 0.97));
      box-shadow: 0 6px 18px rgba(255, 120, 170, 0.35);
      color: #5b2f47; font: 12px/1.4 "Segoe UI", system-ui, sans-serif;
      animation: games-in 0.35s cubic-bezier(0.34, 1.56, 0.64, 1);
    }
    @keyframes games-in { from { opacity: 0; transform: translate(-50%, 10px) scale(0.92); } }
    .games-card .gc-head { display: flex; align-items: center; gap: 6px; font-size: 10.5px; font-weight: 700;
      letter-spacing: 0.05em; text-transform: uppercase; color: #e0458f; margin-bottom: 8px; }
    .games-card .gc-x { margin-left: auto; border: none; background: none; color: #9a6a84; cursor: pointer;
      font-size: 14px; line-height: 1; padding: 0 2px; }
    .games-card .gc-col { display: flex; flex-direction: column; gap: 6px; }
    .games-card .gc-row { display: flex; gap: 6px; }
    .games-card button.gc-btn { flex: 1; border: 1px solid rgba(232, 110, 160, 0.45); background: #fff;
      color: #5b2f47; border-radius: 10px; padding: 7px 6px; font: inherit; font-weight: 600; cursor: pointer; }
    .games-card button.gc-btn:hover { background: #ffe3ef; }
    .games-card button.gc-main { background: #ff73ab; border-color: transparent; color: #fff; }
    .games-card button.gc-main:hover { background: #f05a97; }
    .games-card .gc-score { display: flex; justify-content: space-around; font-weight: 800; font-size: 15px; }
    .games-card .gc-score span { font-size: 10px; font-weight: 600; color: #9a6a84; display: block; text-align: center; }
    .games-card .gc-msg { min-height: 34px; text-align: center; font-weight: 600; }
    .games-card .gc-sub { text-align: center; font-size: 10.5px; color: #9a6a84; }
    .games-card input { flex: 1; min-width: 0; padding: 6px 8px; border-radius: 8px;
      border: 1px solid rgba(232, 110, 160, 0.5); background: #fff; color: #5b2f47; font: inherit; }
    .game-head {
      position: absolute; top: 2px; left: 50%; transform: translateX(-50%);
      z-index: 1100; min-width: 44px; height: 44px; padding: 0 10px;
      display: flex; align-items: center; justify-content: center;
      border-radius: 22px; border: 2px solid rgba(255, 150, 190, 0.7);
      background: rgba(255, 240, 247, 0.95); color: #5b2f47;
      font: 800 20px/1 "Segoe UI", system-ui, sans-serif;
      box-shadow: 0 4px 12px rgba(255, 120, 170, 0.4);
      pointer-events: none; white-space: nowrap;
    }
    .game-head.pop { animation: game-head-pop 0.4s cubic-bezier(0.34, 1.56, 0.64, 1); }
    @keyframes game-head-pop {
      from { transform: translateX(-50%) translateY(-6px) scale(0.5); opacity: 0; }
      to   { transform: translateX(-50%) scale(1); opacity: 1; }
    }
    @media (prefers-reduced-motion: reduce) { .game-head.pop { animation: none; } }
    @media (prefers-color-scheme: dark) {
      .game-head { background: rgba(90, 50, 75, 0.95); color: #ffd4e8; border-color: rgba(200, 120, 170, 0.6); }
      .games-card { background: linear-gradient(135deg, rgba(90, 50, 75, 0.97), rgba(120, 60, 95, 0.97));
        border-color: rgba(200, 120, 170, 0.45); color: #ffd4e8; }
      .games-card button.gc-btn, .games-card input { background: rgba(40, 20, 32, 0.8); color: #ffd4e8; }
      .games-card button.gc-btn:hover { background: rgba(70, 35, 55, 0.9); }
      .games-card button.gc-main { background: #ff73ab; color: #1b1320; }
      .games-card .gc-x, .games-card .gc-sub, .games-card .gc-score span { color: #d9a9c3; }
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

function button(text, onClick, main) {
  const b = el('button', 'gc-btn' + (main ? ' gc-main' : ''), text);
  b.type = 'button';
  b.addEventListener('click', onClick);
  return b;
}

// Biểu tượng nổi trên đầu pet: tay pet vừa ra (oẳn tù tì), số vừa đoán / số
// pet đã nghĩ (đoán số). Gắn vào khung nhân vật, mép trên ≈ đỉnh đầu.
let headBadge = null;

function showHead(text) {
  const wrap = document.getElementById('characterWrapper');
  if (!wrap) return;
  if (!headBadge) {
    headBadge = el('div', 'game-head');
    wrap.appendChild(headBadge);
  }
  headBadge.textContent = text;
  // Chạy lại hiệu ứng nảy mỗi lần đổi.
  headBadge.classList.remove('pop');
  void headBadge.offsetWidth;
  headBadge.classList.add('pop');
}

function hideHead() {
  if (headBadge) headBadge.remove();
  headBadge = null;
}

function close() {
  if (card) card.remove();
  card = null;
  hideHead();
  window.removeEventListener('keydown', onKey, true);
}

function onKey(e) {
  if (e.key === 'Escape') close();
}

function rand() {
  const a = new Uint32Array(1);
  crypto.getRandomValues(a);
  return a[0] / 4294967296;
}

function face(name) {
  try { setExpression(name, EXPR_MS); } catch (_) { /* model chưa sẵn sàng */ }
}

function report(game, won) {
  const vscode = window.__VS_CODE_BRIDGE__;
  if (vscode) vscode.postMessage({ command: 'game:result', game, won: !!won });
}

// Khung chung: tiêu đề + nút đóng, thân bảng thay theo màn hình.
function frame(body) {
  card.textContent = '';
  const head = el('div', 'gc-head', `🎮 ${L.title} ${petName}`);
  const x = el('button', 'gc-x', '×');
  x.type = 'button';
  x.title = L.close;
  x.addEventListener('click', close);
  head.appendChild(x);
  card.append(head, body);
}

// Biểu tượng tay trên đầu pet (oẳn tù tì).
const MOVE_ICON = { rock: '✊', scissors: '✌️', paper: '✋' };

function menu() {
  hideHead();
  const col = el('div', 'gc-col');
  col.append(button(L.rps, rpsView, true), button(L.guess, guessView, true));
  frame(col);
}

function rpsView() {
  const G = window.GamesCore;
  let s = G.rpsNew();
  const col = el('div', 'gc-col');
  const score = el('div', 'gc-score');
  const msg = el('div', 'gc-msg', L.firstTo);
  const moves = el('div', 'gc-row');
  const after = el('div', 'gc-row');

  const drawScore = () => {
    score.textContent = '';
    const you = el('div', '', String(s.user));
    you.appendChild(el('span', '', L.you));
    const pet = el('div', '', String(s.pet));
    pet.appendChild(el('span', '', petName));
    score.append(you, pet);
  };

  const play = (move) => {
    if (s.over) return;
    const petMove = G.rpsPick(rand);
    showHead(MOVE_ICON[petMove]);
    const r = G.rpsRound(move, petMove);
    s = G.rpsScore(s, r);
    const line = fill(L.picked, { pet: petName, move: L.moves[petMove] }) + ' · ' + fill(L[r], { pet: petName });
    msg.textContent = line;
    face(r === 'lose' ? 'happy' : r === 'win' ? 'surprised' : 'shy');
    drawScore();
    if (s.over) {
      const userWon = s.winner === 'user';
      msg.textContent = line + '\n' + fill(userWon ? L.gameWin : L.gameLose, { pet: petName });
      msg.style.whiteSpace = 'pre-line';
      face(userWon ? 'shy' : 'love');
      report('rps', userWon);
      moves.style.display = 'none';
      after.style.display = '';
    }
  };

  hideHead(); // ván mới: chưa ra tay
  for (const m of G.MOVES) moves.appendChild(button(L.moves[m], () => play(m)));
  after.append(button(L.again, rpsView, true), button(L.back, menu));
  after.style.display = 'none';
  drawScore();
  col.append(score, msg, moves, after);
  frame(col);
}

function guessView() {
  const G = window.GamesCore;
  let s = G.guessNew(rand);
  const col = el('div', 'gc-col');
  const msg = el('div', 'gc-msg', fill(L.guessPrompt, { pet: petName }));
  const left = el('div', 'gc-sub', fill(L.turnsLeft, { n: G.GUESS_TURNS }));
  const row = el('div', 'gc-row');
  const input = el('input');
  input.type = 'number';
  input.min = String(G.GUESS_MIN);
  input.max = String(G.GUESS_MAX);
  input.setAttribute('aria-label', L.guessBtn);
  const after = el('div', 'gc-row');

  const submit = () => {
    if (s.done) return;
    const n = Number(String(input.value).trim());
    const r = G.guess(s, input.value);
    if (r.hint === 'invalid') {
      msg.textContent = L.invalid;
      input.focus();
      return;
    }
    s = r.state;
    input.value = '';
    left.textContent = fill(L.turnsLeft, { n: G.GUESS_TURNS - s.turns });
    if (r.hint === 'correct') {
      msg.textContent = fill(L.correct, { n: s.secret });
      showHead('🎯 ' + s.secret);
      face('surprised');
    } else if (s.done) {
      msg.textContent = fill(L.out, { n: s.secret });
      // Hết lượt: lật bài số pet đã nghĩ.
      showHead('🎯 ' + s.secret);
      face('happy');
    } else {
      msg.textContent = r.hint === 'higher' ? L.higher : L.lower;
      // Số vừa đoán + mũi tên gợi ý; số pet nghĩ chỉ lộ khi hết ván.
      showHead(n + (r.hint === 'higher' ? ' ⬆️' : ' ⬇️'));
      const w = G.warmth(s.secret, n);
      face(w === 'hot' ? 'surprised' : w === 'warm' ? 'focus' : 'neutral');
    }
    if (s.done) {
      report('guess', s.won);
      row.style.display = 'none';
      after.style.display = '';
    } else {
      input.focus();
    }
  };

  input.addEventListener('keydown', (e) => {
    if (e.key === 'Enter') { e.preventDefault(); submit(); }
  });
  row.append(input, button(L.guessBtn, submit, true));
  after.append(button(L.again, guessView, true), button(L.back, menu));
  after.style.display = 'none';
  col.append(msg, left, row, after);
  frame(col);
  showHead('❓'); // pet đang nghĩ một số
  setTimeout(() => input.focus(), 60);
}

function open(data) {
  injectStyle();
  close();
  L = T[data.lang] || T.vi;
  petName = (data.pet && String(data.pet).trim()) || L.me;
  card = el('div', 'games-card');
  card.setAttribute('role', 'dialog');
  // Bấm / gõ trong bảng không bị tính là chọc / kéo pet.
  for (const type of ['pointerdown', 'mousedown', 'click', 'dblclick', 'contextmenu', 'wheel']) {
    card.addEventListener(type, (e) => e.stopPropagation());
  }
  (document.querySelector('.companion-container') || document.body).appendChild(card);
  window.addEventListener('keydown', onKey, true);
  menu();
}

/** Xử lý lệnh games:* từ Rust. Trả true nếu đã xử lý. */
export function handleGamesMessage(data) {
  if (data && data.command === 'games:open') {
    open(data);
    return true;
  }
  return false;
}
