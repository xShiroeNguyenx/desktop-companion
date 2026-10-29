// 🛰 Theo dõi website: chữ và định dạng cho widget (monitor-pin.html) và Cài
// đặt. Hàm thuần, dùng trong cửa sổ (window.MonitorCore) và node --test.
// Kết quả do Rust gửi: { state: "up", ms, status } | { state: "down", reason }.
(function (root) {
  const T = {
    vi: {
      title: 'Theo dõi website', top: 'Luôn nổi', unpin: 'Gỡ ghim', running: '{up}/{n} đang chạy',
      checking: 'đang kiểm…', empty: 'Chưa có site nào. Thêm trong Cài đặt → 🛰 Theo dõi website.',
      every: 'kiểm mỗi {m} phút', open: 'Bấm để mở trong trình duyệt',
      reasons: { timeout: 'Hết giờ', dns: 'Lỗi DNS', refused: 'Từ chối', tls: 'Lỗi SSL', error: 'Lỗi kết nối' },
    },
    ja: {
      title: 'サイト監視', top: '常に手前', unpin: '固定をはずす', running: '{up}/{n} 稼働中',
      checking: '確認中…', empty: 'サイトがありません。設定 → 🛰 で追加してね。',
      every: '{m}分ごとに確認', open: 'クリックでブラウザで開く',
      reasons: { timeout: 'タイムアウト', dns: 'DNS エラー', refused: '接続拒否', tls: 'SSL エラー', error: '接続エラー' },
    },
    en: {
      title: 'Site monitor', top: 'Always on top', unpin: 'Unpin', running: '{up}/{n} up',
      checking: 'checking…', empty: 'No sites yet. Add them in Settings → 🛰 Site monitor.',
      every: 'checked every {m} min', open: 'Click to open in the browser',
      reasons: { timeout: 'Timeout', dns: 'DNS error', refused: 'Refused', tls: 'SSL error', error: 'Error' },
    },
  };

  function strings(lang) {
    return T[lang] || T.vi;
  }

  function fill(s, vars) {
    return String(s).replace(/\{(\w+)\}/g, (_, k) => (vars[k] != null ? vars[k] : ''));
  }

  /** 85 → "85 ms", 1234 → "1.2 s". */
  function fmtMs(ms) {
    const n = Number(ms);
    if (ms == null || !Number.isFinite(n) || n < 0) return '';
    return n < 1000 ? `${Math.round(n)} ms` : `${(n / 1000).toFixed(1)} s`;
  }

  /** Chữ ngắn cho một kết quả: "120 ms", "502", "Hết giờ"… */
  function resultText(last, lang) {
    if (!last) return strings(lang).checking;
    if (last.state === 'up') return fmtMs(last.ms);
    const r = String(last.reason || '');
    if (r.startsWith('http')) return r.slice(4);
    return strings(lang).reasons[r] || strings(lang).reasons.error;
  }

  /**
   * Trạng thái chấm màu: "up" | "down" | "warn" (lỗi một lần, chưa đủ để coi
   * là sập) | "wait" (chưa kiểm). `up` là kết luận của Rust (null = chưa kết luận).
   */
  function dot(site) {
    if (!site || !site.last) return 'wait';
    if (site.up === false) return 'down';
    if (site.last.state === 'up') return 'up';
    return 'warn';
  }

  /** "3/4 đang chạy": đếm site không ở trạng thái sập. */
  function summary(sites, lang) {
    const list = sites || [];
    const up = list.filter((s) => dot(s) !== 'down').length;
    return fill(strings(lang).running, { up, n: list.length });
  }

  const api = { T, strings, fill, fmtMs, resultText, dot, summary };
  if (typeof module !== 'undefined' && module.exports) module.exports = api;
  else root.MonitorCore = api;
})(typeof window !== 'undefined' ? window : globalThis);
