// 🖼 Hình nền desktop (PLAN.md 19): đặt ảnh làm hình nền máy, xoay vòng ảnh trong
// một thư mục, bộ hình nền vẽ sẵn trong app (js/wallpaper-art.js), ảnh / video từ
// link Google Drive công khai, và hình nền động (video, wallpaper_live.rs).
//
// Phần quyết định (kiểu hiển thị, ảnh kế tiếp, đến giờ đổi chưa, đọc link Drive,
// nhận dạng file, Range cho video) là hàm thuần có test. Phần gọi hệ điều hành ở
// mod `os`: Windows dùng IDesktopWallpaper rồi ĐỌC LẠI để chắc hình nền đã đổi
// (API này có thể "thành công" mà máy bị chính sách nhóm khoá hình nền).

use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::{AppHandle, Emitter, Manager};

use crate::store;

/// Ảnh Windows đặt làm hình nền được (không cần codec cài thêm).
pub const IMAGE_EXT: &[&str] = &["jpg", "jpeg", "png", "bmp"];
/// Video WebView2 phát được không cần codec thêm.
pub const VIDEO_EXT: &[&str] = &["mp4", "webm"];
/// Kiểu hiển thị ảnh (mã DESKTOP_WALLPAPER_POSITION của Windows).
pub const FITS: &[&str] = &["fill", "fit", "stretch", "center", "tile", "span"];
/// Chu kỳ xoay vòng cho phép (phút). 0 = không tự đổi.
pub const INTERVALS: &[u32] = &[0, 5, 15, 30, 60, 180, 1440];
/// Thư mục Drive: tải tối đa chừng này ảnh.
pub const DRIVE_FOLDER_MAX: usize = 40;
const MAX_IMAGE_BYTES: u64 = 40 * 1024 * 1024;
const MAX_VIDEO_BYTES: u64 = 300 * 1024 * 1024;
/// Một lần trả Range tối đa chừng này byte (video đọc theo từng khúc).
pub const RANGE_CHUNK: u64 = 2 * 1024 * 1024;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct WallSettings {
    /// Một trong FITS.
    pub fit: String,
    /// Ảnh app đặt gần nhất (đường dẫn đầy đủ).
    pub image: String,
    /// Thư mục xoay vòng; rỗng = không xoay.
    pub folder: String,
    /// Phút giữa hai lần đổi (INTERVALS), 0 = chỉ đổi khi bấm "Ảnh kế".
    pub interval_min: u32,
    pub shuffle: bool,
    /// Giây unix lần đổi gần nhất.
    pub last_change: u64,
    /// Link Drive gần nhất (hiện lại trong ô nhập).
    pub drive_url: String,
    /// Video hình nền động.
    pub video: String,
    /// Đang bật hình nền động (wallpaper_live.rs).
    pub live: bool,
    /// Lỗi gần nhất của vòng xoay vòng (hiện trong Cài đặt), rỗng = ổn.
    pub last_error: String,
}

impl Default for WallSettings {
    fn default() -> Self {
        Self {
            fit: "fill".into(),
            image: String::new(),
            folder: String::new(),
            interval_min: 30,
            shuffle: false,
            last_change: 0,
            drive_url: String::new(),
            video: String::new(),
            live: false,
            last_error: String::new(),
        }
    }
}

// ─── Hàm thuần ───────────────────────────────────────────────────────────────

/// Mã DESKTOP_WALLPAPER_POSITION: CENTER 0, TILE 1, STRETCH 2, FIT 3, FILL 4, SPAN 5.
pub fn fit_code(fit: &str) -> i32 {
    match fit {
        "center" => 0,
        "tile" => 1,
        "stretch" => 2,
        "fit" => 3,
        "span" => 5,
        _ => 4,
    }
}

fn ext_of(p: &Path) -> String {
    p.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase()
}

pub fn is_image(p: &Path) -> bool {
    IMAGE_EXT.contains(&ext_of(p).as_str())
}

pub fn is_video(p: &Path) -> bool {
    VIDEO_EXT.contains(&ext_of(p).as_str())
}

/// Lọc ảnh dùng được, sắp theo tên (không phân biệt hoa thường) để thứ tự ổn định.
pub fn sorted_images(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = paths.into_iter().filter(|p| is_image(p)).collect();
    v.sort_by_key(|p| p.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default());
    v
}

fn same_path(a: &Path, b: &Path) -> bool {
    a.to_string_lossy().replace('/', "\\").eq_ignore_ascii_case(&b.to_string_lossy().replace('/', "\\"))
}

