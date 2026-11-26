// 🎬 Hình nền động: phát video đã chọn, dừng khi Rust báo không ai thấy desktop
// (wallpaper_live.rs: "wallvid-pause"), tải lại khi đổi video ("wallvid-reload").
(function () {
  const { listen, log } = window.DC;
  const vid = document.getElementById('vid');
  // Windows (WebView2) gọi giao thức tự đăng ký qua http://<tên>.localhost.
  const BASE = /Windows/.test(navigator.userAgent) ? 'http://wallmedia.localhost/' : 'wallmedia://localhost/';
  let paused = false;

  function load() {
    vid.src = BASE + 'video?v=' + Date.now();
    if (!paused) vid.play().catch((e) => log('wallpaper-live play: ' + e));
  }

  vid.addEventListener('error', () => {
    const err = vid.error;
    log('wallpaper-live: video lỗi ' + (err ? err.code + ' ' + (err.message || '') : '?'));
  });

  listen('wallvid-pause', (e) => {
    paused = !!e.payload;
    if (paused) vid.pause();
    else vid.play().catch((err) => log('wallpaper-live play: ' + err));
  });
  listen('wallvid-reload', load);
  load();
})();
