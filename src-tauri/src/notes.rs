// Ghi chú thường (bảng 🗒️ Ghi chú, notes.html): thông tin hay dùng hằng ngày,
// không có hạn, không có "xong". Lưu riêng ở notes.json (store.rs), tách khỏi
// tasks.json để lịch ghim / nhắc hạn / chat không đụng tới.

use serde::{Deserialize, Serialize};

/// Tiêu đề dài quá thì cắt (ký tự, không phải byte).
pub const TITLE_MAX: usize = 120;
/// Nội dung đủ cho vài trang ghi chép; dài hơn thì cắt.
pub const BODY_MAX: usize = 20_000;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct Note {
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub body: String,
    /// Ghim lên đầu danh sách.
    #[serde(default)]
    pub pinned: bool,
    /// Millis dạng chuỗi, giống Task.
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
    /// Ghim lên màn hình desktop thành tờ giấy (note_pins.rs). Khác `pinned`
    /// (ghim lên đầu danh sách).
    #[serde(default)]
    pub desk: bool,
    /// Vị trí tờ giấy (physical px). None = chưa kéo lần nào.
    #[serde(default)]
    pub desk_pos: Option<(i32, i32)>,
    /// Tờ giấy luôn nổi trên mọi cửa sổ.
    #[serde(default)]
    pub desk_top: bool,
}

/// Chuẩn hoá tiêu đề + nội dung trước khi lưu: bỏ khoảng trắng hai đầu, cắt độ
/// dài, gộp tiêu đề nhiều dòng thành một dòng. Cả hai cùng rỗng thì từ chối.
pub fn clean(title: &str, body: &str) -> Result<(String, String), String> {
    let title: String = title.split_whitespace().collect::<Vec<_>>().join(" ");
    let title: String = title.chars().take(TITLE_MAX).collect();
    let body: String = body.trim().chars().take(BODY_MAX).collect();
    // Cắt có thể để lại khoảng trắng ở cuối.
    let body = body.trim_end().to_string();
    if title.is_empty() && body.is_empty() {
        return Err("Ghi chú trống: nhập tiêu đề hoặc nội dung.".into());
    }
    Ok((title, body))
}

/// Id theo mili-giây, nhích lên tới khi không trùng (giống tasks_add).
pub fn next_id(notes: &[Note], now: u128) -> String {
    let mut id = now;
    while notes.iter().any(|n| n.id == id.to_string()) {
        id += 1;
    }
    id.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rong_ca_hai_thi_tu_choi() {
        assert!(clean("", "").is_err());
        assert!(clean("  \n ", " \t\n").is_err());
    }

    #[test]
    fn chi_can_mot_trong_hai() {
        assert_eq!(clean("Wifi", "").unwrap(), ("Wifi".into(), "".into()));
        assert_eq!(clean("", " mật khẩu ").unwrap(), ("".into(), "mật khẩu".into()));
    }

    #[test]
    fn tieu_de_gop_thanh_mot_dong() {
        let (t, _) = clean("  Số   điện\nthoại  ", "x").unwrap();
        assert_eq!(t, "Số điện thoại");
    }

    #[test]
    fn noi_dung_giu_xuong_dong_ben_trong() {
        let (_, b) = clean("t", "\n dòng 1\n\n  dòng 2 \n").unwrap();
        assert_eq!(b, "dòng 1\n\n  dòng 2");
    }

    #[test]
    fn cat_theo_ky_tu_khong_vo_tieng_viet() {
        let long = "ệ".repeat(TITLE_MAX + 50);
        let (t, _) = clean(&long, "").unwrap();
        assert_eq!(t.chars().count(), TITLE_MAX);
        let body = "ạ".repeat(BODY_MAX + 10);
        let (_, b) = clean("", &body).unwrap();
        assert_eq!(b.chars().count(), BODY_MAX);
    }

    #[test]
    fn id_khong_trung() {
        let n = |id: &str| Note {
            id: id.into(),
            title: String::new(),
            body: String::new(),
            pinned: false,
            created_at: String::new(),
            updated_at: String::new(),
            ..Default::default()
        };
        let notes = vec![n("100"), n("101")];
        assert_eq!(next_id(&notes, 100), "102");
        assert_eq!(next_id(&notes, 50), "50");
    }

    #[test]
    fn file_cu_thieu_truong_van_doc_duoc() {
        let v: Vec<Note> = serde_json::from_str(r#"[{"id":"1","body":"x"}]"#).unwrap();
        assert_eq!(v[0].title, "");
        assert!(!v[0].pinned);
    }
}