/// Ảnh kế tiếp sau `current`. Tuần tự: ảnh sau nó (hết thì vòng lại). Ngẫu nhiên:
/// chọn theo `seed` nhưng không lặp lại đúng ảnh đang đặt (khi có từ 2 ảnh).
pub fn next_image(list: &[PathBuf], current: &Path, shuffle: bool, seed: u64) -> Option<PathBuf> {
    if list.is_empty() {
        return None;
    }
    let len = list.len();
    let cur = list.iter().position(|p| same_path(p, current));
    let idx = match (shuffle, cur) {
        (_, _) if len == 1 => 0,
        (false, Some(i)) => (i + 1) % len,
        (false, None) => 0,
        (true, Some(i)) => (i + 1 + (seed % (len as u64 - 1)) as usize) % len,
        (true, None) => (seed % len as u64) as usize,
    };
    Some(list[idx].clone())
}

/// Đến giờ đổi ảnh chưa. Đồng hồ máy bị lùi (last ở tương lai) thì coi như đến giờ.
pub fn due(last: u64, now: u64, interval_min: u32) -> bool {
    if interval_min == 0 {
        return false;
    }
    last > now + 60 || now.saturating_sub(last) >= u64::from(interval_min) * 60
}

#[derive(Debug, PartialEq, Eq)]
pub enum DriveLink {
    File(String),
    Folder(String),
}

fn valid_drive_id(id: &str) -> bool {
    id.len() >= 10 && id.len() <= 100 && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

/// Đọc link Google Drive: file (`/file/d/<id>`, `open?id=`, `uc?id=`) hoặc thư mục
/// (`/drive/folders/<id>`, kể cả `/drive/u/0/folders/<id>`). Chỉ nhận host của Google.
pub fn drive_link(url: &str) -> Option<DriveLink> {
    let url = url.trim();
    let rest = url.strip_prefix("https://").or_else(|| url.strip_prefix("http://"))?;
    let (host, path) = rest.split_once('/').unwrap_or((rest, ""));
    if !matches!(host.to_ascii_lowercase().as_str(), "drive.google.com" | "docs.google.com" | "drive.usercontent.google.com") {
        return None;
    }
    let (path, query) = path.split_once('?').unwrap_or((path, ""));
    let path = path.split('#').next().unwrap_or("");
    let segs: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    let after = |name: &str| segs.iter().position(|s| *s == name).and_then(|i| segs.get(i + 1)).map(|s| s.to_string());
    if let Some(id) = after("folders").filter(|id| valid_drive_id(id)) {
        return Some(DriveLink::Folder(id));
    }
    if segs.first() == Some(&"file") || segs.contains(&"file") {
        if let Some(id) = after("d").filter(|id| valid_drive_id(id)) {
            return Some(DriveLink::File(id));
        }
    }
    let qid = query
        .split('&')
        .filter_map(|kv| kv.split_once('='))
        .find(|(k, _)| *k == "id")
        .map(|(_, v)| v.split('#').next().unwrap_or("").to_string())
        .filter(|id| valid_drive_id(id))?;
    match segs.last().copied() {
        Some("folderview" | "embeddedfolderview") => Some(DriveLink::Folder(qid)),
        _ => Some(DriveLink::File(qid)),
    }
}

pub fn drive_download_url(id: &str) -> String {
    format!("https://drive.usercontent.google.com/download?id={id}&export=download&confirm=t")
}

pub fn drive_folder_url(id: &str) -> String {
    format!("https://drive.google.com/embeddedfolderview?id={id}")
}

/// Id các FILE trong trang xem thư mục Drive (bỏ thư mục con), giữ thứ tự, không trùng.
pub fn folder_file_ids(html: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for part in html.split("/file/d/").skip(1) {
        let id: String = part.chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_').collect();
        if valid_drive_id(&id) && !out.contains(&id) {
            out.push(id);
        }
    }
    out
}

/// Nhận dạng file theo vài byte đầu (file Drive không mang tên / đuôi). Trả đuôi.
pub fn sniff(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("jpg")
    } else if bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
        Some("png")
    } else if bytes.starts_with(b"BM") && bytes.len() > 14 {
        Some("bmp")
    } else if bytes.len() > 12 && &bytes[4..8] == b"ftyp" {
        Some("mp4")
    } else if bytes.starts_with(&[0x1A, 0x45, 0xDF, 0xA3]) {
        Some("webm")
    } else {
        None
    }
}

/// Header `Range: bytes=a-b` → (đầu, cuối) tính cả hai đầu, cắt theo độ dài file và
/// RANGE_CHUNK. None = header hỏng hoặc ngoài file (trả 416).
pub fn parse_range(header: &str, len: u64) -> Option<(u64, u64)> {
    if len == 0 {
        return None;
    }
    let spec = header.trim().strip_prefix("bytes=")?.split(',').next()?.trim();
    let (a, b) = spec.split_once('-')?;
    let (start, end) = if a.is_empty() {
        // "bytes=-n": n byte cuối.
        let n: u64 = b.parse().ok()?;
        if n == 0 {
            return None;
        }
        (len.saturating_sub(n), len - 1)
    } else {
        let start: u64 = a.parse().ok()?;
        let end = if b.is_empty() { len - 1 } else { b.parse::<u64>().ok()?.min(len - 1) };
        (start, end)
    };
    if start >= len || end < start {
        return None;
    }
    Some((start, end.min(start + RANGE_CHUNK - 1)))
}

