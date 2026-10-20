//! 📦 Gói model Live2D tải riêng (PLAN.md 16.9). Phần thuần, dùng chung cho app
//! (model_dl.rs) và công cụ đóng gói `cargo run --example pack_model`, nên chỉ
//! dựa vào std + crate ngoài, không đụng `crate::`.
//!
//! Một gói = file zip (Stored, không nén: texture PNG vốn đã nén) chứa thư mục
//! model. Danh sách gói = `manifest.json` ký bằng khoá updater (cùng định dạng
//! chữ ký với Tauri updater). App chỉ tin gói khớp SHA-256 trong manifest đã ký.

use std::io::{Cursor, Read, Write};
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

/// Trần dung lượng thư mục model (khuôn 16.3).
pub const MAX_MODEL_BYTES: u64 = 6 * 1024 * 1024;
/// Trần một file zip tải về và tổng dung lượng giải nén.
pub const MAX_ZIP_BYTES: u64 = 20 * 1024 * 1024;
const MAX_ENTRIES: usize = 500;
/// Phiên bản định dạng manifest app này đọc được.
pub const MANIFEST_FORMAT: u32 = 1;
/// Tên file model3.json trong gói, cố định bất kể tác giả đặt tên gì
/// (`Gothic_Aristocrat_2.model3.json`…): app bản nào cũng biết mở file nào.
/// Đổi tên an toàn vì không file nào trong model trỏ ngược về model3.json.
pub const PACK_ENTRY: &str = "model.model3.json";

pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect()
}

/// Đường dẫn tương đối an toàn: không rỗng, không tuyệt đối, không `..`.
fn safe_rel(p: &str) -> Option<PathBuf> {
    let path = Path::new(p);
    // `:` chặn "C:/…" trên mọi hệ điều hành (Linux coi "C:" là tên thường).
    if p.is_empty() || p.contains('\\') || p.contains(':') {
        return None;
    }
    path.components().all(|c| matches!(c, Component::Normal(_))).then(|| path.to_path_buf())
}

/// Mọi file `model3.json` trỏ tới (moc, texture, physics, motion, expression…).
pub fn referenced_files(model3: &Value) -> Vec<String> {
    let refs = &model3["FileReferences"];
    let mut files: Vec<String> = Vec::new();
    let mut push = |v: &Value| {
        if let Some(s) = v.as_str() {
            files.push(s.to_string());
        }
    };
    push(&refs["Moc"]);
    for t in refs["Textures"].as_array().into_iter().flatten() {
        push(t);
    }
    for key in ["Physics", "Pose", "DisplayInfo", "UserData"] {
        push(&refs[key]);
    }
    for group in refs["Motions"].as_object().into_iter().flat_map(|o| o.values()) {
        for m in group.as_array().into_iter().flatten() {
            push(&m["File"]);
        }
    }
    for e in refs["Expressions"].as_array().into_iter().flatten() {
        push(&e["File"]);
    }
    files
}

/// Kiểm thư mục model theo khuôn 16.3: có `entry`, đủ file nó trỏ tới (không
/// trỏ ra ngoài thư mục), có motion `Idle`/`TapHead`/`TapBody`, group
/// `EyeBlink`/`LipSync`, tổng dung lượng ≤ trần. Trả tổng dung lượng.
pub fn check_model_dir(dir: &Path, entry: &str) -> Result<u64, String> {
    let rel = safe_rel(entry).ok_or_else(|| format!("tên file model lạ: {entry}"))?;
    let text = std::fs::read_to_string(dir.join(&rel)).map_err(|e| format!("không đọc được {entry}: {e}"))?;
    let j: Value = serde_json::from_str(&text).map_err(|e| format!("{entry} hỏng: {e}"))?;
    let mut size = text.len() as u64;
    let files = referenced_files(&j);
    if files.is_empty() {
        return Err(format!("{entry} không trỏ tới file nào"));
    }
    for f in &files {
        let p = safe_rel(f).ok_or_else(|| format!("đường dẫn không an toàn: {f}"))?;
        let md = std::fs::metadata(dir.join(p)).map_err(|_| format!("thiếu {f}"))?;
        size += md.len();
    }
    let motions = &j["FileReferences"]["Motions"];
    for g in ["Idle", "TapHead", "TapBody"] {
        if !motions[g].as_array().is_some_and(|a| !a.is_empty()) {
            return Err(format!("thiếu nhóm motion {g}"));
        }
    }
    let groups: Vec<&str> = j["Groups"].as_array().into_iter().flatten().filter_map(|g| g["Name"].as_str()).collect();
    for g in ["EyeBlink", "LipSync"] {
        if !groups.contains(&g) {
            return Err(format!("thiếu group {g}"));
        }
    }
    if size > MAX_MODEL_BYTES {
        return Err(format!("{size} byte, quá trần {MAX_MODEL_BYTES}"));
    }
    Ok(size)
}

