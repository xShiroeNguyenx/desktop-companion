// 📸 Chụp màn hình (PLAN.md 20): phím tắt toàn cục (mặc định Ctrl+Alt+X), khay hoặc menu
// của pet → chụp màn hình đang chứa con trỏ → cửa sổ `shot` phủ kín màn hình đó cho người
// dùng khoanh vùng, vẽ chú thích, rồi chép vào clipboard hoặc lưu PNG.
//
// Ảnh không rời khỏi máy: khung hình gốc nằm trong RAM (FRAME) cho tới khi trang lấy về,
// trang chỉ gửi lại vùng đã cắt khi người dùng bấm chép / lưu. Cửa sổ dựng lúc chụp, đóng
// là huỷ. Hình học vùng chọn: app/js/shot-core.js (thuần). Chỉ Windows.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;

use tauri::{AppHandle, Manager};

pub const LABEL: &str = "shot";
pub const DEFAULT_HOTKEY: &str = "Ctrl+Alt+X";

/// Trần cho ảnh trang gửi về (PNG của vùng cắt): màn 8K nén PNG vẫn dưới mức này.
const PNG_MAX: usize = 60 * 1024 * 1024;
/// Trang không báo sẵn sàng trong ngần này thì coi như hỏng, dọn đi để lần sau còn chụp được.
const READY_TIMEOUT_MS: u64 = 8000;

/// Đang có một lần chụp (từ lúc bấm phím tới khi cửa sổ bị huỷ).
static ACTIVE: AtomicBool = AtomicBool::new(false);
/// Trang đã nạp xong ảnh và cửa sổ đã hiện.
static READY: AtomicBool = AtomicBool::new(false);
/// Số thứ tự lần chụp: bộ canh giờ của lần trước không được dọn nhầm lần sau.
static RUN: AtomicU64 = AtomicU64::new(0);
/// Khung hình gốc (file BMP 32-bit) chờ trang lấy về; lấy xong là bỏ khỏi đây.
static FRAME: Mutex<Option<Vec<u8>>> = Mutex::new(None);

/// Góc trên-trái (pixel thật, toạ độ desktop ảo) của màn hình vừa chụp: ảnh ghim đặt lại
/// đúng chỗ vùng chọn nhờ nó.
static ORIGIN: Mutex<(i32, i32)> = Mutex::new((0, 0));

/// selection.rs hỏi để KHÔNG coi cú kéo khoanh vùng là bôi đen văn bản.
pub fn active() -> bool {
    ACTIVE.load(Ordering::SeqCst)
}

// ─── Hàm thuần ───────────────────────────────────────────────────────────────

/// Header file BMP 32-bit, dòng trên cùng đứng trước (chiều cao âm), cho ảnh w × h.
pub fn bmp_header(w: u32, h: u32) -> [u8; 54] {
    let pixels = w as u64 * h as u64 * 4;
    let mut b = [0u8; 54];
    b[0] = b'B';
    b[1] = b'M';
    b[2..6].copy_from_slice(&((54 + pixels) as u32).to_le_bytes());
    b[10..14].copy_from_slice(&54u32.to_le_bytes());
    b[14..18].copy_from_slice(&40u32.to_le_bytes());
    b[18..22].copy_from_slice(&(w as i32).to_le_bytes());
    b[22..26].copy_from_slice(&(-(h as i32)).to_le_bytes());
    b[26..28].copy_from_slice(&1u16.to_le_bytes());
    b[28..30].copy_from_slice(&32u16.to_le_bytes());
    b[34..38].copy_from_slice(&(pixels as u32).to_le_bytes());
    b
}

/// GDI để byte thứ tư của mỗi pixel là rác (thường 0): ép về 255 để không trình duyệt nào
/// hiểu nó là "trong suốt".
pub fn opaque(bgra: &mut [u8]) {
    for px in bgra.chunks_exact_mut(4) {
        px[3] = 255;
    }
}

/// Màn hình (x, y, w, h) chứa điểm (px, py); không màn nào chứa thì lấy màn đầu.
pub fn monitor_at(mons: &[(i32, i32, i32, i32)], px: i32, py: i32) -> Option<(i32, i32, i32, i32)> {
    mons.iter()
        .copied()
        .find(|&(x, y, w, h)| px >= x && px < x + w && py >= y && py < y + h)
        .or_else(|| mons.first().copied())
}