/// Tên file an toàn cho ảnh vẽ sẵn (id theme): chỉ chữ thường, số, gạch nối.
pub fn safe_name(s: &str) -> Option<String> {
    (!s.is_empty() && s.len() <= 40 && s.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-'))
        .then(|| s.to_string())
}

// ─── Hệ điều hành ────────────────────────────────────────────────────────────

#[cfg(windows)]
mod os {
    use std::path::Path;

    /// Đặt `path` làm hình nền mọi màn hình với kiểu `fit`, rồi đọc lại để chắc.
    /// Chạy trên luồng riêng (COM STA của riêng nó, không đụng luồng đang gọi).
    pub fn set_image(path: &Path, fit: &str) -> Result<(), String> {
        let path = path.to_path_buf();
        let code = super::fit_code(fit);
        std::thread::spawn(move || set_on_this_thread(&path, code))
            .join()
            .map_err(|_| "Luồng đặt hình nền bị lỗi.".to_string())?
    }

    fn set_on_this_thread(path: &Path, code: i32) -> Result<(), String> {
        use windows::core::{HSTRING, PCWSTR};
        use windows::Win32::System::Com::{
            CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize, CLSCTX_ALL, COINIT_APARTMENTTHREADED,
        };
        use windows::Win32::UI::Shell::{DesktopWallpaper, IDesktopWallpaper, DESKTOP_WALLPAPER_POSITION};

        // SAFETY: luồng mới của riêng hàm này; CoUninitialize chỉ gọi khi init thành công,
        // sau khi mọi đối tượng COM (dw) đã được thả ở cuối closure.
        unsafe {
            let init = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            let res = (|| -> Result<String, String> {
                let dw: IDesktopWallpaper = CoCreateInstance(&DesktopWallpaper, None, CLSCTX_ALL)
                    .map_err(|e| format!("Không mở được dịch vụ hình nền của Windows: {e}"))?;
                dw.SetPosition(DESKTOP_WALLPAPER_POSITION(code))
                    .map_err(|e| format!("Windows không nhận kiểu hiển thị: {e}"))?;
                let p = HSTRING::from(path.as_os_str());
                dw.SetWallpaper(PCWSTR::null(), &p).map_err(|e| format!("Windows không đặt được hình nền: {e}"))?;
                // Đọc lại hình nền của màn hình đầu tiên.
                let id = dw.GetMonitorDevicePathAt(0).map_err(|e| format!("Không đọc lại được hình nền: {e}"))?;
                let got = dw.GetWallpaper(PCWSTR(id.0));
                CoTaskMemFree(Some(id.0 as *const _));
                let got = got.map_err(|e| format!("Không đọc lại được hình nền: {e}"))?;
                let s = got.to_string().unwrap_or_default();
                CoTaskMemFree(Some(got.0 as *const _));
                Ok(s)
            })();
            if init.is_ok() {
                CoUninitialize();
            }
            let got = res?;
            if super::same_path(Path::new(&got), path) {
                Ok(())
            } else {
                Err(format!(
                    "Windows nhận lệnh nhưng hình nền vẫn là \"{got}\". Máy có thể đang bị khoá đổi hình nền (chính sách công ty / chưa kích hoạt Windows)."
                ))
            }
        }
    }
}

#[cfg(target_os = "macos")]
mod os {
    use std::path::Path;

    /// macOS: hỏi System Events đặt ảnh cho mọi desktop (kiểu hiển thị do macOS quyết).
    pub fn set_image(path: &Path, _fit: &str) -> Result<(), String> {
        let p = path.to_string_lossy().replace('\\', "\\\\").replace('"', "\\\"");
        let script = format!("tell application \"System Events\" to tell every desktop to set picture to POSIX file \"{p}\"");
        let out = std::process::Command::new("osascript")
            .args(["-e", &script])
            .output()
            .map_err(|e| format!("Không chạy được osascript: {e}"))?;
        if out.status.success() {
            Ok(())
        } else {
            Err(format!("macOS không đặt được hình nền: {}", String::from_utf8_lossy(&out.stderr).trim()))
        }
    }
}

#[cfg(not(any(windows, target_os = "macos")))]
mod os {
    pub fn set_image(_path: &std::path::Path, _fit: &str) -> Result<(), String> {
        Err("Đổi hình nền chưa hỗ trợ trên hệ điều hành này.".into())
    }
}

// ─── Lưu trữ / trạng thái ────────────────────────────────────────────────────

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Thư mục ảnh của app (ảnh vẽ sẵn, ảnh / video tải từ Drive).
pub fn media_dir(app: &AppHandle, sub: &str) -> Result<PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?.join("wallpapers").join(sub);
    std::fs::create_dir_all(&dir).map_err(|e| format!("Không tạo được thư mục {}: {e}", dir.display()))?;
    Ok(dir)
}

