// Click-through đang bật: rê chuột vào nhân vật (hay bong bóng lời thoại) thì pet mờ
// đi để đọc được chữ của app phía sau; chuột rời đi thì hiện lại. Bấm vẫn xuyên qua
// (cửa sổ bỏ qua chuột), nên trang không có mousemove: vị trí con trỏ lấy từ hook
// toàn cục (Rust `cursor-pos` → window.__onCursorPos__, chung luồng với pet-lookat.js).
// Rust bật / tắt qua pet-message `clickThroughState` (src-tauri/src/click_through.rs).
// Thử "điểm có nằm trên nhân vật không": window.__petHitTest__ (media/webview/interaction.js).
//
// Ctrl + chuột phải trên nhân vật (Windows): trang báo Rust mỗi khi con trỏ vào / ra
// nhân vật (`seeThrough:hover`); Rust bắt cú bấm, tạm tắt click-through rồi gửi
// `openMenuAt`. Trang mở menu, canh tới khi menu và mọi panel mở từ đó đóng hết thì
// báo `seeThrough:menuClosed` để Rust bật lại click-through.
(function (root) {
  /** Rời khỏi nhân vật bao lâu mới hiện lại (ms): lướt qua khe tóc / mép bong bóng không nháy. */
  const LEAVE_MS = 250;
  /**
   * Còn thứ cần bấm (mọi menu, panel của trang pet dùng các class này khi hiện), hoặc pet
   * đang trả lời (bong bóng có nút ⏹ dừng) — như "Hỏi em", giữ click-through tắt tới lúc xong.
   */
  const UI_OPEN = '.show, .open, .chat-open, .chat-bubble.streaming';
  /** Canh menu mỗi bấy nhiêu ms; trống liền CLOSE_AFTER lần thì coi như đã đóng hết. */
  const WATCH_MS = 300;
  const CLOSE_AFTER = 2;

  /** Số lần liền nhau không còn gì mở, sau một lần canh. */
  function nextMisses(open, misses) {
    return open ? 0 : misses + 1;
  }

  /** Toạ độ client của trang từ toạ độ màn hình (px vật lý) mà hook gửi. */
  function toClient(pos, win) {
    const dpr = win.devicePixelRatio || 1;
    return { x: pos.x / dpr - win.screenX, y: pos.y / dpr - win.screenY };
  }

  /** Điểm nằm trong trang (w × h) không. */
  function inside(p, w, h) {
    return p.x >= 0 && p.y >= 0 && p.x < w && p.y < h;
  }

  /** Điểm nằm trong hộp (getBoundingClientRect) không. */
  function inRect(p, r) {
    return !!r && p.x >= r.left && p.x <= r.right && p.y >= r.top && p.y <= r.bottom;
  }

  const api = { LEAVE_MS, UI_OPEN, WATCH_MS, CLOSE_AFTER, toClient, inside, inRect, nextMisses };
  if (typeof module !== 'undefined' && module.exports) {
    module.exports = api;
    return;
  }
  root.PetSeeThrough = api;

  let enabled = false;
  let faded = false;
  let leaveTimer = 0;
  /** Lần cuối đã báo Rust con trỏ có trên nhân vật không. */
  let reported = false;
  let watchTimer = 0;

  function post(message) {
    if (window.__VS_CODE_BRIDGE__) window.__VS_CODE_BRIDGE__.postMessage(message);
  }

  function report(over) {
    if (over === reported) return;
    reported = over;
    post({ command: 'seeThrough:hover', over });
  }

  function stopWatch() {
    clearInterval(watchTimer);
    watchTimer = 0;
  }

  function log(message) {
    post({ command: 'seeThrough:log', message: String(message) });
  }

  /**
   * Những gì đang mở VÀ đang hiện thật trên trang, trừ `before` (đã hiện từ trước khi mở
   * menu: vòng đếm giờ 🍅 suốt phiên Tập trung… — không phải thứ người dùng vừa mở).
   */
  function openUi(before) {
    const out = [];
    for (const el of document.querySelectorAll(UI_OPEN)) {
      if (before && before.has(el)) continue;
      if (!el.getClientRects().length) continue; // display: none (cả khi cha ẩn)
      const cs = getComputedStyle(el);
      if (cs.visibility === 'hidden' || Number(cs.opacity) < 0.05) continue;
      out.push(el);
    }
    return out;
  }

  const names = (els) => els.map((el) => String(el.className)).join(' | ');

  /** Canh menu mở bằng Ctrl + chuột phải: đóng hết (cả panel mở từ menu) thì báo Rust. */
  function watchMenu(before) {
    stopWatch();
    let misses = 0;
    let ticks = 0;
    watchTimer = setInterval(() => {
      const open = openUi(before);
      misses = nextMisses(open.length > 0, misses);
      // Giữ lâu bất thường: ghi lại thứ đang giữ (một lần) để biết vì sao.
      if (++ticks === Math.round(30000 / WATCH_MS)) log('still open after 30 s: ' + names(open));
      if (misses >= CLOSE_AFTER) {
        stopWatch();
        post({ command: 'seeThrough:menuClosed' });
      }
    }, WATCH_MS);
  }

  let watchBefore = new Set();

  function openMenuAt(pos) {
    const p = toClient(pos, window);
    watchBefore = new Set(openUi());
    try {
      if (typeof window.__petOpenMenuAt__ !== 'function') throw new Error('menu not ready (Live2D still loading)');
      window.__petOpenMenuAt__(p.x, p.y);
    } catch (err) {
      // Không mở được thì trả click-through lại ngay, đừng để kẹt.
      log('could not open the menu: ' + (err && err.message ? err.message : err));
      post({ command: 'seeThrough:menuClosed' });
      return;
    }
    const open = openUi(watchBefore);
    if (!open.length) {
      log('menu call returned but nothing new is showing (already open: ' + names([...watchBefore]) + ')');
      post({ command: 'seeThrough:menuClosed' });
      return;
    }
    post({ command: 'seeThrough:menuOpened', what: names(open) + (watchBefore.size ? ' / already open: ' + names([...watchBefore]) : '') });
    watchMenu(watchBefore);
  }

  function show(v) {
    document.documentElement.classList.toggle('pet-see-through', v);
  }

  function setFaded(over) {
    if (over) {
      clearTimeout(leaveTimer);
      leaveTimer = 0;
      if (!faded) {
        faded = true;
        show(true);
      }
      return;
    }
    if (!faded || leaveTimer) return;
    leaveTimer = setTimeout(() => {
      leaveTimer = 0;
      faded = false;
      show(false);
    }, LEAVE_MS);
  }

  function overPet(p) {
    if (!inside(p, window.innerWidth, window.innerHeight)) return false;
    const bubble = document.getElementById('chatBubble');
    if (bubble && bubble.classList.contains('visible') && inRect(p, bubble.getBoundingClientRect())) return true;
    return typeof window.__petHitTest__ === 'function' && !!window.__petHitTest__(p.x, p.y);
  }

  window.addEventListener('message', (e) => {
    const d = e.data;
    if (!d) return;
    if (d.command === 'openMenuAt') return openMenuAt(d);
    // Bấm ra ngoài cửa sổ pet: đóng menu và panel nhỏ như popup thường. Khung chat đang
    // gõ dở thì để nguyên; vẫn canh tiếp, đóng hết mới báo Rust bật lại click-through.
    if (d.command === 'closeMenus') {
      if (typeof window.__petCloseMenus__ === 'function') window.__petCloseMenus__();
      if (!watchTimer) watchMenu(watchBefore);
      return;
    }
    if (d.command === 'clickThroughState') {
      heardEvent = true;
      setEnabled(!!d.seeThrough);
    }
  });

  /** Đã nhận sự kiện đổi trạng thái: mới hơn kết quả của lần hỏi lúc nạp trang. */
  let heardEvent = false;

  function setEnabled(on) {
    enabled = on;
    if (!enabled) {
      clearTimeout(leaveTimer);
      leaveTimer = 0;
      faded = false;
      reported = false; // Rust cũng tự bỏ cờ khi tắt
      show(false);
    }
  }

  const lookat = window.__onCursorPos__;
  window.__onCursorPos__ = function (pos) {
    if (enabled) {
      const over = overPet(toClient(pos, window));
      setFaded(over);
      report(over);
    }
    if (lookat) lookat(pos);
  };

  // Trang vừa nạp (mở app, đổi model): hỏi Rust có đang bật không, bằng lệnh trả thẳng
  // giá trị. KHÔNG hỏi rồi chờ sự kiện: lúc app vừa mở, sự kiện trả lời rơi mất vì trang
  // chưa kịp nghe (lỗi thật 2026-11-06: tray ghi "đang bật" mà pet không mờ).
  const invoke = window.__TAURI__ && window.__TAURI__.core && window.__TAURI__.core.invoke;
  if (invoke) {
    invoke('pet_see_through')
      .then((on) => {
        if (!heardEvent) setEnabled(!!on);
      })
      .catch((err) => log('could not ask the click-through state: ' + err));
  }
})(typeof window !== 'undefined' ? window : globalThis);
