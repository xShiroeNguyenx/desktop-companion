// node --test app/js/  (npm test)
const test = require('node:test');
const assert = require('node:assert/strict');
const S = require('./pet-seethrough.js');

test('toạ độ màn hình (px vật lý của hook) → toạ độ trang, theo tỉ lệ màn hình', () => {
  const win = { devicePixelRatio: 1.5, screenX: 100, screenY: 50 };
  // Con trỏ ở góc trên-trái cửa sổ: (100, 50) px CSS = (150, 75) px vật lý.
  assert.deepEqual(S.toClient({ x: 150, y: 75 }, win), { x: 0, y: 0 });
  assert.deepEqual(S.toClient({ x: 300, y: 300 }, win), { x: 100, y: 150 });
  // Không có devicePixelRatio: coi như 1.
  assert.deepEqual(S.toClient({ x: 10, y: 20 }, { screenX: 0, screenY: 0 }), { x: 10, y: 20 });
});

test('trong trang / trong hộp: mép trái-trên tính là trong, mép phải-dưới của trang thì không', () => {
  assert.ok(S.inside({ x: 0, y: 0 }, 300, 420));
  assert.ok(S.inside({ x: 299.5, y: 419.5 }, 300, 420));
  assert.ok(!S.inside({ x: 300, y: 10 }, 300, 420), 'ra khỏi mép phải');
  assert.ok(!S.inside({ x: -1, y: 10 }, 300, 420));
  const r = { left: 10, top: 10, right: 50, bottom: 30 };
  assert.ok(S.inRect({ x: 10, y: 30 }, r));
  assert.ok(!S.inRect({ x: 51, y: 20 }, r));
  assert.ok(!S.inRect({ x: 20, y: 20 }, null), 'chưa có bong bóng');
});

test('menu Ctrl + chuột phải: còn gì mở thì đếm lại từ đầu, trống liền CLOSE_AFTER lần mới báo đóng', () => {
  let m = 0;
  m = S.nextMisses(false, m);
  assert.equal(m, 1);
  m = S.nextMisses(true, m);
  assert.equal(m, 0, 'panel mở ra từ menu: chưa đóng');
  for (let i = 0; i < S.CLOSE_AFTER; i++) m = S.nextMisses(false, m);
  assert.ok(m >= S.CLOSE_AFTER);
  // Báo đóng trong khoảng 0.5–1.5 giây sau khi đóng hết: đủ nhanh, không đóng nhầm
  // lúc menu đang chuyển cấp (vòng con) hay panel đang mở ra.
  const ms = S.WATCH_MS * S.CLOSE_AFTER;
  assert.ok(ms >= 400 && ms <= 1500, `${ms} ms`);
  assert.match(S.UI_OPEN, /\.show/);
});

test('hiện lại sau một nhịp ngắn (không nháy khi lướt qua khe tóc)', () => {
  assert.ok(S.LEAVE_MS >= 100 && S.LEAVE_MS <= 600);
});

/** Selector (khớp đúng nguyên văn) có quy tắc nào chứa `animation` trong đoạn CSS không. */
function animated(css, selector) {
  const plain = css.replace(/\/\*[\s\S]*?\*\//g, '');
  for (const [, sel, body] of plain.matchAll(/([^{}]+)\{([^{}]*)\}/g)) {
    if (sel.split(',').some((s) => s.trim() === selector) && /animation/.test(body)) return true;
  }
  return false;
}

test('phần tử được làm mờ không có animation (giá trị cuối của animation đè opacity)', () => {
  // Lỗi thật 2026-11-06: làm mờ .companion-container, mà nó có `animation: fadeIn … both`
  // → class gắn đúng nhưng pet không mờ chút nào.
  const fs = require('node:fs');
  const path = require('node:path');
  const html = fs.readFileSync(path.join(__dirname, '..', 'pet.html'), 'utf8');
  const m = html.match(/html\.pet-see-through\s+([^\s{]+)\s*\{[^}]*opacity/);
  assert.ok(m, 'pet.html có quy tắc làm mờ html.pet-see-through …');
  const css = fs.readFileSync(path.join(__dirname, '..', 'media', 'companion.css'), 'utf8');
  assert.ok(animated(css, '.companion-container'), 'phép kiểm vẫn nhận ra phần tử có animation');
  assert.ok(!animated(css, m[1]), `${m[1]} có animation trong companion.css: đổi phần tử làm mờ`);
});
