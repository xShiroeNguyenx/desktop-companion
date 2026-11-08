// node --test app/js/  (npm test)
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const M = require('./merge-core.js');

const fixed = (...values) => {
  let i = 0;
  return () => values[i++ % values.length];
};
// Bàn từ 4 hàng cho dễ đọc.
const B = (...rows) => rows.flat();

test('thang 13 bậc: Miu tím → tuyết → hồng (mỗi màu 2 bậc), 4 model, rồi Angel tím → tuyết → hồng', () => {
  assert.equal(M.MAX_LEVEL, 13);
  const mius = [1, 2, 3, 4, 5, 6].map((l) => M.tier(l));
  assert.ok(mius.every((t) => t.kind === 'miu'));
  assert.deepEqual(mius.map((t) => t.name), ['gothic', 'gothic', 'snow', 'snow', 'pink', 'pink']);
  assert.deepEqual(mius.map((t) => t.face), ['sleepy', 'cheer', 'sleepy', 'cheer', 'sleepy', 'cheer']);
  assert.deepEqual([7, 8, 9, 10].map((l) => M.tier(l).name), ['lolita', 'kawaii', 'pastel', 'aristocrat']);
  assert.equal(M.tier(7).kind, 'model');
  const angels = [11, 12, 13].map((l) => M.tier(l));
  assert.ok(angels.every((t) => t.kind === 'angel' && t.name === 'angel'));
  assert.deepEqual(angels.map((t) => t.color), ['gothic', 'snow', 'pink'], 'cùng thứ tự màu với Miu');
  assert.equal(M.tier(0), null);
  assert.equal(M.tier(14), null);
});

test('mọi ảnh của thang đều có thật trong app/media', () => {
  const app = path.join(__dirname, '..');
  for (const series of M.SERIES) {
    for (let l = 1; l <= M.MAX_LEVEL; l++) {
      const src = M.tileSrc(l, series);
      assert.ok(fs.existsSync(path.join(app, src)), src);
    }
  }
});

test('ảnh: Miu và Angel cố định theo màu, model theo dòng; dòng lạ về gothic', () => {
  assert.equal(M.tileSrc(1, 'snow'), 'media/miu/gothic/rps-sleepy.webp');
  assert.equal(M.tileSrc(2, 'snow'), 'media/miu/gothic/rps-cheer.webp');
  assert.equal(M.tileSrc(3, 'gothic'), 'media/miu/snow/rps-sleepy.webp');
  assert.equal(M.tileSrc(6, 'gothic'), 'media/game/rps-cheer.png');
  assert.equal(M.tileSrc(7, 'snow'), 'media/model-previews/snow-lolita.png');
  assert.equal(M.tileSrc(7, 'pink'), 'media/model-previews/pink-lolita.png');
  assert.equal(M.tileSrc(10, 'pink'), 'media/model-previews/pink-aristocrat.png');
  assert.equal(M.tileSrc(10, 'nope'), 'media/model-previews/gothic-aristocrat.png');
  assert.equal(M.tileSrc(11, 'snow'), 'media/model-previews/gothic-angel.png');
  assert.equal(M.tileSrc(12, 'gothic'), 'media/model-previews/snow-angel.png');
  assert.equal(M.tileSrc(13, 'gothic'), 'media/model-previews/pink-angel.png');
  assert.equal(M.seriesForMiu('media/miu/snow'), 'snow');
  assert.equal(M.seriesForMiu('media/miu/gothic'), 'gothic');
  assert.equal(M.seriesForMiu('media/game'), 'gothic');
});

test('trượt một hàng: dồn, ghép một lần mỗi ô, Angel không ghép', () => {
  assert.deepEqual(M.slideLine([0, 1, 0, 1]).line, [2, 0, 0, 0]);
  assert.deepEqual(M.slideLine([1, 1, 1, 1]).line, [2, 2, 0, 0]);
  assert.deepEqual(M.slideLine([2, 1, 1, 0]).line, [2, 2, 0, 0], 'ô vừa ghép không ghép tiếp');
  assert.deepEqual(M.slideLine([1, 1, 1, 0]).line, [2, 1, 0, 0]);
  assert.deepEqual(M.slideLine([13, 13, 0, 0]).line, [13, 13, 0, 0]);
  assert.deepEqual(M.slideLine([11, 11, 0, 0]).line, [12, 0, 0, 0], 'Angel tím + Angel tím = Angel tuyết');
  const r = M.slideLine([12, 12, 3, 3]);
  assert.deepEqual(r.line, [13, 4, 0, 0]);
  assert.deepEqual(r.merged, [0, 1]);
  assert.equal(r.gained, 2 ** 13 + 2 ** 4);
});

