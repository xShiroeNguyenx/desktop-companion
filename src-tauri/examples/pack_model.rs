//! Đóng gói một model Live2D tải riêng để đăng lên release `models` (RELEASE.md).
//!
//!   cd src-tauri
//!   cargo run --example pack_model -- <thư-mục-model> <id> <phiên-bản>
//!
//! Kiểm model theo khuôn 16.3 (cùng code app dùng khi cài), tạo
//! `dist-models/<id>-<phiên-bản>.zip` và ghi dòng của model vào `models/manifest.json`.
//! Sau đó ký manifest và tải lên release (lệnh in ra ở cuối).

#[path = "../src/model_pack.rs"]
#[allow(dead_code)]
mod model_pack;

use std::path::{Path, PathBuf};

fn main() {
    if let Err(e) = run() {
        eprintln!("❌ {e}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [dir, id, version] = args.as_slice() else {
        return Err("cách dùng: cargo run --example pack_model -- <thư-mục-model> <id> <phiên-bản>".into());
    };
    let dir = PathBuf::from(dir);
    let version: u32 = version.parse().map_err(|_| format!("phiên bản phải là số nguyên dương: {version}"))?;
    if version == 0 {
        return Err("phiên bản bắt đầu từ 1".into());
    }
    if id.is_empty() || !id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
        return Err(format!("id chỉ gồm chữ thường, số, gạch nối: {id}"));
    }

    let entry = find_entry(&dir)?;
    let size = model_pack::check_model_dir(&dir, &entry)?;
    println!("✔ {} hợp lệ: {entry}, {size} byte", dir.display());

    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let out_dir = repo.join("dist-models");
    std::fs::create_dir_all(&out_dir).map_err(|e| e.to_string())?;
    let file = format!("{id}-{version}.zip");
    let bytes = model_pack::zip_model(&dir, &entry)?;
    let zip_path = out_dir.join(&file);
    std::fs::write(&zip_path, &bytes).map_err(|e| e.to_string())?;
    let sha256 = model_pack::sha256_hex(&bytes);
    println!("✔ {} ({} byte)", zip_path.display(), bytes.len());

    let man_path = repo.join("models").join("manifest.json");
    let mut man = match std::fs::read_to_string(&man_path) {
        Ok(text) => model_pack::parse_manifest(&text)?,
        Err(_) => model_pack::Manifest { format: model_pack::MANIFEST_FORMAT, models: Vec::new() },
    };
    if let Some(old) = man.get(id) {
        if old.version >= version {
            return Err(format!("{id} đã có bản {} trong manifest; phiên bản mới phải lớn hơn", old.version));
        }
    }
    man.models.retain(|m| m.id != *id);
    man.models.push(model_pack::PackEntry { id: id.clone(), version, file: file.clone(), sha256, size: bytes.len() as u64 });
    man.models.sort_by(|a, b| a.id.cmp(&b.id));
    let text = serde_json::to_string_pretty(&man).map_err(|e| e.to_string())? + "\n";
    // Kiểm lại đúng như app sẽ đọc.
    model_pack::parse_manifest(&text)?;
    std::fs::create_dir_all(man_path.parent().unwrap()).map_err(|e| e.to_string())?;
    std::fs::write(&man_path, text).map_err(|e| e.to_string())?;
    println!("✔ {} → {id} v{version}", man_path.display());

    println!();
    println!("  (file chính {entry} nằm trong gói với tên {})", model_pack::PACK_ENTRY);
    println!("Tiếp theo (ở thư mục gốc repo, xem RELEASE.md):");
    println!("  npx tauri signer sign -f <đường-dẫn-khoá-riêng> models/manifest.json");
    println!("  gh release upload models dist-models/{file} models/manifest.json models/manifest.json.sig --clobber");
    Ok(())
}

/// Đúng một `*.model3.json` ở gốc thư mục model.
fn find_entry(dir: &Path) -> Result<String, String> {
    let mut found: Vec<String> = std::fs::read_dir(dir)
        .map_err(|e| format!("{}: {e}", dir.display()))?
        .filter_map(|e| e.ok())
        .filter_map(|e| e.file_name().to_str().map(String::from))
        .filter(|n| n.to_ascii_lowercase().ends_with(".model3.json"))
        .collect();
    match found.len() {
        1 => Ok(found.remove(0)),
        0 => Err(format!("không thấy *.model3.json ở {}", dir.display())),
        _ => Err(format!("có nhiều *.model3.json ở {}: {found:?}", dir.display())),
    }
}