fn update(app: &AppHandle, f: impl FnOnce(&mut WallSettings)) -> Result<(), String> {
    let mut s = store::load_settings(app);
    f(&mut s.wallpaper);
    store::save_settings(app, &s)
}

/// Báo trang Cài đặt vẽ lại mục Hình nền.
pub fn broadcast(app: &AppHandle) {
    let _ = app.emit_to("settings", "wallpaper-changed", status(app));
}

pub fn status(app: &AppHandle) -> Value {
    let w = store::load_settings(app).wallpaper;
    let image_ok = !w.image.is_empty() && Path::new(&w.image).is_file();
    json!({
        "fit": w.fit,
        "image": w.image,
        "imageOk": image_ok,
        "folder": w.folder,
        "intervalMin": w.interval_min,
        "shuffle": w.shuffle,
        "driveUrl": w.drive_url,
        "video": w.video,
        "live": w.live,
        "lastError": w.last_error,
        "liveSupported": cfg!(windows),
        "stamp": w.last_change,
    })
}

/// Đặt một ảnh làm hình nền và nhớ lại. `keep_folder` = đổi trong vòng xoay vòng
/// (không thì chọn ảnh lẻ tắt xoay vòng).
pub fn apply_image(app: &AppHandle, path: &Path, keep_folder: bool) -> Result<(), String> {
    if !is_image(path) {
        return Err(format!("Chỉ nhận ảnh {}.", IMAGE_EXT.join(", ")));
    }
    if !path.is_file() {
        return Err(format!("Không thấy file {}.", path.display()));
    }
    let fit = store::load_settings(app).wallpaper.fit;
    os::set_image(path, &fit)?;
    update(app, |w| {
        w.image = path.to_string_lossy().into_owned();
        w.last_change = now_secs();
        w.last_error.clear();
        if !keep_folder {
            w.folder.clear();
        }
    })?;
    broadcast(app);
    Ok(())
}

pub fn set_fit(app: &AppHandle, fit: &str) -> Result<(), String> {
    if !FITS.contains(&fit) {
        return Err(format!("Kiểu hiển thị lạ: {fit}"));
    }
    update(app, |w| w.fit = fit.to_string())?;
    // Áp ngay cho ảnh đang đặt (nếu còn file).
    let w = store::load_settings(app).wallpaper;
    let p = PathBuf::from(&w.image);
    if !w.image.is_empty() && p.is_file() {
        os::set_image(&p, fit)?;
    }
    broadcast(app);
    Ok(())
}

fn list_folder(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let rd = std::fs::read_dir(dir).map_err(|e| format!("Không đọc được thư mục {}: {e}", dir.display()))?;
    Ok(sorted_images(rd.filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.is_file()).collect()))
}

/// Chọn thư mục xoay vòng: đặt ngay một ảnh trong đó. Trả số ảnh tìm thấy.
pub fn set_folder(app: &AppHandle, folder: &str, interval_min: u32, shuffle: bool) -> Result<usize, String> {
    if !INTERVALS.contains(&interval_min) {
        return Err(format!("Chu kỳ lạ: {interval_min} phút"));
    }
    let dir = PathBuf::from(folder);
    let list = list_folder(&dir)?;
    if list.is_empty() {
        return Err(format!("Thư mục không có ảnh {}.", IMAGE_EXT.join(", ")));
    }
    let cur = PathBuf::from(store::load_settings(app).wallpaper.image);
    let first = next_image(&list, &cur, shuffle, now_secs()).expect("list không rỗng");
    update(app, |w| {
        w.folder = folder.to_string();
        w.interval_min = interval_min;
        w.shuffle = shuffle;
    })?;
    apply_image(app, &first, true)?;
    Ok(list.len())
}

/// Đổi chu kỳ / ngẫu nhiên mà không đổi ảnh.
pub fn set_rotation(app: &AppHandle, interval_min: u32, shuffle: bool) -> Result<(), String> {
    if !INTERVALS.contains(&interval_min) {
        return Err(format!("Chu kỳ lạ: {interval_min} phút"));
    }
    update(app, |w| {
        w.interval_min = interval_min;
        w.shuffle = shuffle;
    })?;
    broadcast(app);
    Ok(())
}

pub fn stop_rotation(app: &AppHandle) -> Result<(), String> {
    update(app, |w| w.folder.clear())?;
    broadcast(app);
    Ok(())
}

/// Ảnh kế trong thư mục xoay vòng.
pub fn next(app: &AppHandle) -> Result<(), String> {
    let w = store::load_settings(app).wallpaper;
    if w.folder.is_empty() {
        return Err("Chưa chọn thư mục xoay vòng.".into());
    }
    let list = list_folder(Path::new(&w.folder))?;
    let p = next_image(&list, Path::new(&w.image), w.shuffle, now_secs())
        .ok_or_else(|| "Thư mục không còn ảnh nào.".to_string())?;
    apply_image(app, &p, true)
}

