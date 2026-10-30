// Widget Thông số máy (sysmon-pin.html): định dạng số, mức màu, mặt Miu, biểu
// đồ CPU. Hàm thuần, dùng trong cửa sổ (window.SysmonCore) và node --test.
// Số đo do Rust gửi (sysmon_pin.rs → "sysmon-stats").
(function (root) {
  const UNITS = ['B', 'KB', 'MB', 'GB', 'TB'];

  /** 1536 → "1.5 KB", 476 GB → "476 GB": dưới 10 thì một số lẻ, còn lại số tròn. */
  function fmtBytes(n) {
    let v = Number(n);
    if (!Number.isFinite(v) || v < 0) return '–';
    let i = 0;
    while (v >= 1024 && i < UNITS.length - 1) {
      v /= 1024;
      i++;
    }
    const s = i === 0 || v >= 10 ? String(Math.round(v)) : v.toFixed(1).replace(/\.0$/, '');
    return `${s} ${UNITS[i]}`;
  }

  function fmtRate(n) {
    return fmtBytes(n) + '/s';
  }

  /** Thời gian máy đã chạy: "3 ngày 4 giờ", "5 giờ 12 phút", "8 phút". */
  function fmtUptime(secs, lang) {
    const t = strings(lang);
    const s = Math.max(0, Math.floor(Number(secs) || 0));
    const d = Math.floor(s / 86400);
    const h = Math.floor((s % 86400) / 3600);
    const m = Math.floor((s % 3600) / 60);
    if (d > 0) return fill(t.dh, { d, h });
    if (h > 0) return fill(t.hm, { h, m });
    return fill(t.m, { m });
  }

  /** % CPU của một ứng dụng: "0.4%", "12%"; chưa đo được thì "…". */
  function fmtPct(v) {
    const n = Number(v);
    if (v == null || !Number.isFinite(n)) return '…';
    return (n < 10 ? n.toFixed(1).replace(/\.0$/, '') : String(Math.round(n))) + '%';
  }

  /** Mức tô màu của một phần trăm: dưới 70 bình thường, 70–89 cao, từ 90 rất cao. */
  function level(pct) {
    if (pct == null || !Number.isFinite(Number(pct))) return '';
    return pct >= 90 ? 'hot' : pct >= 70 ? 'warn' : 'ok';
  }

  /**
   * Mặt Miu theo máy: quá tải thì giật mình, cao thì "cố lên", rảnh thì ngủ gật,
   * còn lại nháy mắt. Chưa có số CPU (lần đo đầu) thì nháy mắt.
   */
  function face(stats) {
    const cpu = stats && stats.cpu;
    const ram = stats && stats.ram && stats.ram.pct;
    const worst = Math.max(cpu || 0, ram || 0);
    if (level(worst) === 'hot') return 'surprised';
    if (level(worst) === 'warn') return 'fight';
    if (cpu != null && cpu < 10) return 'sleepy';
    return 'wink';
  }

  /**
   * Toạ độ polyline cho biểu đồ CPU trong khung w×h: điểm mới nhất ở mép phải,
   * 100% ở đỉnh. `slots` = số điểm tối đa (để biểu đồ chạy từ phải sang trái).
   */
  function sparkPoints(history, w, h, slots) {
    const pts = (history || []).filter((v) => Number.isFinite(v));
    if (!pts.length) return '';
    const n = Math.max(slots || pts.length, 2);
    const step = w / (n - 1);
    const start = n - pts.length;
    return pts
      .map((v, i) => {
        const x = (start + i) * step;
        const y = h - (Math.min(Math.max(v, 0), 100) / 100) * h;
        return `${+x.toFixed(1)},${+y.toFixed(1)}`;
      })
      .join(' ');
  }

  const T = {
    vi: {
      title: 'Thông số máy', top: 'Luôn nổi', unpin: 'Gỡ ghim', ram: 'RAM', net: 'Mạng',
      uptime: 'Máy đã chạy {t}', free: 'trống {x}', charging: 'đang sạc', battery: 'Pin', waiting: 'đang đo…',
      dh: '{d} ngày {h} giờ', hm: '{h} giờ {m} phút', m: '{m} phút',
      freeOf: 'còn trống {free} / {total}', usedOf: 'đã dùng {used} / {total}',
      back: 'Quay lại', taskmgr: 'Task Manager', taskmgrTitle: 'Mở Task Manager của Windows',
      taskmgrMac: 'Activity Monitor', taskmgrMacTitle: 'Mở Activity Monitor của macOS', disk: 'Ổ',
      clickCpu: 'Bấm để xem ứng dụng dùng CPU nhiều nhất', clickRam: 'Bấm để xem ứng dụng dùng RAM nhiều nhất',
      procs: '{n} tiến trình', noProcs: 'Không đọc được danh sách ứng dụng.',
    },
    ja: {
      title: 'PCの状態', top: '常に手前', unpin: '固定をはずす', ram: 'メモリ', net: 'ネット',
      uptime: '起動から {t}', free: '空き {x}', charging: '充電中', battery: 'バッテリー', waiting: '計測中…',
      dh: '{d}日{h}時間', hm: '{h}時間{m}分', m: '{m}分',
      freeOf: '空き {free} / {total}', usedOf: '使用 {used} / {total}',
      back: '戻る', taskmgr: 'タスクマネージャー', taskmgrTitle: 'Windows のタスクマネージャーを開く',
      taskmgrMac: 'アクティビティモニタ', taskmgrMacTitle: 'macOS のアクティビティモニタを開く', disk: 'ディスク',
      clickCpu: 'クリックで CPU を多く使うアプリを表示', clickRam: 'クリックでメモリを多く使うアプリを表示',
      procs: '{n} プロセス', noProcs: 'アプリの一覧を読めませんでした。',
    },
    en: {
      title: 'System', top: 'Always on top', unpin: 'Unpin', ram: 'RAM', net: 'Network',
      uptime: 'Up {t}', free: '{x} free', charging: 'charging', battery: 'Battery', waiting: 'measuring…',
      dh: '{d}d {h}h', hm: '{h}h {m}m', m: '{m}m',
      freeOf: '{free} free of {total}', usedOf: '{used} used of {total}',
      back: 'Back', taskmgr: 'Task Manager', taskmgrTitle: 'Open Windows Task Manager',
      taskmgrMac: 'Activity Monitor', taskmgrMacTitle: 'Open macOS Activity Monitor', disk: 'Disk',
      clickCpu: 'Click to see the apps using the most CPU', clickRam: 'Click to see the apps using the most RAM',
      procs: '{n} processes', noProcs: "Couldn't read the app list.",
    },
  };

  function strings(lang) {
    return T[lang] || T.vi;
  }

  function fill(s, vars) {
    return String(s).replace(/\{(\w+)\}/g, (_, k) => (vars[k] != null ? vars[k] : ''));
  }

  /** Nhãn ngắn của ổ đĩa trên widget: "C:" giữ nguyên, tên dài ("Macintosh HD") thành "Ổ". */
  function diskLabel(drive, t) {
    const d = String(drive || '').trim();
    if (!d) return 'C:';
    return d.length <= 3 ? d : t.disk;
  }

  const api = { T, strings, fill, diskLabel, fmtBytes, fmtRate, fmtUptime, fmtPct, level, face, sparkPoints };
  if (typeof module !== 'undefined' && module.exports) module.exports = api;
  else root.SysmonCore = api;
})(typeof window !== 'undefined' ? window : globalThis);
