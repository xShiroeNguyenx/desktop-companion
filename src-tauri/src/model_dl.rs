//! ⬇️ Model Live2D tải riêng (PLAN.md 16.9): lấy manifest đã ký từ release
//! `models` trên GitHub, tải gói zip của model đã mở khoá, kiểm SHA-256, giải nén
//! vào thư mục dữ liệu app, và tự cập nhật khi manifest có bản mới hơn.
//!
//! Model đã cài nằm ở `<app_data>/models/<id>/`, phiên bản ghi ở
//! `<app_data>/models/installed.json`. Cửa sổ pet đọc model qua giao thức
//! `petmodel` như model tự thêm (main.rs). Phần kiểm gói thuần ở model_pack.rs.
//!
//! Lỗi trả về là MÃ để trang Bộ sưu tập dịch ra 3 thứ tiếng: offline · no_package
//! · bad_signature · bad_manifest · bad_checksum · bad_package · not_owned ·
//! not_found · busy. Chi tiết in ra log.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri::{AppHandle, Emitter, Manager};

use crate::model_pack::{self, Manifest};
use crate::{collection, models, store};

/// Release cố định chứa gói model + manifest. Đánh dấu prerelease để không thành
/// "latest" (updater của app đọc releases/latest).
pub const RELEASE_BASE: &str = "https://github.com/xShiroeNguyenx/desktop-companion/releases/download/models/";
const MANIFEST_FILE: &str = "manifest.json";
/// Manifest đọc lại sau chừng này (mở Bộ sưu tập liên tục không gọi mạng mỗi lần).
const MANIFEST_TTL: Duration = Duration::from_secs(10 * 60);

static MANIFEST: Mutex<Option<(Instant, Manifest)>> = Mutex::new(None);
/// Model đang tải: (id, phần trăm).
static BUSY: Mutex<Vec<(String, u8)>> = Mutex::new(Vec::new());

#[derive(Default, Serialize, Deserialize)]
struct Installed {
    /// id → phiên bản gói đã cài.
    #[serde(default)]
    models: HashMap<String, u32>,
}

fn root(app: &AppHandle) -> Option<PathBuf> {
    app.path().app_data_dir().ok().map(|d| d.join("models"))
}

fn load_installed(app: &AppHandle) -> Installed {
    root(app)
        .and_then(|r| std::fs::read_to_string(r.join("installed.json")).ok())
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

fn save_installed(app: &AppHandle, inst: &Installed) -> Result<(), String> {
    let r = root(app).ok_or("không có thư mục dữ liệu")?;
    std::fs::create_dir_all(&r).map_err(|e| e.to_string())?;
    let text = serde_json::to_string_pretty(inst).map_err(|e| e.to_string())?;
    std::fs::write(r.join("installed.json"), text).map_err(|e| e.to_string())
}

/// Phiên bản đã cài, chỉ khi thư mục còn đủ file model (người dùng xoá tay → coi như chưa cài).
pub fn installed_version(app: &AppHandle, id: &str) -> Option<u32> {
    let v = *load_installed(app).models.get(id)?;
    let m = models::builtin(id)?;
    root(app)?.join(id).join(m.entry).is_file().then_some(v)
}

/// Id các model tải riêng đã cài (để biết model nào dùng được).
pub fn installed_ids(app: &AppHandle) -> Vec<String> {
    models::BUILTIN
        .iter()
        .filter(|m| m.downloadable() && installed_version(app, m.id).is_some())
        .map(|m| m.id.to_string())
        .collect()
}

/// Thư mục của model đã cài (giao thức `petmodel` đọc file từ đây).
pub fn installed_dir(app: &AppHandle, id: &str) -> Option<PathBuf> {
    installed_version(app, id)?;
    root(app).map(|r| r.join(id))
}

/// Phần trăm đang tải, `None` nếu không tải.
pub fn progress(id: &str) -> Option<u8> {
    BUSY.lock().ok()?.iter().find(|(i, _)| i == id).map(|(_, p)| *p)
}

fn set_progress(id: &str, pct: u8) {
    if let Ok(mut b) = BUSY.lock() {
        if let Some(e) = b.iter_mut().find(|(i, _)| i == id) {
            e.1 = pct;
        }
    }
}

/// Manifest đã tải gần nhất (không gọi mạng).
pub fn cached_manifest() -> Option<Manifest> {
    MANIFEST.lock().ok()?.as_ref().map(|(_, m)| m.clone())
}

fn client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(180))
        .connect_timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())
}