/// Ảnh vẽ sẵn (canvas trong Cài đặt, PNG base64): lưu vào thư mục app rồi đặt.
pub fn save_generated(app: &AppHandle, name: &str, png_base64: &str) -> Result<(), String> {
    use base64::Engine;
    let name = safe_name(name).ok_or("Tên ảnh lạ.")?;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(png_base64.trim())
        .map_err(|e| format!("Ảnh hỏng: {e}"))?;
    if sniff(&bytes) != Some("png") || bytes.len() as u64 > MAX_IMAGE_BYTES {
        return Err("Ảnh vẽ ra không hợp lệ.".into());
    }
    let path = media_dir(app, "builtin")?.join(format!("{name}.png"));
    std::fs::write(&path, &bytes).map_err(|e| format!("Không lưu được ảnh: {e}"))?;
    apply_image(app, &path, false)
}

// ─── Google Drive ────────────────────────────────────────────────────────────

fn client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent("DesktopCompanion")
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(600))
        .build()
        .map_err(|e| e.to_string())
}

const NOT_PUBLIC: &str =
    "Drive trả về trang web thay vì file. Mở chia sẻ link thành \"Bất kỳ ai có đường liên kết\" rồi thử lại.";

/// Tải một file Drive (có giới hạn dung lượng). Trả (byte, đuôi nhận dạng).
async fn download(client: &reqwest::Client, id: &str) -> Result<(Vec<u8>, &'static str), String> {
    let mut resp = client
        .get(drive_download_url(id))
        .send()
        .await
        .map_err(|e| format!("Không tải được từ Drive: {e}"))?;
    if !resp.status().is_success() {
        return Err(if resp.status().as_u16() == 404 || resp.status().as_u16() == 403 {
            NOT_PUBLIC.to_string()
        } else {
            format!("Drive trả mã {}.", resp.status())
        });
    }
    if resp.content_length().is_some_and(|n| n > MAX_VIDEO_BYTES) {
        return Err(format!("File quá lớn (tối đa {} MB).", MAX_VIDEO_BYTES / 1024 / 1024));
    }
    let mut buf: Vec<u8> = Vec::new();
    while let Some(chunk) = resp.chunk().await.map_err(|e| format!("Tải dở thì lỗi: {e}"))? {
        buf.extend_from_slice(&chunk);
        if buf.len() as u64 > MAX_VIDEO_BYTES {
            return Err(format!("File quá lớn (tối đa {} MB).", MAX_VIDEO_BYTES / 1024 / 1024));
        }
    }
    let ext = sniff(&buf).ok_or_else(|| {
        if buf.trim_ascii_start().starts_with(b"<") {
            NOT_PUBLIC.to_string()
        } else {
            format!("Không phải ảnh ({}) hay video ({}).", IMAGE_EXT.join(", "), VIDEO_EXT.join(", "))
        }
    })?;
    if IMAGE_EXT.contains(&ext) && buf.len() as u64 > MAX_IMAGE_BYTES {
        return Err(format!("Ảnh quá lớn (tối đa {} MB).", MAX_IMAGE_BYTES / 1024 / 1024));
    }
    Ok((buf, ext))
}

