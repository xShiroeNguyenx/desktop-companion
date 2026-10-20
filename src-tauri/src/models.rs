//! 🎭 Model Live2D có sẵn trong app (PLAN.md mục 16).
//!
//! Một chỗ khai báo cho cả Rust lẫn cửa sổ pet (`window.__BUILTIN_MODELS__`,
//! pet-config.js đọc). Hai loại:
//! - **Đóng gói** (`item = None`): miễn phí, file nằm trong `app/media/live2d/<dir>/`.
//! - **Tải riêng** (`item = Some(món)`): món Bộ sưu tập (collection.rs, nhóm Model),
//!   KHÔNG nằm trong bộ cài. Mở khoá rồi mới tải từ release `models` (model_dl.rs);
//!   dùng được khi đã sở hữu **và** đã cài.
//!
//! Model người dùng tự thêm từ thư mục (`store::CustomModel`) luôn dùng được và
//! nằm ngoài catalog.
//!
//! Thêm model tải riêng: thêm một dòng `BUILTIN`, món `model.<id>` trong
//! `collection::CATALOG`, tỉ lệ trong `gacha::LEGENDARY_PCT`, ảnh xem trước trong
//! `app/media/model-previews/`, rồi đóng gói và đăng lên release (RELEASE.md).

use serde_json::{json, Value};

use crate::collection::{self, Collection};
use crate::store::CustomModel;

pub struct Builtin {
    pub id: &'static str,
    /// Đóng gói: thư mục trong `app/media/live2d/`. Tải riêng: chỉ để ghi chú
    /// (thư mục nguồn khi đóng gói), file cài vào `<app_data>/models/<id>/`.
    pub dir: &'static str,
    /// Tên file `*.model3.json` trong thư mục model. Tải riêng: luôn
    /// `model_pack::PACK_ENTRY` (gói đổi tên file chính thành tên cố định).
    pub entry: &'static str,
    pub name: &'static str,
    /// Dòng mô tả trong menu Diện mạo (menu chưa dịch, như các mục khác).
    pub desc: &'static str,
    /// Món Bộ sưu tập mở khoá model này; `None` = miễn phí, đóng gói sẵn.
    pub item: Option<&'static str>,
    /// Ảnh xem trước trong Bộ sưu tập, so với thư mục `app/` (PNG nền trong suốt,
    /// cao 512 px). Luôn đóng gói, để thẻ có hình trước khi tải model.
    pub preview: Option<&'static str>,
}

impl Builtin {
    /// Model tải riêng (không nằm trong bộ cài).
    pub fn downloadable(&self) -> bool {
        self.item.is_some()
    }
}

pub const DEFAULT_ID: &str = "hiyori";

pub const BUILTIN: &[Builtin] = &[
    Builtin { id: "hiyori", dir: "Hiyori", entry: "Hiyori.model3.json", name: "Hiyori", desc: "Cô gái học sinh dễ thương", item: None, preview: None },
    Builtin { id: "haru", dir: "Haru", entry: "haru_greeter_t05.model3.json", name: "Haru", desc: "Cô gái chào đón", item: None, preview: None },
    Builtin { id: "mao", dir: "Mao", entry: "mao_pro.model3.json", name: "Mao", desc: "Cô gái tai mèo", item: None, preview: None },
    Builtin { id: "miara", dir: "Miara", entry: "miara_pro_t03.model3.json", name: "Miara", desc: "Phép thuật thiếu nữ", item: None, preview: None },
    Builtin {
        id: "gothic-aristocrat",
        dir: "Gothic_Aristocrat",
        entry: crate::model_pack::PACK_ENTRY,
        name: "Gothic Aristocrat",
        desc: "Tiểu thư gothic quý tộc 🌈",
        item: Some("model.gothic-aristocrat"),
        preview: Some("media/model-previews/gothic-aristocrat.png"),
    },
];

pub fn builtin(id: &str) -> Option<&'static Builtin> {
    BUILTIN.iter().find(|m| m.id == id)
}

