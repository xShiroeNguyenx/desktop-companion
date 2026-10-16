// Luật gỡ mìn (cửa sổ mines.html). Hàm thuần, không đụng DOM; nguồn ngẫu
// nhiên được truyền vào (`rand` trả số trong [0, 1)) để test được. Bàn cờ là
// mảng phẳng `cells` (chỉ số = hàng * cols + cột); các hàm sửa `state` tại chỗ
// (bàn 16×16 = 256 ô, chép lại mỗi lần bấm là phí) và trả về những ô vừa đổi.
// Dùng trong cửa sổ game (window.MinesCore) và node --test.
(function (root) {
  const LEVELS = {
    easy: { rows: 8, cols: 8, mines: 10 },
    normal: { rows: 12, cols: 12, mines: 25 },
    hard: { rows: 16, cols: 16, mines: 40 },
  };

  /** Ván mới. Mìn chưa đặt: lần mở đầu tiên mới đặt, để ô đầu luôn an toàn. */
  function minesNew(level) {
    const cfg = typeof level === 'string' ? LEVELS[level] || LEVELS.easy : level || LEVELS.easy;
    const rows = Math.max(2, cfg.rows | 0);
    const cols = Math.max(2, cfg.cols | 0);
    const mines = Math.min(Math.max(1, cfg.mines | 0), rows * cols - 1);
    const cells = [];
    for (let i = 0; i < rows * cols; i++) cells.push({ mine: false, adj: 0, open: false, flag: false });
    return { rows, cols, mines, cells, placed: false, opened: 0, flags: 0, done: false, won: false, boom: -1 };
  }

  /** Chỉ số các ô kề (tối đa 8) của ô `i`. */
  function neighbors(state, i) {
    const { rows, cols } = state;
    const r = Math.floor(i / cols);
    const c = i % cols;
    const out = [];
    for (let dr = -1; dr <= 1; dr++) {
      for (let dc = -1; dc <= 1; dc++) {
        if (!dr && !dc) continue;
        const rr = r + dr;
        const cc = c + dc;
        if (rr >= 0 && rr < rows && cc >= 0 && cc < cols) out.push(rr * cols + cc);
      }
    }
    return out;
  }

  /**
   * Đặt mìn tránh ô `safe` và (nếu đủ chỗ) cả 8 ô quanh nó, để lần mở đầu
   * luôn loang ra một vùng. Xáo Fisher–Yates trên các ô được phép.
   */
  function place(state, safe, rand) {
    const total = state.rows * state.cols;
    const zone = new Set([safe, ...neighbors(state, safe)]);
    let pool = [];
    for (let i = 0; i < total; i++) if (!zone.has(i)) pool.push(i);
    if (pool.length < state.mines) {
      pool = [];
      for (let i = 0; i < total; i++) if (i !== safe) pool.push(i);
    }
    for (let k = 0; k < state.mines; k++) {
      const r = Math.min(Math.max(Number(rand()) || 0, 0), 0.999999999);
      const j = k + Math.floor(r * (pool.length - k));
      [pool[k], pool[j]] = [pool[j], pool[k]];
      state.cells[pool[k]].mine = true;
    }
    state.cells.forEach((cell, i) => {
      cell.adj = neighbors(state, i).filter((n) => state.cells[n].mine).length;
    });
    state.placed = true;
  }

  function finishWin(state) {
    state.done = true;
    state.won = true;
    for (const cell of state.cells) if (cell.mine && !cell.flag) { cell.flag = true; state.flags += 1; }
  }

  /**
   * Mở ô `i`. Ô 0 loang ra các ô kề (BFS, không mở ô đã cắm cờ).
   * Trả { result, opened } — result: 'none' (không làm gì) | 'open' | 'boom' | 'win';
   * opened: chỉ số các ô vừa mở (khi nổ: ô nổ + mọi ô mìn chưa cắm cờ).
   */
  function reveal(state, i, rand) {
    const cell = state.cells[i];
    if (state.done || !cell || cell.open || cell.flag) return { result: 'none', opened: [] };
    if (!state.placed) place(state, i, rand);
    if (cell.mine) {
      cell.open = true;
      state.done = true;
      state.boom = i;
      const opened = [i];
      state.cells.forEach((c, k) => { if (c.mine && !c.open && !c.flag) { c.open = true; opened.push(k); } });
      return { result: 'boom', opened };
    }
    const opened = [];
    const queue = [i];
    cell.open = true;
    while (queue.length) {
      const k = queue.shift();
      opened.push(k);
      state.opened += 1;
      if (state.cells[k].adj !== 0) continue;
      for (const n of neighbors(state, k)) {
        const c = state.cells[n];
        if (!c.open && !c.flag && !c.mine) { c.open = true; queue.push(n); }
      }
    }
    if (state.opened === state.rows * state.cols - state.mines) {
      finishWin(state);
      return { result: 'win', opened };
    }
    return { result: 'open', opened };
  }

  /** Cắm / gỡ cờ trên ô chưa mở. Trả true nếu có đổi. */
  function toggleFlag(state, i) {
    const cell = state.cells[i];
    if (state.done || !cell || cell.open) return false;
    cell.flag = !cell.flag;
    state.flags += cell.flag ? 1 : -1;
    return true;
  }

  /**
   * Mở nhanh (bấm vào ô số đã mở): nếu số cờ quanh nó bằng đúng con số thì mở
   * hết các ô kề còn lại. Cắm cờ sai thì có thể nổ, như luật gốc.
   */
  function chord(state, i, rand) {
    const cell = state.cells[i];
    if (state.done || !cell || !cell.open || cell.adj === 0) return { result: 'none', opened: [] };
    const around = neighbors(state, i);
    const flags = around.filter((n) => state.cells[n].flag).length;
    if (flags !== cell.adj) return { result: 'none', opened: [] };
    let result = 'none';
    const opened = [];
    for (const n of around) {
      const r = reveal(state, n, rand);
      opened.push(...r.opened);
      if (r.result === 'boom' || r.result === 'win') return { result: r.result, opened };
      if (r.result === 'open') result = 'open';
    }
    return { result, opened };
  }

  /** Số mìn còn lại theo người chơi (tổng mìn − số cờ), không âm. */
  function minesLeft(state) {
    return Math.max(0, state.mines - state.flags);
  }

  /**
   * Gợi ý: một ô chưa mở, chưa cắm cờ mà chắc chắn an toàn — ưu tiên ô nằm cạnh
   * vùng đã mở (người chơi thấy được lý do). Trả -1 nếu chưa có ván hoặc hết ô.
   */
  function safeHint(state) {
    if (!state.placed || state.done) return -1;
    let fallback = -1;
    for (let i = 0; i < state.cells.length; i++) {
      const c = state.cells[i];
      if (c.open || c.flag || c.mine) continue;
      if (neighbors(state, i).some((n) => state.cells[n].open)) return i;
      if (fallback < 0) fallback = i;
    }
    return fallback;
  }

  const api = { LEVELS, minesNew, neighbors, reveal, toggleFlag, chord, minesLeft, safeHint };
  if (typeof module !== 'undefined' && module.exports) module.exports = api;
  else root.MinesCore = api;
})(typeof window !== 'undefined' ? window : globalThis);
