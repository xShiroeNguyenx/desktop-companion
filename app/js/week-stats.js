// Thẻ "📊 Tuần này" trong bảng Công việc: dựng chữ và chiều cao cột từ số liệu
// Rust gửi (stats.rs → week_json). Hàm thuần, dùng được cả trong trình duyệt
// (window.WeekStats) lẫn node --test.
(function (root) {
  // Chữ theo ngôn ngữ giao diện (I18n.lang()); mọi hàm nhận `lang`, mặc định "vi".
  const S = {
    vi: {
      days: ['T2', 'T3', 'T4', 'T5', 'T6', 'T7', 'CN'],
      min: (m) => `${m} phút`, hour: (h) => `${h} giờ`, hourMin: (h, m) => `${h} giờ ${m} phút`,
      tasks: (n) => `${n} việc`, streak: (n) => `${n} ngày liền`, same: 'bằng',
      footer: (t, f, q) => `So với tuần trước: việc ${t} · tập trung ${f} · 📋 xong nhiệm vụ ${q}/7 ngày`,
      sessions: (n, m) => `${n} phiên 🍅 (${m})`, quests: 'xong cả 3 nhiệm vụ',
    },
    ja: {
      days: ['月', '火', '水', '木', '金', '土', '日'],
      min: (m) => `${m}分`, hour: (h) => `${h}時間`, hourMin: (h, m) => `${h}時間${m}分`,
      tasks: (n) => `${n}件`, streak: (n) => `${n}日連続`, same: '同じ',
      footer: (t, f, q) => `先週比：タスク ${t} · 集中 ${f} · 📋 ミッション達成 ${q}/7日`,
      sessions: (n, m) => `🍅 ${n}回 (${m})`, quests: '3つのミッションを達成',
    },
    en: {
      days: ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'],
      min: (m) => `${m} min`, hour: (h) => `${h} h`, hourMin: (h, m) => `${h} h ${m} min`,
      tasks: (n) => `${n} task${n === 1 ? '' : 's'}`, streak: (n) => `${n}-day streak`, same: 'same',
      footer: (t, f, q) => `Vs last week: tasks ${t} · focus ${f} · 📋 quests done ${q}/7 days`,
      sessions: (n, m) => `${n} 🍅 session${n === 1 ? '' : 's'} (${m})`, quests: 'all 3 quests done',
    },
  };
  const L = (lang) => S[lang] || S.vi;
  const DAY_LABELS = S.vi.days;

  /** Nhãn 7 ngày, Thứ Hai trước. */
  function dayLabels(lang) {
    return L(lang).days;
  }

  /** "75 phút", "1 giờ 15 phút", "2 giờ". */
  function minutesText(min, lang) {
    const T = L(lang);
    const m = Math.max(0, Math.round(Number(min) || 0));
    if (m < 60) return T.min(m);
    const h = Math.floor(m / 60);
    return m % 60 ? T.hourMin(h, m % 60) : T.hour(h);
  }

  /** Dòng tóm tắt hiện cả khi thẻ đang gập. */
  function summary(week, lang) {
    const T = L(lang);
    const t = (week && week.total) || {};
    const parts = [`✅ ${T.tasks(t.tasks || 0)}`, `🍅 ${minutesText(t.focusMin, lang)}`];
    if (week && week.streak > 0) parts.push(`🔥 ${T.streak(week.streak)}`);
    return parts.join(' · ');
  }

  /** So với tuần trước: "+2", "−10 phút", "bằng". */
  function delta(now, prev, unit, lang) {
    const d = (Number(now) || 0) - (Number(prev) || 0);
    if (d === 0) return L(lang).same;
    const n = unit === 'min' ? minutesText(Math.abs(d), lang) : String(Math.abs(d));
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
  function footer(week, lang) {
    const t = (week && week.total) || {};
    const l = (week && week.last) || {};
    return L(lang).footer(delta(t.tasks, l.tasks, null, lang), delta(t.focusMin, l.focusMin, 'min', lang), t.questDays || 0);
  }

  /** Chú thích khi rê chuột lên một ngày. */
  function dayTitle(d, i, lang) {
    const T = L(lang);
    const bits = [`${T.days[i] || ''}: ${T.tasks(d.tasks || 0)}`, T.sessions(d.sessions || 0, minutesText(d.focusMin, lang))];
    if (d.quests) bits.push(T.quests);
    return bits.join(' · ');
  }

  const api = { DAY_LABELS, dayLabels, minutesText, summary, delta, bars, footer, dayTitle };
  if (typeof module !== 'undefined' && module.exports) module.exports = api;
  else root.WeekStats = api;
})(typeof window !== 'undefined' ? window : globalThis);
