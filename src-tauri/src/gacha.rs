//! 🎰 Candy Capsule: quay kẹo lấy món ngẫu nhiên trong Bộ sưu tập (PLAN.md 14.6).
//!
//! Hàm thuần, `rand` truyền vào (trả số trong [0, 1)) để test được. Quy tắc:
//! - 1 lượt 100 🍬, 10 lượt 1000 🍬 (rẻ hơn đổi thẳng nhiều lần, đổi lại may rủi).
//!   Đổi thẳng trong cửa hàng vẫn luôn có.
//! - Tỉ lệ Thường 65 · Hiếm 25 · Sử thi 10 (%), quay trong mọi món giá > 0.
//! - Bảo hiểm: 9 lượt liền không ra Hiếm+ thì lượt thứ 10 chắc chắn Hiếm+;
//!   49 lượt liền không ra Sử thi thì lượt thứ 50 chắc chắn Sử thi.
//! - Trùng món đã có → hoàn 10% giá đổi thẳng bằng kẹo. Không bao giờ nhận trùng vào `owned`.
//! - Không bán kẹo bằng tiền thật: gacha hoàn toàn miễn phí nên không vướng luật loot box.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::collection::{items, owns, BuyError, Collection, Item, Rarity};

pub const PULL_COST: u32 = 100;
pub const TEN_COST: u32 = 1000;
/// Tỉ lệ theo phần trăm, cộng lại đúng 100.
pub const RATES: [(Rarity, u32); 3] = [(Rarity::Common, 65), (Rarity::Rare, 25), (Rarity::Epic, 10)];
/// Lượt thứ N kể từ lần cuối ra Hiếm+ chắc chắn Hiếm+.
pub const PITY_RARE: u32 = 10;
/// Lượt thứ N kể từ lần cuối ra Sử thi chắc chắn Sử thi.
pub const PITY_EPIC: u32 = 50;
/// Dưới số món này thì máy chưa mở (gacha nhàm khi quay toàn trùng).
pub const MIN_POOL: usize = 15;
/// Số lượt gần nhất giữ trong lịch sử.
pub const HISTORY_MAX: usize = 50;

/// Một dòng lịch sử quay (collection.json).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HistEntry {
    pub id: String,
    /// Mili giây Unix.
    pub t: u64,
    #[serde(default)]
    pub dup: bool,
}

#[derive(Debug, PartialEq)]
pub struct Pull {
    pub item: &'static Item,
    pub dup: bool,
    /// Kẹo hoàn lại khi trùng (0 nếu món mới).
    pub refund: u32,
    /// Lượt này do bảo hiểm (không phải may mắn tự nhiên).
    pub pity_hit: bool,
}

/// Giá của `n` lượt. Chỉ cho 1 hoặc 10.
pub fn cost(n: u32) -> Option<u32> {
    match n {
        1 => Some(PULL_COST),
        10 => Some(TEN_COST),
        _ => None,
    }
}

/// Kẹo hoàn lại khi quay trùng: 10% giá đổi thẳng của hạng.
pub fn refund(r: Rarity) -> u32 {
    match r {
        Rarity::Common => 60,
        Rarity::Rare => 150,
        Rarity::Epic => 300,
    }
}

/// Mọi món quay được: món giá > 0.
pub fn pool() -> impl Iterator<Item = &'static Item> {
    items().filter(|i| i.price > 0)
}

pub fn enabled() -> bool {
    pool().count() >= MIN_POOL
}

/// Chọn hạng cho một lượt. `r` cho hạng tự nhiên; khi bảo hiểm nổ mà ra thấp
/// hơn thì `r2` chia lại giữa các hạng đủ cao theo đúng tỉ lệ của chúng.
fn roll_rarity(r: f64, r2: f64, min: Rarity) -> (Rarity, bool) {
    // So bằng số nguyên (phần trăm) để ngưỡng không lệch vì làm tròn dấu phẩy động.
    let pick = |list: &[(Rarity, u32)], x: f64| {
        let total: u32 = list.iter().map(|(_, p)| p).sum();
        let v = ((x.clamp(0.0, 1.0) * total as f64) as u32).min(total.saturating_sub(1));
        let mut acc = 0;
        for (rar, pct) in list {
            acc += pct;
            if v < acc {
                return *rar;
            }
        }
        list.last().map(|(r, _)| *r).unwrap_or(min)
    };
    let natural = pick(&RATES, r);
    if natural >= min {
        return (natural, false);
    }
    let allowed: Vec<(Rarity, u32)> = RATES.iter().copied().filter(|(rar, _)| *rar >= min).collect();
    (pick(&allowed, r2), true)
}