/// Đóng gói model thành zip: `entry` (lưu dưới tên `PACK_ENTRY`) và các file nó
/// trỏ tới (file nguồn .psd/.cmo3 hay ảnh nháp trong thư mục không lọt vào gói).
/// Stored, thời gian cố định, thứ tự sắp xếp → cùng nội dung ra cùng file, cùng SHA-256.
#[cfg_attr(not(test), allow(dead_code))] // app chỉ giải nén; đóng gói dùng ở examples/pack_model.rs
pub fn zip_model(dir: &Path, entry: &str) -> Result<Vec<u8>, String> {
    check_model_dir(dir, entry)?;
    let text = std::fs::read_to_string(dir.join(entry)).map_err(|e| e.to_string())?;
    let j: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let mut names: Vec<String> = referenced_files(&j);
    if names.iter().any(|n| n == PACK_ENTRY) {
        return Err(format!("model có file trùng tên {PACK_ENTRY}"));
    }
    names.sort();
    names.dedup();
    // (tên trong gói, file nguồn)
    let mut files: Vec<(String, String)> = names.into_iter().map(|n| (n.clone(), n)).collect();
    files.push((PACK_ENTRY.to_string(), entry.to_string()));
    files.sort();
    let mut w = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let opts = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Stored)
        .last_modified_time(zip::DateTime::default());
    for (name, src) in files {
        let bytes = std::fs::read(dir.join(&src)).map_err(|e| format!("{src}: {e}"))?;
        w.start_file(name.as_str(), opts).map_err(|e| e.to_string())?;
        w.write_all(&bytes).map_err(|e| e.to_string())?;
    }
    Ok(w.finish().map_err(|e| e.to_string())?.into_inner())
}