/// Tải `url`, tối đa `cap` byte; `on_progress(đã tải)` sau mỗi khúc.
async fn fetch(url: &str, cap: u64, mut on_progress: impl FnMut(u64)) -> Result<Vec<u8>, String> {
    let mut resp = client()?.get(url).send().await.map_err(|e| {
        eprintln!("[model_dl] {url}: {e}");
        "offline".to_string()
    })?;
    if resp.status() == reqwest::StatusCode::NOT_FOUND {
        return Err("no_package".into());
    }
    if !resp.status().is_success() {
        eprintln!("[model_dl] {url}: HTTP {}", resp.status());
        return Err("offline".into());
    }
    if resp.content_length().is_some_and(|n| n > cap) {
        return Err("bad_checksum".into());
    }
    let mut out = Vec::new();
    while let Some(chunk) = resp.chunk().await.map_err(|e| {
        eprintln!("[model_dl] {url}: {e}");
        "offline".to_string()
    })? {
        out.extend_from_slice(&chunk);
        if out.len() as u64 > cap {
            return Err("bad_checksum".into());
        }
        on_progress(out.len() as u64);
    }
    Ok(out)
}

/// Khoá công khai của updater (tauri.conf.json): manifest model ký bằng cùng khoá.
fn pubkey(app: &AppHandle) -> Option<String> {
    app.config().plugins.0.get("updater")?.get("pubkey")?.as_str().map(String::from)
}

/// Manifest đã kiểm chữ ký. Dùng bản nhớ nếu còn mới, trừ khi `fresh`.
pub async fn manifest(app: &AppHandle, fresh: bool) -> Result<Manifest, String> {
    if !fresh {
        if let Some((at, m)) = MANIFEST.lock().ok().and_then(|g| g.clone()) {
            if at.elapsed() < MANIFEST_TTL {
                return Ok(m);
            }
        }
    }
    let data = fetch(&format!("{RELEASE_BASE}{MANIFEST_FILE}"), 256 * 1024, |_| {}).await?;
    let sig = fetch(&format!("{RELEASE_BASE}{MANIFEST_FILE}.sig"), 16 * 1024, |_| {}).await?;
    let key = pubkey(app).ok_or("bad_signature")?;
    model_pack::verify_signature(&data, &String::from_utf8_lossy(&sig), &key).map_err(|e| {
        eprintln!("[model_dl] manifest: {e}");
        "bad_signature".to_string()
    })?;
    let m = model_pack::parse_manifest(&String::from_utf8_lossy(&data)).map_err(|e| {
        eprintln!("[model_dl] manifest: {e}");
        "bad_manifest".to_string()
    })?;
    if let Ok(mut g) = MANIFEST.lock() {
        *g = Some((Instant::now(), m.clone()));
    }
    Ok(m)
}

/// Có model tải riêng nào đã mở khoá không (không thì khỏi gọi mạng).
fn owned_downloadables(app: &AppHandle) -> Vec<&'static models::Builtin> {
    let c = store::load_collection(app);
    models::BUILTIN
        .iter()
        .filter(|m| m.downloadable() && m.item.and_then(collection::item).is_some_and(|it| collection::owns(&c, it)))
        .collect()
}

/// Mở Bộ sưu tập (hoặc khởi động): đọc lại manifest nếu cũ để thẻ model hiện
/// dung lượng và nút Cập nhật. Có gì đổi thì báo trang.
pub async fn refresh_manifest(app: AppHandle) {
    let before = cached_manifest();
    if manifest(&app, false).await.is_ok() && cached_manifest() != before {
        let _ = app.emit("collection-changed", collection::view(&app));
    }
}

/// Tải + cài (hoặc cập nhật) model `id`. Trả phiên bản đã cài. Báo tiến độ qua
/// sự kiện `model-install` {id, state: progress|done|error, pct|version|error}.
pub async fn install(app: AppHandle, id: String) -> Result<u32, String> {
    let b = models::builtin(&id).filter(|m| m.downloadable()).ok_or("not_found")?;
    if !owned_downloadables(&app).iter().any(|m| m.id == id) {
        return Err("not_owned".into());
    }
    {
        let mut busy = BUSY.lock().map_err(|_| "busy")?;
        if busy.iter().any(|(i, _)| *i == id) {
            return Err("busy".into());
        }
        busy.push((id.clone(), 0));
    }
    let _ = app.emit("model-install", json!({ "id": id, "state": "progress", "pct": 0 }));
    let res = install_inner(&app, b).await;
    if let Ok(mut busy) = BUSY.lock() {
        busy.retain(|(i, _)| *i != id);
    }
    match &res {
        Ok(v) => {
            let _ = app.emit("model-install", json!({ "id": id, "state": "done", "version": v }));
            // Đang chọn model này (vd. cài lại máy, vừa mua rồi bấm Dùng) → pet đổi ngay.
            if store::load_settings(&app).pet_model == id {
                let _ = app.emit_to("main", "model-chosen", id.clone());
            }
        }
        Err(e) => {
            eprintln!("[model_dl] cài {id}: {e}");
            let _ = app.emit("model-install", json!({ "id": id, "state": "error", "error": e }));
        }
    }
    let _ = app.emit("collection-changed", collection::view(&app));
    res
}

