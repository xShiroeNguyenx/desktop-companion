// 🖼 Mục Hình nền trong Cài đặt (wallpaper.rs, wallpaper_live.rs). Mọi thao tác áp
// dụng ngay, không chờ nút Lưu. Trạng thái luôn đọc lại từ Rust (wallpaper_status /
// sự kiện wallpaper-changed), trang không tự đoán.
(function () {
  const { invoke, listen, log } = window.DC;
  const $ = (id) => document.getElementById(id);
  const t = (key, vars) => window.I18n.t(key, vars);
  const Art = window.WallArt;
  const IS_WIN = /Windows/.test(navigator.userAgent);
  const MEDIA = IS_WIN ? 'http://wallmedia.localhost/' : 'wallmedia://localhost/';
  const INTERVALS = [0, 5, 15, 30, 60, 180, 1440];

  let st = null; // wallpaper_status gần nhất
  let busy = false;

  function say(text, cls) {
    const el = $('wallStatus');
    el.className = 'status' + (cls ? ' ' + cls : '');
    el.textContent = text;
  }
  const baseName = (p) => (p ? String(p).split(/[\\/]/).pop() : '');

  function intervalLabel(m) {
    if (!m) return t('settings.wall.manual');
    if (m === 1440) return t('settings.wall.daily');
    if (m >= 60) return t('settings.wall.hours', { n: m / 60 });
    return t('settings.minutes', { n: m });
  }
  function fillIntervals(sel) {
    const el = $('wallInterval');
    el.innerHTML = '';
    for (const m of INTERVALS) {
      const o = document.createElement('option');
      o.value = String(m);
      o.textContent = intervalLabel(m);
      el.appendChild(o);
    }
    el.value = String(sel);
  }

  function render() {
    if (!st) return;
    $('wallFit').value = st.fit;
    const img = $('wallPreview');
    if (st.imageOk) {
      img.src = MEDIA + 'image?v=' + st.stamp;
      img.hidden = false;
      $('wallPreviewEmpty').hidden = true;
      $('wallName').textContent = baseName(st.image);
    } else {
      img.hidden = true;
      img.removeAttribute('src');
      $('wallPreviewEmpty').hidden = false;
      $('wallName').textContent = t('settings.wall.none');
    }
    $('wallFolderBlock').hidden = !st.folder;
    $('wallFolderName').textContent = st.folder ? '📁 ' + st.folder : '';
    fillIntervals(st.intervalMin);
    $('wallShuffle').checked = !!st.shuffle;
    if (document.activeElement !== $('wallDrive')) $('wallDrive').value = st.driveUrl || '';
    $('wallLiveBlock').hidden = !st.liveSupported;
    $('wallVideoName').textContent = st.video ? '🎬 ' + baseName(st.video) : t('settings.wall.noVideo');
    $('wallLive').checked = !!st.live;
    $('wallLive').disabled = !st.video;
    if (st.lastError && !busy) say(st.lastError, 'err');
  }

  async function refresh() {
    try {
      st = await invoke('wallpaper_status');
      render();
    } catch (e) {
      log('wallpaper_status: ' + e);
    }
  }

  // Chạy một thao tác: khoá nút, báo đang làm, báo kết quả, đọc lại trạng thái.
  async function run(working, fn, done) {
    if (busy) return;
    busy = true;
    say(working);
    try {
      const r = await fn();
      say(typeof done === 'function' ? done(r) : done, 'ok');
    } catch (e) {
      say(String(e), 'err');
      log('wallpaper: ' + e);
    } finally {
      busy = false;
      refresh();
    }
  }

  async function pick(opts) {
    const dialog = window.__TAURI__ && window.__TAURI__.dialog;
    if (!dialog) throw new Error('dialog');
    const r = await dialog.open(Object.assign({ multiple: false }, opts));
    return Array.isArray(r) ? r[0] : r;
  }

  $('wallFit').addEventListener('change', () => {
    const fit = $('wallFit').value;
    run(t('settings.wall.applying'), () => invoke('wallpaper_set_fit', { fit }), t('settings.wall.done'));
  });

  $('wallPickImage').addEventListener('click', async () => {
    const path = await pick({ title: t('settings.wall.pickImage'), filters: [{ name: 'Image', extensions: ['jpg', 'jpeg', 'png', 'bmp'] }] }).catch(() => null);
    if (!path) return;
    run(t('settings.wall.applying'), () => invoke('wallpaper_set_image', { path }), t('settings.wall.done'));
  });

  $('wallPickFolder').addEventListener('click', async () => {
    const folder = await pick({ directory: true, title: t('settings.wall.pickFolder') }).catch(() => null);
    if (!folder) return;
    const intervalMin = st && st.intervalMin ? st.intervalMin : 30;
    const shuffle = $('wallShuffle').checked;
    run(t('settings.wall.applying'), () => invoke('wallpaper_set_folder', { folder, intervalMin, shuffle }), (n) => t('settings.wall.folderSet', { n }));
  });

  function saveRotation() {
    const intervalMin = Number($('wallInterval').value);
    const shuffle = $('wallShuffle').checked;
    run(t('settings.wall.applying'), () => invoke('wallpaper_set_rotation', { intervalMin, shuffle }), t('settings.wall.saved'));
  }
  $('wallInterval').addEventListener('change', saveRotation);
  $('wallShuffle').addEventListener('change', saveRotation);
  $('wallNext').addEventListener('click', () => run(t('settings.wall.applying'), () => invoke('wallpaper_next'), t('settings.wall.done')));
  $('wallStop').addEventListener('click', () => run('', () => invoke('wallpaper_stop_rotation'), t('settings.wall.stopped')));

  // ── Hình nền vẽ sẵn ──
  function screenSize() {
    const dpr = window.devicePixelRatio || 1;
    // Tối đa 8K; nhỏ nhất Full HD cho màn hình báo cỡ lạ.
    const w = Math.min(7680, Math.max(1920, Math.round(screen.width * dpr)));
    const h = Math.min(4320, Math.max(1080, Math.round(screen.height * dpr)));
    return [w, h];
  }

  function buildGrid() {
    const grid = $('wallGrid');
    grid.innerHTML = '';
    const [sw, sh] = screenSize();
    const tw = 192, th = Math.round((192 * sh) / sw);
    for (const [id] of Art.THEMES) {
      const b = document.createElement('button');
      b.type = 'button';
      b.dataset.id = id;
      b.title = Art.themeName(id, window.I18n.lang());
      const c = document.createElement('canvas');
      c.width = tw;
      c.height = th;
      Art.paint(c.getContext('2d'), Art.scene(id, tw, th));
      const label = document.createElement('span');
      label.textContent = b.title;
      b.append(c, label);
      b.addEventListener('click', () => applyArt(id));
      grid.appendChild(b);
    }
  }

  function toBase64(blob) {
    return new Promise((resolve, reject) => {
      const fr = new FileReader();
      fr.onload = () => resolve(String(fr.result).split(',')[1] || '');
      fr.onerror = () => reject(fr.error);
      fr.readAsDataURL(blob);
    });
  }

  function applyArt(id) {
    const buttons = $('wallGrid').querySelectorAll('button');
    run(
      t('settings.wall.drawing'),
      async () => {
        buttons.forEach((b) => (b.disabled = true));
        try {
          const [w, h] = screenSize();
          const c = document.createElement('canvas');
          c.width = w;
          c.height = h;
          Art.paint(c.getContext('2d'), Art.scene(id, w, h));
          const blob = await new Promise((res) => c.toBlob(res, 'image/png'));
          if (!blob) throw new Error(t('settings.wall.drawFail'));
          await invoke('wallpaper_save_generated', { name: id, png: await toBase64(blob) });
        } finally {
          buttons.forEach((b) => (b.disabled = false));
        }
      },
      t('settings.wall.done'),
    );
  }

  // ── Google Drive ──
  $('wallDriveGo').addEventListener('click', () => {
    const url = $('wallDrive').value.trim();
    if (!url) return;
    run(t('settings.wall.downloading'), () => invoke('wallpaper_from_drive', { url }), (kind) => {
      if (kind === 'video') return t('settings.wall.liveStarted');
      if (String(kind).startsWith('folder:')) return t('settings.wall.folderSet', { n: kind.slice(7) });
      return t('settings.wall.done');
    });
  });
  listen('wallpaper-progress', (e) => {
    const p = e.payload || {};
    if (busy && p.total) say(t('settings.wall.downloadingN', { done: p.done, total: p.total }));
  });

  // ── Hình nền động ──
  $('wallPickVideo').addEventListener('click', async () => {
    const path = await pick({ title: t('settings.wall.pickVideo'), filters: [{ name: 'Video', extensions: ['mp4', 'webm'] }] }).catch(() => null);
    if (!path) return;
    run(t('settings.wall.applying'), () => invoke('wallpaper_live_video', { path }), t('settings.wall.liveStarted'));
  });
  $('wallLive').addEventListener('change', () => {
    const on = $('wallLive').checked;
    run(t('settings.wall.applying'), () => invoke('wallpaper_live_set', { on }), on ? t('settings.wall.liveStarted') : t('settings.wall.liveStopped'));
  });

  listen('wallpaper-changed', (e) => {
    st = e.payload;
    render();
  });
  window.addEventListener('focus', refresh);
  window.I18n.onChange(() => {
    buildGrid();
    render();
  });

  buildGrid();
  refresh();
})();