/// Một lượt (đã trừ kẹo ở `pull_many`). Cập nhật bảo hiểm, `owned`, lịch sử.
fn pull_one(c: &mut Collection, now_ms: u64, rand: &mut impl FnMut() -> f64) -> Pull {
    let min = if c.epic_pity + 1 >= PITY_EPIC {
        Rarity::Epic
    } else if c.pity + 1 >= PITY_RARE {
        Rarity::Rare
    } else {
        Rarity::Common
    };
    let (r, r2) = (rand(), rand());
    let (rarity, pity_hit) = roll_rarity(r, r2, min);
    let mut bucket: Vec<&'static Item> = pool().filter(|i| i.rarity == rarity).collect();
    if bucket.is_empty() {
        bucket = pool().collect();
    }
    let idx = ((rand() * bucket.len() as f64) as usize).min(bucket.len() - 1);
    let item = bucket[idx];

    c.pity = if item.rarity >= Rarity::Rare { 0 } else { c.pity + 1 };
    c.epic_pity = if item.rarity == Rarity::Epic { 0 } else { c.epic_pity + 1 };
    c.pulls = c.pulls.saturating_add(1);
    let dup = owns(c, item);
    let back = if dup { refund(item.rarity) } else { 0 };
    if dup {
        c.candy = c.candy.saturating_add(back);
        c.spent_total = c.spent_total.saturating_sub(back);
    } else {
        c.owned.push(item.id.to_string());
    }
    c.history.insert(0, HistEntry { id: item.id.to_string(), t: now_ms, dup });
    c.history.truncate(HISTORY_MAX);
    Pull { item, dup, refund: back, pity_hit }
}

/// Quay `n` lượt (1 hoặc 10): trừ kẹo một lần, tất cả hoặc không.
pub fn pull_many(
    c: &mut Collection,
    n: u32,
    now_ms: u64,
    rand: &mut impl FnMut() -> f64,
) -> Result<Vec<Pull>, BuyError> {
    let price = cost(n).ok_or(BuyError::NotFound)?;
    if !enabled() {
        return Err(BuyError::NotFound);
    }
    if c.candy < price {
        return Err(BuyError::NotEnough { need: price - c.candy });
    }
    c.candy -= price;
    c.spent_total = c.spent_total.saturating_add(price);
    Ok((0..n).map(|_| pull_one(c, now_ms, rand)).collect())
}

/// Nguồn ngẫu nhiên cho bản chạy thật: SplitMix64, gieo từ đồng hồ nano giây
/// trộn bộ đếm toàn cục (hai lần quay sát nhau vẫn khác). Đủ cho trò chơi,
/// không dùng cho bảo mật; không thêm crate `rand`.
pub fn rng() -> impl FnMut() -> f64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    let mut state = nanos ^ COUNTER.fetch_add(0x9E37_79B9_7F4A_7C15, Ordering::Relaxed).rotate_left(17);
    move || {
        state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        // 53 bit cao → [0, 1).
        (z >> 11) as f64 / (1u64 << 53) as f64
    }
}

