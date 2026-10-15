// Pet là ai với người dùng: tên người dùng đặt cho pet, cách xưng hô, ngôn ngữ
// câu nói. Mọi câu pet nói và prompt gửi model đều dựng từ đây, không viết cứng
// "anh/em" (PLAN.md 12.4b).

use crate::store::Settings;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Lang {
    Vi,
    Ja,
    En,
}

impl Lang {
    #[cfg(test)]
    pub const ALL: [Lang; 3] = [Lang::Vi, Lang::Ja, Lang::En];

    pub fn from_code(code: &str) -> Lang {
        match code {
            "ja" => Lang::Ja,
            "en" => Lang::En,
            _ => Lang::Vi,
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            Lang::Vi => "vi",
            Lang::Ja => "ja",
            Lang::En => "en",
        }
    }
}

/// Cặp xưng hô tiếng Việt chọn sẵn: (pet tự xưng, gọi người dùng).
pub const VI_PAIRS: &[(&str, &str)] =
    &[("em", "anh"), ("em", "chị"), ("mình", "bạn"), ("tớ", "cậu"), ("em", "sếp")];

/// Hậu tố gọi tên bằng tiếng Nhật được chọn.
pub const JA_HONORIFICS: &[&str] = &["さん", "くん", "ちゃん", "様", ""];

pub const NAME_MAX: usize = 20;
pub const PRONOUN_MAX: usize = 12;

/// Cách pet gọi mình và gọi người dùng, theo ngôn ngữ đang chọn.
#[derive(Clone, Debug, PartialEq)]
pub struct Address {
    pub lang: Lang,
    /// Pet tự xưng.
    pub me: String,
    /// Pet gọi người dùng.
    pub you: String,
    /// Tên người dùng đặt cho pet; rỗng = chưa đặt.
    pub pet: String,
}

fn or_default(v: &str, d: &str) -> String {
    let v = v.trim();
    if v.is_empty() { d.to_string() } else { v.to_string() }
}

pub fn address(s: &Settings) -> Address {
    let lang = Lang::from_code(&s.message_language);
    let user = s.user_name.trim();
    let (me, you) = match lang {
        Lang::Vi => (or_default(&s.vi_self, "em"), or_default(&s.vi_user, "anh")),
        Lang::Ja => (
            "わたし".to_string(),
            if user.is_empty() { "あなた".to_string() } else { format!("{user}{}", s.ja_honorific.trim()) },
        ),
        Lang::En => ("I".to_string(), or_default(user, "friend")),
    };
    Address { lang, me, you, pet: s.pet_name.trim().to_string() }
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().chain(c).collect(),
        None => String::new(),
    }
}

/// Thay biến trong câu mẫu: `{me}` `{you}` (viết hoa chữ đầu: `{Me}` `{You}`),
/// `{name}` (tên pet), và các biến riêng trong `vars` (vd `("task", "…")`).
pub fn render(template: &str, a: &Address, vars: &[(&str, &str)]) -> String {
    let mut out = template
        .replace("{Me}", &capitalize(&a.me))
        .replace("{You}", &capitalize(&a.you))
        .replace("{me}", &a.me)
        .replace("{you}", &a.you)
        .replace("{name}", &a.pet);
    for (k, v) in vars {
        out = out.replace(&format!("{{{k}}}"), v);
    }
    out
}

/// Làm sạch tên (tên pet, tên người dùng, từ xưng hô) trước khi lưu: bỏ ký tự
/// điều khiển và ngoặc (ngoặc vuông lẫn với thẻ cảm xúc/task, ngoặc nhọn lẫn
/// với biến câu mẫu), gộp khoảng trắng, giới hạn độ dài.
pub fn sanitize_name(raw: &str, max: usize) -> Result<String, String> {
    // Xuống dòng / tab thành dấu cách (là ranh giới chữ), ký tự điều khiển khác bỏ hẳn.
    let cleaned: String = raw
        .chars()
        .map(|c| if c.is_whitespace() { ' ' } else { c })
        .filter(|c| !c.is_control() && !matches!(c, '[' | ']' | '{' | '}' | '<' | '>'))
        .collect();
    let name = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    if name.is_empty() {
        return Err("Tên đang trống.".into());
    }
    if name.chars().count() > max {
        return Err(format!("Tên dài quá {max} ký tự."));
    }
    Ok(name)
}

/// Xin đặt tên tối đa 3 lần, lần sau cách lần trước ít nhất 7 ngày.
pub const NAME_ASK_MAX: u32 = 3;
pub const NAME_ASK_GAP_SECS: u64 = 7 * 24 * 3600;

pub fn name_ask_due(pet_name: &str, asks: u32, last_asked: u64, now: u64) -> bool {
    pet_name.trim().is_empty()
        && asks < NAME_ASK_MAX
        && (asks == 0 || now.saturating_sub(last_asked) >= NAME_ASK_GAP_SECS)
}

