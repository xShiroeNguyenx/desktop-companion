// 🔎 Đọc chữ trong ảnh (PLAN.md 20.5) bằng bộ OCR có sẵn của Windows (Windows.Media.Ocr):
// chạy ngay trên máy, không cần mạng, không gửi ảnh đi đâu. Chỉ đọc được những ngôn ngữ
// mà Windows đã cài gói OCR (`languages()`); thiếu thì báo rõ thay vì trả chữ rác.
//
// Dùng cho nút 🌐 / 📄 của lớp phủ chụp màn hình (shot.rs). Chỉ Windows.

// ─── Hàm thuần ───────────────────────────────────────────────────────────────

/// Tiếng Nhật / Trung viết liền: Windows tách mỗi chữ thành một "từ", ghép lại không chèn
/// dấu cách.
pub fn is_cjk(tag: &str) -> bool {
    let t = tag.to_ascii_lowercase();
    t.starts_with("ja") || t.starts_with("zh")
}

/// Ghép các từ của một dòng theo ngôn ngữ của bộ đọc.
pub fn join_line(words: &[String], tag: &str) -> String {
    words.join(if is_cjk(tag) { "" } else { " " })
}

/// Điểm "đọc ra chữ thật" của một kết quả: mỗi chữ / số được 1 điểm, chữ ngoài ASCII (có
/// dấu, kana, kanji) được 2 vì bộ đọc sai ngôn ngữ hiếm khi ra được những chữ đó.
pub fn score(text: &str) -> usize {
    text.chars()
        .filter(|c| c.is_alphanumeric())
        .map(|c| if c.is_ascii() { 1 } else { 2 })
        .sum()
}

/// Chế độ tự động: trong các kết quả `(ngôn ngữ, chữ)` chọn cái điểm cao nhất; bằng điểm
/// thì lấy cái đứng trước. Toàn rỗng thì None.
pub fn pick_best(results: Vec<(String, String)>) -> Option<(String, String)> {
    let mut best: Option<(usize, (String, String))> = None;
    for r in results {
        let s = score(&r.1);
        if s > 0 && best.as_ref().is_none_or(|(b, _)| s > *b) {
            best = Some((s, r));
        }
    }
    best.map(|(_, r)| r)
}

/// Ngôn ngữ người dùng chọn trong Cài đặt, nếu máy còn cài nó; rỗng / đã gỡ = tự động.
pub fn clean_lang(want: &str, available: &[String]) -> Option<String> {
    let want = want.trim();
    available.iter().find(|a| a.eq_ignore_ascii_case(want)).cloned()
}

// ─── Windows.Media.Ocr ───────────────────────────────────────────────────────

/// Các ngôn ngữ OCR đã cài: `(mã, tên hiển thị)`, vd `("en-US", "English (United States)")`.
#[cfg(windows)]
pub fn languages() -> Vec<(String, String)> {
    use windows::Media::Ocr::OcrEngine;
    let Ok(list) = OcrEngine::AvailableRecognizerLanguages() else {
        return Vec::new();
    };
    list.into_iter()
        .filter_map(|l| Some((l.LanguageTag().ok()?.to_string(), l.DisplayName().ok()?.to_string())))
        .collect()
}

#[cfg(not(windows))]
pub fn languages() -> Vec<(String, String)> {
    Vec::new()
}