/// Giải `data:image/png;base64,…` từ canvas của trang. Lỗi là mã cho giao diện tự dịch
/// (`shot.err.<mã>`): sai dạng, không phải PNG, hoặc quá lớn đều là "image".
pub fn png_from_data_url(url: &str) -> Result<Vec<u8>, &'static str> {
    use base64::Engine;
    let b64 = url.strip_prefix("data:image/png;base64,").ok_or("image")?;
    if b64.len() / 4 * 3 > PNG_MAX {
        return Err("image");
    }
    let bytes = base64::engine::general_purpose::STANDARD.decode(b64).map_err(|_| "image")?;
    if !bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
        return Err("image");
    }
    Ok(bytes)
}

/// Tên file gợi ý cho hộp Lưu: chỉ giữ chữ, số, `-`, `_`, `.`; luôn kết thúc bằng `.png`.
pub fn safe_name(name: &str) -> String {
    let stem = name.strip_suffix(".png").unwrap_or(name);
    let clean: String = stem
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
        .take(80)
        .collect();
    format!("{}.png", if clean.is_empty() { "Screenshot" } else { &clean })
}

// ─── Phím tắt (cùng khuôn với launcher.rs) ───────────────────────────────────

struct Rt {
    shortcut: Option<tauri_plugin_global_shortcut::Shortcut>,
    status: String,
}

static RT: Mutex<Rt> = Mutex::new(Rt { shortcut: None, status: String::new() });

/// Đăng ký (lại) phím tắt. Gọi lúc khởi động, khi đổi trong Cài đặt, và sau mỗi lần phím
/// bắt chữ gọi `unregister_all` (main.rs). Ngoài Windows thì không đăng ký gì.
pub fn register(app: &AppHandle) {
    use std::str::FromStr;
    use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};
    let spec = crate::store::load_settings(app).shot_hotkey.trim().to_string();
    let gs = app.global_shortcut();
    let mut rt = RT.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(sc) = rt.shortcut.take() {
        let _ = gs.unregister(sc);
    }
    rt.status.clear();
    if spec.is_empty() || !cfg!(windows) {
        return;
    }
    // Đăng ký thất bại (sai cú pháp, trùng app khác) thì API không ồn ào gì: ghi lại để
    // Cài đặt báo cho người dùng.
    match Shortcut::from_str(&spec) {
        Ok(sc) => match gs.register(sc) {
            Ok(()) => rt.shortcut = Some(sc),
            Err(e) => {
                rt.status = format!("Phím {spec} không đăng ký được (app khác đã dùng?): {e}");
                log(&rt.status);
            }
        },
        Err(_) => rt.status = format!("Phím {spec} sai cú pháp (vd: Ctrl+Alt+X hoặc PrintScreen)."),
    }
}

/// Lỗi đăng ký phím gần nhất, rỗng = ổn.
pub fn status() -> String {
    RT.lock().map(|rt| rt.status.clone()).unwrap_or_default()
}

pub fn is_shortcut(sc: &tauri_plugin_global_shortcut::Shortcut) -> bool {
    RT.lock().ok().and_then(|rt| rt.shortcut).is_some_and(|mine| mine.id() == sc.id())
}

// ─── Chụp + cửa sổ ───────────────────────────────────────────────────────────

fn log(msg: &str) {
    eprintln!("[shot] {msg}");
}

/// Bắt đầu một lần chụp. `delay_ms`: mở từ menu thì chờ menu đóng hẳn rồi mới chụp, kẻo
/// menu dính vào ảnh. Đang chụp dở thì bỏ qua.
pub fn start(app: &AppHandle, delay_ms: u64) {
    if !cfg!(windows) || ACTIVE.swap(true, Ordering::SeqCst) {
        return;
    }
    READY.store(false, Ordering::SeqCst);
    let run = RUN.fetch_add(1, Ordering::SeqCst) + 1;
    let app = app.clone();
    // Luồng riêng: dựng cửa sổ trên main thread làm treo WebView2 (xem windows.rs).
    std::thread::spawn(move || {
        if delay_ms > 0 {
            std::thread::sleep(std::time::Duration::from_millis(delay_ms));
        }
        if let Err(e) = open(&app) {
            log(&format!("không chụp được: {e}"));
            close(&app);
            return;
        }
        // Trang hỏng (ảnh không nạp, JS lỗi) thì không bao giờ gọi shot_ready: dọn đi,
        // nếu không ACTIVE kẹt mãi và phím tắt thành vô dụng.
        std::thread::sleep(std::time::Duration::from_millis(READY_TIMEOUT_MS));
        if RUN.load(Ordering::SeqCst) == run && ACTIVE.load(Ordering::SeqCst) && !READY.load(Ordering::SeqCst) {
            log("trang không báo sẵn sàng, huỷ lần chụp");
            close(&app);
        }
    });
}

