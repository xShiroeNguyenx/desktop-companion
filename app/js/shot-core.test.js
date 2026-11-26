// node --test app/js/  (npm test)
const test = require('node:test');
const assert = require('node:assert/strict');
const S = require('./shot-core.js');

const W = 1920;
const H = 1080;

test('normRect: hai góc bất kỳ ra hình chữ nhật không âm, số nguyên', () => {
  assert.deepEqual(S.normRect(10, 20, 110, 70), { x: 10, y: 20, w: 100, h: 50 });
  // Kéo ngược từ dưới-phải lên trên-trái.
  assert.deepEqual(S.normRect(110, 70, 10, 20), { x: 10, y: 20, w: 100, h: 50 });
  assert.deepEqual(S.normRect(10.4, 20.6, 110.5, 70.2), { x: 10, y: 21, w: 101, h: 49 });
});

test('cropRect: phần tràn ra ngoài ảnh bị cắt', () => {
  assert.deepEqual(S.cropRect({ x: -50, y: -10, w: 200, h: 100 }, W, H), { x: 0, y: 0, w: 150, h: 90 });
  assert.deepEqual(S.cropRect({ x: 1900, y: 1000, w: 500, h: 500 }, W, H), { x: 1900, y: 1000, w: 20, h: 80 });
  assert.deepEqual(S.cropRect({ x: 5, y: 5, w: 10, h: 10 }, W, H), { x: 5, y: 5, w: 10, h: 10 });
});

test('usable: bấm nhầm (vùng quá nhỏ) không tính là vùng chọn', () => {
  assert.equal(S.usable(null), false);
  assert.equal(S.usable({ x: 0, y: 0, w: 3, h: 200 }), false);
  assert.equal(S.usable({ x: 0, y: 0, w: 4, h: 4 }), true);
});

test('hitTest: tay nắm thắng phần bên trong, ngoài vùng là khoanh mới', () => {
  const r = { x: 100, y: 100, w: 200, h: 100 };
  assert.equal(S.hitTest(null, 5, 5, 8), 'new');
  assert.equal(S.hitTest(r, 100, 100, 8), 'nw');
  assert.equal(S.hitTest(r, 305, 196, 8), 'se');
  assert.equal(S.hitTest(r, 200, 104, 8), 'n');
  assert.equal(S.hitTest(r, 96, 150, 8), 'w');
  assert.equal(S.hitTest(r, 200, 150, 8), 'move');
  assert.equal(S.hitTest(r, 50, 50, 8), 'new');
  assert.equal(S.hitTest(r, 400, 150, 8), 'new');
  // Mỗi tay nắm tự bắt được chính tâm của nó.
  const pts = S.handlePoints(r);
  for (const name of S.HANDLES) assert.equal(S.hitTest(r, pts[name][0], pts[name][1], 8), name);
});

test('dragRect: dời giữ nguyên cỡ và không ra khỏi ảnh', () => {
  const r = { x: 100, y: 100, w: 200, h: 100 };
  assert.deepEqual(S.dragRect(r, 'move', 30, -20, W, H), { x: 130, y: 80, w: 200, h: 100 });
  assert.deepEqual(S.dragRect(r, 'move', -500, -500, W, H), { x: 0, y: 0, w: 200, h: 100 });
  assert.deepEqual(S.dragRect(r, 'move', 5000, 5000, W, H), { x: 1720, y: 980, w: 200, h: 100 });
});

test('dragRect: đổi cỡ theo đúng cạnh, kéo qua cạnh đối diện thì lật lại', () => {
  const r = { x: 100, y: 100, w: 200, h: 100 };
  assert.deepEqual(S.dragRect(r, 'e', 50, 999, W, H), { x: 100, y: 100, w: 250, h: 100 });
  assert.deepEqual(S.dragRect(r, 'n', 999, -40, W, H), { x: 100, y: 60, w: 200, h: 140 });
  assert.deepEqual(S.dragRect(r, 'se', 20, 30, W, H), { x: 100, y: 100, w: 220, h: 130 });
  assert.deepEqual(S.dragRect(r, 'nw', -20, -30, W, H), { x: 80, y: 70, w: 220, h: 130 });
  // Kéo cạnh phải vượt qua cạnh trái.
  assert.deepEqual(S.dragRect(r, 'e', -260, 0, W, H), { x: 40, y: 100, w: 60, h: 100 });
  // Kéo ra ngoài ảnh thì dừng ở mép.
  assert.deepEqual(S.dragRect(r, 'se', 5000, 5000, W, H), { x: 100, y: 100, w: 1820, h: 980 });
});

test('cursorFor: con trỏ theo tay nắm', () => {
  assert.equal(S.cursorFor('nw'), 'nwse-resize');
  assert.equal(S.cursorFor('sw'), 'nesw-resize');
  assert.equal(S.cursorFor('s'), 'ns-resize');
  assert.equal(S.cursorFor('w'), 'ew-resize');
  assert.equal(S.cursorFor('move'), 'move');
  assert.equal(S.cursorFor('new'), 'crosshair');
});

