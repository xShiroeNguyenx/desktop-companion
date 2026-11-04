// Luật game Ghép Miu (kiểu 2048, ghép hình thay vì số). Hàm thuần, không đụng
// DOM; nguồn ngẫu nhiên được truyền vào (`rand` trả số trong [0, 1)) để test được.
// Dùng trong merge.html (window.MergeCore) và node --test.
//
// Bàn 4×4 là mảng 16 số, đọc theo hàng: 0 = ô trống, 1..13 = bậc của hình.
// Hai ô cùng bậc trượt vào nhau thì thành bậc kế tiếp; bậc 13 (Angel hồng) là
// đích, hai ô bậc 13 không ghép tiếp.
(function (root) {
  const SIZE = 4;
  const CELLS = SIZE * SIZE;
  // Bậc 1–6: Miu theo MÀU, mỗi màu 2 bậc (ngái ngủ → cổ vũ). Màu khác hẳn nhau
  // để dễ phân biệt; trong cùng màu thì phân biệt bằng biểu cảm.
  // Bậc 7–10: ảnh đại diện model theo dòng người chơi chọn (SERIES).
  // Bậc 11–13: Angel theo MÀU, cùng thứ tự với Miu: tím → tuyết → hồng.
  // ĐỒNG BỘ đường dẫn với miu_sets.rs và media/model-previews.
  const COLORS = [
    { name: 'gothic', dir: 'media/miu/gothic', ext: 'webp' }, // tím
    { name: 'snow', dir: 'media/miu/snow', ext: 'webp' }, // tuyết
    { name: 'pink', dir: 'media/game', ext: 'png' }, // áo hồng (bộ gốc)
  ];
  const MIU_FACES = ['sleepy', 'cheer'];
  const MIUS = COLORS.flatMap((c) => MIU_FACES.map((face) => ({ ...c, face })));
  const MODELS = ['lolita', 'kawaii', 'pastel', 'aristocrat'];
  const ANGELS = COLORS.map((c) => c.name); // gothic, snow, pink
  const FIRST_MODEL = MIUS.length + 1; // 7 = Lolita
  const FIRST_ANGEL = FIRST_MODEL + MODELS.length; // 11 = Angel tím
  const MAX_LEVEL = MIUS.length + MODELS.length + ANGELS.length; // 13, như 2 → 8192
  // Thắng ở Angel tím = mốc 2048; Angel tuyết / hồng là phần chơi tiếp lấy thêm điểm.
  const WIN_LEVEL = FIRST_ANGEL; // 11
  // Dòng model có đủ 5 bậc (pink chưa có lolita nên chưa có mặt ở đây).
  const SERIES = ['gothic', 'snow'];
  const DIRS = ['left', 'right', 'up', 'down'];

  /**
   * Tên hình của một bậc, hoặc null:
   *   { kind: 'miu', name: màu, face } | { kind: 'model', name } | { kind: 'angel', name: 'angel', color }
   */
  function tier(level) {
    if (!Number.isInteger(level) || level < 1 || level > MAX_LEVEL) return null;
    if (level < FIRST_MODEL) {
      const m = MIUS[level - 1];
      return { kind: 'miu', name: m.name, face: m.face };
    }
    if (level < FIRST_ANGEL) return { kind: 'model', name: MODELS[level - FIRST_MODEL] };
    return { kind: 'angel', name: 'angel', color: ANGELS[level - FIRST_ANGEL] };
  }

  /** Đường dẫn ảnh của một bậc. Miu và Angel cố định màu theo bậc, model theo dòng (`series`). */
  function tileSrc(level, series) {
    const t = tier(level);
    if (!t) return '';
    if (t.kind === 'miu') {
      const m = MIUS[level - 1];
      return `${m.dir}/rps-${m.face}.${m.ext}`;
    }
    if (t.kind === 'angel') return `media/model-previews/${t.color}-angel.png`;
    const s = SERIES.includes(series) ? series : SERIES[0];
    return `media/model-previews/${s}-${t.name}.png`;
  }

  /** Dòng model mặc định theo bộ mặt Miu: bộ snow → dòng snow, còn lại → gothic. */
  function seriesForMiu(miuDir) {
    return /(^|\/)snow$/.test(String(miuDir || '')) ? 'snow' : 'gothic';
  }

  function emptyCells(board) {
    const out = [];
    board.forEach((v, i) => { if (!v) out.push(i); });
    return out;
  }

  /** Thêm một ô mới vào chỗ trống: 90% bậc 1, 10% bậc 2. Trả { board, at } (at = -1 nếu đầy). */
  function spawn(board, rand) {
    const free = emptyCells(board);
    if (!free.length) return { board, at: -1 };
    const pick = Math.min(free.length - 1, Math.floor(Math.max(0, Number(rand()) || 0) * free.length));
    const at = free[pick];
    const b = board.slice();
    b[at] = rand() < 0.9 ? 1 : 2;
    return { board: b, at };
  }

  /** Điểm khi ghép ra bậc `level`: 2^level, giống giá trị ô trong 2048. */
  function points(level) {
    return 2 ** level;
  }

  /**
   * Trượt một hàng về phía đầu mảng (trái). Mỗi ô chỉ ghép một lần mỗi lượt.
   * Trả { line, merged: [vị trí trong hàng], gained }.
   */
  function slideLine(line) {
    const vals = line.filter((v) => v);
    const out = [];
    const merged = [];
    let gained = 0;
    for (let i = 0; i < vals.length; i++) {
      const v = vals[i];
      if (i + 1 < vals.length && vals[i + 1] === v && v < MAX_LEVEL) {
        out.push(v + 1);
        merged.push(out.length - 1);
        gained += points(v + 1);
        i += 1;
      } else {
        out.push(v);
      }
    }
    while (out.length < line.length) out.push(0);
    return { line: out, merged, gained };
  }

  /** Chỉ số các ô của hàng/cột `k`, xếp theo chiều trượt (ô đầu = phía hình dồn về). */
  function lineIndexes(dir, k) {
    const idx = [];
    for (let j = 0; j < SIZE; j++) {
      if (dir === 'left') idx.push(k * SIZE + j);
      else if (dir === 'right') idx.push(k * SIZE + (SIZE - 1 - j));
      else if (dir === 'up') idx.push(j * SIZE + k);
      else idx.push((SIZE - 1 - j) * SIZE + k);
    }
    return idx;
  }

  /**
   * Trượt cả bàn theo `dir`. Trả { board, moved, merged: [chỉ số ô vừa ghép], gained }.
   * Không thêm ô mới (xem move).
   */
  function slide(board, dir) {
    if (!DIRS.includes(dir)) throw new Error('hướng lạ: ' + dir);
    const b = board.slice();
    const merged = [];
    let gained = 0;
    for (let k = 0; k < SIZE; k++) {
      const idx = lineIndexes(dir, k);
      const r = slideLine(idx.map((i) => board[i]));
      idx.forEach((i, j) => { b[i] = r.line[j]; });
      r.merged.forEach((j) => merged.push(idx[j]));
      gained += r.gained;
    }
    const moved = b.some((v, i) => v !== board[i]);
    return { board: b, moved, merged, gained };
  }

  /** Còn nước đi không: còn ô trống, hoặc hai ô cạnh nhau cùng bậc (chưa tới Angel). */
  function canMove(board) {
    if (board.some((v) => !v)) return true;
    for (let r = 0; r < SIZE; r++) {
      for (let c = 0; c < SIZE; c++) {
        const v = board[r * SIZE + c];
        if (v >= MAX_LEVEL) continue;
        if (c + 1 < SIZE && board[r * SIZE + c + 1] === v) return true;
        if (r + 1 < SIZE && board[(r + 1) * SIZE + c] === v) return true;
      }
    }
    return false;
  }

  function highest(board) {
    return board.reduce((m, v) => Math.max(m, v), 0);
  }

  /** Ván mới: bàn trống + 2 ô. */
  function newGame(rand) {
    let board = new Array(CELLS).fill(0);
    board = spawn(board, rand).board;
    board = spawn(board, rand).board;
    return { board, score: 0, moves: 0, best: highest(board), won: false, over: false };
  }

  /**
   * Một lượt. Trả { state, moved, merged, spawned, reached }:
   *  - moved = false: bàn không đổi, không tính lượt, không thêm ô.
   *  - reached = các bậc lần đầu đạt được TRONG VÁN này (tăng dần), để báo "mở khoá".
   * Ghép ra Angel tím (WIN_LEVEL, mốc 2048) lần đầu → state.won = true (vẫn chơi tiếp được).
   * Hết nước đi → state.over = true.
   */
  function move(state, dir, rand) {
    if (state.over) return { state, moved: false, merged: [], spawned: -1, reached: [] };
    const s = slide(state.board, dir);
    if (!s.moved) return { state, moved: false, merged: [], spawned: -1, reached: [] };
    const sp = spawn(s.board, rand);
    const top = highest(sp.board);
    const reached = [];
    for (let l = state.best + 1; l <= top; l++) reached.push(l);
    const next = {
      board: sp.board,
      score: state.score + s.gained,
      moves: state.moves + 1,
      best: Math.max(state.best, top),
      won: state.won || top >= WIN_LEVEL,
      over: !canMove(sp.board),
    };
    return { state: next, moved: true, merged: s.merged, spawned: sp.at, reached };
  }

  /** Ván lưu trong localStorage còn dùng được không (đúng hình dạng, chưa kết thúc). */
  function resumable(saved) {
    if (!saved || typeof saved !== 'object') return null;
    const b = saved.board;
    if (!Array.isArray(b) || b.length !== CELLS) return null;
    if (!b.every((v) => Number.isInteger(v) && v >= 0 && v <= MAX_LEVEL)) return null;
    if (saved.over || !canMove(b)) return null;
    const num = (v) => (Number.isFinite(v) && v >= 0 ? Math.floor(v) : 0);
    return { board: b.slice(), score: num(saved.score), moves: num(saved.moves), best: highest(b), won: !!saved.won, over: false };
  }

  /** Phím → hướng (mũi tên và WASD). */
  function keyDir(key) {
    const k = String(key || '');
    if (k === 'ArrowLeft' || k === 'a' || k === 'A') return 'left';
    if (k === 'ArrowRight' || k === 'd' || k === 'D') return 'right';
    if (k === 'ArrowUp' || k === 'w' || k === 'W') return 'up';
    if (k === 'ArrowDown' || k === 's' || k === 'S') return 'down';
    return null;
  }

  /** Vuốt (dx, dy theo px) → hướng; ngắn hơn `min` thì null. */
  function swipeDir(dx, dy, min = 24) {
    if (Math.max(Math.abs(dx), Math.abs(dy)) < min) return null;
    if (Math.abs(dx) > Math.abs(dy)) return dx > 0 ? 'right' : 'left';
    return dy > 0 ? 'down' : 'up';
  }

  const api = {
    SIZE, CELLS, MIUS, MODELS, ANGELS, MAX_LEVEL, WIN_LEVEL, FIRST_MODEL, FIRST_ANGEL, SERIES, DIRS,
    tier, tileSrc, seriesForMiu, emptyCells, spawn, points, slideLine, slide, canMove, highest,
    newGame, move, resumable, keyDir, swipeDir,
  };
  if (typeof module !== 'undefined' && module.exports) module.exports = api;
  else root.MergeCore = api;
})(typeof window !== 'undefined' ? window : globalThis);