fn open(app: &AppHandle) -> Result<(), String> {
    let mons: Vec<(i32, i32, i32, i32)> = app
        .available_monitors()
        .map_err(|e| e.to_string())?
        .iter()
        .map(|m| (m.position().x, m.position().y, m.size().width as i32, m.size().height as i32))
        .collect();
    let (cx, cy) = cursor();
    let (x, y, w, h) = monitor_at(&mons, cx, cy).ok_or("không thấy màn hình nào")?;
    let frame = capture(x, y, w, h)?;
    *ORIGIN.lock().unwrap_or_else(|e| e.into_inner()) = (x, y);
    *FRAME.lock().unwrap_or_else(|e| e.into_inner()) = Some(frame);

    let win = tauri::WebviewWindowBuilder::new(app, LABEL, tauri::WebviewUrl::App("shot.html".into()))
        .title("Chụp màn hình")
        .decorations(false)
        .shadow(false)
        .resizable(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .visible(false)
        .build()
        .map_err(|e| e.to_string())?;
    // Dời sang đúng màn hình trước rồi mới đặt cỡ: đổi màn hình là đổi DPI của cửa sổ.
    let _ = win.set_position(tauri::PhysicalPosition::new(x, y));
    let _ = win.set_size(tauri::PhysicalSize::new(w as u32, h as u32));
    Ok(())
}

/// Trang lấy khung hình gốc (một lần): trả file BMP rồi bỏ khỏi RAM phía Rust.
pub fn take_frame() -> Option<Vec<u8>> {
    FRAME.lock().unwrap_or_else(|e| e.into_inner()).take()
}

/// Trang đã vẽ xong ảnh: giờ mới hiện cửa sổ, để không nháy một khung trắng phủ kín màn hình.
pub fn ready(app: &AppHandle) {
    if !ACTIVE.load(Ordering::SeqCst) {
        return;
    }
    if let Some(w) = app.get_webview_window(LABEL) {
        READY.store(true, Ordering::SeqCst);
        let _ = w.show();
        let _ = w.set_focus();
    }
}

/// Kết thúc lần chụp (Esc, ✕, Alt+F4, chép / lưu xong, hoặc lỗi): ẩn ngay, bỏ khung hình,
/// huỷ cửa sổ ở luồng riêng rồi mới cho chụp lần mới.
pub fn close(app: &AppHandle) {
    take_frame();
    if let Some(w) = app.get_webview_window(LABEL) {
        let _ = w.hide();
    }
    let app = app.clone();
    std::thread::spawn(move || {
        if let Some(w) = app.get_webview_window(LABEL) {
            let _ = w.destroy();
        }
        READY.store(false, Ordering::SeqCst);
        ACTIVE.store(false, Ordering::SeqCst);
    });
}

/// Đặt ảnh PNG vào clipboard dưới dạng ảnh. Lỗi là mã (`shot.err.<mã>`).
pub fn copy(png: &[u8]) -> Result<(), &'static str> {
    let img = tauri::image::Image::from_bytes(png).map_err(|_| "image")?;
    let (w, h) = (img.width() as usize, img.height() as usize);
    let mut cb = arboard::Clipboard::new().map_err(|_| "clipboard")?;
    cb.set_image(arboard::ImageData { width: w, height: h, bytes: std::borrow::Cow::Borrowed(img.rgba()) })
        .map_err(|_| "clipboard")?;
    // Đọc lại cho chắc ảnh đã nằm trong clipboard (app khác có thể đang giữ clipboard).
    match cb.get_image() {
        Ok(back) if back.width == w && back.height == h => Ok(()),
        _ => Err("clipboard"),
    }
}

/// Đặt chữ (kết quả đọc chữ trong ảnh) vào clipboard, đọc lại để chắc là đã vào.
pub fn copy_text(text: &str) -> Result<(), &'static str> {
    let mut cb = arboard::Clipboard::new().map_err(|_| "clipboard")?;
    cb.set_text(text.to_string()).map_err(|_| "clipboard")?;
    match cb.get_text() {
        Ok(back) if back.replace("\r\n", "\n") == text.replace("\r\n", "\n") => Ok(()),
        _ => Err("clipboard"),
    }
}

