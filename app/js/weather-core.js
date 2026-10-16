// Hiển thị thời tiết (thẻ dự báo weather.html + widget ghim weather-pin.html):
// icon / mô tả theo mã WMO, đổi °C ↔ °F, nhãn ngày giờ, toạ độ biểu đồ. Hàm
// thuần, không đụng DOM. Dữ liệu do Rust gửi (weather_net::view), nhiệt độ luôn
// bằng °C. Dùng trong cửa sổ (window.WeatherCore) và node --test.
(function (root) {
  const T = {
    vi: {
      today: 'Hôm nay', updated: 'Cập nhật {n} phút trước', justNow: 'Vừa cập nhật', hoursAgo: 'Cập nhật {n} giờ trước',
      never: 'Chưa có dữ liệu', stale: 'Chưa cập nhật được — đang hiện số liệu cũ', offline: 'Chưa lấy được dữ liệu thời tiết',
      hi: 'C', lo: 'T', pick: 'Chọn thành phố', search: 'Gõ tên thành phố…', find: 'Tìm', none: 'Không tìm thấy nơi nào tên như vậy',
      change: 'Đổi thành phố', pin: 'Ghim lên màn hình', unpin: 'Gỡ ghim', refresh: 'Làm mới', close: 'Đóng',
      useLive: '🌦 Bật hiệu ứng như ngoài trời', liveOn: '🌦 Hiệu ứng đang theo trời thật', credit: 'Dữ liệu thời tiết: Open-Meteo.com',
      noCity: 'Chọn thành phố để em xem thời tiết cho bạn nha~', rainNext: 'Mưa giờ tới', top: 'Luôn nổi', size: 'Đổi cỡ',
      privacy: 'Em chỉ gửi toạ độ thành phố bạn chọn tới Open-Meteo, không dò vị trí của bạn.',
      refreshing: 'Đang cập nhật…', pinnedNote: '📌 Đã ghim lên màn hình, ngay cạnh thẻ này. Kéo đi đâu cũng được~',
      unpinnedNote: 'Đã gỡ ghim thời tiết.',
      days: ['CN', 'T2', 'T3', 'T4', 'T5', 'T6', 'T7'],
    },
    ja: {
      today: '今日', updated: '{n}分前に更新', justNow: 'さっき更新', hoursAgo: '{n}時間前に更新',
      never: 'データなし', stale: '更新できません — 前のデータを表示中', offline: '天気を取得できませんでした',
      hi: '最高', lo: '最低', pick: '都市を選ぶ', search: '都市名を入力…', find: '検索', none: '見つかりませんでした',
      change: '都市を変える', pin: 'デスクトップに固定', unpin: '固定をはずす', refresh: '更新', close: '閉じる',
      useLive: '🌦 外と同じエフェクトにする', liveOn: '🌦 エフェクトは実際の天気どおり', credit: '天気データ: Open-Meteo.com',
      noCity: '都市を選んでね、お天気を見てあげる～', rainNext: '次の1時間の雨', top: '常に手前', size: 'サイズ',
      privacy: '送るのは選んだ都市の座標だけ。あなたの位置は調べないよ。',
      refreshing: '更新中…', pinnedNote: '📌 このカードの横に固定したよ。好きな場所に動かしてね～',
      unpinnedNote: '天気の固定をはずしたよ。',
      days: ['日', '月', '火', '水', '木', '金', '土'],
    },
    en: {
      today: 'Today', updated: 'Updated {n} min ago', justNow: 'Updated just now', hoursAgo: 'Updated {n} h ago',
      never: 'No data yet', stale: "Couldn't update — showing older data", offline: "Couldn't get the weather",
      hi: 'H', lo: 'L', pick: 'Choose a city', search: 'Type a city name…', find: 'Search', none: 'No place found with that name',
      change: 'Change city', pin: 'Pin to desktop', unpin: 'Unpin', refresh: 'Refresh', close: 'Close',
      useLive: '🌦 Match the effect to real weather', liveOn: '🌦 Effect follows the real weather', credit: 'Weather data: Open-Meteo.com',
      noCity: "Pick a city and I'll check the weather for you~", rainNext: 'Rain next hour', top: 'Always on top', size: 'Size',
      privacy: 'Only the coordinates of the city you choose are sent to Open-Meteo. Your location is never looked up.',
      refreshing: 'Updating…', pinnedNote: '📌 Pinned to your desktop, right next to this card. Drag it anywhere~',
      unpinnedNote: 'Weather unpinned.',
      days: ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'],
    },
  };

  // Nhóm mã WMO → khoá mô tả. https://open-meteo.com/en/docs (WMO Weather interpretation codes)
  const DESC = {
    vi: { clear: 'Trời quang', mostly: 'Ít mây', partly: 'Có mây', cloudy: 'Nhiều mây', fog: 'Sương mù', drizzle: 'Mưa phùn',
      rainL: 'Mưa nhẹ', rain: 'Mưa', rainH: 'Mưa to', freezing: 'Mưa lạnh', snowL: 'Tuyết nhẹ', snow: 'Tuyết', snowH: 'Tuyết dày',
      showers: 'Mưa rào', showersH: 'Mưa rào to', storm: 'Dông', unknown: 'Không rõ' },
    ja: { clear: '快晴', mostly: '晴れ', partly: '晴れ時々くもり', cloudy: 'くもり', fog: '霧', drizzle: '霧雨',
      rainL: '小雨', rain: '雨', rainH: '大雨', freezing: '着氷性の雨', snowL: '小雪', snow: '雪', snowH: '大雪',
      showers: 'にわか雨', showersH: '強いにわか雨', storm: '雷雨', unknown: '不明' },
    en: { clear: 'Clear sky', mostly: 'Mostly clear', partly: 'Partly cloudy', cloudy: 'Cloudy', fog: 'Fog', drizzle: 'Drizzle',
      rainL: 'Light rain', rain: 'Rain', rainH: 'Heavy rain', freezing: 'Freezing rain', snowL: 'Light snow', snow: 'Snow', snowH: 'Heavy snow',
      showers: 'Rain showers', showersH: 'Heavy showers', storm: 'Thunderstorm', unknown: 'Unknown' },
  };

  function group(code) {
    const c = Number(code);
    if (c === 0) return 'clear';
    if (c === 1) return 'mostly';
    if (c === 2) return 'partly';
    if (c === 3) return 'cloudy';
    if (c === 45 || c === 48) return 'fog';
    if (c >= 51 && c <= 57) return 'drizzle';
    if (c === 61) return 'rainL';
    if (c === 63) return 'rain';
    if (c === 65) return 'rainH';
    if (c === 66 || c === 67) return 'freezing';
    if (c === 71 || c === 77 || c === 85) return 'snowL';
    if (c === 73) return 'snow';
    if (c === 75 || c === 86) return 'snowH';
    if (c === 80 || c === 81) return 'showers';
    if (c === 82) return 'showersH';
    if (c >= 95 && c <= 99) return 'storm';
    return 'unknown';
  }

  const lang = (l) => (T[l] ? l : 'vi');
  const strings = (l) => T[lang(l)];
  function describe(code, l) {
    return DESC[lang(l)][group(code)];
  }

  function icon(code, isDay) {
    const g = group(code);
    const day = isDay !== false;
    return {
      clear: day ? '☀️' : '🌙', mostly: day ? '🌤️' : '🌙', partly: day ? '⛅' : '☁️', cloudy: '☁️', fog: '🌫️',
      drizzle: '🌦️', rainL: '🌦️', rain: '🌧️', rainH: '🌧️', freezing: '🌧️', snowL: '🌨️', snow: '🌨️', snowH: '❄️',
      showers: '🌦️', showersH: '🌧️', storm: '⛈️', unknown: '🌡️',
    }[g];
  }

  /** °C → số hiển thị theo đơn vị ('c' | 'f'), làm tròn. */
  function temp(c, unit) {
    const v = unit === 'f' ? (Number(c) * 9) / 5 + 32 : Number(c);
    return Math.round(v);
  }

  const fill = (s, vars) => String(s).replace(/\{(\w+)\}/g, (m, k) => (k in vars ? vars[k] : m));

  /** "Cập nhật 5 phút trước" (giây unix). */
  function ago(fetchedAt, now, l) {
    const L = strings(l);
    if (!fetchedAt) return L.never;
    const min = Math.max(0, Math.floor((now - fetchedAt) / 60));
    if (min < 1) return L.justNow;
    if (min < 60) return fill(L.updated, { n: min });
    return fill(L.hoursAgo, { n: Math.floor(min / 60) });
  }

  /** Nhãn ô ngày: "Hôm nay" cho ô đầu, còn lại "T7 17" / "Sat 17" / "17(土)". */
  function dayName(date, index, l) {
    const L = strings(l);
    if (index === 0) return L.today;
    const m = /^(\d{4})-(\d{2})-(\d{2})/.exec(String(date || ''));
    if (!m) return '';
    const wd = new Date(Date.UTC(+m[1], +m[2] - 1, +m[3])).getUTCDay();
    const d = String(+m[3]);
    return lang(l) === 'ja' ? `${d}(${L.days[wd]})` : `${L.days[wd]} ${d}`;
  }

  /** Giờ trong "2026-10-16T15:00": "15:00" (vi/ja), "3 PM" (en). */
  function hourLabel(time, l) {
    const m = /T(\d{2}):(\d{2})/.exec(String(time || ''));
    if (!m) return '';
    const h = +m[1];
    if (lang(l) !== 'en') return `${m[1]}:${m[2]}`;
    const h12 = h % 12 === 0 ? 12 : h % 12;
    return `${h12} ${h < 12 ? 'AM' : 'PM'}`;
  }

  /** "HH:MM" của giờ địa phương thành phố trong "2026-10-16T14:15". */
  function clock(time) {
    const m = /T(\d{2}:\d{2})/.exec(String(time || ''));
    return m ? m[1] : '';
  }

  /**
   * Biểu đồ nhiệt độ theo giờ trong khung w×h (px). Trả về các điểm (x, y),
   * đường cong `line`, vùng tô `area` (SVG path) và các mốc ghi nhãn mỗi `every` giờ.
   */
  function chart(hours, w, h, opts) {
    const o = { top: 22, bottom: 30, every: 5, ...(opts || {}) };
    const list = (hours || []).filter((x) => Number.isFinite(Number(x.temp)));
    if (list.length < 2) return { points: [], line: '', area: '', marks: [] };
    const temps = list.map((x) => Number(x.temp));
    const lo = Math.min(...temps);
    const hi = Math.max(...temps);
    const span = Math.max(1, hi - lo);
    const usable = h - o.top - o.bottom;
    const points = list.map((x, i) => ({
      x: (i / (list.length - 1)) * w,
      y: o.top + (1 - (Number(x.temp) - lo) / span) * usable,
      hour: x,
    }));
    const f = (v) => Math.round(v * 10) / 10;
    const line = points.map((p, i) => `${i ? 'L' : 'M'}${f(p.x)},${f(p.y)}`).join(' ');
    const base = h - o.bottom;
    const area = `${line} L${f(w)},${f(base)} L0,${f(base)} Z`;
    const marks = points.filter((_, i) => i % o.every === 0 || i === points.length - 1 && (points.length - 1) % o.every > o.every / 2);
    return { points, line, area, marks, lo, hi };
  }

  /**
   * Biểu cảm Miu (media/game/rps-<face>.png) cho thẻ dự báo và widget ghim.
   * trouble = mất mạng / số liệu cũ / chưa có dữ liệu → ngại; dông → bất ngờ;
   * đêm → buồn ngủ; trời quang ban ngày → reo; còn lại → nháy mắt.
   */
  function face(code, isDay, trouble) {
    if (trouble) return 'shy';
    const g = group(code);
    if (g === 'storm') return 'surprised';
    if (!isDay) return 'sleepy';
    return g === 'clear' || g === 'mostly' ? 'cheer' : 'wink';
  }

  const api = { T, strings, describe, icon, temp, ago, dayName, hourLabel, clock, chart, group, fill, face };
  if (typeof module !== 'undefined' && module.exports) module.exports = api;
  else root.WeatherCore = api;
})(typeof window !== 'undefined' ? window : globalThis);
