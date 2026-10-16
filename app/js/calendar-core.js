// Lịch tháng ghim trên desktop (calendar-pin.html): lưới tháng bắt đầu từ thứ
// Hai, đổi dương → âm lịch Việt Nam, ngày lễ, can chi, chấm hạn chót của task.
// Hàm thuần, không đụng DOM. Dùng trong cửa sổ (window.CalendarCore) và node --test.
//
// Âm lịch: thuật toán của Hồ Ngọc Đức (tính điểm sóc và trung khí theo thiên
// văn), múi giờ +7. Lịch Trung Quốc tính theo +8 nên có năm lệch tháng nhuận
// hoặc lệch Tết một ngày (vd. Tết 1985); dùng +7 mới khớp lịch Việt Nam.
(function (root) {
  const TZ = 7;
  const PI = Math.PI;
  const INT = Math.floor;

  // ─── Âm lịch ───────────────────────────────────────────────────────────────

  /** Số ngày Julius của một ngày dương lịch (lịch Gregory). */
  function jdFromDate(dd, mm, yy) {
    const a = INT((14 - mm) / 12);
    const y = yy + 4800 - a;
    const m = mm + 12 * a - 3;
    return dd + INT((153 * m + 2) / 5) + 365 * y + INT(y / 4) - INT(y / 100) + INT(y / 400) - 32045;
  }

  /** Ngày (số Julius, theo giờ địa phương) của điểm sóc thứ k kể từ 1/1/1900. */
  function newMoonDay(k) {
    const T = k / 1236.85;
    const T2 = T * T;
    const T3 = T2 * T;
    const dr = PI / 180;
    let jd1 = 2415020.75933 + 29.53058868 * k + 0.0001178 * T2 - 0.000000155 * T3;
    jd1 += 0.00033 * Math.sin((166.56 + 132.87 * T - 0.009173 * T2) * dr);
    const M = 359.2242 + 29.10535608 * k - 0.0000333 * T2 - 0.00000347 * T3;
    const Mpr = 306.0253 + 385.81691806 * k + 0.0107306 * T2 + 0.00001236 * T3;
    const F = 21.2964 + 390.67050646 * k - 0.0016528 * T2 - 0.00000239 * T3;
    let c1 = (0.1734 - 0.000393 * T) * Math.sin(M * dr) + 0.0021 * Math.sin(2 * dr * M);
    c1 = c1 - 0.4068 * Math.sin(Mpr * dr) + 0.0161 * Math.sin(dr * 2 * Mpr);
    c1 = c1 - 0.0004 * Math.sin(dr * 3 * Mpr);
    c1 = c1 + 0.0104 * Math.sin(dr * 2 * F) - 0.0051 * Math.sin(dr * (M + Mpr));
    c1 = c1 - 0.0074 * Math.sin(dr * (M - Mpr)) + 0.0004 * Math.sin(dr * (2 * F + M));
    c1 = c1 - 0.0004 * Math.sin(dr * (2 * F - M)) - 0.0006 * Math.sin(dr * (2 * F + Mpr));
    c1 = c1 + 0.001 * Math.sin(dr * (2 * F - Mpr)) + 0.0005 * Math.sin(dr * (2 * Mpr + M));
    const deltaT = T < -11
      ? 0.001 + 0.000839 * T + 0.0002261 * T2 - 0.00000845 * T3 - 0.000000081 * T * T3
      : -0.000278 + 0.000265 * T + 0.000262 * T2;
    return INT(jd1 + c1 - deltaT + 0.5 + TZ / 24);
  }

  /** Cung hoàng đạo (0..11, mỗi cung 30°) của Mặt Trời lúc đầu ngày `jdn`. */
  function sunLongitude(jdn) {
    const T = (jdn - 2451545.5 - TZ / 24) / 36525;
    const T2 = T * T;
    const dr = PI / 180;
    const M = 357.5291 + 35999.0503 * T - 0.0001559 * T2 - 0.00000048 * T * T2;
    const L0 = 280.46645 + 36000.76983 * T + 0.0003032 * T2;
    let DL = (1.9146 - 0.004817 * T - 0.000014 * T2) * Math.sin(dr * M);
    DL += (0.019993 - 0.000101 * T) * Math.sin(dr * 2 * M) + 0.00029 * Math.sin(dr * 3 * M);
    let L = (L0 + DL) * dr;
    L -= PI * 2 * INT(L / (PI * 2));
    return INT((L / PI) * 6);
  }

  /** Ngày bắt đầu tháng 11 âm (tháng chứa Đông chí) của năm dương `yy`. */
  function lunarMonth11(yy) {
    const off = jdFromDate(31, 12, yy) - 2415021;
    const k = INT(off / 29.530588853);
    const nm = newMoonDay(k);
    return sunLongitude(nm) >= 9 ? newMoonDay(k - 1) : nm;
  }

  /** Vị trí tháng nhuận (tính từ sau tháng 11 âm `a11`): tháng đầu tiên không có trung khí. */
  function leapMonthOffset(a11) {
    const k = INT((a11 - 2415021.076998695) / 29.530588853 + 0.5);
    let last;
    let i = 1;
    let arc = sunLongitude(newMoonDay(k + i));
    do {
      last = arc;
      i++;
      arc = sunLongitude(newMoonDay(k + i));
    } while (arc !== last && i < 14);
    return i - 1;
  }

  /**
   * Dương → âm lịch Việt Nam.
   * @returns {{day: number, month: number, year: number, leap: boolean}}
   */
  function toLunar(dd, mm, yy) {
    const dayNumber = jdFromDate(dd, mm, yy);
    const k = INT((dayNumber - 2415021.076998695) / 29.530588853);
    let monthStart = newMoonDay(k + 1);
    if (monthStart > dayNumber) monthStart = newMoonDay(k);
    let a11 = lunarMonth11(yy);
    let b11 = a11;
    let year;
    if (a11 >= monthStart) {
      year = yy;
      a11 = lunarMonth11(yy - 1);
    } else {
      year = yy + 1;
      b11 = lunarMonth11(yy + 1);
    }
    const day = dayNumber - monthStart + 1;
    const diff = INT((monthStart - a11) / 29);
    let leap = false;
    let month = diff + 11;
    if (b11 - a11 > 365) {
      const leapDiff = leapMonthOffset(a11);
      if (diff >= leapDiff) {
        month = diff + 10;
        if (diff === leapDiff) leap = true;
      }
    }
    if (month > 12) month -= 12;
    if (month >= 11 && diff < 4) year -= 1;
    return { day, month, year, leap };
  }

  // ─── Can chi ───────────────────────────────────────────────────────────────

  const CAN = {
    vi: ['Giáp', 'Ất', 'Bính', 'Đinh', 'Mậu', 'Kỷ', 'Canh', 'Tân', 'Nhâm', 'Quý'],
    ja: ['甲', '乙', '丙', '丁', '戊', '己', '庚', '辛', '壬', '癸'],
  };
  const CHI = {
    vi: ['Tý', 'Sửu', 'Dần', 'Mão', 'Thìn', 'Tỵ', 'Ngọ', 'Mùi', 'Thân', 'Dậu', 'Tuất', 'Hợi'],
    ja: ['子', '丑', '寅', '卯', '辰', '巳', '午', '未', '申', '酉', '戌', '亥'],
    // Con giáp kiểu Việt: Mão là Mèo, không phải Thỏ.
    en: ['Rat', 'Ox', 'Tiger', 'Cat', 'Dragon', 'Snake', 'Horse', 'Goat', 'Monkey', 'Rooster', 'Dog', 'Pig'],
  };

  /** Tên năm âm lịch: "Bính Ngọ" / "丙午" / "Year of the Horse". */
  function canChi(lunarYear, lang) {
    const can = (lunarYear + 6) % 10;
    const chi = (lunarYear + 8) % 12;
    if (lang === 'ja') return CAN.ja[can] + CHI.ja[chi];
    if (lang === 'en') return `Year of the ${CHI.en[chi]}`;
    return `${CAN.vi[can]} ${CHI.vi[chi]}`;
  }

  // ─── Lưới tháng ────────────────────────────────────────────────────────────

  function daysInMonth(year, month) {
    return new Date(year, month, 0).getDate();
  }

  /** Lùi / tiến `delta` tháng (month 1..12). */
  function addMonths(year, month, delta) {
    const n = year * 12 + (month - 1) + delta;
    return { year: INT(n / 12), month: (n % 12 + 12) % 12 + 1 };
  }

  /**
   * Các ô của tháng (month 1..12), tuần bắt đầu từ thứ Hai, đủ số hàng (4..6)
   * để chứa trọn tháng, ô ngoài tháng là ngày của tháng trước / sau.
   * @returns {{year:number, month:number, day:number, inMonth:boolean, weekday:number}[]}
   *          weekday: 0 = thứ Hai … 6 = Chủ nhật.
   */
  function monthGrid(year, month) {
    const lead = (new Date(year, month - 1, 1).getDay() + 6) % 7;
    const total = Math.ceil((lead + daysInMonth(year, month)) / 7) * 7;
    const cells = [];
    for (let i = 0; i < total; i++) {
      const d = new Date(year, month - 1, 1 - lead + i);
      cells.push({
        year: d.getFullYear(),
        month: d.getMonth() + 1,
        day: d.getDate(),
        inMonth: d.getMonth() + 1 === month,
        weekday: i % 7,
      });
    }
    return cells;
  }

  /** Khoá ngày địa phương "2026-10-16" (để so sánh / gom task). */
  function dayKey(year, month, day) {
    return `${year}-${String(month).padStart(2, '0')}-${String(day).padStart(2, '0')}`;
  }

  function keyOfDate(d) {
    return dayKey(d.getFullYear(), d.getMonth() + 1, d.getDate());
  }

  // ─── Ngày lễ ───────────────────────────────────────────────────────────────

  // off = ngày nghỉ lễ chính thức (tô đỏ). Khoá "m-d".
  const SOLAR_DAYS = {
    '1-1': { id: 'newYear', off: true },
    '2-14': { id: 'valentine' },
    '3-8': { id: 'women' },
    '4-30': { id: 'reunification', off: true },
    '5-1': { id: 'labour', off: true },
    '6-1': { id: 'children' },
    '9-2': { id: 'national', off: true },
    '10-20': { id: 'vnWomen' },
    '11-20': { id: 'teachers' },
    '12-24': { id: 'xmasEve' },
    '12-25': { id: 'xmas' },
  };
  const LUNAR_DAYS = {
    '1-1': { id: 'tet1', off: true },
    '1-2': { id: 'tet2', off: true },
    '1-3': { id: 'tet3', off: true },
    '1-15': { id: 'lantern' },
    '3-10': { id: 'hung', off: true },
    '4-15': { id: 'vesak' },
    '5-5': { id: 'doanNgo' },
    '7-15': { id: 'vuLan' },
    '8-15': { id: 'midAutumn' },
    '12-23': { id: 'kitchen' },
  };

  /**
   * Ngày lễ của một ngày (dương + âm). Tháng nhuận không có lễ (Rằm tháng 7
   * nhuận không phải Vu Lan). Giao thừa = ngày cuối năm âm, 29 hoặc 30 tháng Chạp.
   * @returns {{id: string, off: boolean}[]}
   */
  function holidays(year, month, day, lunar) {
    const out = [];
    const s = SOLAR_DAYS[`${month}-${day}`];
    if (s) out.push({ id: s.id, off: !!s.off });
    const l = lunar || toLunar(day, month, year);
    if (!l.leap) {
      const h = LUNAR_DAYS[`${l.month}-${l.day}`];
      if (h) out.push({ id: h.id, off: !!h.off });
      if (l.month === 12 && l.day >= 29) {
        const next = new Date(year, month - 1, day + 1);
        const n = toLunar(next.getDate(), next.getMonth() + 1, next.getFullYear());
        if (n.month === 1 && n.day === 1 && !n.leap) out.push({ id: 'newYearEve', off: true });
      }
    }
    return out;
  }

  // ─── Task có hạn chót ──────────────────────────────────────────────────────

  /**
   * Gom task chưa xong có hạn chót theo ngày địa phương.
   * @param {{id:string,text:string,done:boolean,due_at:string}[]} tasks
   * @returns {Object<string, {id:string,text:string,due:number,over:boolean}[]>}
   *          khoá dayKey, mỗi ngày sắp theo giờ hạn.
   */
  function tasksByDay(tasks, now) {
    const out = {};
    for (const t of tasks || []) {
      const due = Number(t && t.due_at);
      if (!t || t.done || !Number.isFinite(due) || due <= 0) continue;
      const key = keyOfDate(new Date(due));
      (out[key] = out[key] || []).push({ id: t.id, text: t.text, due, over: due < now });
    }
    for (const k of Object.keys(out)) out[k].sort((a, b) => a.due - b.due);
    return out;
  }

  /** Mốc millis cho "ngày `key` lúc `hhmm`" theo giờ địa phương, null nếu sai. */
  function dueAt(key, hhmm) {
    const m = /^(\d{4})-(\d{2})-(\d{2})$/.exec(key || '');
    const t = /^(\d{1,2}):(\d{2})$/.exec(hhmm || '');
    if (!m || !t || +t[1] > 23 || +t[2] > 59) return null;
    const ms = new Date(+m[1], +m[2] - 1, +m[3], +t[1], +t[2]).getTime();
    return Number.isFinite(ms) ? ms : null;
  }

  // ─── Chữ ───────────────────────────────────────────────────────────────────

  const T = {
    vi: {
      title: 'Tháng {m}, {y}', weekdays: ['T2', 'T3', 'T4', 'T5', 'T6', 'T7', 'CN'],
      weekdaysLong: ['Thứ Hai', 'Thứ Ba', 'Thứ Tư', 'Thứ Năm', 'Thứ Sáu', 'Thứ Bảy', 'Chủ Nhật'],
      lunarMonth: 'Tháng {m} âm · {cc}', lunarFull: 'Âm lịch {d}/{m}{leap} · năm {cc}', leap: ' nhuận',
      full: 'Rằm', today: 'Hôm nay', prev: 'Tháng trước', next: 'Tháng sau', top: 'Luôn nổi',
      unpin: 'Gỡ ghim', add: '＋ Việc', addTitle: 'Thêm việc có hạn vào ngày này', placeholder: 'Việc cần làm…',
      save: 'Lưu', cancel: 'Huỷ', noTasks: 'Không có việc nào tới hạn', more: '+{n} việc nữa',
      overdue: 'quá hạn', saved: 'Đã thêm việc ✓', openTasks: 'Mở bảng Công việc',
      h: {
        newYear: 'Tết Dương lịch', valentine: 'Lễ Tình nhân', women: 'Quốc tế Phụ nữ',
        reunification: 'Giải phóng miền Nam', labour: 'Quốc tế Lao động', children: 'Quốc tế Thiếu nhi',
        national: 'Quốc khánh', vnWomen: 'Phụ nữ Việt Nam', teachers: 'Nhà giáo Việt Nam',
        xmasEve: 'Đêm Giáng sinh', xmas: 'Giáng sinh', tet1: 'Mùng 1 Tết', tet2: 'Mùng 2 Tết', tet3: 'Mùng 3 Tết',
        lantern: 'Rằm tháng Giêng', hung: 'Giỗ Tổ Hùng Vương', vesak: 'Lễ Phật đản', doanNgo: 'Tết Đoan ngọ',
        vuLan: 'Lễ Vu Lan', midAutumn: 'Tết Trung thu', kitchen: 'Ông Công Ông Táo', newYearEve: 'Giao thừa',
      },
    },
    ja: {
      title: '{y}年{m}月', weekdays: ['月', '火', '水', '木', '金', '土', '日'],
      weekdaysLong: ['月曜日', '火曜日', '水曜日', '木曜日', '金曜日', '土曜日', '日曜日'],
      lunarMonth: '旧暦{m}月 · {cc}', lunarFull: '旧暦 {m}月{d}日{leap} · {cc}年', leap: '（閏）',
      full: '満月', today: '今日', prev: '前の月', next: '次の月', top: '常に手前',
      unpin: '固定をはずす', add: '＋ タスク', addTitle: 'この日が期限のタスクを追加', placeholder: 'やること…',
      save: '保存', cancel: 'やめる', noTasks: '期限のタスクはありません', more: 'ほか{n}件',
      overdue: '期限切れ', saved: '追加したよ ✓', openTasks: 'タスク一覧を開く',
      h: {
        newYear: '元日', valentine: 'バレンタイン', women: '国際女性デー',
        reunification: '南部解放記念日', labour: 'メーデー', children: '国際子どもの日',
        national: 'ベトナム建国記念日', vnWomen: 'ベトナム女性の日', teachers: 'ベトナム教師の日',
        xmasEve: 'クリスマスイブ', xmas: 'クリスマス', tet1: 'テト（旧正月）', tet2: 'テト2日目', tet3: 'テト3日目',
        lantern: '元宵節', hung: 'フン王の命日', vesak: '仏誕節', doanNgo: '端午節',
        vuLan: '盂蘭盆', midAutumn: '中秋節', kitchen: 'かまど神の日', newYearEve: '大晦日（旧暦）',
      },
    },
    en: {
      title: '{M} {y}', weekdays: ['Mo', 'Tu', 'We', 'Th', 'Fr', 'Sa', 'Su'],
      weekdaysLong: ['Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday', 'Sunday'],
      lunarMonth: 'Lunar month {m} · {cc}', lunarFull: 'Lunar {d}/{m}{leap} · {cc}', leap: ' (leap)',
      full: 'Full', today: 'Today', prev: 'Previous month', next: 'Next month', top: 'Always on top',
      unpin: 'Unpin', add: '＋ Task', addTitle: 'Add a task due on this day', placeholder: 'Something to do…',
      save: 'Save', cancel: 'Cancel', noTasks: 'Nothing due', more: '+{n} more',
      overdue: 'overdue', saved: 'Task added ✓', openTasks: 'Open Tasks',
      h: {
        newYear: "New Year's Day", valentine: "Valentine's Day", women: "International Women's Day",
        reunification: 'Reunification Day', labour: 'Labour Day', children: "Children's Day",
        national: 'National Day', vnWomen: "Vietnamese Women's Day", teachers: "Teachers' Day",
        xmasEve: 'Christmas Eve', xmas: 'Christmas', tet1: 'Tết (Lunar New Year)', tet2: 'Tết, day 2', tet3: 'Tết, day 3',
        lantern: 'Lantern Festival', hung: 'Hung Kings Day', vesak: "Buddha's Birthday", doanNgo: 'Dragon Boat Festival',
        vuLan: 'Vu Lan', midAutumn: 'Mid-Autumn Festival', kitchen: 'Kitchen Gods Day', newYearEve: 'Lunar New Year Eve',
      },
    },
  };
  const MONTHS_EN = ['January', 'February', 'March', 'April', 'May', 'June', 'July', 'August', 'September', 'October', 'November', 'December'];

  function strings(lang) {
    return T[lang] || T.vi;
  }

  function fill(s, vars) {
    return String(s).replace(/\{(\w+)\}/g, (_, k) => (vars[k] != null ? vars[k] : ''));
  }

  /** Tiêu đề lưới: "Tháng 10, 2026" / "2026年10月" / "October 2026". */
  function monthTitle(year, month, lang) {
    return fill(strings(lang).title, { y: year, m: month, M: MONTHS_EN[month - 1] });
  }

  /** Chữ âm lịch nhỏ trong ô: "1/9" ngày đầu tháng, "Rằm" ngày 15, còn lại là số ngày. */
  function lunarLabel(lunar, lang) {
    if (lunar.day === 1) return `${lunar.day}/${lunar.month}${lunar.leap ? '*' : ''}`;
    if (lunar.day === 15) return strings(lang).full;
    return String(lunar.day);
  }

  const api = {
    T, strings, fill, toLunar, jdFromDate, canChi, daysInMonth, addMonths, monthGrid,
    dayKey, keyOfDate, holidays, tasksByDay, dueAt, monthTitle, lunarLabel,
  };
  if (typeof module !== 'undefined' && module.exports) module.exports = api;
  else root.CalendarCore = api;
})(typeof window !== 'undefined' ? window : globalThis);
