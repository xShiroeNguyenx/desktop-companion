// Đồng hồ ghim (clock-pin.html) + chọn múi giờ trong Cài đặt: danh sách thành
// phố, giờ ở một múi (Intl, tự tính giờ mùa hè), chênh giờ và lệch ngày so với
// máy, đếm lùi Tập trung. Hàm thuần; dùng trong cửa sổ (window.ClockCore) và node --test.
(function (root) {
  // [múi IANA, vi, ja, en]. Không dùng cờ emoji: Windows không vẽ cờ.
  const ZONES = [
    ['Asia/Ho_Chi_Minh', 'Việt Nam', 'ベトナム', 'Vietnam'],
    ['Asia/Tokyo', 'Tokyo', '東京', 'Tokyo'],
    ['Asia/Seoul', 'Seoul', 'ソウル', 'Seoul'],
    ['Asia/Shanghai', 'Bắc Kinh', '北京', 'Beijing'],
    ['Asia/Taipei', 'Đài Bắc', '台北', 'Taipei'],
    ['Asia/Singapore', 'Singapore', 'シンガポール', 'Singapore'],
    ['Asia/Bangkok', 'Bangkok', 'バンコク', 'Bangkok'],
    ['Asia/Kolkata', 'Ấn Độ', 'インド', 'India'],
    ['Asia/Dubai', 'Dubai', 'ドバイ', 'Dubai'],
    ['Australia/Sydney', 'Sydney', 'シドニー', 'Sydney'],
    ['Europe/London', 'London', 'ロンドン', 'London'],
    ['Europe/Paris', 'Paris', 'パリ', 'Paris'],
    ['Europe/Berlin', 'Berlin', 'ベルリン', 'Berlin'],
    ['America/New_York', 'New York', 'ニューヨーク', 'New York'],
    ['America/Chicago', 'Chicago', 'シカゴ', 'Chicago'],
    ['America/Los_Angeles', 'Los Angeles', 'ロサンゼルス', 'Los Angeles'],
    ['UTC', 'UTC', 'UTC', 'UTC'],
  ];
  const COL = { vi: 1, ja: 2, en: 3 };

  const T = {
    vi: {
      weekdays: ['Chủ Nhật', 'Thứ Hai', 'Thứ Ba', 'Thứ Tư', 'Thứ Năm', 'Thứ Sáu', 'Thứ Bảy'],
      lunar: '{d}/{m} âm', yesterday: 'hôm qua', tomorrow: 'ngày mai', same: 'cùng giờ',
      focusIdle: '🍅 Tập trung {min} phút', focusOn: '🍅 Tập trung · còn {left}', breakOn: '☕ Nghỉ · còn {left}',
      focusTitle: 'Bắt đầu / dừng phiên Tập trung', top: 'Luôn nổi', unpin: 'Gỡ ghim', none: '— Không —',
    },
    ja: {
      weekdays: ['日曜日', '月曜日', '火曜日', '水曜日', '木曜日', '金曜日', '土曜日'],
      lunar: '旧暦{m}/{d}', yesterday: '昨日', tomorrow: '明日', same: '同じ時刻',
      focusIdle: '🍅 集中 {min}分', focusOn: '🍅 集中 · 残り{left}', breakOn: '☕ 休憩 · 残り{left}',
      focusTitle: '集中を開始／停止', top: '常に手前', unpin: '固定をはずす', none: '— なし —',
    },
    en: {
      weekdays: ['Sunday', 'Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday'],
      lunar: 'Lunar {d}/{m}', yesterday: 'yesterday', tomorrow: 'tomorrow', same: 'same time',
      focusIdle: '🍅 Focus {min} min', focusOn: '🍅 Focus · {left} left', breakOn: '☕ Break · {left} left',
      focusTitle: 'Start / stop a focus session', top: 'Always on top', unpin: 'Unpin', none: '— None —',
    },
  };

  function strings(lang) {
    return T[lang] || T.vi;
  }

  function fill(s, vars) {
    return String(s).replace(/\{(\w+)\}/g, (_, k) => (vars[k] != null ? vars[k] : ''));
  }

  /** Tên thành phố của múi `zone`; múi ngoài danh sách thì lấy đuôi tên ("America/Sao_Paulo" → "Sao Paulo"). */
  function zoneName(zone, lang) {
    const z = ZONES.find((r) => r[0] === zone);
    if (z) return z[COL[lang] || 1];
    return String(zone || '').split('/').pop().replace(/_/g, ' ');
  }

  const fmtCache = {};
  /**
   * Giờ / ngày tại múi `zone` lúc `ms`. Múi không có thật → null (Intl ném lỗi).
   * @returns {{year:number, month:number, day:number, hour:number, minute:number} | null}
   */
  function partsIn(ms, zone) {
    let f = fmtCache[zone];
    if (!f) {
      try {
        f = new Intl.DateTimeFormat('en-US', {
          timeZone: zone, hourCycle: 'h23',
          year: 'numeric', month: 'numeric', day: 'numeric', hour: 'numeric', minute: 'numeric',
        });
      } catch (_) {
        return null;
      }
      fmtCache[zone] = f;
    }
    const p = {};
    for (const x of f.formatToParts(new Date(ms))) p[x.type] = x.value;
    return { year: +p.year, month: +p.month, day: +p.day, hour: +p.hour % 24, minute: +p.minute };
  }

  /** Giờ máy (giờ địa phương của trình duyệt) lúc `ms`, cùng dạng với partsIn. */
  function localParts(ms) {
    const d = new Date(ms);
    return { year: d.getFullYear(), month: d.getMonth() + 1, day: d.getDate(), hour: d.getHours(), minute: d.getMinutes() };
  }

  // Phút tính từ một mốc chung, để so hai bộ giờ ở hai múi.
  function stamp(p) {
    return Date.UTC(p.year, p.month - 1, p.day, p.hour, p.minute) / 60000;
  }

  /**
   * Múi `zone` so với giờ máy lúc `ms`: lệch bao nhiêu phút (dương = đi trước)
   * và lệch ngày (-1 hôm qua, 0 cùng ngày, 1 ngày mai).
   */
  function compare(ms, zone, local) {
    const z = partsIn(ms, zone);
    if (!z) return null;
    const l = local || localParts(ms);
    const diff = Math.round(stamp(z) - stamp(l));
    const dayDiff = Math.round((Date.UTC(z.year, z.month - 1, z.day) - Date.UTC(l.year, l.month - 1, l.day)) / 86400000);
    return { parts: z, diffMin: diff, dayShift: Math.max(-1, Math.min(1, dayDiff)) };
  }

  /** Chênh giờ ngắn gọn: "+2h", "−11h", "+5h30", "" khi bằng nhau. */
  function diffLabel(min) {
    if (!min) return '';
    const sign = min > 0 ? '+' : '−';
    const a = Math.abs(min);
    const h = Math.floor(a / 60);
    const m = a % 60;
    return `${sign}${h}h${m ? String(m).padStart(2, '0') : ''}`;
  }

  const p2 = (x) => String(x).padStart(2, '0');

  function hhmm(p) {
    return `${p2(p.hour)}:${p2(p.minute)}`;
  }

  /** Đếm lùi: 1122 → "18:42", 3900 → "1:05:00", số âm / sai → "0:00". */
  function countdown(sec) {
    const s = Math.max(0, Math.floor(Number(sec) || 0));
    const h = Math.floor(s / 3600);
    const m = Math.floor((s % 3600) / 60);
    const r = s % 60;
    return h ? `${h}:${p2(m)}:${p2(r)}` : `${m}:${p2(r)}`;
  }

  /**
   * Biểu cảm Miu (media/game/rps-<face>.png): đang Tập trung thì "cố lên",
   * đang nghỉ thì nháy mắt, khuya (23:00–4:59) thì buồn ngủ, còn lại nháy mắt.
   */
  function face(phase, hour) {
    if (phase === 'focus') return 'fight';
    if (phase !== 'break' && (hour >= 23 || hour < 5)) return 'sleepy';
    return 'wink';
  }

  const api = { ZONES, T, strings, fill, zoneName, partsIn, localParts, compare, diffLabel, hhmm, countdown, face };
  if (typeof module !== 'undefined' && module.exports) module.exports = api;
  else root.ClockCore = api;
})(typeof window !== 'undefined' ? window : globalThis);