test('toolbarPos: dưới vùng chọn, hết chỗ thì lên trên, hết nữa thì vào trong', () => {
  const tb = { w: 300, h: 40 };
  // Còn chỗ bên dưới: canh mép phải vùng chọn.
  assert.deepEqual(S.toolbarPos({ x: 500, y: 200, w: 400, h: 300 }, tb, W, H, 8), { x: 600, y: 508 });
  // Sát đáy màn hình: lên trên.
  assert.deepEqual(S.toolbarPos({ x: 500, y: 700, w: 400, h: 370 }, tb, W, H, 8), { x: 600, y: 652 });
  // Chọn kín cả màn hình: nằm trong, sát đáy.
  assert.deepEqual(S.toolbarPos({ x: 0, y: 0, w: W, h: H }, tb, W, H, 8), { x: 1620, y: 1032 });
  // Vùng chọn hẹp sát mép trái: không tràn ra ngoài màn hình.
  assert.deepEqual(S.toolbarPos({ x: 10, y: 100, w: 50, h: 50 }, tb, W, H, 8), { x: 0, y: 158 });
});

test('arrowHead: hai cánh đối xứng qua thân mũi tên', () => {
  assert.equal(S.arrowHead(10, 10, 10, 10, 12), null);
  const [a, b] = S.arrowHead(0, 0, 100, 0, 20);
  assert.ok(a[0] < 100 && b[0] < 100, 'cánh nằm phía sau mũi');
  assert.ok(Math.abs(a[0] - b[0]) < 1e-9);
  assert.ok(Math.abs(a[1] + b[1]) < 1e-9, 'đối xứng qua trục ngang');
  assert.ok(Math.abs(Math.hypot(100 - a[0], a[1]) - 20) < 1e-9, 'cánh dài đúng size');
});

test('snapEnd: giữ Shift thì đường thẳng bẻ về ngang / dọc / chéo 45°', () => {
  // Gần ngang → ngang hẳn, giữ độ dài.
  assert.deepEqual(S.snapEnd(0, 0, 100, 8), [100, 0]);
  // Gần dọc (kéo lên trên).
  assert.deepEqual(S.snapEnd(50, 50, 55, -50), [50, -50]);
  // Gần chéo → đúng 45°: hai chiều bằng nhau.
  const [x, y] = S.snapEnd(0, 0, 100, 90);
  assert.equal(x, y);
  assert.ok(Math.abs(Math.hypot(x, y) - Math.hypot(100, 90)) < 1);
  // Kéo sang trái.
  assert.deepEqual(S.snapEnd(100, 100, 0, 104), [0, 100]);
  // Chưa kéo thì giữ nguyên.
  assert.deepEqual(S.snapEnd(10, 10, 10, 10), [10, 10]);
});

test('stepWidth: lăn chuột đi từng nấc, hết nấc thì đứng yên', () => {
  assert.equal(S.stepWidth(3, 1), 4);
  assert.equal(S.stepWidth(3, -1), 2);
  assert.equal(S.stepWidth(4, 1), 6);
  assert.equal(S.stepWidth(1, -1), 1);
  assert.equal(S.stepWidth(16, 1), 16);
  // Giá trị lạ thì coi như đang ở nấc mặc định.
  assert.equal(S.stepWidth(5, 1), 4);
  // Đi hết thang rồi quay lại.
  let w = S.DEFAULT_WIDTH;
  for (let i = 0; i < 20; i++) w = S.stepWidth(w, 1);
  assert.equal(w, S.WIDTHS[S.WIDTHS.length - 1]);
  for (let i = 0; i < 20; i++) w = S.stepWidth(w, -1);
  assert.equal(w, S.WIDTHS[0]);
});

test('strokeFor: bút dạ quang to hơn nét thường, chữ có cỡ riêng', () => {
  assert.equal(S.strokeFor('pen', 3), 3);
  assert.equal(S.strokeFor('arrow', 8), 8);
  assert.equal(S.strokeFor('marker', 3), 16);
  assert.equal(S.strokeFor('text', 3), 18);
  assert.ok(S.strokeFor('text', 1) >= 12, 'chữ nhỏ nhất vẫn đọc được');
});

test('ocrScale: vùng nhỏ phóng gấp đôi cho dễ đọc chữ, vùng lớn giữ nguyên', () => {
  assert.equal(S.ocrScale(600, 200), 2);
  assert.equal(S.ocrScale(2000, 900), 2);
  assert.equal(S.ocrScale(2001, 900), 1);
  assert.equal(S.ocrScale(800, 3840), 1);
});

test('fileName: theo giờ máy, đủ hai chữ số', () => {
  assert.equal(S.fileName(new Date(2026, 10, 26, 14, 3, 9)), 'Screenshot_2026-11-26_14-03-09.png');
  assert.equal(S.fileName(new Date(2027, 0, 5, 0, 0, 0)), 'Screenshot_2027-01-05_00-00-00.png');
});