/// Đường dẫn ảnh xem trước (so với thư mục app), rỗng nếu model chưa có ảnh.
pub fn preview_url(id: &str) -> String {
    builtin(id).and_then(|m| m.preview).unwrap_or_default().to_string()
}

/// Model tải riêng này đã sở hữu chưa.
pub fn owned(m: &Builtin, c: &Collection) -> bool {
    match m.item {
        None => true,
        Some(item) => collection::item(item).is_some_and(|it| collection::owns(c, it)),
    }
}

/// Model này có dùng được không: miễn phí; tải riêng thì đã sở hữu và đã cài
/// (`installed`: id các model tải riêng đã cài, model_dl::installed_ids); hoặc
/// model tự thêm.
pub fn usable(id: &str, c: &Collection, custom: &[CustomModel], installed: &[String]) -> bool {
    match builtin(id) {
        Some(m) => owned(m, c) && (!m.downloadable() || installed.iter().any(|i| i == id)),
        None => custom.iter().any(|m| m.id == id),
    }
}

/// Model thật sự dùng cho id đã lưu: id lạ, model đã gỡ, model chưa mở khoá
/// (sửa tay settings.json…) hay chưa tải xong → model mặc định.
pub fn effective(id: &str, c: &Collection, custom: &[CustomModel], installed: &[String]) -> String {
    if usable(id, c, custom, installed) {
        id.to_string()
    } else {
        DEFAULT_ID.to_string()
    }
}

/// Id model chưa dùng được: chưa mở khoá hoặc chưa tải (menu Diện mạo hiện 🔒).
pub fn locked(c: &Collection, installed: &[String]) -> Vec<&'static str> {
    BUILTIN.iter().filter(|m| !usable(m.id, c, &[], installed)).map(|m| m.id).collect()
}

/// Id cho model tự thêm tiếp theo: lớn hơn mọi số đang có. Đếm số phần tử thì
/// sau khi xoá một model, model mới trùng id với model còn lại.
pub fn next_custom_id(existing: &[CustomModel]) -> String {
    let max = existing
        .iter()
        .filter_map(|m| m.id.strip_prefix("custom-")?.parse::<u32>().ok())
        .max()
        .unwrap_or(0);
    format!("custom-{}", max + 1)
}

/// Địa chỉ model3.json cho cửa sổ pet: đóng gói đọc từ `/media`, tải riêng đọc
/// qua giao thức `petmodel` (main.rs tìm thư mục đã cài).
pub fn url(m: &Builtin) -> String {
    if m.downloadable() {
        format!("http://petmodel.localhost/{}/{}", m.id, m.entry)
    } else {
        format!("/media/live2d/{}/{}", m.dir, m.entry)
    }
}