/// RGBA (canvas, PNG) → BGRA (GDI, Windows.Media.Ocr): đổi chỗ kênh đỏ và xanh dương.
pub fn rgba_to_bgra(rgba: &[u8]) -> Vec<u8> {
    let mut out = rgba.to_vec();
    for px in out.chunks_exact_mut(4) {
        px.swap(0, 2);
    }
    out
}

/// Đọc chữ trong ảnh PNG của vùng chọn (ocr.rs). Lỗi là mã (`shot.err.<mã>`).
pub fn read_text(png: &[u8], lang: &str) -> Result<String, &'static str> {
    let img = tauri::image::Image::from_bytes(png).map_err(|_| "image")?;
    crate::ocr::recognize(&rgba_to_bgra(img.rgba()), img.width(), img.height(), lang)
}

// ─── 📌 Ghim ảnh lên màn hình ────────────────────────────────────────────────
// Mỗi ảnh ghim là một cửa sổ nhỏ luôn nổi (`shotpin-<id>`), đặt đúng chỗ vùng vừa chụp, kéo
// đi được. Ghim giống tờ note: ảnh lưu thành file `<app_data>/shot-pins/<id>.png`, vị trí
// trong `index.json` cùng thư mục, mở app lại là hiện lại đúng chỗ; chỉ mất khi gỡ ghim
// (lúc đó file ảnh bị xoá luôn).

pub const PIN_PREFIX: &str = "shotpin-";
/// Mỗi ảnh ghim giữ một tiến trình WebView2 (~30–50 MB), nên có trần.
pub const PIN_MAX: usize = 5;

/// Một ảnh đang ghim: vị trí + cỡ cửa sổ tính bằng pixel thật trên desktop ảo.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PinMeta {
    pub id: u64,
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
}

/// Danh sách ghim (bản trong RAM của index.json); None = chưa đọc từ đĩa.
static PINS: Mutex<Option<Vec<PinMeta>>> = Mutex::new(None);
/// Lần dời gần nhất: kéo xong đứng yên 400 ms mới ghi đĩa (như note_pins.rs).
static PIN_MOVE_GEN: AtomicU64 = AtomicU64::new(0);

pub fn is_pin(label: &str) -> bool {
    pin_id(label).is_some()
}

/// Id của ảnh ghim từ label cửa sổ (`shotpin-7` → 7).
pub fn pin_id(label: &str) -> Option<u64> {
    label.strip_prefix(PIN_PREFIX)?.parse().ok()
}

/// Cỡ cửa sổ ghim cho ảnh w × h: không nhỏ hơn 48 px mỗi chiều (còn chỗ cho nút ✕).
pub fn pin_size(w: u32, h: u32) -> (u32, u32) {
    (w.max(48), h.max(48))
}

/// Id cho ảnh ghim mới: lớn hơn mọi id đang có (không dùng lại id cũ, kẻo trùng tên file).
pub fn next_pin_id(pins: &[PinMeta]) -> u64 {
    pins.iter().map(|p| p.id).max().unwrap_or(0) + 1
}

/// Vị trí để hiện lại ảnh ghim: giữ nguyên nếu còn thấy được ít nhất 40 px mỗi chiều trên
/// một màn hình nào đó; không thì (rút màn hình phụ, đổi độ phân giải) kéo về góc màn đầu.
pub fn pin_place(p: &PinMeta, mons: &[(i32, i32, i32, i32)]) -> (i32, i32) {
    let seen = mons.iter().any(|&(mx, my, mw, mh)| {
        let ox = (p.x + p.w as i32).min(mx + mw) - p.x.max(mx);
        let oy = (p.y + p.h as i32).min(my + mh) - p.y.max(my);
        ox >= 40 && oy >= 40
    });
    match (seen, mons.first()) {
        (false, Some(&(mx, my, _, _))) => (mx + 40, my + 40),
        _ => (p.x, p.y),
    }
}

fn pin_dir(app: &AppHandle) -> Option<std::path::PathBuf> {
    let dir = app.path().app_data_dir().ok()?.join("shot-pins");
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir)
}