/// Phần gacha trong collection::view: giá, tỉ lệ, bảo hiểm, lịch sử.
pub fn view_json(c: &Collection, lang_idx: usize) -> Value {
    let li = lang_idx.min(2);
    let history: Vec<Value> = c
        .history
        .iter()
        .filter_map(|h| {
            let it = crate::collection::item(&h.id)?;
            Some(json!({ "id": it.id, "name": it.name[li], "rarity": it.rarity.code(), "cat": it.cat.code(), "t": h.t, "dup": h.dup }))
        })
        .collect();
    json!({
        "enabled": enabled(),
        "pool": pool().count(),
        "minPool": MIN_POOL,
        "cost1": PULL_COST,
        "cost10": TEN_COST,
        "rates": RATES.iter().map(|(r, p)| json!({ "rarity": r.code(), "pct": p })).collect::<Vec<_>>(),
        "refund": RATES.iter().map(|(r, _)| json!({ "rarity": r.code(), "candy": refund(*r) })).collect::<Vec<_>>(),
        // Còn bao nhiêu lượt nữa thì chắc chắn Hiếm+ / Sử thi (tính cả lượt đó).
        "rareIn": PITY_RARE - c.pity.min(PITY_RARE - 1),
        "epicIn": PITY_EPIC - c.epic_pity.min(PITY_EPIC - 1),
        "pityRare": PITY_RARE,
        "pityEpic": PITY_EPIC,
        "pulls": c.pulls,
        "history": history,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Trả lần lượt các số trong `seq`, lặp lại.
    fn seq(v: &'static [f64]) -> impl FnMut() -> f64 {
        let mut i = 0;
        move || {
            let x = v[i % v.len()];
            i += 1;
            x
        }
    }

    #[test]
    fn rules_are_sane() {
        assert_eq!(RATES.iter().map(|(_, p)| p).sum::<u32>(), 100);
        assert!(enabled(), "catalog phải có ≥ {MIN_POOL} món mua được");
        assert_eq!(cost(10), Some(10 * PULL_COST));
        assert_eq!(cost(3), None);
        for (r, _) in RATES {
            assert!(pool().any(|i| i.rarity == r), "{r:?} không có món nào để quay");
        }
        // Quay rẻ hơn đổi thẳng: món rẻ nhất phải đắt hơn ít nhất 5 lượt.
        assert!(pool().all(|i| i.price >= 5 * PULL_COST));
    }

    #[test]
    fn rarity_thresholds() {
        assert_eq!(roll_rarity(0.0, 0.0, Rarity::Common), (Rarity::Common, false));
        assert_eq!(roll_rarity(0.649, 0.0, Rarity::Common), (Rarity::Common, false));
        assert_eq!(roll_rarity(0.65, 0.0, Rarity::Common), (Rarity::Rare, false));
        assert_eq!(roll_rarity(0.899, 0.0, Rarity::Common), (Rarity::Rare, false));
        assert_eq!(roll_rarity(0.9, 0.0, Rarity::Common), (Rarity::Epic, false));
        assert_eq!(roll_rarity(0.9999, 0.0, Rarity::Common), (Rarity::Epic, false));
        // Bảo hiểm Hiếm+: Thường bị đổi, chia 25:10 giữa Hiếm và Sử thi.
        assert_eq!(roll_rarity(0.1, 0.0, Rarity::Rare), (Rarity::Rare, true));
        assert_eq!(roll_rarity(0.1, 0.71, Rarity::Rare), (Rarity::Rare, true));
        assert_eq!(roll_rarity(0.1, 0.72, Rarity::Rare), (Rarity::Epic, true));
        assert_eq!(roll_rarity(0.95, 0.0, Rarity::Rare), (Rarity::Epic, false), "tự nhiên ra cao thì không tính bảo hiểm");
        assert_eq!(roll_rarity(0.1, 0.0, Rarity::Epic), (Rarity::Epic, true));
    }

    #[test]
    fn not_enough_changes_nothing() {
        let mut c = Collection::default();
        c.candy = 999;
        let before = c.clone();
        assert_eq!(pull_many(&mut c, 10, 1, &mut seq(&[0.0])).unwrap_err(), BuyError::NotEnough { need: 1 });
        assert_eq!(pull_many(&mut c, 2, 1, &mut seq(&[0.0])).unwrap_err(), BuyError::NotFound);
        assert_eq!(c, before);
    }

    #[test]
    fn rare_pity_fires_on_tenth_pull_then_resets() {
        let mut c = Collection::default();
        c.candy = 10_000;
        // Luôn ra Thường tự nhiên; r2 = 0 → bảo hiểm cho Hiếm.
        let got = pull_many(&mut c, 10, 5, &mut seq(&[0.0])).unwrap();
        assert!(got[..9].iter().all(|p| p.item.rarity == Rarity::Common && !p.pity_hit));
        assert_eq!((got[9].item.rarity, got[9].pity_hit), (Rarity::Rare, true));
        assert_eq!(c.pity, 0);
        assert_eq!(c.epic_pity, 10);
        assert_eq!((c.candy, c.pulls), (9_000 + got.iter().map(|p| p.refund).sum::<u32>(), 10));
    }

    #[test]
    fn epic_pity_fires_on_fiftieth_pull() {
        let mut c = Collection::default();
        c.candy = 100_000;
        let mut r = seq(&[0.0]);
        for _ in 0..4 {
            pull_many(&mut c, 10, 0, &mut r).unwrap();
        }
        assert_eq!(c.epic_pity, 40);
        let got = pull_many(&mut c, 10, 0, &mut r).unwrap();
        assert_eq!((got[9].item.rarity, got[9].pity_hit), (Rarity::Epic, true));
        assert_eq!(c.epic_pity, 0);
    }

    #[test]
    fn duplicates_refund_and_never_double_own() {
        let mut c = Collection::default();
        c.candy = 200;
        // Hai lượt cùng một món (r=0 → Thường, món đầu nhóm).
        let a = pull_many(&mut c, 1, 1, &mut seq(&[0.0])).unwrap();
        let b = pull_many(&mut c, 1, 2, &mut seq(&[0.0])).unwrap();
        assert_eq!(a[0].item.id, b[0].item.id);
        assert!(!a[0].dup && b[0].dup);
        assert_eq!(b[0].refund, refund(Rarity::Common));
        assert_eq!(c.owned.iter().filter(|o| *o == a[0].item.id).count(), 1);
        assert_eq!(c.candy, refund(Rarity::Common));
        assert_eq!(c.history.len(), 2);
        assert_eq!((c.history[0].t, c.history[0].dup), (2, true), "mới nhất lên đầu");
    }

    #[test]
    fn history_is_capped() {
        let mut c = Collection::default();
        c.candy = 100_000;
        let mut r = seq(&[0.3, 0.5, 0.7]);
        for _ in 0..7 {
            pull_many(&mut c, 10, 0, &mut r).unwrap();
        }
        assert_eq!(c.history.len(), HISTORY_MAX);
        let v = view_json(&c, 0);
        assert_eq!(v["history"].as_array().unwrap().len(), HISTORY_MAX);
        assert_eq!(v["pulls"], 70);
    }
}
