// Thẻ "📊 Tuần này" trong bảng Công việc: dựng chữ và chiều cao cột từ số liệu
// Rust gửi (stats.rs → week_json). Hàm thuần, dùng được cả trong trình duyệt
// (window.WeekStats) lẫn node --test.
(function (root) {
  const DAY_LABELS = ['T2', 'T3', 'T4', 'T5', 'T6', 'T7', 'CN'];

  /** "75 phút", "1 giờ 15 phút", "2 giờ". */
  function minutesText(min) {
    const m = Math.max(0, Math.round(Number(min) || 0));
    if (m < 60) return `${m} phút`;
    const h = Math.floor(m / 60);
    return m % 60 ? `${h} giờ ${m % 60} phút` : `${h} giờ`;
  }

  /** Dòng tóm tắt hiện cả khi thẻ đang gập. */
  function summary(week) {
    const t = (week && week.total) || {};
    const parts = [`✅ ${t.tasks || 0} việc`, `🍅 ${minutesText(t.focusMin)}`];
    if (week && week.streak > 0) parts.push(`🔥 ${week.streak} ngày liền`);
    return parts.join(' · ');
  }

  /** So với tuần trước: "+2", "−10 phút", "bằng". */
  function delta(now, prev, unit) {
    const d = (Number(now) || 0) - (Number(prev) || 0);
    if (d === 0) return 'bằng';
    const n = unit === 'min' ? minutesText(Math.abs(d)) : String(Math.abs(d));
    return (d > 0 ? '+' : '−') + n;
  }

  /**
   * Chiều cao cột (0–100) theo phút tập trung. Mốc trên cùng ít nhất 25 phút
   * để một phiên lẻ không vọt kín cột; ngày có phút thì cột cao ít nhất 8.
   */
  function bars(days) {
    const list = days || [];
    const top = Math.max(25, ...list.map((d) => Number(d.focusMin) || 0));
    return list.map((d) => {
      const m = Number(d.focusMin) || 0;
      return m > 0 ? Math.max(8, Math.round((m * 100) / top)) : 0;
    });
  }

  /** Dòng cuối thẻ: so tuần trước + số ngày xong nhiệm vụ. */
  function footer(week) {
    const t = (week && week.total) || {};
    const l = (week && week.last) || {};
    return (
      `So với tuần trước: việc ${delta(t.tasks, l.tasks)} · tập trung ${delta(t.focusMin, l.focusMin, 'min')}` +
      ` · 📋 xong nhiệm vụ ${t.questDays || 0}/7 ngày`
    );
  }

  /** Chú thích khi rê chuột lên một ngày. */
  function dayTitle(d, i) {
    const bits = [`${DAY_LABELS[i] || ''}: ${d.tasks || 0} việc`, `${d.sessions || 0} phiên 🍅 (${minutesText(d.focusMin)})`];
    if (d.quests) bits.push('xong cả 3 nhiệm vụ');
    return bits.join(' · ');
  }

  const api = { DAY_LABELS, minutesText, summary, delta, bars, footer, dayTitle };
  if (typeof module !== 'undefined' && module.exports) module.exports = api;
  else root.WeekStats = api;
})(typeof window !== 'undefined' ? window : globalThis);