/// Đọc chữ trong ảnh BGRA `w × h`. `lang` = mã ngôn ngữ muốn dùng, rỗng = thử mọi bộ đọc
/// đã cài rồi chọn kết quả tốt nhất (`pick_best`). Lỗi là mã cho giao diện tự dịch
/// (`shot.err.<mã>`): "noocr" (máy chưa cài bộ đọc nào), "notext" (không thấy chữ),
/// "image" (ảnh quá lớn / sai cỡ), "ocr" (Windows báo lỗi).
#[cfg(windows)]
pub fn recognize(bgra: &[u8], w: u32, h: u32, lang: &str) -> Result<String, &'static str> {
    use windows::core::HSTRING;
    use windows::Globalization::Language;
    use windows::Graphics::Imaging::{BitmapPixelFormat, SoftwareBitmap};
    use windows::Media::Ocr::OcrEngine;
    use windows::Storage::Streams::DataWriter;

    let max = OcrEngine::MaxImageDimension().unwrap_or(10_000);
    if w == 0 || h == 0 || w > max || h > max || bgra.len() != w as usize * h as usize * 4 {
        return Err("image");
    }
    let available: Vec<String> = languages().into_iter().map(|(tag, _)| tag).collect();
    if available.is_empty() {
        return Err("noocr");
    }
    let tags = match clean_lang(lang, &available) {
        Some(one) => vec![one],
        None => available,
    };

    let run = || -> windows::core::Result<Vec<(String, String)>> {
        let writer = DataWriter::new()?;
        writer.WriteBytes(bgra)?;
        let buffer = writer.DetachBuffer()?;
        let bitmap = SoftwareBitmap::CreateCopyFromBuffer(&buffer, BitmapPixelFormat::Bgra8, w as i32, h as i32)?;
        let mut out = Vec::new();
        for tag in tags {
            let engine = OcrEngine::TryCreateFromLanguage(&Language::CreateLanguage(&HSTRING::from(tag.as_str()))?)?;
            let result = engine.RecognizeAsync(&bitmap)?.get()?;
            let mut lines = Vec::new();
            for line in result.Lines()? {
                let words: Vec<String> = line.Words()?.into_iter().filter_map(|x| x.Text().ok()).map(|t| t.to_string()).collect();
                lines.push(join_line(&words, &tag));
            }
            out.push((tag, lines.join("\n")));
        }
        Ok(out)
    };
    let results = run().map_err(|e| {
        eprintln!("[ocr] Windows.Media.Ocr lỗi: {e}");
        "ocr"
    })?;
    pick_best(results).map(|(_, text)| text).ok_or("notext")
}

#[cfg(not(windows))]
pub fn recognize(_bgra: &[u8], _w: u32, _h: u32, _lang: &str) -> Result<String, &'static str> {
    Err("noocr")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn cjk_lines_are_joined_without_spaces() {
        assert_eq!(join_line(&s(&["Hello", "world"]), "en-US"), "Hello world");
        assert_eq!(join_line(&s(&["こ", "ん", "に", "ち", "は"]), "ja"), "こんにちは");
        assert_eq!(join_line(&s(&["你", "好"]), "zh-Hans-CN"), "你好");
        // Tiếng Hàn có dấu cách giữa các từ.
        assert_eq!(join_line(&s(&["안녕", "하세요"]), "ko"), "안녕 하세요");
        assert_eq!(join_line(&[], "en-US"), "");
    }

    #[test]
    fn auto_mode_prefers_the_reader_that_found_real_text() {
        let r = |a: &str, b: &str| (a.to_string(), b.to_string());
        // Ảnh chữ Nhật: bộ đọc tiếng Anh ra vài ký tự rác, bộ tiếng Nhật ra chữ thật.
        let best = pick_best(vec![r("en-US", "L l ; -"), r("ja", "今日は良い天気です")]).unwrap();
        assert_eq!(best.0, "ja");
        // Ảnh chữ Anh: hai bộ đọc ra như nhau → giữ cái đứng trước.
        let best = pick_best(vec![r("en-US", "Hello world"), r("ja", "Hello world")]).unwrap();
        assert_eq!(best.0, "en-US");
        // Chỉ có dấu câu / khoảng trắng = không có chữ.
        assert_eq!(pick_best(vec![r("en-US", " .. - "), r("ja", "")]), None);
        assert_eq!(pick_best(Vec::new()), None);
    }

    #[test]
    fn score_counts_letters_and_digits_only() {
        assert_eq!(score("ab 12!"), 4);
        assert_eq!(score("việt"), 5); // "ệ" ngoài ASCII được 2
        assert_eq!(score("  ... "), 0);
    }

    #[test]
    fn chosen_language_must_still_be_installed() {
        let have = s(&["en-US", "ja"]);
        assert_eq!(clean_lang("ja", &have), Some("ja".to_string()));
        assert_eq!(clean_lang(" EN-us ", &have), Some("en-US".to_string()));
        assert_eq!(clean_lang("", &have), None);
        assert_eq!(clean_lang("fr-FR", &have), None);
    }

    /// Gọi OCR thật của Windows trên một ảnh trắng trơn: phải chạy được tới cùng và báo
    /// "không thấy chữ" (hoặc "noocr" nếu máy chưa cài bộ đọc nào), không được lỗi WinRT.
    /// `cargo test ocr -- --ignored`.
    #[cfg(windows)]
    #[test]
    #[ignore]
    fn live_blank_image_has_no_text() {
        let (w, h) = (320u32, 120u32);
        let white = vec![255u8; (w * h * 4) as usize];
        let got = recognize(&white, w, h, "");
        assert!(matches!(got, Err("notext") | Err("noocr")), "{got:?}");
        assert_eq!(recognize(&white[..16], w, h, ""), Err("image"));
    }
}
