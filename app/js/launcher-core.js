// 🔎 Ô mở nhanh (launcher.html): tách lệnh người dùng gõ, điền từ khoá vào
// URL, xếp hạng bookmark. Hàm thuần, dùng trong cửa sổ (window.LauncherCore)
// và node --test. Tìm không dấu nhờ NotesCore.fold. Ý tưởng "bang" từ widget
// Search của Glance / DuckDuckGo — tự viết lại.
(function (root) {
  const fold = (typeof module !== 'undefined' && module.exports)
    ? require('./notes-core.js').fold
    : root.NotesCore.fold;

  /** Bang có sẵn: [lệnh, URL có {q}, tên]. Bang tự thêm trùng tên thì thắng. */
  const DEFAULT_BANGS = [
    ['!g', 'https://www.google.com/search?q={q}', 'Google'],
    ['!yt', 'https://www.youtube.com/results?search_query={q}', 'YouTube'],
    ['!gh', 'https://github.com/search?q={q}', 'GitHub'],
    ['!w', 'https://vi.wikipedia.org/w/index.php?search={q}', 'Wikipedia'],
    ['!map', 'https://www.google.com/maps/search/{q}', 'Google Maps'],
    ['!tr', 'https://translate.google.com/?sl=auto&tl=vi&text={q}', 'Google Dịch'],
    ['!so', 'https://stackoverflow.com/search?q={q}', 'Stack Overflow'],
    ['!npm', 'https://www.npmjs.com/search?q={q}', 'npm'],
    ['!ddg', 'https://duckduckgo.com/?q={q}', 'DuckDuckGo'],
  ];

  const ENGINES = {
    google: ['https://www.google.com/search?q={q}', 'Google'],
    duckduckgo: ['https://duckduckgo.com/?q={q}', 'DuckDuckGo'],
    bing: ['https://www.bing.com/search?q={q}', 'Bing'],
  };

  /** Chỉ mở http / https có tên máy (Rust kiểm lại lần nữa). */
  function isSafe(url) {
    return /^https?:\/\/[^\s/?#]+/i.test(String(url || ''));
  }

  /** Điền từ khoá (mã hoá) vào chỗ {q}. Kết quả không an toàn thì null. */
  function fill(template, q) {
    const url = String(template || '').split('{q}').join(encodeURIComponent(q));
    return isSafe(url) ? url : null;
  }

  /** "example.com/docs", "app-01.example.net:8443" → coi là địa chỉ web. */
  function toUrl(text) {
    const s = String(text || '').trim();
    if (/\s/.test(s)) return null;
    if (/^https?:\/\//i.test(s)) return isSafe(s) ? s : null;
    if (/^[\w-]+(\.[\w-]+)+(:\d+)?([/?#]\S*)?$/.test(s) && /[a-z]/i.test(s.split(/[/?#:]/)[0].split('.').pop())) {
      return 'https://' + s;
    }
    return null;
  }

  /** Bảng bang: mặc định + tự thêm ([[key, url], …] từ Rust). */
  function bangTable(custom) {
    const map = new Map(DEFAULT_BANGS.map(([k, u, n]) => [k, { key: k, url: u, name: n }]));
    for (const [k, u] of custom || []) {
      const key = String(k).toLowerCase();
      let name = key.slice(1);
      try { name = new URL(u.split('{q}').join('x')).hostname; } catch (_) { /* giữ tên lệnh */ }
      map.set(key, { key, url: u, name });
    }
    return map;
  }

  /** Điểm khớp bookmark với từ khoá (0 = không khớp). */
  function score(b, q) {
    const words = fold(q).split(/\s+/).filter(Boolean);
    if (!words.length) return 0;
    const title = fold(b.title);
    const rest = fold(`${b.group || ''} ${b.url || ''}`);
    let total = 0;
    for (const w of words) {
      if (title.startsWith(w)) total += 3;
      else if (title.split(/\s+/).some((t) => t.startsWith(w))) total += 2;
      else if (title.includes(w)) total += 1;
      else if (rest.includes(w)) total += 0.5;
      else return 0; // mọi từ phải khớp ở đâu đó
    }
    return total;
  }

  function rank(bookmarks, q, limit = 6) {
    return (bookmarks || [])
      .map((b) => ({ b, s: score(b, q) }))
      .filter((x) => x.s > 0)
      .sort((x, y) => y.s - x.s || String(x.b.title).localeCompare(String(y.b.title)))
      .slice(0, limit)
      .map((x) => x.b);
  }

  /**
   * Danh sách dòng kết quả cho chữ đang gõ. Mỗi dòng:
   * { type: 'bang'|'hint'|'url'|'bookmark'|'search', title, sub, url?, insert? }
   * Dòng đầu là dòng Enter sẽ mở.
   */
  function results(input, data) {
    const text = String(input || '');
    const s = text.trim();
    if (!s) return [];
    const bangs = bangTable(data && data.bangs);
    const engine = ENGINES[data && data.engine] || ENGINES.google;
    const rows = [];

    if (s.startsWith('!')) {
      const [tok, ...rest] = s.split(/\s+/);
      const key = tok.toLowerCase();
      const q = rest.join(' ');
      const bang = bangs.get(key);
      if (bang && (q || /\s$/.test(text))) {
        const url = q ? fill(bang.url, q) : fill(bang.url, '').replace(/[?&#][^?&#]*=?$/, '');
        if (url) rows.push({ type: 'bang', title: q ? `${bang.name}: ${q}` : bang.name, sub: key, url });
        return rows;
      }
      // Đang gõ tên bang: gợi ý, Tab / Enter để điền.
      for (const b of bangs.values()) {
        if (b.key.startsWith(key)) rows.push({ type: 'hint', title: `${b.key} ${b.name}`, sub: b.url.replace('{q}', '…'), insert: b.key + ' ' });
      }
      if (rows.length) return rows.slice(0, 8);
    }

    const direct = toUrl(s);
    if (direct) rows.push({ type: 'url', title: s, sub: direct, url: direct });
    for (const b of rank(data && data.bookmarks, s)) {
      rows.push({ type: 'bookmark', title: b.title, sub: (b.group ? b.group + ' · ' : '') + b.url, url: b.url });
    }
    const search = fill(engine[0], s);
    if (search) rows.push({ type: 'search', title: s, sub: engine[1], url: search });
    return rows;
  }

  const api = { DEFAULT_BANGS, ENGINES, isSafe, fill, toUrl, bangTable, score, rank, results };
  if (typeof module !== 'undefined' && module.exports) module.exports = api;
  else root.LauncherCore = api;
})(typeof window !== 'undefined' ? window : globalThis);