test('trượt cả bàn theo 4 hướng', () => {
  const b = B([1, 1, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0], [1, 0, 0, 2]);
  assert.deepEqual(M.slide(b, 'left').board, B([2, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0], [1, 2, 0, 0]));
  assert.deepEqual(M.slide(b, 'right').board, B([0, 0, 0, 2], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 1, 2]));
  assert.deepEqual(M.slide(b, 'up').board, B([2, 1, 0, 2], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0]));
  assert.deepEqual(M.slide(b, 'down').board, B([0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0], [2, 1, 0, 2]));
  const r = M.slide(b, 'up');
  assert.deepEqual(r.merged, [0]);
  assert.equal(M.slide(B([1, 2, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0]), 'left').moved, false);
  assert.throws(() => M.slide(b, 'chéo'));
});

test('sinh ô mới: chỗ trống, 90% bậc 1 / 10% bậc 2', () => {
  const b = B([1, 1, 1, 1], [1, 1, 1, 1], [1, 1, 1, 1], [1, 1, 0, 1]);
  const a = M.spawn(b, fixed(0, 0.5));
  assert.equal(a.at, 14);
  assert.equal(a.board[14], 1);
  assert.equal(M.spawn(b, fixed(0, 0.95)).board[14], 2);
  assert.equal(M.spawn(b.map(() => 1), fixed(0)).at, -1);
});

test('hết nước: đầy và không cặp nào cạnh nhau ghép được', () => {
  const stuck = B([1, 2, 1, 2], [2, 1, 2, 1], [1, 2, 1, 2], [2, 1, 2, 1]);
  assert.equal(M.canMove(stuck), false);
  const pair = stuck.slice();
  pair[1] = 1;
  assert.equal(M.canMove(pair), true);
  const angels = B([13, 13, 1, 2], [2, 1, 2, 1], [1, 2, 1, 2], [2, 1, 2, 1]);
  assert.equal(M.canMove(angels), false, 'hai Angel cạnh nhau không phải nước đi');
});

test('một lượt: cộng điểm, đếm lượt, báo bậc mới, thắng khi ra Angel', () => {
  const state = { board: B([12, 12, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0]), score: 5, moves: 3, best: 12, won: false, over: false };
  const r = M.move(state, 'left', fixed(0, 0.5));
  assert.equal(r.moved, true);
  assert.equal(r.state.board[0], 13);
  assert.equal(r.state.score, 5 + 2 ** 13);
  assert.equal(r.state.moves, 4);
  assert.deepEqual(r.reached, [13]);
  assert.equal(r.state.won, true);

  // Thắng ngay ở Angel tím (mốc 2048), không phải đợi tới Angel hồng.
  assert.equal(M.WIN_LEVEL, 11);
  const near = { board: B([10, 10, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0]), score: 0, moves: 0, best: 10, won: false, over: false };
  const w = M.move(near, 'left', fixed(0, 0.5));
  assert.equal(w.state.board[0], 11);
  assert.equal(w.state.won, true);
  assert.equal(w.state.over, false, 'thắng rồi vẫn chơi tiếp được');
  const more = M.move(w.state, 'right', fixed(0, 0.5));
  assert.equal(more.state.won, true, 'đã thắng thì giữ nguyên');
  assert.equal(r.state.over, false);
  assert.ok(r.spawned > 0);

  const same = M.move(r.state, 'left', fixed(0));
  assert.equal(same.moved, false, 'không đổi thì không tính lượt');
  assert.equal(same.state, r.state);
});

test('ván mới có đúng 2 ô; ván lưu hỏng hoặc đã hết thì bỏ', () => {
  const g = M.newGame(fixed(0.1, 0.5, 0.7, 0.5));
  assert.equal(g.board.filter((v) => v).length, 2);
  assert.equal(g.score, 0);
  assert.equal(M.resumable(null), null);
  assert.equal(M.resumable({ board: [1, 2] }), null);
  assert.equal(M.resumable({ board: new Array(16).fill(14) }), null);
  const stuck = B([1, 2, 1, 2], [2, 1, 2, 1], [1, 2, 1, 2], [2, 1, 2, 1]);
  assert.equal(M.resumable({ board: stuck }), null);
  const ok = M.resumable({ board: g.board, score: 40, moves: 7, won: 1 });
  assert.deepEqual(ok, { board: g.board, score: 40, moves: 7, best: M.highest(g.board), won: true, over: false });
});

test('phím và vuốt → hướng', () => {
  assert.equal(M.keyDir('ArrowLeft'), 'left');
  assert.equal(M.keyDir('w'), 'up');
  assert.equal(M.keyDir('Enter'), null);
  assert.equal(M.swipeDir(40, 5), 'right');
  assert.equal(M.swipeDir(-3, -50), 'up');
  assert.equal(M.swipeDir(5, 5), null);
});