/// Giải nén vào `dest` (phải chưa có hoặc rỗng). Từ chối entry trỏ ra ngoài
/// (`../`, đường dẫn tuyệt đối), quá nhiều file, hoặc tổng dung lượng quá trần.
pub fn unzip_to(bytes: &[u8], dest: &Path) -> Result<(), String> {
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).map_err(|e| format!("zip hỏng: {e}"))?;
    if zip.len() > MAX_ENTRIES {
        return Err(format!("{} file, quá {MAX_ENTRIES}", zip.len()));
    }
    let mut total: u64 = 0;
    for i in 0..zip.len() {
        let mut f = zip.by_index(i).map_err(|e| format!("zip hỏng: {e}"))?;
        let name = f.name().to_string();
        let rel = safe_rel(name.trim_end_matches('/')).ok_or_else(|| format!("đường dẫn không an toàn: {name}"))?;
        let out = dest.join(rel);
        if f.is_dir() {
            std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
            continue;
        }
        total += f.size();
        if total > MAX_ZIP_BYTES {
            return Err("giải nén quá trần dung lượng".into());
        }
        if let Some(parent) = out.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let mut buf = Vec::with_capacity(f.size() as usize);
        f.read_to_end(&mut buf).map_err(|e| format!("{name}: {e}"))?;
        std::fs::write(&out, &buf).map_err(|e| format!("{name}: {e}"))?;
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PackEntry {
    /// Id trong models::BUILTIN.
    pub id: String,
    /// Tăng mỗi lần cập nhật model; app cài bản lớn hơn bản đang có.
    pub version: u32,
    /// Tên file zip trên release (ghép với địa chỉ gốc cố định trong app).
    pub file: String,
    pub sha256: String,
    pub size: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Manifest {
    pub format: u32,
    pub models: Vec<PackEntry>,
}

impl Manifest {
    pub fn get(&self, id: &str) -> Option<&PackEntry> {
        self.models.iter().find(|m| m.id == id)
    }
}

/// Đọc và kiểm manifest: định dạng, tên file chỉ gồm `[a-z0-9.-]` và đuôi `.zip`,
/// SHA-256 đủ 64 ký tự hex, dung lượng trong trần, không trùng id.
pub fn parse_manifest(text: &str) -> Result<Manifest, String> {
    let m: Manifest = serde_json::from_str(text).map_err(|e| format!("manifest hỏng: {e}"))?;
    if m.format != MANIFEST_FORMAT {
        return Err(format!("manifest định dạng {} (app đọc {MANIFEST_FORMAT})", m.format));
    }
    for (i, e) in m.models.iter().enumerate() {
        let name_ok = !e.file.is_empty()
            && e.file.ends_with(".zip")
            && e.file.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.' || c == '-')
            && !e.file.starts_with('.');
        if !name_ok {
            return Err(format!("{}: tên file lạ {:?}", e.id, e.file));
        }
        if e.sha256.len() != 64 || !e.sha256.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(format!("{}: sha256 lạ", e.id));
        }
        if e.size == 0 || e.size > MAX_ZIP_BYTES {
            return Err(format!("{}: dung lượng {} ngoài trần", e.id, e.size));
        }
        if m.models[..i].iter().any(|o| o.id == e.id) {
            return Err(format!("{}: trùng id", e.id));
        }
    }
    Ok(m)
}

/// Kiểm chữ ký theo đúng định dạng Tauri updater: `sig_b64` là nội dung file
/// `.sig` (base64 của chữ ký minisign), `pubkey_b64` là `plugins.updater.pubkey`.
pub fn verify_signature(data: &[u8], sig_b64: &str, pubkey_b64: &str) -> Result<(), String> {
    use base64::Engine;
    let decode = |s: &str| -> Result<String, String> {
        let raw = base64::engine::general_purpose::STANDARD.decode(s.trim()).map_err(|e| e.to_string())?;
        String::from_utf8(raw).map_err(|e| e.to_string())
    };
    let pk = minisign_verify::PublicKey::decode(&decode(pubkey_b64)?).map_err(|e| format!("khoá công khai lạ: {e}"))?;
    let sig = minisign_verify::Signature::decode(&decode(sig_b64)?).map_err(|e| format!("chữ ký lạ: {e}"))?;
    pk.verify(data, &sig, true).map_err(|e| format!("sai chữ ký: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Thư mục tạm riêng cho mỗi test (xoá khi xong).
    struct Tmp(PathBuf);
    impl Tmp {
        fn new(tag: &str) -> Tmp {
            let p = std::env::temp_dir().join(format!("dc-modelpack-{tag}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&p);
            std::fs::create_dir_all(&p).unwrap();
            Tmp(p)
        }
    }
    impl Drop for Tmp {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// Model tối thiểu hợp lệ theo khuôn 16.3.
    fn write_model(dir: &Path) {
        let j = serde_json::json!({
            "Version": 3,
            "FileReferences": {
                "Moc": "m.moc3",
                "Textures": ["m.2048/texture_00.png"],
                "Motions": {
                    "Idle": [{ "File": "motions/idle.motion3.json" }],
                    "TapHead": [{ "File": "motions/nod.motion3.json" }],
                    "TapBody": [{ "File": "motions/happy.motion3.json" }]
                },
                "Expressions": [{ "Name": "Smile", "File": "expressions/smile.exp3.json" }]
            },
            "Groups": [{ "Name": "EyeBlink", "Ids": [] }, { "Name": "LipSync", "Ids": [] }]
        });
        std::fs::write(dir.join("m.model3.json"), j.to_string()).unwrap();
        for f in ["m.moc3", "m.2048/texture_00.png", "motions/idle.motion3.json", "motions/nod.motion3.json", "motions/happy.motion3.json", "expressions/smile.exp3.json"] {
            let p = dir.join(f);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, f.as_bytes()).unwrap();
        }
    }

    #[test]
    fn check_accepts_a_good_model_and_names_what_is_missing() {
        let t = Tmp::new("check");
        write_model(&t.0);
        assert!(check_model_dir(&t.0, "m.model3.json").is_ok());
        assert!(check_model_dir(&t.0, "../m.model3.json").is_err());
        std::fs::remove_file(t.0.join("motions/nod.motion3.json")).unwrap();
        assert_eq!(check_model_dir(&t.0, "m.model3.json"), Err("thiếu motions/nod.motion3.json".into()));
    }

    #[test]
    fn zip_roundtrip_is_deterministic() {
        let src = Tmp::new("zip-src");
        write_model(&src.0);
        // File nguồn không được model3.json trỏ tới thì không vào gói.
        std::fs::write(src.0.join("draft.psd"), b"psd").unwrap();
        let a = zip_model(&src.0, "m.model3.json").unwrap();
        assert_eq!(sha256_hex(&a), sha256_hex(&zip_model(&src.0, "m.model3.json").unwrap()), "cùng nội dung → cùng file");
        let dst = Tmp::new("zip-dst");
        unzip_to(&a, &dst.0).unwrap();
        // File chính luôn mang tên cố định trong gói, tên gốc không còn.
        assert!(check_model_dir(&dst.0, PACK_ENTRY).is_ok());
        assert!(!dst.0.join("m.model3.json").exists());
        assert_eq!(std::fs::read(dst.0.join("m.2048/texture_00.png")).unwrap(), b"m.2048/texture_00.png");
        assert!(!dst.0.join("draft.psd").exists());
    }

    #[test]
    fn unzip_refuses_paths_outside_the_target() {
        for evil in ["../evil.txt", "a/../../evil.txt", "/abs.txt", "C:/abs.txt"] {
            let mut w = zip::ZipWriter::new(Cursor::new(Vec::new()));
            w.start_file(evil, zip::write::SimpleFileOptions::default()).unwrap();
            w.write_all(b"x").unwrap();
            let bytes = w.finish().unwrap().into_inner();
            let dst = Tmp::new("evil");
            assert!(unzip_to(&bytes, &dst.0).is_err(), "{evil}");
        }
    }

    #[test]
    fn manifest_validation() {
        let good = format!(r#"{{"format":1,"models":[{{"id":"a","version":2,"file":"a-2.zip","sha256":"{}","size":10}}]}}"#, "ab".repeat(32));
        let m = parse_manifest(&good).unwrap();
        assert_eq!(m.get("a").map(|e| e.version), Some(2));
        assert!(m.get("b").is_none());
        for bad in [
            good.replace("\"format\":1", "\"format\":2"),
            good.replace("a-2.zip", "../a.zip"),
            good.replace("a-2.zip", "a-2.exe"),
            good.replace(&"ab".repeat(32), "xyz"),
            good.replace("\"size\":10", "\"size\":0"),
        ] {
            assert!(parse_manifest(&bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn signature_roundtrip_in_tauri_format() {
        use base64::Engine;
        // Khoá sinh lúc chạy test, không nằm trong repo.
        let kp = minisign::KeyPair::generate_unencrypted_keypair().unwrap();
        let b64 = |s: String| base64::engine::general_purpose::STANDARD.encode(s);
        let pubkey = b64(kp.pk.to_box().unwrap().to_string());
        let data = br#"{"format":1,"models":[]}"#;
        let sig = minisign::sign(None, &kp.sk, Cursor::new(&data[..]), None, None).unwrap();
        let sig = b64(sig.to_string());
        assert!(verify_signature(data, &sig, &pubkey).is_ok());
        assert!(verify_signature(b"{\"format\":1,\"models\":[1]}", &sig, &pubkey).is_err(), "nội dung bị sửa");
        let other = minisign::KeyPair::generate_unencrypted_keypair().unwrap();
        assert!(verify_signature(data, &sig, &b64(other.pk.to_box().unwrap().to_string())).is_err(), "khoá khác");
        assert!(verify_signature(data, "không-phải-base64", &pubkey).is_err());
    }
}