/// Danh sách tiêm vào cửa sổ pet (`window.__BUILTIN_MODELS__`).
pub fn builtin_json() -> Value {
    json!(BUILTIN
        .iter()
        .map(|m| json!({
            "id": m.id,
            "url": url(m),
            "name": m.name,
            "description": m.desc,
            "item": m.item,
        }))
        .collect::<Vec<_>>())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collection::Category;

    fn custom(id: &str) -> CustomModel {
        CustomModel { id: id.into(), name: id.into(), dir: String::new(), entry: String::new() }
    }

    fn app_dir() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../app")
    }

    #[test]
    fn ids_are_unique_and_default_is_free() {
        let mut ids: Vec<&str> = BUILTIN.iter().map(|m| m.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), BUILTIN.len());
        assert!(builtin(DEFAULT_ID).is_some_and(|m| m.item.is_none()));
        // Id đi vào URL petmodel và tên thư mục cài: chỉ chữ thường, số, gạch nối.
        for m in BUILTIN {
            assert!(!m.id.starts_with("custom-"), "{}: trùng tiền tố model tự thêm", m.id);
            assert!(m.id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'), "{}", m.id);
        }
    }

    #[test]
    fn locked_models_match_the_catalog() {
        // Mỗi model có món mở khoá ↔ đúng một món nhóm Model trỏ về nó.
        for m in BUILTIN {
            if let Some(id) = m.item {
                let it = collection::item(id).unwrap_or_else(|| panic!("{}: thiếu món {id}", m.id));
                assert_eq!((it.cat, it.asset), (Category::Model, m.id), "{id}");
            }
        }
        for it in collection::items().filter(|i| i.cat == Category::Model) {
            assert!(builtin(it.asset).is_some_and(|m| m.item == Some(it.id)), "{}: không có model", it.id);
        }
    }

    #[test]
    fn effective_needs_owned_and_installed() {
        let mut c = Collection::default();
        let mine = [custom("custom-2")];
        let none: Vec<String> = Vec::new();
        let got = vec!["gothic-aristocrat".to_string()];
        assert_eq!(effective("mao", &c, &[], &none), "mao");
        assert_eq!(effective("custom-2", &c, &mine, &none), "custom-2");
        assert_eq!(effective("custom-9", &c, &mine, &none), DEFAULT_ID);
        assert_eq!(effective("nope", &c, &[], &none), DEFAULT_ID);
        assert_eq!(effective("gothic-aristocrat", &c, &[], &got), DEFAULT_ID, "đã tải (máy khác chép sang) nhưng chưa mua");
        assert_eq!(locked(&c, &none), vec!["gothic-aristocrat"]);
        c.owned.push("model.gothic-aristocrat".into());
        assert_eq!(effective("gothic-aristocrat", &c, &[], &none), DEFAULT_ID, "đã mua nhưng chưa tải xong");
        assert_eq!(locked(&c, &none), vec!["gothic-aristocrat"]);
        assert_eq!(effective("gothic-aristocrat", &c, &[], &got), "gothic-aristocrat");
        assert!(locked(&c, &got).is_empty());
    }

    #[test]
    fn urls_point_at_the_bundle_or_the_download_protocol() {
        assert_eq!(url(builtin("hiyori").unwrap()), "/media/live2d/Hiyori/Hiyori.model3.json");
        assert_eq!(url(builtin("gothic-aristocrat").unwrap()), "http://petmodel.localhost/gothic-aristocrat/model.model3.json");
    }

    #[test]
    fn custom_ids_never_collide_after_removal() {
        assert_eq!(next_custom_id(&[]), "custom-1");
        // Có custom-1, custom-2; xoá custom-1 → model mới không được là custom-2.
        assert_eq!(next_custom_id(&[custom("custom-2")]), "custom-3");
        assert_eq!(next_custom_id(&[custom("custom-1"), custom("weird")]), "custom-2");
    }

    #[test]
    fn bundled_models_are_complete_and_downloadables_are_not_shipped() {
        let live2d = app_dir().join("media/live2d");
        for m in BUILTIN {
            if m.downloadable() {
                assert_eq!(m.entry, crate::model_pack::PACK_ENTRY, "{}: model tải riêng mở file chính theo tên cố định trong gói", m.id);
                // Model tải riêng không được lọt vào bộ cài (frontendDist = app/).
                assert!(!live2d.join(m.dir).exists(), "{}: đang nằm trong app/media/live2d, sẽ bị đóng gói", m.id);
                let p = m.preview.unwrap_or_else(|| panic!("{}: model tải riêng cần ảnh xem trước", m.id));
                let size = std::fs::metadata(app_dir().join(p)).map(|md| md.len()).unwrap_or_else(|e| panic!("{}: thiếu {p}: {e}", m.id));
                assert!(size <= 650 * 1024, "{}: {p} {size} byte, quá 650 KB", m.id);
                continue;
            }
            // Model mẫu (không theo khuôn 16.3) chỉ cần đủ file.
            let dir = live2d.join(m.dir);
            let text = std::fs::read_to_string(dir.join(m.entry)).unwrap_or_else(|e| panic!("{}: {e}", m.id));
            let j: Value = serde_json::from_str(&text).unwrap();
            for f in crate::model_pack::referenced_files(&j) {
                assert!(dir.join(&f).is_file(), "{}: thiếu {f}", m.id);
            }
        }
    }
}
