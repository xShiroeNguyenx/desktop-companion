// Chỉ tới một task từ chat: mã ngắn "#4821" (đuôi id) hiện trong ngữ cảnh gửi
// model, và khớp thẻ `[done: …]` / `[delete: …]` về đúng một task. Không chắc
// chắn thì KHÔNG làm gì — thà hỏi lại còn hơn đánh dấu/xoá nhầm việc.

/// Mã ngắn cho từng id: đuôi ngắn nhất (≥ 4 chữ số) không trùng với id nào khác.
pub fn short_ids(ids: &[&str]) -> Vec<String> {
    ids.iter()
        .map(|id| {
            let mut n = 4.min(id.len());
            while n < id.len()
                && ids.iter().filter(|o| o.ends_with(&id[id.len() - n..])).count() > 1
            {
                n += 1;
            }
            format!("#{}", &id[id.len() - n..])
        })
        .collect()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Match {
    One(usize),
    None,
    /// Nhiều task khớp: chỉ số các task đó.
    Many(Vec<usize>),
}

fn norm(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase()
}

/// Tìm task (id, nội dung) theo `query`: "#4821" / "4821" khớp đuôi id; còn lại
/// khớp nội dung (trùng hẳn trước, rồi tới "có chứa").
pub fn match_task(tasks: &[(&str, &str)], query: &str) -> Match {
    let q = query.trim();
    let digits = q.strip_prefix('#').unwrap_or(q);
    let pick = |hits: Vec<usize>| match hits.len() {
        0 => Match::None,
        1 => Match::One(hits[0]),
        _ => Match::Many(hits),
    };
    if !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()) {
        return pick(
            tasks.iter().enumerate().filter(|(_, (id, _))| id.ends_with(digits)).map(|(i, _)| i).collect(),
        );
    }
    let q = norm(q);
    if q.is_empty() {
        return Match::None;
    }
    let exact: Vec<usize> =
        tasks.iter().enumerate().filter(|(_, (_, t))| norm(t) == q).map(|(i, _)| i).collect();
    if !exact.is_empty() {
        return pick(exact);
    }
    pick(tasks.iter().enumerate().filter(|(_, (_, t))| norm(t).contains(&q)).map(|(i, _)| i).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_ids_grow_until_unique() {
        let ids = ["1760000004821", "1760000014821", "1760000001234", "12"];
        assert_eq!(short_ids(&ids), vec!["#04821", "#14821", "#1234", "#12"]);
        assert_eq!(short_ids(&[]), Vec::<String>::new());
    }

    #[test]
    fn matching() {
        let tasks = [
            ("1760000004821", "Gửi báo cáo quý cho sếp"),
            ("1760000001234", "Gửi báo cáo tuần"),
            ("1760000005555", "Mua sữa"),
        ];
        assert_eq!(match_task(&tasks, "#4821"), Match::One(0));
        assert_eq!(match_task(&tasks, " 5555 "), Match::One(2));
        assert_eq!(match_task(&tasks, "#9999"), Match::None);
        assert_eq!(match_task(&tasks, "mua  SỮA"), Match::One(2));
        assert_eq!(match_task(&tasks, "báo cáo"), Match::Many(vec![0, 1]));
        assert_eq!(match_task(&tasks, "báo cáo tuần"), Match::One(1));
        assert_eq!(match_task(&tasks, "đi chợ"), Match::None);
        assert_eq!(match_task(&tasks, "   "), Match::None);
        assert_eq!(match_task(&tasks, "#"), Match::None);
        // Trùng hẳn thắng "có chứa".
        let t2 = [("1", "gửi mail"), ("2", "gửi mail cho khách")];
        assert_eq!(match_task(&t2, "Gửi mail"), Match::One(0));
    }
}