/// Link Drive: ảnh → đặt làm hình nền; video → hình nền động; thư mục → tải các
/// ảnh về rồi xoay vòng. Trả câu báo kết quả.
pub async fn from_drive(app: AppHandle, url: String) -> Result<String, String> {
    let link = drive_link(&url).ok_or("Không phải link Google Drive của một file hoặc thư mục.")?;
    update(&app, |w| w.drive_url = url.trim().to_string())?;
    let client = client()?;
    match link {
        DriveLink::File(id) => {
            let (bytes, ext) = download(&client, &id).await?;
            let path = media_dir(&app, "drive")?.join(format!("{id}.{ext}"));
            std::fs::write(&path, &bytes).map_err(|e| format!("Không lưu được file: {e}"))?;
            if VIDEO_EXT.contains(&ext) {
                crate::wallpaper_live::set_video(&app, Some(path.to_string_lossy().into_owned()))?;
                Ok("video".into())
            } else {
                let app2 = app.clone();
                tauri::async_runtime::spawn_blocking(move || apply_image(&app2, &path, false))
                    .await
                    .map_err(|e| e.to_string())??;
                Ok("image".into())
            }
        }
        DriveLink::Folder(fid) => {
            let html = client
                .get(drive_folder_url(&fid))
                .send()
                .await
                .and_then(|r| r.error_for_status())
                .map_err(|_| NOT_PUBLIC.to_string())?
                .text()
                .await
                .map_err(|e| e.to_string())?;
            let ids: Vec<String> = folder_file_ids(&html).into_iter().take(DRIVE_FOLDER_MAX).collect();
            if ids.is_empty() {
                return Err("Thư mục Drive trống, hoặc chưa mở chia sẻ \"Bất kỳ ai có đường liên kết\".".into());
            }
            let dir = media_dir(&app, &format!("drive-{fid}"))?;
            let total = ids.len();
            let mut saved = 0usize;
            for (i, id) in ids.iter().enumerate() {
                let _ = app.emit_to("settings", "wallpaper-progress", json!({ "done": i, "total": total }));
                // Đã tải ở lần trước: giữ nguyên.
                if IMAGE_EXT.iter().any(|e| dir.join(format!("{id}.{e}")).is_file()) {
                    saved += 1;
                    continue;
                }
                match download(&client, id).await {
                    Ok((bytes, ext)) if IMAGE_EXT.contains(&ext) => {
                        if std::fs::write(dir.join(format!("{id}.{ext}")), &bytes).is_ok() {
                            saved += 1;
                        }
                    }
                    // Video / file khác trong thư mục: bỏ qua.
                    Ok(_) => {}
                    Err(e) => eprintln!("[wallpaper] Drive {id}: {e}"),
                }
            }
            // Ảnh đã xoá khỏi thư mục Drive: xoá bản tải về.
            if let Ok(rd) = std::fs::read_dir(&dir) {
                for e in rd.flatten() {
                    let stem = e.path().file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
                    if !ids.contains(&stem) {
                        let _ = std::fs::remove_file(e.path());
                    }
                }
            }
            let _ = app.emit_to("settings", "wallpaper-progress", json!({ "done": total, "total": total }));
            if saved == 0 {
                return Err("Thư mục Drive không có ảnh JPG / PNG / BMP tải được.".into());
            }
            let w = store::load_settings(&app).wallpaper;
            let interval = if w.interval_min == 0 { 30 } else { w.interval_min };
            let folder = dir.to_string_lossy().into_owned();
            let app2 = app.clone();
            tauri::async_runtime::spawn_blocking(move || set_folder(&app2, &folder, interval, w.shuffle))
                .await
                .map_err(|e| e.to_string())??;
            Ok(format!("folder:{saved}"))
        }
    }
}

// ─── Giao thức wallmedia:// (xem trước ảnh, phát video hình nền động) ─────────
// Chỉ phát ĐÚNG hai file trong cài đặt (ảnh đang đặt, video đang chọn): trang web
// không xin được file nào khác trên máy qua giao thức này.

pub fn serve(app: &AppHandle, request: &tauri::http::Request<Vec<u8>>) -> tauri::http::Response<Vec<u8>> {
    use std::io::{Read, Seek, SeekFrom};
    use tauri::http::Response;

    let empty = |code: u16| Response::builder().status(code).body(Vec::new()).unwrap();
    let w = store::load_settings(app).wallpaper;
    let path = request.uri().path().trim_start_matches('/');
    let file = match path {
        "image" if !w.image.is_empty() => PathBuf::from(&w.image),
        "video" if !w.video.is_empty() => PathBuf::from(&w.video),
        _ => return empty(404),
    };
    let ct = match ext_of(&file).as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "bmp" => "image/bmp",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        _ => return empty(404),
    };
    let Ok(mut f) = std::fs::File::open(&file) else { return empty(404) };
    let len = f.metadata().map(|m| m.len()).unwrap_or(0);
    let range = request.headers().get("range").and_then(|v| v.to_str().ok());
    match range {
        Some(h) => {
            let Some((start, end)) = parse_range(h, len) else {
                return Response::builder()
                    .status(416)
                    .header("Content-Range", format!("bytes */{len}"))
                    .body(Vec::new())
                    .unwrap();
            };
            let mut buf = vec![0u8; (end - start + 1) as usize];
            if f.seek(SeekFrom::Start(start)).is_err() || f.read_exact(&mut buf).is_err() {
                return empty(500);
            }
            Response::builder()
                .status(206)
                .header("Content-Type", ct)
                .header("Accept-Ranges", "bytes")
                .header("Content-Range", format!("bytes {start}-{end}/{len}"))
                .body(buf)
                .unwrap()
        }
        None => {
            // Ảnh xem trước: trả cả file. Video luôn hỏi theo Range; không có thì trả khúc đầu.
            if ct.starts_with("video") && len > RANGE_CHUNK {
                let mut buf = vec![0u8; RANGE_CHUNK as usize];
                if f.read_exact(&mut buf).is_err() {
                    return empty(500);
                }
                return Response::builder()
                    .status(206)
                    .header("Content-Type", ct)
                    .header("Accept-Ranges", "bytes")
                    .header("Content-Range", format!("bytes 0-{}/{len}", RANGE_CHUNK - 1))
                    .body(buf)
                    .unwrap();
            }
            if len > MAX_IMAGE_BYTES {
                return empty(413);
            }
            let mut buf = Vec::with_capacity(len as usize);
            if f.read_to_end(&mut buf).is_err() {
                return empty(500);
            }
            Response::builder()
                .header("Content-Type", ct)
                .header("Accept-Ranges", "bytes")
                .header("Cache-Control", "no-store")
                .body(buf)
                .unwrap()
        }
    }
}