fn pin_file(app: &AppHandle, id: u64) -> Option<std::path::PathBuf> {
    Some(pin_dir(app)?.join(format!("{id}.png")))
}

/// Chạy `f` trên danh sách ghim (đọc index.json ở lần đầu); `f` trả true thì ghi lại đĩa.
fn with_pins<R>(app: &AppHandle, f: impl FnOnce(&mut Vec<PinMeta>) -> (R, bool)) -> R {
    let mut guard = PINS.lock().unwrap_or_else(|e| e.into_inner());
    let pins = guard.get_or_insert_with(|| {
        pin_dir(app)
            .and_then(|d| std::fs::read(d.join("index.json")).ok())
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    });
    let (out, dirty) = f(pins);
    if dirty {
        // Ghi file tạm rồi đổi tên: mất điện giữa chừng không làm hỏng danh sách.
        let saved = pin_dir(app).and_then(|d| {
            let tmp = d.join("index.json.tmp");
            std::fs::write(&tmp, serde_json::to_vec(pins).ok()?).ok()?;
            std::fs::rename(&tmp, d.join("index.json")).ok()
        });
        if saved.is_none() {
            log("không ghi được danh sách ảnh ghim");
        }
    }
    out
}

fn monitors(app: &AppHandle) -> Vec<(i32, i32, i32, i32)> {
    app.available_monitors()
        .unwrap_or_default()
        .iter()
        .map(|m| (m.position().x, m.position().y, m.size().width as i32, m.size().height as i32))
        .collect()
}

/// Dựng cửa sổ cho một ảnh ghim (ẩn; trang nạp xong ảnh thì gọi shotpin_ready để hiện).
/// Chạy ngoài main thread.
fn pin_open(app: &AppHandle, p: &PinMeta, at: (i32, i32)) -> bool {
    let label = format!("{PIN_PREFIX}{}", p.id);
    if app.get_webview_window(&label).is_some() {
        return true;
    }
    let built = tauri::WebviewWindowBuilder::new(app, &label, tauri::WebviewUrl::App("shot-pin.html".into()))
        .title("Ảnh ghim")
        .decorations(false)
        .resizable(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .visible(false)
        .build();
    match built {
        Ok(win) => {
            let _ = win.set_position(tauri::PhysicalPosition::new(at.0, at.1));
            let _ = win.set_size(tauri::PhysicalSize::new(p.w, p.h));
            true
        }
        Err(e) => {
            log(&format!("không dựng được cửa sổ ghim: {e}"));
            false
        }
    }
}

/// Ghim `png` (vùng chọn kèm nét vẽ) tại vùng (x, y, w, h) của màn hình vừa chụp, tính
/// bằng pixel ảnh. Chạy ngoài main thread. Lỗi là mã: "pins" = đã đủ số ghim, "write" =
/// không lưu được file ảnh.
pub fn pin(app: &AppHandle, png: &[u8], x: i32, y: i32, w: u32, h: u32) -> Result<(), &'static str> {
    let (ox, oy) = *ORIGIN.lock().unwrap_or_else(|e| e.into_inner());
    let (pw, ph) = pin_size(w, h);
    let meta = with_pins(app, |pins| {
        if pins.len() >= PIN_MAX {
            return (Err("pins"), false);
        }
        let meta = PinMeta { id: next_pin_id(pins), x: ox + x, y: oy + y, w: pw, h: ph };
        // Ảnh phải nằm trên đĩa trước khi ghi vào danh sách.
        if pin_file(app, meta.id).and_then(|f| std::fs::write(f, png).ok()).is_none() {
            return (Err("write"), false);
        }
        pins.push(meta.clone());
        (Ok(meta), true)
    })?;
    if !pin_open(app, &meta, (meta.x, meta.y)) {
        pin_close(app, &format!("{PIN_PREFIX}{}", meta.id));
        return Err("fail");
    }
    Ok(())
}

/// Mở app: hiện lại các ảnh đang ghim. Mục nào mất file ảnh thì bỏ khỏi danh sách.
pub fn restore_pins(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        let pins = with_pins(&app, |pins| {
            let before = pins.len();
            pins.retain(|p| pin_file(&app, p.id).is_some_and(|f| f.is_file()));
            (pins.clone(), pins.len() != before)
        });
        let mons = monitors(&app);
        for p in &pins {
            pin_open(&app, p, pin_place(p, &mons));
        }
    });
}

