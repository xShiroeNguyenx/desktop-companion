// node --test app/js/  (npm test)
const test = require('node:test');
const assert = require('node:assert/strict');
const M = require('./mines-core.js');

// Bàn dựng tay: mìn ở các chỉ số cho trước, đã "đặt" sẵn.
function board(rows, cols, mineAt) {
  const s = M.minesNew({ rows, cols, mines: mineAt.length });
  for (const i of mineAt) s.cells[i].mine = true;
  s.cells.forEach((c, i) => { c.adj = M.neighbors(s, i).filter((n) => s.cells[n].mine).length; });
  s.placed = true;
  return s;
}

// Rand giả: chạy lần lượt 0, 0.37, 0.74… cho đủ đa dạng mà vẫn lặp lại được.
const seq = () => { let k = 0; return () => ((k++ * 0.37) % 1); };

test('ba mức độ khớp với bảng chọn', () => {
  assert.deepEqual(M.LEVELS.easy, { rows: 8, cols: 8, mines: 10 });
  assert.deepEqual(M.LEVELS.normal, { rows: 12, cols: 12, mines: 25 });
  assert.deepEqual(M.LEVELS.hard, { rows: 16, cols: 16, mines: 40 });
  const s = M.minesNew('hard');
  assert.equal(s.cells.length, 256);
  assert.equal(M.minesNew('lạ').cells.length, 64);
});

test('ô kề: góc 3, cạnh 5, giữa 8', () => {
  const s = M.minesNew('easy');
  assert.deepEqual(M.neighbors(s, 0).sort((a, b) => a - b), [1, 8, 9]);
  assert.equal(M.neighbors(s, 3).length, 5);
  assert.equal(M.neighbors(s, 9).length, 8);
});

test('lần mở đầu luôn an toàn và loang ra (mìn tránh cả 8 ô quanh)', () => {
  for (const level of ['easy', 'normal', 'hard']) {
    for (const first of [0, 27, 63]) {
      const s = M.minesNew(level);
      const r = M.reveal(s, first, seq());
      assert.notEqual(r.result, 'boom');
      assert.equal(s.cells.filter((c) => c.mine).length, s.mines);
      assert.equal(s.cells[first].adj, 0);
      assert.ok(r.opened.length > 1, 'ô đầu là 0 nên phải loang');
    }
  }
});

test('mở ô số không loang; mở ô 0 loang tới viền số', () => {
  // 3×3, mìn ở góc dưới phải (8).
  const s = board(3, 3, [8]);
  assert.deepEqual(M.reveal(s, 4).opened, [4]); // ô giữa kề mìn → số 1
  const r = M.reveal(s, 0);
  assert.equal(r.result, 'win'); // còn lại đều an toàn → mở hết là thắng
  assert.equal(s.opened, 8);
  assert.ok(s.cells[8].flag, 'thắng thì cắm cờ hộ mọi quả mìn');
  assert.equal(M.minesLeft(s), 0);
});

test('đạp mìn: hết ván, lộ mọi quả mìn chưa cắm cờ', () => {
  const s = board(3, 3, [0, 8]);
  M.toggleFlag(s, 8);
  const r = M.reveal(s, 0);
  assert.equal(r.result, 'boom');
  assert.equal(s.boom, 0);
  assert.ok(s.done && !s.won);
  assert.deepEqual(r.opened, [0]); // ô 8 đã cắm cờ nên giữ nguyên
  assert.equal(M.reveal(s, 4).result, 'none', 'hết ván thì không mở tiếp');
});

test('cờ: chặn mở, đếm mìn còn lại, không cắm lên ô đã mở', () => {
  const s = board(3, 3, [8]);
  assert.ok(M.toggleFlag(s, 2));
  assert.equal(M.minesLeft(s), 0);
  assert.equal(M.reveal(s, 2).result, 'none');
  assert.ok(M.toggleFlag(s, 2));
  assert.equal(s.flags, 0);
  M.reveal(s, 4);
  assert.equal(M.toggleFlag(s, 4), false);
  M.toggleFlag(s, 1); M.toggleFlag(s, 3);
  assert.equal(M.minesLeft(s), 0, 'cắm nhiều cờ hơn số mìn thì vẫn không âm');
});

test('mở nhanh: chỉ chạy khi số cờ quanh ô bằng con số', () => {
  // 2×3, mìn ở ô 0: mở ô 1 (số 1) rồi cắm cờ ô 0.
  const s = board(2, 3, [0]);
  M.reveal(s, 1);
  assert.equal(M.chord(s, 1).result, 'none', 'chưa cắm cờ thì không mở nhanh');
  M.toggleFlag(s, 0);
  const r = M.chord(s, 1);
  assert.equal(r.result, 'win');
  assert.ok(s.won);
});

test('mở nhanh với cờ sai thì nổ', () => {
  const s = board(2, 3, [0]);
  M.reveal(s, 4); // ô (1,1) kề mìn → 1
  M.toggleFlag(s, 2); // cờ sai chỗ
  assert.equal(M.chord(s, 4).result, 'boom');
});

test('gợi ý trả ô an toàn cạnh vùng đã mở', () => {
  const s = board(3, 3, [8]);
  assert.equal(M.safeHint(M.minesNew('easy')), -1, 'chưa đặt mìn thì chưa gợi ý');
  M.reveal(s, 4);
  const h = M.safeHint(s);
  assert.ok(h >= 0 && !s.cells[h].mine && !s.cells[h].open);
  assert.ok(M.neighbors(s, h).includes(4));
});

test('resumable: chỉ chơi tiếp ván đang dở, đúng mức, bàn nguyên vẹn', () => {
  const s = M.minesNew('easy');
  M.reveal(s, 0, () => 0.5);
  const saved = JSON.parse(JSON.stringify({ level: 'easy', state: s, elapsed: 42, hints: 1 }));
  const r = M.resumable(saved, 'easy');
  assert.ok(r);
  assert.equal(r.elapsed, 42);
  assert.equal(r.hints, 1);
  assert.equal(r.state.cells.length, 64);
  // Đổi mức, ván chưa bắt đầu, ván đã xong, dữ liệu hỏng → ván mới.
  assert.equal(M.resumable(saved, 'hard'), null);
  assert.equal(M.resumable({ ...saved, state: M.minesNew('easy') }, 'easy'), null);
  assert.equal(M.resumable({ ...saved, state: { ...saved.state, done: true } }, 'easy'), null);
  assert.equal(M.resumable({ ...saved, state: { ...saved.state, cells: [] } }, 'easy'), null);
  assert.equal(M.resumable(null, 'easy'), null);
  assert.equal(M.resumable({ level: 'easy' }, 'easy'), null);
});