async fn install_inner(app: &AppHandle, b: &'static models::Builtin) -> Result<u32, String> {
    let man = manifest(app, true).await?;
    let e = man.get(b.id).cloned().ok_or("no_package")?;
    if installed_version(app, b.id).is_some_and(|v| v >= e.version) {
        return Ok(e.version);
    }
    let total = e.size.max(1);
    let mut last = 0u8;
    let bytes = fetch(&format!("{RELEASE_BASE}{}", e.file), e.size, |got| {
        let pct = ((got * 100) / total).min(99) as u8;
        if pct != last {
            last = pct;
            set_progress(b.id, pct);
            let _ = app.emit("model-install", json!({ "id": b.id, "state": "progress", "pct": pct }));
        }
    })
    .await?;
    if bytes.len() as u64 != e.size || model_pack::sha256_hex(&bytes) != e.sha256.to_ascii_lowercase() {
        return Err("bad_checksum".into());
    }
    let r = root(app).ok_or("bad_package")?;
    let id = b.id;
    let entry = b.entry;
    // Giải nén vào thư mục tạm, kiểm đủ file, rồi mới thay bản cũ: hỏng giữa
    // chừng thì bản đang dùng vẫn nguyên.
    tauri::async_runtime::spawn_blocking(move || -> Result<(), String> {
        let tmp = r.join(format!(".{id}.tmp"));
        let old = r.join(format!(".{id}.old"));
        let dest = r.join(id);
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;
        let checked = model_pack::unzip_to(&bytes, &tmp).and_then(|_| model_pack::check_model_dir(&tmp, entry));
        // entry = model_pack::PACK_ENTRY (models.rs test bắt model tải riêng phải khai đúng tên này).
        if let Err(err) = checked {
            let _ = std::fs::remove_dir_all(&tmp);
            eprintln!("[model_dl] gói {id} hỏng: {err}");
            return Err("bad_package".into());
        }
        let _ = std::fs::remove_dir_all(&old);
        if dest.exists() {
            std::fs::rename(&dest, &old).map_err(|e| e.to_string())?;
        }
        if let Err(err) = std::fs::rename(&tmp, &dest) {
            let _ = std::fs::rename(&old, &dest);
            return Err(err.to_string());
        }
        let _ = std::fs::remove_dir_all(&old);
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())??;
    let mut inst = load_installed(app);
    inst.models.insert(id.to_string(), e.version);
    save_installed(app, &inst)?;
    Ok(e.version)
}

/// Lúc mở app: model đã mở khoá mà chưa cài (cài lại máy) hoặc có bản mới → tải.
pub async fn sync_on_start(app: AppHandle) {
    let owned = owned_downloadables(&app);
    if owned.is_empty() {
        return;
    }
    let Ok(man) = manifest(&app, false).await else { return };
    for m in owned {
        let newer = man.get(m.id).is_some_and(|e| installed_version(&app, m.id).is_none_or(|v| v < e.version));
        if newer {
            let _ = install(app.clone(), m.id.to_string()).await;
        }
    }
}

/// Trạng thái model cho thẻ trong Bộ sưu tập.
pub fn status_json(app: &AppHandle, id: &str) -> serde_json::Value {
    let installed = installed_version(app, id);
    let latest = cached_manifest().and_then(|m| m.get(id).cloned());
    json!({
        "installed": installed.is_some(),
        "version": installed,
        "progress": progress(id),
        "size": latest.as_ref().map(|e| e.size),
        // Bản mới hơn bản đang cài (nút Cập nhật).
        "update": match (installed, &latest) {
            (Some(v), Some(e)) if e.version > v => Some(e.version),
            _ => None,
        },
    })
}