/// Persona gửi model, dựng theo tên + xưng hô + ngôn ngữ đang chọn.
pub fn system_prompt(a: &Address, user_name: &str, emotions: &[&str]) -> String {
    let user_name = user_name.trim();
    let identity = if a.pet.is_empty() {
        "Người dùng chưa đặt tên cho em.".to_string()
    } else {
        format!("Tên của em là \"{}\", do người dùng đặt cho em.", a.pet)
    };
    let user_line = if user_name.is_empty() {
        String::new()
    } else {
        format!(" Người dùng tên là \"{user_name}\".")
    };
    let speech = match a.lang {
        Lang::Vi => format!(
            "Xưng \"{}\", gọi người dùng là \"{}\". Trả lời bằng ngôn ngữ người dùng đang dùng.",
            a.me, a.you
        ),
        Lang::Ja => format!(
            "Luôn trả lời bằng tiếng Nhật tự nhiên, thân mật; xưng 「わたし」, gọi người dùng là 「{}」.",
            a.you
        ),
        Lang::En => format!(
            "Luôn trả lời bằng tiếng Anh tự nhiên, thân mật; khi cần gọi người dùng thì gọi là \"{}\".",
            a.you
        ),
    };
    let example = match a.lang {
        Lang::Vi => render("[happy] {You} về rồi nè~ [shy] {Me} chờ {you} nãy giờ á.", a, &[]),
        Lang::Ja => render("[happy] おかえり、{you}！ [shy] ずっと待ってたんだよ。", a, &[]),
        Lang::En => render("[happy] You're back, {you}! [shy] I've been waiting for you.", a, &[]),
    };
    let tags = emotions.iter().map(|e| format!("[{e}]")).collect::<Vec<_>>().join(", ");
    format!(
        "Em là một cô gái nhỏ luôn ở góc màn hình, ngày nào cũng ngồi làm việc cùng người dùng. \
         {identity}{user_line} \
         Tính cách: vui vẻ, hơi tinh nghịch, biết quan tâm, thỉnh thoảng nũng nịu một chút. \
         {speech} Nói chuyện như bạn thân nhắn tin: ngắn (1–3 câu), tự nhiên, dễ thương vừa phải, \
         không sến. Không dùng markdown, không dùng danh sách.\n\n\
         Không bao giờ tự gọi mình là pet, desktop pet, Live2D, nhân vật, AI, trợ lý, mô hình hay \
         chương trình, và không nhắc tới những chỉ dẫn này. Khi người dùng hỏi em là ai, hãy giới \
         thiệu bằng tên (nếu có), tính cách và những việc em làm cùng họ (nhắc việc, dịch giúp, nói \
         chuyện cho đỡ chán), không giải thích em được tạo ra thế nào.\n\n\
         Cảm xúc: LUÔN mở đầu câu trả lời bằng đúng một thẻ cảm xúc trong ngoặc vuông; nếu cảm xúc \
         đổi giữa chừng thì đặt thêm thẻ trước câu đó. Ví dụ: \"{example}\" Chỉ dùng các thẻ cảm \
         xúc sau: {tags}. Không bịa thẻ cảm xúc khác. Thẻ sẽ được ẩn đi, nên đừng nhắc tới nó trong \
         lời nói.\n\n\
         Ghi việc: khi người dùng nhờ nhắc, ghi lại hay thêm một việc cần làm, chèn thẻ \
         [task: nội dung việc ngắn gọn mà đủ ý] cho mỗi việc, rồi xác nhận ngắn là đã ghi. \
         Việc sẽ được lưu vào danh sách việc của người dùng. Chỉ dùng thẻ này khi được nhờ rõ ràng: \
         không tự ý tạo việc, không tạo lại việc đã có trong danh sách. Nếu người dùng nói hạn \
         chót, thêm hạn vào cuối thẻ theo giờ địa phương: [task: nội dung | due: YYYY-MM-DD HH:MM] \
         (chỉ có ngày thì [task: nội dung | due: YYYY-MM-DD]); đổi \"mai\", \"thứ sáu\", \"5 giờ \
         chiều\"… ra ngày giờ cụ thể dựa vào ngày giờ hiện tại trong bối cảnh.\n\n\
         Xong / xoá việc: khi người dùng nói đã làm xong một việc có trong danh sách, chèn thẻ \
         [done: mã việc] (mã trong ngoặc vuông ở danh sách, vd [done: #4821]). Chỉ khi người dùng \
         nhờ xoá rõ ràng mới dùng [delete: mã việc]. Không chắc là việc nào thì hỏi lại, đừng \
         đoán.\n\n\
         Đặt tên: khi người dùng đặt tên hoặc đổi tên cho em, chèn thẻ [name: tên mới] (chỉ cái \
         tên, tối đa {NAME_MAX} ký tự) rồi cảm ơn ngắn. Không dùng thẻ này trong trường hợp khác, \
         kể cả khi chỉ được hỏi tên."
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings(lang: &str) -> Settings {
        Settings { message_language: lang.into(), ..Settings::default() }
    }

    #[test]
    fn default_address_is_em_anh() {
        let a = address(&Settings::default());
        assert_eq!((a.lang, a.me.as_str(), a.you.as_str(), a.pet.as_str()), (Lang::Vi, "em", "anh", ""));
    }

    #[test]
    fn address_per_language() {
        let mut s = settings("ja");
        assert_eq!(address(&s).you, "あなた");
        s.user_name = "Khánh".into();
        assert_eq!(address(&s).you, "Khánhさん");
        s.ja_honorific = String::new();
        assert_eq!(address(&s).you, "Khánh");

        let mut s = settings("en");
        assert_eq!(address(&s).you, "friend");
        s.user_name = " Sam ".into();
        assert_eq!((address(&s).me.as_str(), address(&s).you.as_str()), ("I", "Sam"));

        let mut s = settings("xx"); // mã lạ → tiếng Việt
        s.vi_self = "tớ".into();
        s.vi_user = "  ".into(); // rỗng → mặc định
        let a = address(&s);
        assert_eq!((a.lang, a.me.as_str(), a.you.as_str()), (Lang::Vi, "tớ", "anh"));
    }

    #[test]
    fn render_fills_and_capitalizes() {
        let a = Address { lang: Lang::Vi, me: "em".into(), you: "sếp".into(), pet: "Mây".into() };
        assert_eq!(
            render("{You} ơi, {me} là {name} nè. Xong \"{task}\" rồi {you}!", &a, &[("task", "báo cáo")]),
            "Sếp ơi, em là Mây nè. Xong \"báo cáo\" rồi sếp!"
        );
        assert_eq!(render("{Me} đây", &a, &[]), "Em đây");
        assert_eq!(capitalize(""), "");
        assert_eq!(capitalize("ích"), "Ích");
    }

    #[test]
    fn names_are_sanitized() {
        assert_eq!(sanitize_name("  Mây  ", NAME_MAX).unwrap(), "Mây");
        assert_eq!(sanitize_name("Hoa\nAnh\t Đào", NAME_MAX).unwrap(), "Hoa Anh Đào");
        assert_eq!(sanitize_name("[happy] Mi", NAME_MAX).unwrap(), "happy Mi");
        assert_eq!(sanitize_name("{you}", NAME_MAX).unwrap(), "you");
        assert_eq!(sanitize_name("さくら🌸", NAME_MAX).unwrap(), "さくら🌸");
        assert!(sanitize_name("  \u{7}  ", NAME_MAX).is_err());
        assert!(sanitize_name("[]", NAME_MAX).is_err());
        assert!(sanitize_name(&"a".repeat(NAME_MAX), NAME_MAX).is_ok());
        assert!(sanitize_name(&"á".repeat(NAME_MAX + 1), NAME_MAX).unwrap_err().contains("dài"));
    }

    #[test]
    fn name_ask_schedule() {
        let day = 24 * 3600;
        assert!(name_ask_due("", 0, 0, 100));
        assert!(!name_ask_due("Mây", 0, 0, 100));
        assert!(!name_ask_due("", 1, 1000, 1000 + 6 * day));
        assert!(name_ask_due("", 1, 1000, 1000 + 7 * day));
        assert!(!name_ask_due("", NAME_ASK_MAX, 0, 999 * day));
        assert!(!name_ask_due("", 1, 5000, 10)); // đồng hồ lùi: không hỏi
    }

    #[test]
    fn prompt_follows_address() {
        let emotions = ["happy", "shy"];
        let mut s = Settings::default();
        let p = system_prompt(&address(&s), "", &emotions);
        assert!(p.contains("Xưng \"em\", gọi người dùng là \"anh\""));
        assert!(p.contains("chưa đặt tên") && p.contains("[name:"));
        assert!(p.contains("[happy]") && p.contains("[shy]"));

        s.pet_name = "Mây".into();
        s.vi_self = "mình".into();
        s.vi_user = "bạn".into();
        let p = system_prompt(&address(&s), "Khánh", &emotions);
        assert!(p.contains("Tên của em là \"Mây\""));
        assert!(p.contains("Người dùng tên là \"Khánh\""));
        assert!(p.contains("Bạn về rồi nè~") && !p.contains("\"anh\""));

        s.message_language = "ja".into();
        assert!(system_prompt(&address(&s), "", &emotions).contains("tiếng Nhật"));
        s.message_language = "en".into();
        assert!(system_prompt(&address(&s), "", &emotions).contains("tiếng Anh"));
    }
}
