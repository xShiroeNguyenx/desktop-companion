// 📸 Lớp phủ chụp màn hình (shot.html, PLAN.md 20): nối chuột / phím với canvas.
// Mọi toạ độ lưu theo pixel ảnh (pixel thật của màn hình) để ảnh xuất ra đúng từng pixel
// ở mọi tỉ lệ DPI; hình học nằm ở shot-core.js. Rust (shot.rs) giữ khung hình gốc, đặt
// ảnh vào clipboard, mở hộp Lưu file và huỷ cửa sổ.
(function () {
  const { invoke, log } = window.DC;
  const C = window.ShotCore;
  const $ = (id) => document.getElementById(id);
  const T = (key, vars) => window.I18n.t(key, vars);

  const img = $('frame');
  const canvas = $('layer');
  const ctx = canvas.getContext('2d');
  const toolbar = $('toolbar');
  const statusEl = $('status');
  const textIn = $('textIn');

  const COLORS = ['#ff3b30', '#ffcc00', '#34c759', '#0a84ff', '#ffffff', '#111111'];
  const ERR_CODES = ['image', 'clipboard', 'write', 'noocr', 'notext', 'ocr', 'pins'];

  let W = 0; // cỡ ảnh = cỡ màn hình tính bằng pixel thật
  let H = 0;
  let sel = null; // vùng chọn đã chốt { x, y, w, h }
  let shapes = []; // nét chú thích đã vẽ, theo thứ tự (hoàn tác = bỏ cái cuối)
  let tool = 'select'; // 'select' | 'pen' | 'line' | 'arrow' | 'rect' | 'marker' | 'text'
  let color = COLORS[0];
  let width = C.DEFAULT_WIDTH; // nấc độ dày nét, lăn chuột để đổi
  let widthTimer = 0;
  let drag = null; // { kind: 'sel', mode, start, x0, y0, rect } | { kind: 'shape', shape }
  let textAt = null; // chỗ đang gõ chữ (pixel ảnh)
  let busy = false; // đang chép / lưu
  let queued = false;

  // Pixel ảnh trên mỗi CSS px, theo từng chiều (≈ devicePixelRatio; đo trực tiếp để lệch
  // 1–2 px do bo cỡ cửa sổ không làm sai toạ độ).
  const kx = () => (W && window.innerWidth ? W / window.innerWidth : 1);
  const ky = () => (H && window.innerHeight ? H / window.innerHeight : 1);
  const toImg = (e) => [e.clientX * kx(), e.clientY * ky()];

  // ─── Vẽ ────────────────────────────────────────────────────────────────────

  function paintShape(c, s) {
    c.save();
    c.strokeStyle = s.color;
    c.fillStyle = s.color;
    c.lineWidth = s.lw;
    c.lineCap = 'round';
    c.lineJoin = 'round';
    if (s.type === 'pen' || s.type === 'marker') {
      // Bút dạ quang: nét to, trong mờ. Vẽ cả nét bằng một lần stroke nên chỗ nét tự cắt
      // nhau không bị đậm lên.
      if (s.type === 'marker') c.globalAlpha = 0.4;
      c.beginPath();
      s.pts.forEach(([x, y], i) => (i ? c.lineTo(x, y) : c.moveTo(x, y)));
      c.stroke();
    } else if (s.type === 'rect') {
      const r = C.normRect(s.x1, s.y1, s.x2, s.y2);
      c.strokeRect(r.x, r.y, r.w, r.h);
    } else if (s.type === 'line') {
      c.beginPath();
      c.moveTo(s.x1, s.y1);
      c.lineTo(s.x2, s.y2);
      c.stroke();
    } else if (s.type === 'arrow') {
      const head = C.arrowHead(s.x1, s.y1, s.x2, s.y2, s.lw * 5);
      c.beginPath();
      c.moveTo(s.x1, s.y1);
      c.lineTo(s.x2, s.y2);
      c.stroke();
      if (head) {
        c.beginPath();
        c.moveTo(s.x2, s.y2);
        c.lineTo(head[0][0], head[0][1]);
        c.lineTo(head[1][0], head[1][1]);
        c.closePath();
        c.fill();
        c.stroke();
      }
    } else if (s.type === 'text') {
      c.font = `600 ${s.size}px "Segoe UI", sans-serif`;
      c.textBaseline = 'top';
      // Viền mảnh tương phản để chữ đọc được trên mọi nền.
      c.lineWidth = Math.max(1, s.size / 9);
      c.strokeStyle = s.color === '#111111' ? 'rgba(255,255,255,0.75)' : 'rgba(0,0,0,0.55)';
      c.strokeText(s.text, s.x, s.y);
      c.fillText(s.text, s.x, s.y);
    }
    c.restore();
  }

  const liveSel = () => (drag && drag.kind === 'sel' && drag.rect ? drag.rect : sel);

  function draw() {
    queued = false;
    if (!W) return;
    const u = kx();
    ctx.clearRect(0, 0, W, H);
    ctx.fillStyle = 'rgba(0, 0, 0, 0.45)';
    ctx.fillRect(0, 0, W, H);
    const r = liveSel();
    if (!C.usable(r)) return;

    // Vùng chọn: khoét lớp tối để lộ ảnh gốc, rồi vẽ nét chú thích (cắt theo vùng chọn).
    ctx.clearRect(r.x, r.y, r.w, r.h);
    ctx.save();
    ctx.beginPath();
    ctx.rect(r.x, r.y, r.w, r.h);
    ctx.clip();
    shapes.forEach((s) => paintShape(ctx, s));
    if (drag && drag.kind === 'shape') paintShape(ctx, drag.shape);
    ctx.restore();

    // Viền nét đứt hai màu: thấy được trên cả nền sáng lẫn nền tối.
    ctx.lineWidth = Math.max(1, Math.round(u));
    ctx.strokeStyle = 'rgba(0, 0, 0, 0.7)';
    ctx.strokeRect(r.x, r.y, r.w, r.h);
    ctx.setLineDash([6 * u, 4 * u]);
    ctx.strokeStyle = '#fff';
    ctx.strokeRect(r.x, r.y, r.w, r.h);
    ctx.setLineDash([]);

    // 8 tay nắm.
    const hs = 7 * u;
    const pts = C.handlePoints(r);
    for (const name of C.HANDLES) {
      const [hx, hy] = pts[name];
      ctx.fillStyle = '#fff';
      ctx.fillRect(hx - hs / 2, hy - hs / 2, hs, hs);
      ctx.strokeStyle = 'rgba(0, 0, 0, 0.75)';
      ctx.strokeRect(hx - hs / 2, hy - hs / 2, hs, hs);
    }

    // Nhãn kích thước (pixel thật), phía trên góc trái; sát mép trên thì chui vào trong.
    const label = `${r.w} × ${r.h}`;
    ctx.font = `600 ${12 * u}px "Segoe UI", sans-serif`;
    ctx.textBaseline = 'middle';
    const lw = ctx.measureText(label).width + 12 * u;
    const lh = 20 * u;
    const ly = r.y - lh - 6 * u >= 0 ? r.y - lh - 6 * u : r.y + 6 * u;
    const lx = Math.min(r.x, W - lw);
    ctx.fillStyle = 'rgba(20, 20, 28, 0.85)';
    ctx.fillRect(lx, ly, lw, lh);
    ctx.fillStyle = '#fff';
    ctx.fillText(label, lx + 6 * u, ly + lh / 2);
  }

  function redraw() {
    if (queued) return;
    queued = true;
    requestAnimationFrame(draw);
  }

  // ─── Thanh công cụ ─────────────────────────────────────────────────────────

  function placeToolbar() {
    const show = C.usable(sel) && !(drag && drag.kind === 'sel');
    toolbar.classList.toggle('show', show);
    $('hint').style.display = C.usable(liveSel()) || drag ? 'none' : '';
    if (!show) {
      statusEl.classList.remove('show');
      return;
    }
    const css = { x: sel.x / kx(), y: sel.y / ky(), w: sel.w / kx(), h: sel.h / ky() };
    const box = toolbar.getBoundingClientRect();
    const p = C.toolbarPos(css, { w: box.width, h: box.height }, window.innerWidth, window.innerHeight, 8);
    toolbar.style.left = p.x + 'px';
    toolbar.style.top = p.y + 'px';
    // Dòng trạng thái nằm ngay trên thanh công cụ (hết chỗ thì xuống dưới).
    const sy = p.y - 36 >= 0 ? p.y - 36 : p.y + box.height + 6;
    statusEl.style.left = p.x + 'px';
    statusEl.style.top = sy + 'px';
  }

  function setStatus(msg, kind) {
    statusEl.textContent = msg || '';
    statusEl.className = (msg ? 'show' : '') + (kind ? ' ' + kind : '');
  }

  function setTool(next) {
    commitText();
    tool = tool === next ? 'select' : next; // bấm lại công cụ đang bật = quay về chỉnh vùng chọn
    toolbar.querySelectorAll('[data-tool]').forEach((b) => b.classList.toggle('on', b.dataset.tool === tool));
  }

  function setColor(next) {
    color = next;
    document.querySelectorAll('#colors button').forEach((b) => b.firstChild.classList.toggle('on', b.dataset.color === color));
    textIn.style.color = color;
  }

  function undo() {
    if (textAt) { cancelText(); return; }
    shapes.pop();
    syncUndo();
    redraw();
  }

  const syncUndo = () => { $('undoBtn').disabled = shapes.length === 0; };

  // ─── Chữ ───────────────────────────────────────────────────────────────────

  function openText(x, y) {
    textAt = { x, y };
    textIn.value = '';
    textIn.style.color = color;
    textIn.style.fontSize = C.strokeFor('text', width) + 'px';
    textIn.style.left = x / kx() + 'px';
    textIn.style.top = y / ky() + 'px';
    textIn.style.display = 'block';
    setTimeout(() => textIn.focus(), 0);
  }

  function cancelText() {
    textAt = null;
    textIn.style.display = 'none';
    textIn.value = '';
  }

  function commitText() {
    if (!textAt) return;
    const text = textIn.value.trim();
    if (text) {
      // Ô gõ có viền 1px + đệm 2px: dịch chữ vào cho trùng chỗ đang thấy.
      shapes.push({ type: 'text', x: textAt.x + 3 * kx(), y: textAt.y + 3 * ky(), text, color, size: C.strokeFor('text', width) * kx() });
      syncUndo();
    }
    cancelText();
    redraw();
  }

  textIn.addEventListener('keydown', (e) => {
    e.stopPropagation(); // phím trong ô chữ không phải phím tắt của lớp phủ
    if (e.key === 'Enter') { e.preventDefault(); commitText(); }
    else if (e.key === 'Escape') { e.preventDefault(); cancelText(); }
  });
  textIn.addEventListener('blur', commitText);

  // ─── Chuột ─────────────────────────────────────────────────────────────────

  canvas.addEventListener('pointerdown', (e) => {
    if (e.button !== 0 || busy || !W) return;
    commitText();
    setStatus('');
    const [x, y] = toImg(e);
    const hit = C.hitTest(sel, x, y, 8 * kx());
    canvas.setPointerCapture(e.pointerId);
    if (tool !== 'select' && hit === 'move') {
      if (tool === 'text') { openText(x, y); e.preventDefault(); return; }
      const free = tool === 'pen' || tool === 'marker'; // nét vẽ tay: chuỗi điểm
      const base = { type: tool, color, lw: C.strokeFor(tool, width) * kx() };
      drag = { kind: 'shape', shape: free ? { ...base, pts: [[x, y]] } : { ...base, x1: x, y1: y, x2: x, y2: y } };
    } else {
      drag = { kind: 'sel', mode: hit, start: sel ? { ...sel } : null, x0: x, y0: y, rect: null };
    }
    placeToolbar();
  });

  canvas.addEventListener('pointermove', (e) => {
    const [x, y] = toImg(e);
    if (!drag) {
      const hit = C.hitTest(sel, x, y, 8 * kx());
      canvas.style.cursor = tool !== 'select' && hit === 'move' ? (tool === 'text' ? 'text' : 'crosshair') : C.cursorFor(hit);
      return;
    }
    if (drag.kind === 'sel') {
      if (drag.mode === 'new') {
        const cand = C.cropRect(C.normRect(drag.x0, drag.y0, x, y), W, H);
        drag.rect = C.usable(cand) ? cand : null; // nhích vài pixel chưa tính là khoanh lại
      } else {
        const cand = C.dragRect(drag.start, drag.mode, x - drag.x0, y - drag.y0, W, H);
        drag.rect = C.usable(cand) ? cand : drag.rect;
      }
      placeToolbar();
    } else if (drag.shape.pts) {
      drag.shape.pts.push([x, y]);
    } else {
      const s = drag.shape;
      // Giữ Shift: đường thẳng / mũi tên bẻ về ngang, dọc hoặc chéo 45°.
      const straight = e.shiftKey && (s.type === 'line' || s.type === 'arrow');
      [s.x2, s.y2] = straight ? C.snapEnd(s.x1, s.y1, x, y) : [x, y];
    }
    redraw();
  });

  function endDrag() {
    if (!drag) return;
    if (drag.kind === 'sel') {
      if (drag.rect) sel = drag.rect;
    } else {
      const s = drag.shape;
      const long = s.pts ? s.pts.length > 1 : Math.hypot(s.x2 - s.x1, s.y2 - s.y1) >= 3 * kx();
      if (long) shapes.push(s);
      syncUndo();
    }
    drag = null;
    placeToolbar();
    redraw();
  }
  canvas.addEventListener('pointerup', endDrag);
  canvas.addEventListener('pointercancel', endDrag);
  canvas.addEventListener('contextmenu', (e) => e.preventDefault());

  // Lăn chuột: lên = nét dày hơn, xuống = mỏng hơn (chữ thì to / nhỏ). Đang kéo dở một nét
  // hay đang gõ chữ thì đổi luôn trên nét / ô chữ đó để thấy ngay.
  window.addEventListener('wheel', (e) => {
    e.preventDefault();
    if (busy || !e.deltaY) return;
    width = C.stepWidth(width, e.deltaY < 0 ? 1 : -1);
    if (drag && drag.kind === 'shape') drag.shape.lw = C.strokeFor(drag.shape.type, width) * kx();
    if (textAt) textIn.style.fontSize = C.strokeFor('text', width) + 'px';
    setStatus(T('shot.width', { n: width }));
    clearTimeout(widthTimer);
    widthTimer = setTimeout(() => setStatus(''), 900);
    redraw();
  }, { passive: false });

  // ─── Xuất ảnh ──────────────────────────────────────────────────────────────

  /**
   * PNG (data URL) của vùng chọn: phần ảnh gốc được cắt + các nét chú thích.
   * `plain`: bỏ nét chú thích và phóng `scale` lần (ảnh đưa cho bộ đọc chữ).
   */
  function render(plain, scale = 1) {
    const out = document.createElement('canvas');
    out.width = sel.w * scale;
    out.height = sel.h * scale;
    const c = out.getContext('2d');
    c.imageSmoothingQuality = 'high';
    c.drawImage(img, sel.x, sel.y, sel.w, sel.h, 0, 0, out.width, out.height);
    if (!plain) {
      c.translate(-sel.x, -sel.y);
      shapes.forEach((s) => paintShape(c, s));
    }
    return out.toDataURL('image/png');
  }

  const errText = (e) => {
    const code = String(e);
    return T(ERR_CODES.includes(code) ? 'shot.err.' + code : 'shot.err.fail');
  };

  /**
   * Kết thúc lần chụp theo `kind`: 'copy' (chép ảnh), 'save' (lưu file), 'translate' / 'text'
   * (đọc chữ trong ảnh rồi dịch / chép chữ), 'pin' (ghim lên màn hình). Xong thì Rust tự
   * đóng cửa sổ; lỗi thì ở lại và báo lý do.
   */
  async function finish(kind) {
    commitText();
    if (busy || !C.usable(sel)) return;
    busy = true;
    clearTimeout(widthTimer);
    setStatus(T(kind === 'translate' || kind === 'text' ? 'shot.reading' : 'shot.working'));
    try {
      if (kind === 'save') {
        const done = await invoke('shot_save', { png: render(), name: C.fileName(new Date()) });
        if (!done) setStatus(''); // bấm Huỷ ở hộp Lưu: ở lại để chép hoặc lưu lại
      } else if (kind === 'translate' || kind === 'text') {
        const png = render(true, C.ocrScale(sel.w, sel.h));
        await invoke('shot_ocr', { png, action: kind === 'translate' ? 'translate' : 'copy' });
      } else if (kind === 'pin') {
        await invoke('shot_pin', { png: render(), x: sel.x, y: sel.y, w: sel.w, h: sel.h });
      } else {
        await invoke('shot_copy', { png: render() });
      }
    } catch (e) {
      log('[shot] ' + kind + ': ' + e);
      setStatus(errText(e), 'err');
    }
    busy = false;
  }

  const close = () => invoke('shot_close');

  // ─── Phím + nút ────────────────────────────────────────────────────────────

  window.addEventListener('keydown', (e) => {
    const key = e.key.toLowerCase();
    if (e.key === 'Escape') { e.preventDefault(); close(); return; }
    if (!(e.ctrlKey || e.metaKey)) return;
    if (key === 'c') { e.preventDefault(); finish('copy'); }
    else if (key === 's') { e.preventDefault(); finish('save'); }
    else if (key === 'z') { e.preventDefault(); undo(); }
    else if (key === 'a' && W && !busy) {
      e.preventDefault();
      sel = { x: 0, y: 0, w: W, h: H }; // chọn cả màn hình
      placeToolbar();
      redraw();
    }
  });

  toolbar.querySelectorAll('[data-tool]').forEach((b) => b.addEventListener('click', () => setTool(b.dataset.tool)));
  $('undoBtn').addEventListener('click', undo);
  $('copyBtn').addEventListener('click', () => finish('copy'));
  $('saveBtn').addEventListener('click', () => finish('save'));
  $('ocrBtn').addEventListener('click', () => finish('translate'));
  $('textBtn').addEventListener('click', () => finish('text'));
  $('pinBtn').addEventListener('click', () => finish('pin'));
  $('closeBtn').addEventListener('click', close);
  // Bấm nút không được lấy focus khỏi ô chữ đang gõ (blur = chốt chữ) trước khi kịp đổi màu.
  toolbar.addEventListener('pointerdown', (e) => { if (e.target.closest('#colors')) e.preventDefault(); });

  for (const c of COLORS) {
    const b = document.createElement('button');
    b.type = 'button';
    b.dataset.color = c;
    b.title = c;
    const dot = document.createElement('span');
    dot.className = 'swatch';
    dot.style.background = c;
    b.appendChild(dot);
    b.addEventListener('click', () => setColor(c));
    $('colors').appendChild(b);
  }
  setColor(color);
  syncUndo();

  window.addEventListener('resize', () => { placeToolbar(); redraw(); });

  // ─── Nạp khung hình ────────────────────────────────────────────────────────

  function fail(why) {
    log('[shot] không nạp được khung hình: ' + why);
    close();
  }

  (async function boot() {
    let buf;
    try {
      buf = await invoke('shot_frame'); // byte thô của file BMP
    } catch (e) {
      fail(e);
      return;
    }
    // Blob cùng nguồn với trang: canvas xuất ảnh được (ảnh từ nguồn khác sẽ làm canvas bị khoá).
    const url = URL.createObjectURL(new Blob([buf], { type: 'image/bmp' }));
    img.onload = () => {
      W = img.naturalWidth;
      H = img.naturalHeight;
      canvas.width = W;
      canvas.height = H;
      draw();
      // Ảnh đã giải mã xong: bảo Rust hiện cửa sổ (hiện sớm hơn sẽ nháy một khung trắng).
      // Dùng setTimeout chứ không phải requestAnimationFrame: cửa sổ còn ẩn thì rAF không chạy.
      setTimeout(() => invoke('shot_ready'), 30);
    };
    img.onerror = () => fail('WebView không giải mã được BMP');
    img.src = url;
  })();
})();
