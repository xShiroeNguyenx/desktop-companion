// 🔖 Bookmark người dùng tự thêm (bookmarks.json, PLAN.md 17.3): quản lý ở tab
// 🔖 của bảng Ghi chú, tìm và mở bằng ô mở nhanh (launcher.rs). Phần thuần.

use serde::{Deserialize, Serialize};

pub const TITLE_MAX: usize = 80;
pub const GROUP_MAX: usize = 30;
pub const MAX_BOOKMARKS: usize = 500;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct Bookmark {
    pub id: String,
    pub title: String,
    pub url: String,
    /// Nhóm tuỳ chọn ("Công việc", "Giải trí"…). Rỗng = không nhóm.
    #[serde(default)]
    pub group: String,
}

fn one_line(s: &str, max: usize) -> String {
    let s: String = s.split_whitespace().collect::<Vec<_>>().join(" ");
    s.chars().take(max).collect()
}

/// Kiểm và chuẩn hoá: URL phải http / https; tên trống thì lấy tên máy trong URL.
pub fn clean(title: &str, url: &str, group: &str) -> Result<(String, String, String), String> {
    let url = crate::links::safe_web_url(url)?;
    let title = one_line(title, TITLE_MAX);
    let title = if title.is_empty() { crate::monitor::host_of(&url) } else { title };
    Ok((title, url, one_line(group, GROUP_MAX)))
}

pub fn next_id(list: &[Bookmark], now: u128) -> String {
    let mut id = now;
    while list.iter().any(|b| b.id == id.to_string()) {
        id += 1;
    }
    id.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleans_and_rejects_bad_urls() {
        let (t, u, g) = clean("  Tài liệu   API ", " https://example.com/docs ", "  Công   việc ").unwrap();
        assert_eq!((t.as_str(), u.as_str(), g.as_str()), ("Tài liệu API", "https://example.com/docs", "Công việc"));
        assert_eq!(clean("", "https://app-01.example.net/x", "").unwrap().0, "app-01.example.net");
        assert!(clean("x", "javascript:alert(1)", "").is_err());
        assert!(clean("x", "file:///C:/Windows", "").is_err());
        assert!(clean("x", "", "").is_err());
        assert_eq!(clean(&"a".repeat(200), "https://example.com", "").unwrap().0.chars().count(), TITLE_MAX);
    }

    #[test]
    fn ids_are_unique() {
        let b = |id: &str| Bookmark { id: id.into(), ..Default::default() };
        assert_eq!(next_id(&[b("5"), b("6")], 5), "7");
    }
}
