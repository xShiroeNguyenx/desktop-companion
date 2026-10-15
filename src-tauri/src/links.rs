// Mở link trong nội dung / ghi chú task bằng trình duyệt mặc định. Nội dung
// task có thể đến từ văn bản bôi đen ở app khác hoặc từ chat, nên chỉ mở
// http(s): không file://, không giao thức lạ có thể chạy chương trình.

/// Link dài quá thế này thì không phải link người ta gõ / dán bình thường.
const URL_MAX: usize = 2048;

/// Kiểm tra `raw` là link web an toàn để mở. Trả lại link đã cắt khoảng trắng.
pub fn safe_web_url(raw: &str) -> Result<String, String> {
    let url = raw.trim();
    if url.is_empty() || url.len() > URL_MAX {
        return Err("Link trống hoặc quá dài.".into());
    }
    let lower = url.to_ascii_lowercase();
    let rest = lower
        .strip_prefix("https://")
        .or_else(|| lower.strip_prefix("http://"))
        .ok_or("Chỉ mở được link http:// hoặc https://.")?;
    // Phải có tên máy chủ ngay sau "://".
    // Phần "user:pass@" (nếu có) không tính là tên máy chủ.
    let host_end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let host = rest[..host_end].rsplit('@').next().unwrap_or("");
    if host.is_empty() {
        return Err("Link thiếu tên trang web.".into());
    }
    if url.chars().any(|c| c.is_whitespace() || c.is_control() || c == '"' || c == '<' || c == '>') {
        return Err("Link có ký tự không hợp lệ.".into());
    }
    Ok(url.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_web_links() {
        assert_eq!(safe_web_url(" https://example.com/a?b=1#c ").unwrap(), "https://example.com/a?b=1#c");
        assert!(safe_web_url("http://example.net").is_ok());
        assert!(safe_web_url("HTTPS://Example.com/Path").is_ok());
        for bad in [
            "",
            "example.com",
            "file:///C:/Windows/System32/calc.exe",
            "javascript:alert(1)",
            "ms-settings:privacy",
            "https://",
            "https:///path",
            "https://user@",
            "https://example.com/a b",
            "https://example.com/\"x",
            "https://example.com/\u{7}",
        ] {
            assert!(safe_web_url(bad).is_err(), "{bad}");
        }
        assert!(safe_web_url(&format!("https://example.com/{}", "a".repeat(URL_MAX))).is_err());
    }
}