/// PNG của ảnh ghim `label` (trang shot-pin.html lấy về để hiện, hoặc để chép lại).
pub fn pin_image(app: &AppHandle, label: &str) -> Option<Vec<u8>> {
    std::fs::read(pin_file(app, pin_id(label)?)?).ok()
}

/// Người dùng kéo ảnh ghim: nhớ chỗ mới, đứng yên 400 ms thì ghi đĩa.
pub fn pin_moved(app: &AppHandle, label: &str, x: i32, y: i32) {
    let Some(id) = pin_id(label) else { return };
    with_pins(app, |pins| {
        if let Some(p) = pins.iter_mut().find(|p| p.id == id) {
            p.x = x;
            p.y = y;
        }
        ((), false)
    });
    let gen = PIN_MOVE_GEN.fetch_add(1, Ordering::SeqCst) + 1;
    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(400));
        if PIN_MOVE_GEN.load(Ordering::SeqCst) == gen {
            with_pins(&app, |_| ((), true));
        }
    });
}

/// Gỡ ghim: ẩn ngay, xoá khỏi danh sách và xoá file ảnh, huỷ cửa sổ ở luồng riêng.
pub fn pin_close(app: &AppHandle, label: &str) {
    if let Some(id) = pin_id(label) {
        with_pins(app, |pins| {
            let before = pins.len();
            pins.retain(|p| p.id != id);
            ((), pins.len() != before)
        });
        if let Some(f) = pin_file(app, id) {
            let _ = std::fs::remove_file(f);
        }
    }
    if let Some(w) = app.get_webview_window(label) {
        let _ = w.hide();
    }
    let app = app.clone();
    let label = label.to_string();
    std::thread::spawn(move || {
        if let Some(w) = app.get_webview_window(&label) {
            let _ = w.destroy();
        }
    });
}

#[cfg(windows)]
fn cursor() -> (i32, i32) {
    use windows_sys::Win32::Foundation::POINT;
    use windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos;
    let mut p = POINT { x: 0, y: 0 };
    // SAFETY: chỉ ghi vào biến cục bộ.
    unsafe { GetCursorPos(&mut p) };
    (p.x, p.y)
}

#[cfg(not(windows))]
fn cursor() -> (i32, i32) {
    (0, 0)
}

/// Chụp vùng (x, y, w, h) của desktop ảo (pixel thật) thành file BMP 32-bit trong RAM.
#[cfg(windows)]
fn capture(x: i32, y: i32, w: i32, h: i32) -> Result<Vec<u8>, String> {
    use windows_sys::Win32::Graphics::Gdi::{
        BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC, GetDIBits, ReleaseDC,
        SelectObject, BITMAPINFO, CAPTUREBLT, DIB_RGB_COLORS, SRCCOPY,
    };

    if w <= 0 || h <= 0 || w > 16384 || h > 16384 {
        return Err(format!("cỡ màn hình lạ: {w}×{h}"));
    }
    let len = w as usize * h as usize * 4;
    let mut out = Vec::with_capacity(54 + len);
    out.extend_from_slice(&bmp_header(w as u32, h as u32));
    out.resize(54 + len, 0);

    // SAFETY: mọi handle GDI lấy ra đều được trả lại bên dưới; GetDIBits ghi đúng `len`
    // byte (w × h pixel 32-bit) vào vùng đệm đã cấp đủ.
    let ok = unsafe {
        let screen = GetDC(std::ptr::null_mut());
        if screen.is_null() {
            return Err("GetDC thất bại".into());
        }
        let mem = CreateCompatibleDC(screen);
        let bmp = CreateCompatibleBitmap(screen, w, h);
        let mut ok = false;
        if !mem.is_null() && !bmp.is_null() {
            let old = SelectObject(mem, bmp as _);
            // CAPTUREBLT: lấy cả cửa sổ layered (trong suốt), như pet và các widget.
            let blt = BitBlt(mem, 0, 0, w, h, screen, x, y, SRCCOPY | CAPTUREBLT);
            // GetDIBits yêu cầu bitmap không còn được chọn vào DC nào.
            SelectObject(mem, old);
            let mut info: BITMAPINFO = std::mem::zeroed();
            info.bmiHeader.biSize = 40;
            info.bmiHeader.biWidth = w;
            info.bmiHeader.biHeight = -h; // âm = dòng trên cùng đứng trước
            info.bmiHeader.biPlanes = 1;
            info.bmiHeader.biBitCount = 32;
            let lines = GetDIBits(mem, bmp, 0, h as u32, out[54..].as_mut_ptr() as _, &mut info, DIB_RGB_COLORS);
            ok = blt != 0 && lines == h;
        }
        if !bmp.is_null() {
            DeleteObject(bmp as _);
        }
        if !mem.is_null() {
            DeleteDC(mem);
        }
        ReleaseDC(std::ptr::null_mut(), screen);
        ok
    };
    if !ok {
        return Err("chụp màn hình bằng GDI thất bại".into());
    }
    opaque(&mut out[54..]);
    Ok(out)
}