// ─── Vòng xoay vòng ──────────────────────────────────────────────────────────

/// Mỗi 30 giây xem đến giờ đổi ảnh chưa. Lỗi ghi vào last_error (Cài đặt hiện ra)
/// thay vì im lặng; lỗi giống lần trước thì không ghi lại.
pub fn start(app: AppHandle) {
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_secs(30));
        let w = store::load_settings(&app).wallpaper;
        if w.folder.is_empty() || !due(w.last_change, now_secs(), w.interval_min) {
            continue;
        }
        if let Err(e) = next(&app) {
            eprintln!("[wallpaper] xoay vòng lỗi: {e}");
            let _ = update(&app, |w| {
                w.last_error = e;
                // Không thử lại mỗi 30 giây: chờ hết một chu kỳ nữa.
                w.last_change = now_secs();
            });
            broadcast(&app);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> PathBuf {
        PathBuf::from(s)
    }

    #[test]
    fn fit_codes_match_windows() {
        assert_eq!(fit_code("center"), 0);
        assert_eq!(fit_code("tile"), 1);
        assert_eq!(fit_code("stretch"), 2);
        assert_eq!(fit_code("fit"), 3);
        assert_eq!(fit_code("fill"), 4);
        assert_eq!(fit_code("span"), 5);
        assert_eq!(fit_code("lạ"), 4, "mặc định Lấp đầy");
        for f in FITS {
            assert!(fit_code(f) >= 0);
        }
    }

    #[test]
    fn only_supported_images_sorted_by_name() {
        let v = sorted_images(vec![p("b.PNG"), p("notes.txt"), p("A.jpg"), p("c.webp"), p("d.bmp"), p("clip.mp4")]);
        assert_eq!(v, vec![p("A.jpg"), p("b.PNG"), p("d.bmp")]);
        assert!(is_video(&p("x.MP4")) && is_video(&p("x.webm")) && !is_video(&p("x.mov")));
    }

    #[test]
    fn next_image_in_order_and_wraps() {
        let list = vec![p("C:\\w\\a.jpg"), p("C:\\w\\b.jpg"), p("C:\\w\\c.jpg")];
        assert_eq!(next_image(&list, &p("C:\\w\\a.jpg"), false, 0), Some(p("C:\\w\\b.jpg")));
        assert_eq!(next_image(&list, &p("c:/W/C.JPG"), false, 0), Some(p("C:\\w\\a.jpg")), "so đường dẫn không phân biệt hoa thường");
        assert_eq!(next_image(&list, &p("D:\\khác.jpg"), false, 0), Some(p("C:\\w\\a.jpg")));
        assert_eq!(next_image(&[], &p("a"), false, 0), None);
        assert_eq!(next_image(&list[..1], &list[0], true, 7), Some(list[0].clone()));
    }

    #[test]
    fn shuffle_never_repeats_current() {
        let list: Vec<PathBuf> = (0..5).map(|i| p(&format!("{i}.png"))).collect();
        for seed in 0..50 {
            for cur in &list {
                let n = next_image(&list, cur, true, seed).unwrap();
                assert_ne!(&n, cur, "seed {seed}");
            }
        }
    }

    #[test]
    fn due_by_interval() {
        assert!(!due(1000, 1000 + 29 * 60, 30));
        assert!(due(1000, 1000 + 30 * 60, 30));
        assert!(!due(0, 999_999, 0), "0 = không tự đổi");
        assert!(due(10_000, 100, 30), "đồng hồ lùi");
    }

    #[test]
    fn reads_drive_links() {
        let id = "1AbCdEfGhIjK_lmn-OPq";
        assert_eq!(drive_link(&format!("https://drive.google.com/file/d/{id}/view?usp=sharing")), Some(DriveLink::File(id.into())));
        assert_eq!(drive_link(&format!("https://drive.google.com/open?id={id}")), Some(DriveLink::File(id.into())));
        assert_eq!(drive_link(&format!("https://drive.google.com/uc?export=download&id={id}")), Some(DriveLink::File(id.into())));
        assert_eq!(drive_link(&format!(" https://drive.google.com/drive/folders/{id}?usp=drive_link ")), Some(DriveLink::Folder(id.into())));
        assert_eq!(drive_link(&format!("https://drive.google.com/drive/u/0/folders/{id}")), Some(DriveLink::Folder(id.into())));
        assert_eq!(drive_link(&format!("https://drive.google.com/embeddedfolderview?id={id}")), Some(DriveLink::Folder(id.into())));
        assert_eq!(drive_link(&format!("https://example.com/file/d/{id}/view")), None, "host lạ");
        assert_eq!(drive_link(&format!("https://drive.google.com.example.com/file/d/{id}")), None);
        assert_eq!(drive_link("https://drive.google.com/file/d/short/view"), None);
        assert_eq!(drive_link("https://drive.google.com/file/d/abc%2F..%2Fxxxxxx/view"), None, "id chỉ có chữ, số, - _");
        assert_eq!(drive_link("ftp://drive.google.com/file/d/1AbCdEfGhIjK/view"), None);
    }

    #[test]
    fn folder_page_lists_files_only_once() {
        let html = r#"<a href="https://drive.google.com/file/d/1aaaaaaaaaaa/view?usp=drive_web">a</a>
            <a href="https://drive.google.com/drive/folders/1sub_folder_xx">sub</a>
            <a href="https://drive.google.com/file/d/1bbbbbbbbbbb-_/view">b</a>
            <a href="https://drive.google.com/file/d/1aaaaaaaaaaa/view">a again</a>"#;
        assert_eq!(folder_file_ids(html), vec!["1aaaaaaaaaaa".to_string(), "1bbbbbbbbbbb-_".to_string()]);
        assert!(folder_file_ids("<html>empty</html>").is_empty());
    }

    #[test]
    fn sniffs_file_types() {
        assert_eq!(sniff(&[0xFF, 0xD8, 0xFF, 0xE0]), Some("jpg"));
        assert_eq!(sniff(b"\x89PNG\r\n\x1a\n"), Some("png"));
        assert_eq!(sniff(b"BM0000000000000000"), Some("bmp"));
        assert_eq!(sniff(b"\0\0\0\x20ftypisom0000"), Some("mp4"));
        assert_eq!(sniff(&[0x1A, 0x45, 0xDF, 0xA3, 0]), Some("webm"));
        assert_eq!(sniff(b"<!DOCTYPE html>"), None);
        assert_eq!(sniff(b""), None);
    }

    #[test]
    fn parses_byte_ranges() {
        let len = 10 * RANGE_CHUNK;
        assert_eq!(parse_range("bytes=0-", len), Some((0, RANGE_CHUNK - 1)), "khúc đầu, cắt theo RANGE_CHUNK");
        assert_eq!(parse_range("bytes=100-199", len), Some((100, 199)));
        assert_eq!(parse_range("bytes=-100", len), Some((len - 100, len - 1)));
        assert_eq!(parse_range("bytes=5-3", len), None);
        assert_eq!(parse_range(&format!("bytes={len}-"), len), None, "ngoài file");
        assert_eq!(parse_range("bytes=0-999999999999", 50), Some((0, 49)));
        assert_eq!(parse_range("items=0-1", len), None);
        assert_eq!(parse_range("bytes=0-", 0), None);
    }

    #[test]
    fn generated_names_are_safe() {
        assert_eq!(safe_name("night-sky"), Some("night-sky".into()));
        assert_eq!(safe_name("../x"), None);
        assert_eq!(safe_name("A"), None);
        assert_eq!(safe_name(""), None);
    }

    #[test]
    fn old_settings_files_get_defaults() {
        let w: WallSettings = serde_json::from_str("{}").unwrap();
        assert_eq!(w, WallSettings::default());
        assert_eq!(w.fit, "fill");
        assert!(INTERVALS.contains(&w.interval_min));
        let w: WallSettings = serde_json::from_str(r#"{"fit":"tile","live":true}"#).unwrap();
        assert!(w.live && w.fit == "tile" && w.folder.is_empty());
    }

    /// Thử thật trên máy (không đổi gì): đặt lại đúng hình nền đang có rồi đọc lại.
    /// `cargo test wallpaper -- --ignored`
    #[cfg(windows)]
    #[test]
    #[ignore]
    fn live_reapply_current_wallpaper() {
        use windows::core::PCWSTR;
        use windows::Win32::System::Com::*;
        use windows::Win32::UI::Shell::*;
        // SAFETY: luồng test riêng; thả COM trước CoUninitialize.
        let cur = unsafe {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            let s = {
                let dw: IDesktopWallpaper = CoCreateInstance(&DesktopWallpaper, None, CLSCTX_ALL).unwrap();
                let id = dw.GetMonitorDevicePathAt(0).unwrap();
                let pos = dw.GetPosition().unwrap();
                let got = dw.GetWallpaper(PCWSTR(id.0)).unwrap();
                (got.to_string().unwrap(), pos.0)
            };
            CoUninitialize();
            s
        };
        eprintln!("hình nền hiện tại: {:?}", cur);
        if cur.0.is_empty() {
            eprintln!("máy đang dùng màu trơn / slideshow: bỏ qua");
            return;
        }
        let fit = FITS.iter().find(|f| fit_code(f) == cur.1).copied().unwrap_or("fill");
        os::set_image(Path::new(&cur.0), fit).expect("đặt lại hình nền đang có");
    }
}
