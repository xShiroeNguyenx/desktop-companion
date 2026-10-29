// Widget Thông số máy (sysmon-pin.html). Rust gửi phần ít đổi (sysmon_get /
// "sysmon-data": ngôn ngữ, luôn nổi, skin) và số đo mỗi 2 giây ("sysmon-stats",
// chỉ khi widget đang hiện). Trang chỉ vẽ.
(function () {
  const { invoke, listen, log } = window.DC;
  const S = window.SysmonCore;
  const $ = (id) => document.getElementById(id);
  // ĐỒNG BỘ với sysmon_pin::HISTORY và viewBox của #spark.
  const HISTORY = 30;
  const SPARK_W = 160;
  const SPARK_H = 30;

  let data = { pinTop: true, lang: 'vi' };
  let stats = null;
  // Bảng "ngốn máy nhất": null = đóng, "cpu" | "ram" = đang xếp theo gì.
  // Danh sách ứng dụng chỉ được hỏi khi bảng đang mở (xem top_procs.rs).
  let topBy = null;
  let topTimer = 0;

  function setLevel(el, pct) {
    el.classList.remove('lv-ok', 'lv-warn', 'lv-hot');
    const lv = S.level(pct);
    if (lv) el.classList.add('lv-' + lv);
  }

  function renderStatic() {
    const t = S.strings(data.lang);
    $('title').textContent = '📊 ' + t.title;
    $('ramK').textContent = t.ram;
    $('topBtn').title = t.top;
    $('topBtn').classList.toggle('on', !!data.pinTop);
    $('unpinBtn').title = t.unpin;
    $('cpuBox').title = t.clickCpu;
    $('ramRow').title = t.clickRam;
    $('topBack').title = t.back;
    $('taskmgr').textContent = t.taskmgr + ' ↗';
    $('taskmgr').title = t.taskmgrTitle;
  }

  function renderStats() {
    const t = S.strings(data.lang);
    const st = stats || {};
    // Chỗ hẹp (Miu ở góc phải): chỉ ghi ⏱ + thời gian, câu đầy đủ trong tooltip.
    const up = st.uptime ? S.fmtUptime(st.uptime, data.lang) : '';
    $('uptime').textContent = up ? '⏱ ' + up : t.waiting;
    $('uptime').title = up ? S.fill(t.uptime, { t: up }) : '';

    // CPU: lần đo đầu chưa có số (cần hai lần đọc).
    $('cpu').innerHTML = st.cpu == null ? '–' : `${st.cpu}<small>%</small>`;
    setLevel($('cpuBox'), st.cpu);
    $('sparkLine').setAttribute('points', S.sparkPoints(st.cpuHistory, SPARK_W, SPARK_H, HISTORY));

    const ram = st.ram;
    $('ramBar').style.width = ram ? ram.pct + '%' : '0';
    $('ramV').textContent = ram ? `${ram.pct}% · ${S.fmtBytes(ram.used)}/${S.fmtBytes(ram.total)}` : '–';
    $('ramRow').title = (ram ? S.fill(t.usedOf, { used: S.fmtBytes(ram.used), total: S.fmtBytes(ram.total) }) + '\n' : '') + t.clickRam;
    setLevel($('ramRow'), ram && ram.pct);

    const disk = st.disk;
    $('diskK').textContent = disk ? disk.drive : 'C:';
    $('diskBar').style.width = disk ? disk.pct + '%' : '0';
    $('diskV').textContent = disk ? `${disk.pct}% · ${S.fill(t.free, { x: S.fmtBytes(disk.total - disk.used) })}` : '–';
    $('diskRow').title = disk ? S.fill(t.freeOf, { free: S.fmtBytes(disk.total - disk.used), total: S.fmtBytes(disk.total) }) : '';
    setLevel($('diskRow'), disk && disk.pct);

    $('netDn').textContent = '↓ ' + (st.net ? S.fmtRate(st.net.rx) : '–');
    $('netUp').textContent = '↑ ' + (st.net ? S.fmtRate(st.net.tx) : '–');
    $('netDn').title = $('netUp').title = t.net;

    // Máy bàn không có pin: ẩn hẳn.
    const bat = st.battery;
    $('bat').hidden = !bat;
    if (bat) {
      $('bat').textContent = `${bat.charging ? '⚡' : bat.pct <= 20 ? '🪫' : '🔋'} ${bat.pct}%`;
      $('bat').title = t.battery + (bat.charging ? ` (${t.charging})` : '');
    }

    const src = `${data.miuDir || 'media/game'}/rps-${S.face(stats)}.${data.miuExt || 'png'}`;
    if ($('miu').getAttribute('src') !== src) $('miu').setAttribute('src', src);
  }

  // ─── Bảng "ngốn máy nhất" ───────────────────────────────────────────────────

  function renderTop(rows) {
    const t = S.strings(data.lang);
    const list = $('topList');
    list.textContent = '';
    $('sortCpu').classList.toggle('on', topBy === 'cpu');
    $('sortRam').classList.toggle('on', topBy === 'ram');
    if (!rows || !rows.length) {
      const d = document.createElement('div');
      d.className = 'proc';
      d.textContent = rows ? t.noProcs : t.waiting;
      list.append(d);
      return;
    }
    for (const r of rows) {
      const row = document.createElement('div');
      row.className = 'proc';
      const n = document.createElement('span');
      n.className = 'n';
      n.textContent = r.name;
      if (r.count > 1) {
        const c = document.createElement('small');
        c.textContent = '×' + r.count;
        n.append(c);
      }
      n.title = r.name + (r.count > 1 ? ' · ' + S.fill(t.procs, { n: r.count }) : '');
      const cpu = document.createElement('span');
      cpu.className = 'c' + (topBy === 'cpu' ? ' on' : '');
      cpu.textContent = S.fmtPct(r.cpu);
      const mem = document.createElement('span');
      mem.className = 'm' + (topBy === 'ram' ? ' on' : '');
      mem.textContent = S.fmtBytes(r.mem);
      row.append(n, cpu, mem);
      list.append(row);
    }
  }

  async function loadTop() {
    if (!topBy) return;
    try {
      renderTop(await invoke('sysmon_top', { by: topBy }));
    } catch (e) {
      renderTop([]);
      log('[sysmon] sysmon_top: ' + e);
    }
  }

  function openTop(by) {
    topBy = by;
    $('card').classList.add('showing-top');
    $('topPanel').hidden = false;
    renderTop(null);
    loadTop();
    clearInterval(topTimer);
    topTimer = setInterval(loadTop, 2000);
  }

  function closeTop() {
    topBy = null;
    clearInterval(topTimer);
    $('card').classList.remove('showing-top');
    $('topPanel').hidden = true;
  }

  function receive(v) {
    if (!v) return;
    data = v;
    // Skin màu và trang trí chọn trong Bộ sưu tập (skins.css, widget-deco.css).
    document.body.dataset.skin = data.skin || 'lavender';
    window.WidgetDeco.mount($('wrap'), data.deco);
    renderStatic();
    renderStats();
  }

  function currentWindow() {
    const T = window.__TAURI__ || {};
    return (T.window && T.window.getCurrentWindow && T.window.getCurrentWindow()) || null;
  }
  $('card').addEventListener('mousedown', (e) => {
    if (e.button !== 0 || e.target.closest('button')) return;
    const w = currentWindow();
    if (w && w.startDragging) w.startDragging().catch(() => {});
  });
  $('topBtn').addEventListener('click', () =>
    invoke('widget_pin_top', { id: 'sysmon', top: !data.pinTop }).catch((e) => log('[sysmon] top: ' + e)));
  $('unpinBtn').addEventListener('click', () =>
    invoke('widget_pin_set', { id: 'sysmon', pinned: false }).catch((e) => log('[sysmon] unpin: ' + e)));

  $('cpuBox').addEventListener('click', () => openTop('cpu'));
  $('ramRow').addEventListener('click', () => openTop('ram'));
  $('topBack').addEventListener('click', closeTop);
  for (const b of [$('sortCpu'), $('sortRam')]) {
    b.addEventListener('click', () => {
      topBy = b.dataset.by;
      loadTop();
    });
  }
  $('taskmgr').addEventListener('click', () =>
    invoke('open_task_manager').catch((e) => log('[sysmon] task manager: ' + e)));

  listen('sysmon-data', (e) => receive(e && e.payload));
  listen('sysmon-stats', (e) => {
    stats = (e && e.payload) || null;
    renderStats();
  });
  invoke('sysmon_get').then(receive).catch((e) => log('[sysmon] sysmon_get: ' + e));
  renderStatic();
  renderStats();
})();