#[cfg(not(windows))]
fn capture(_x: i32, _y: i32, _w: i32, _h: i32) -> Result<Vec<u8>, String> {
    Err("chỉ hỗ trợ Windows".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bmp_header_describes_a_top_down_32_bit_image() {
        let h = bmp_header(1920, 1080);
        assert_eq!(&h[0..2], b"BM");
        assert_eq!(u32::from_le_bytes(h[2..6].try_into().unwrap()), 54 + 1920 * 1080 * 4);
        assert_eq!(u32::from_le_bytes(h[10..14].try_into().unwrap()), 54);
        assert_eq!(u32::from_le_bytes(h[14..18].try_into().unwrap()), 40);
        assert_eq!(i32::from_le_bytes(h[18..22].try_into().unwrap()), 1920);
        assert_eq!(i32::from_le_bytes(h[22..26].try_into().unwrap()), -1080);
        assert_eq!(u16::from_le_bytes(h[26..28].try_into().unwrap()), 1);
        assert_eq!(u16::from_le_bytes(h[28..30].try_into().unwrap()), 32);
        // Không nén (BI_RGB = 0).
        assert_eq!(u32::from_le_bytes(h[30..34].try_into().unwrap()), 0);
    }

    #[test]
    fn opaque_only_touches_the_alpha_byte() {
        let mut px = [1, 2, 3, 0, 4, 5, 6, 77];
        opaque(&mut px);
        assert_eq!(px, [1, 2, 3, 255, 4, 5, 6, 255]);
    }

    #[test]
    fn monitor_under_the_cursor_or_the_first_one() {
        let mons = [(0, 0, 1920, 1080), (-2560, 0, 2560, 1440), (1920, 0, 1280, 1024)];
        assert_eq!(monitor_at(&mons, 100, 100), Some(mons[0]));
        assert_eq!(monitor_at(&mons, -1, 500), Some(mons[1]));
        assert_eq!(monitor_at(&mons, 1920, 0), Some(mons[2]));
        // Mép phải / đáy không thuộc màn hình đó.
        assert_eq!(monitor_at(&mons, 1919, 1079), Some(mons[0]));
        // Con trỏ ở chỗ không màn nào phủ: lấy màn đầu.
        assert_eq!(monitor_at(&mons, 9999, 9999), Some(mons[0]));
        assert_eq!(monitor_at(&[], 0, 0), None);
    }

    #[test]
    fn data_url_must_be_a_real_png() {
        use base64::Engine;
        let enc = |b: &[u8]| base64::engine::general_purpose::STANDARD.encode(b);
        let png = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 1, 2, 3];
        assert_eq!(png_from_data_url(&format!("data:image/png;base64,{}", enc(&png))).unwrap(), png);
        assert_eq!(png_from_data_url(&enc(&png)), Err("image"));
        assert_eq!(png_from_data_url("data:image/png;base64,@@@"), Err("image"));
        assert_eq!(png_from_data_url(&format!("data:image/png;base64,{}", enc(b"GIF89a"))), Err("image"));
        assert_eq!(png_from_data_url(&format!("data:image/jpeg;base64,{}", enc(&png))), Err("image"));
    }

    #[test]
    fn suggested_file_names_cannot_carry_a_path() {
        assert_eq!(safe_name("Screenshot_2026-11-26_14-03-09.png"), "Screenshot_2026-11-26_14-03-09.png");
        assert_eq!(safe_name(r"..\..\evil.exe"), "evilexe.png");
        assert_eq!(safe_name("a/b:c*?.png"), "abc.png");
        assert_eq!(safe_name(""), "Screenshot.png");
        assert_eq!(safe_name(&"x".repeat(200)).len(), 84);
    }

    #[test]
    fn rgba_and_bgra_swap_red_and_blue_only() {
        assert_eq!(rgba_to_bgra(&[1, 2, 3, 4, 10, 20, 30, 40]), vec![3, 2, 1, 4, 30, 20, 10, 40]);
    }

    #[test]
    fn pinned_images_keep_room_for_the_close_button() {
        assert_eq!(pin_size(800, 600), (800, 600));
        assert_eq!(pin_size(10, 300), (48, 300));
        assert_eq!(pin_size(4, 4), (48, 48));
        assert!(is_pin("shotpin-3"));
        assert!(!is_pin("pin-3"));
        assert!(!is_pin("shot"));
        assert_eq!(pin_id("shotpin-12"), Some(12));
        assert_eq!(pin_id("shotpin-"), None);
        assert_eq!(pin_id("shotpin-x"), None);
    }

    #[test]
    fn pin_ids_are_never_reused_while_listed() {
        let p = |id| PinMeta { id, x: 0, y: 0, w: 100, h: 100 };
        assert_eq!(next_pin_id(&[]), 1);
        assert_eq!(next_pin_id(&[p(1), p(7), p(3)]), 8);
    }

    #[test]
    fn restored_pins_come_back_where_they_were_unless_off_screen() {
        let mons = [(0, 0, 1920, 1080), (1920, 0, 1280, 1024)];
        let p = |x, y| PinMeta { id: 1, x, y, w: 300, h: 200 };
        assert_eq!(pin_place(&p(500, 400), &mons), (500, 400));
        // Trên màn hình phụ.
        assert_eq!(pin_place(&p(2500, 100), &mons), (2500, 100));
        // Thò ra mép nhưng còn thấy được hơn 40 px: giữ nguyên.
        assert_eq!(pin_place(&p(-200, 500), &mons), (-200, 500));
        // Màn hình phụ đã rút: ảnh nằm ngoài mọi màn hình → về góc màn đầu.
        assert_eq!(pin_place(&p(2500, 100), &mons[..1]), (40, 40));
        // Chỉ còn thấy một mẩu dưới 40 px cũng coi là mất.
        assert_eq!(pin_place(&p(1900, 1060), &mons[..1]), (40, 40));
        // Không đọc được màn hình nào: đừng dời lung tung.
        assert_eq!(pin_place(&p(9000, 9000), &[]), (9000, 9000));
    }

    #[test]
    fn pin_list_round_trips_through_json() {
        let pins = vec![PinMeta { id: 2, x: -100, y: 50, w: 640, h: 360 }];
        let json = serde_json::to_string(&pins).unwrap();
        assert_eq!(serde_json::from_str::<Vec<PinMeta>>(&json).unwrap(), pins);
        // File hỏng thì coi như chưa ghim gì (with_pins dùng unwrap_or_default).
        assert!(serde_json::from_str::<Vec<PinMeta>>("{oops").is_err());
    }

    /// Đọc chữ thật từ một file PNG bằng OCR của Windows (cần gói OCR đúng ngôn ngữ):
    /// đặt `DC_OCR_PNG` = đường dẫn ảnh, `DC_OCR_EXPECT` = chữ phải thấy, rồi
    /// `cargo test live_ocr -- --ignored --nocapture`.
    #[cfg(windows)]
    #[test]
    #[ignore]
    fn live_ocr_reads_text_from_a_png() {
        let path = std::env::var("DC_OCR_PNG").expect("đặt DC_OCR_PNG");
        let expect = std::env::var("DC_OCR_EXPECT").expect("đặt DC_OCR_EXPECT");
        let text = read_text(&std::fs::read(path).unwrap(), "").unwrap();
        println!("OCR: {text:?}");
        assert!(text.contains(&expect), "{text:?} không chứa {expect:?}");
    }

    /// Chụp thật màn hình chính (cần phiên desktop): `cargo test shot -- --ignored`.
    #[cfg(windows)]
    #[test]
    #[ignore]
    fn live_capture_fills_a_bmp_of_the_right_size() {
        let bmp = capture(0, 0, 200, 100).unwrap();
        assert_eq!(bmp.len(), 54 + 200 * 100 * 4);
        assert!(bmp[54..].chunks_exact(4).all(|px| px[3] == 255));
        // Màn hình thật hiếm khi đen tuyệt đối cả vùng 200×100.
        assert!(bmp[54..].chunks_exact(4).any(|px| px[0] != 0 || px[1] != 0 || px[2] != 0));
    }
}
