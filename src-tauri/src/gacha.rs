//! 🎰 Candy Capsule: quay kẹo lấy món ngẫu nhiên trong Bộ sưu tập (PLAN.md 14.6).
//!
//! Hàm thuần, `rand` truyền vào (trả số trong [0, 1)) để test được. Quy tắc:
//! - 1 lượt 100 🍬, 10 lượt 1000 🍬 (rẻ hơn đổi thẳng nhiều lần, đổi lại may rủi).
//!   Đổi thẳng trong cửa hàng vẫn luôn có.
//! - Tỉ lệ Thường 65 · Hiếm 25 · Sử thi 10 (%), quay trong mọi món giá > 0.
//!   Hạng Huyền thoại (model Live2D) có tỉ lệ riêng từng món (`LEGENDARY_PCT`),
//!   lấy từ phần của Thường để tỉ lệ Hiếm / Sử thi giữ nguyên.
//! - Tỉ lệ tính bằng phần nghìn (‰) để chia được 1% cho nhiều model; JSON gửi UI đổi lại %.
//! - Bảo hiểm: 9 lượt liền không ra Hiếm+ thì lượt thứ 10 chắc chắn Hiếm+;
//!   49 lượt liền không ra Sử thi+ thì lượt thứ 50 chắc chắn Sử thi+. Huyền thoại
//!   không có bảo hiểm riêng, chỉ có phần của nó trong hai lần bảo hiểm trên.
//! - Trùng món đã có → hoàn 10% giá đổi thẳng của món bằng kẹo. Không bao giờ nhận trùng vào `owned`.
//! - Không bán kẹo bằng tiền thật: gacha hoàn toàn miễn phí nên không vướng luật loot box.
//! - 🎁 Quà một lần (`grant_gift`): 20 lượt không tốn kẹo; trong 20 lượt đó chắc
//!   chắn có một model chưa sở hữu (lượt quà cuối bị ép nếu chưa ra), chỉ một lần.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::collection::{items, owns, BuyError, Collection, Item, Rarity};

pub const PULL_COST: u32 = 100;
pub const TEN_COST: u32 = 1000;
/// Tỉ lệ theo phần nghìn khi chưa có món Huyền thoại, cộng lại đúng `RATE_TOTAL`.
pub const BASE_RATES: [(Rarity, u32); 3] = [(Rarity::Common, 650), (Rarity::Rare, 250), (Rarity::Epic, 100)];
/// Mẫu số của mọi tỉ lệ: phần nghìn.
pub const RATE_TOTAL: u32 = 1000;
/// Tỉ lệ (‰) của từng món Huyền thoại (PLAN.md 16.4), trừ vào phần của Thường.
/// Theo giá: model 10000 🍬 mỗi con 0.1%, 7500 🍬 mỗi con 0.2%, 5000 🍬 mỗi con 0.4%
/// (rẻ hơn thì ra dễ hơn; 4 × 1 + 3 × 2 + 6 × 4 = 34‰, cả hạng 3.4%). Xếp theo dòng
/// như catalog: Kawaii → Pastel → Aristocrat → Angel → Lolita → Dancer. Món chưa có
/// trong catalog thì bỏ qua.
pub const LEGENDARY_PCT: &[(&str, u32)] = &[
    ("model.gothic-kawaii", 4),
    ("model.pink-kawaii", 4),
    ("model.snow-kawaii", 4),
    ("model.gothic-pastel", 2),
    ("model.pink-pastel", 2),
    ("model.snow-pastel", 2),
    ("model.gothic-aristocrat", 1),
    ("model.pink-aristocrat", 1),
    ("model.snow-aristocrat", 1),
    ("model.pink-angel", 1),
    ("model.gothic-lolita", 4),
    ("model.snow-lolita", 4),
    ("model.snow-dancer", 4),
];
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

/// Kẹo hoàn lại khi quay trùng: 10% giá đổi thẳng của món (60 / 150 / 300 với
/// ba hạng giá chung; model Huyền thoại theo giá riêng).
pub fn refund(it: &Item) -> u32 {
    it.price / 10
}

/// Các món Huyền thoại quay được kèm tỉ lệ (‰).
fn legendary() -> impl Iterator<Item = (&'static Item, u32)> {
    LEGENDARY_PCT.iter().filter_map(|&(id, p)| {
        crate::collection::item(id)
            .filter(|i| i.price > 0 && i.rarity == Rarity::Legendary)
            .map(|i| (i, p))
    })
}

/// Tỉ lệ đang áp dụng (‰, cộng lại đúng `RATE_TOTAL`). Không có món Huyền thoại thì không có hạng đó.
pub fn rates() -> Vec<(Rarity, u32)> {
    let legend: u32 = legendary().map(|(_, p)| p).sum();
    let mut v: Vec<(Rarity, u32)> = BASE_RATES
        .iter()
        .map(|&(r, p)| (r, if r == Rarity::Common { p.saturating_sub(legend) } else { p }))
        .collect();
    if legend > 0 {
        v.push((Rarity::Legendary, legend));
    }
    v
}

/// ‰ → % cho UI (5 → 0.5).
fn pct(permille: u32) -> f64 {
    permille as f64 * 100.0 / RATE_TOTAL as f64
}

/// Chọn theo trọng số với `x` trong [0, 1). So bằng số nguyên để ngưỡng không lệch
/// vì làm tròn dấu phẩy động.
fn pick_weighted<T: Copy>(list: &[(T, u32)], x: f64) -> Option<T> {
    let total: u32 = list.iter().map(|(_, w)| w).sum();
    let v = ((x.clamp(0.0, 1.0) * total as f64) as u32).min(total.saturating_sub(1));
    let mut acc = 0;
    for &(t, w) in list {
        acc += w;
        if v < acc {
            return Some(t);
        }
    }
    list.last().map(|&(t, _)| t)
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
fn roll_rarity(rates: &[(Rarity, u32)], r: f64, r2: f64, min: Rarity) -> (Rarity, bool) {
    let natural = pick_weighted(rates, r).unwrap_or(min);
    if natural >= min {
        return (natural, false);
    }
    let allowed: Vec<(Rarity, u32)> = rates.iter().copied().filter(|(rar, _)| *rar >= min).collect();
    (pick_weighted(&allowed, r2).unwrap_or(min), true)
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
    let (rarity, pity_hit) = roll_rarity(&rates(), r, r2, min);
    let x = rand();
    let item = if rarity == Rarity::Legendary {
        // Mỗi model một tỉ lệ riêng (LEGENDARY_PCT), không chia đều.
        let list: Vec<(&'static Item, u32)> = legendary().collect();
        pick_weighted(&list, x)
    } else {
        None
    }
    .unwrap_or_else(|| {
        let mut bucket: Vec<&'static Item> = pool().filter(|i| i.rarity == rarity).collect();
        if bucket.is_empty() {
            bucket = pool().collect();
        }
        bucket[((x * bucket.len() as f64) as usize).min(bucket.len() - 1)]
    });
    record(c, item, now_ms, pity_hit)
}

/// Ghi một lượt đã chọn xong món: bảo hiểm, tổng lượt, `owned` hoặc hoàn kẹo, lịch sử.
fn record(c: &mut Collection, item: &'static Item, now_ms: u64, pity_hit: bool) -> Pull {
    c.pity = if item.rarity >= Rarity::Rare { 0 } else { c.pity + 1 };
    c.epic_pity = if item.rarity >= Rarity::Epic { 0 } else { c.epic_pity + 1 };
    c.pulls = c.pulls.saturating_add(1);
    let dup = owns(c, item);
    let back = if dup { refund(item) } else { 0 };
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

/// Hôm nay (`today`, bond::Day) còn lượt quay miễn phí không (khung 🎁 cạnh pet).
pub fn free_ready(c: &Collection, today: crate::bond::Day) -> bool {
    enabled() && c.free_day != today
}

/// 🎁 Lượt quay miễn phí mỗi ngày: một lượt thường (tính bảo hiểm, lịch sử, hoàn
/// khi trùng) nhưng không trừ kẹo. Mỗi ngày một lần; đồng hồ lùi về ngày cũ
/// cũng chỉ được thêm đúng một lượt cho ngày đó.
pub fn pull_free(
    c: &mut Collection,
    today: crate::bond::Day,
    now_ms: u64,
    rand: &mut impl FnMut() -> f64,
) -> Result<Pull, BuyError> {
    if !enabled() {
        return Err(BuyError::NotFound);
    }
    if c.free_day == today {
        return Err(BuyError::UsedToday);
    }
    c.free_day = today;
    Ok(pull_one(c, now_ms, rand))
}

/// 🎁 Quà một lần trong đời: số lượt quay miễn phí tặng mỗi người dùng.
pub const GIFT_SPINS: u32 = 20;

/// Tặng `GIFT_SPINS` lượt nếu chưa từng tặng. Gọi mỗi lần nạp collection.json
/// (store::load_collection), nên người dùng cũ lẫn mới đều nhận đúng một lần.
pub fn grant_gift(c: &mut Collection) -> bool {
    if c.gift_given {
        return false;
    }
    c.gift_given = true;
    c.gift_spins = c.gift_spins.saturating_add(GIFT_SPINS);
    true
}

/// Model Huyền thoại chưa có, kèm tỉ lệ (‰): ứng viên cho lượt bảo đảm.
fn unowned_models(c: &Collection) -> Vec<(&'static Item, u32)> {
    legendary().filter(|(it, _)| !owns(c, it)).collect()
}

/// Lượt quà còn bảo đảm model mới không (cho UI ghi "chắc chắn có 1 model mới").
pub fn gift_promise(c: &Collection) -> bool {
    c.gift_spins > 0 && !c.gift_model && !unowned_models(c).is_empty()
}

/// Quay `n` lượt quà (tối đa số lượt còn lại), không tốn kẹo. Bảo đảm một lần:
/// nếu tới lượt quà cuối mà chưa ra model mới thì lượt đó chắc chắn là một model
/// chưa có (chọn theo `LEGENDARY_PCT`). Ra model mới tự nhiên sớm hơn thì thôi ép.
pub fn pull_gift(
    c: &mut Collection,
    n: u32,
    now_ms: u64,
    rand: &mut impl FnMut() -> f64,
) -> Result<Vec<Pull>, BuyError> {
    if !enabled() {
        return Err(BuyError::NotFound);
    }
    let n = n.min(c.gift_spins);
    if n == 0 {
        return Err(BuyError::NoGift);
    }
    let mut out = Vec::with_capacity(n as usize);
    for _ in 0..n {
        c.gift_spins -= 1;
        let forced = if !c.gift_model && c.gift_spins == 0 {
            let x = rand();
            pick_weighted(&unowned_models(c), x)
        } else {
            None
        };
        let p = match forced {
            Some(item) => record(c, item, now_ms, true),
            None => pull_one(c, now_ms, rand),
        };
        if !p.dup && p.item.rarity == Rarity::Legendary {
            c.gift_model = true;
        }
        out.push(p);
    }
    Ok(out)
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
        "rates": rates().iter().map(|(r, p)| json!({ "rarity": r.code(), "pct": pct(*p) })).collect::<Vec<_>>(),
        // Hoàn khi trùng theo hạng (món rẻ nhất của hạng; ba hạng đầu mỗi hạng một giá).
        "refund": rates()
            .iter()
            .map(|(r, _)| json!({ "rarity": r.code(), "candy": pool().filter(|i| i.rarity == *r).map(refund).min().unwrap_or(0) }))
            .collect::<Vec<_>>(),
        // Từng món Huyền thoại: tên, tỉ lệ, kẹo hoàn khi trùng (ô tỉ lệ trên máy kẹo).
        "legendary": legendary()
            .map(|(it, p)| json!({ "id": it.id, "name": it.name[li], "pct": pct(p), "refund": refund(it) }))
            .collect::<Vec<_>>(),
        // Còn bao nhiêu lượt nữa thì chắc chắn Hiếm+ / Sử thi+ (tính cả lượt đó).
        "rareIn": PITY_RARE - c.pity.min(PITY_RARE - 1),
        "epicIn": PITY_EPIC - c.epic_pity.min(PITY_EPIC - 1),
        "pityRare": PITY_RARE,
        "pityEpic": PITY_EPIC,
        "pulls": c.pulls,
        "history": history,
        // 🎁 Quà 20 lượt: còn bao nhiêu, còn bảo đảm model mới không.
        "gift": c.gift_spins,
        "giftPromise": gift_promise(c),
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
        assert_eq!(BASE_RATES.iter().map(|(_, p)| p).sum::<u32>(), RATE_TOTAL);
        assert_eq!(rates().iter().map(|(_, p)| p).sum::<u32>(), RATE_TOTAL);
        assert!(enabled(), "catalog phải có ≥ {MIN_POOL} món mua được");
        assert_eq!(cost(10), Some(10 * PULL_COST));
        assert_eq!(cost(3), None);
        for (r, _) in rates() {
            assert!(pool().any(|i| i.rarity == r), "{r:?} không có món nào để quay");
        }
        // Mỗi món Huyền thoại có tỉ lệ riêng, không thì gacha không bao giờ ra nó.
        for it in pool().filter(|i| i.rarity == Rarity::Legendary) {
            assert!(LEGENDARY_PCT.iter().any(|(id, p)| *id == it.id && *p > 0), "{}: thiếu tỉ lệ", it.id);
        }
        for (id, _) in LEGENDARY_PCT {
            assert!(crate::collection::item(id).is_some_and(|i| i.rarity == Rarity::Legendary), "{id}: không phải món Huyền thoại");
        }
        // Quay rẻ hơn đổi thẳng: món rẻ nhất phải đắt hơn ít nhất 5 lượt.
        assert!(pool().all(|i| i.price >= 5 * PULL_COST));
    }

    #[test]
    fn rarity_thresholds() {
        let b = &BASE_RATES;
        assert_eq!(roll_rarity(b, 0.0, 0.0, Rarity::Common), (Rarity::Common, false));
        assert_eq!(roll_rarity(b, 0.649, 0.0, Rarity::Common), (Rarity::Common, false));
        assert_eq!(roll_rarity(b, 0.65, 0.0, Rarity::Common), (Rarity::Rare, false));
        assert_eq!(roll_rarity(b, 0.899, 0.0, Rarity::Common), (Rarity::Rare, false));
        assert_eq!(roll_rarity(b, 0.9, 0.0, Rarity::Common), (Rarity::Epic, false));
        assert_eq!(roll_rarity(b, 0.9999, 0.0, Rarity::Common), (Rarity::Epic, false));
        // Bảo hiểm Hiếm+: Thường bị đổi, chia 25:10 giữa Hiếm và Sử thi.
        assert_eq!(roll_rarity(b, 0.1, 0.0, Rarity::Rare), (Rarity::Rare, true));
        assert_eq!(roll_rarity(b, 0.1, 0.71, Rarity::Rare), (Rarity::Rare, true));
        assert_eq!(roll_rarity(b, 0.1, 0.72, Rarity::Rare), (Rarity::Epic, true));
        assert_eq!(roll_rarity(b, 0.95, 0.0, Rarity::Rare), (Rarity::Epic, false), "tự nhiên ra cao thì không tính bảo hiểm");
        assert_eq!(roll_rarity(b, 0.1, 0.0, Rarity::Epic), (Rarity::Epic, true));
    }

    #[test]
    fn legendary_comes_out_of_common_share() {
        let r = rates();
        assert_eq!(r, vec![(Rarity::Common, 616), (Rarity::Rare, 250), (Rarity::Epic, 100), (Rarity::Legendary, 34)]);
        assert_eq!(roll_rarity(&r, 0.615, 0.0, Rarity::Common), (Rarity::Common, false));
        assert_eq!(roll_rarity(&r, 0.616, 0.0, Rarity::Common), (Rarity::Rare, false));
        assert_eq!(roll_rarity(&r, 0.965, 0.0, Rarity::Common), (Rarity::Epic, false));
        assert_eq!(roll_rarity(&r, 0.966, 0.0, Rarity::Common), (Rarity::Legendary, false));
        // Bảo hiểm Hiếm+ chia 250:100:34; bảo hiểm Sử thi+ chia 100:34.
        assert_eq!(roll_rarity(&r, 0.1, 0.91, Rarity::Rare), (Rarity::Epic, true));
        assert_eq!(roll_rarity(&r, 0.1, 0.92, Rarity::Rare), (Rarity::Legendary, true));
        assert_eq!(roll_rarity(&r, 0.1, 0.74, Rarity::Epic), (Rarity::Epic, true));
        assert_eq!(roll_rarity(&r, 0.1, 0.75, Rarity::Epic), (Rarity::Legendary, true));
        // Trong hạng Huyền thoại (34‰): 0.4% mỗi model 5000 🍬, 0.2% mỗi model 7500 🍬,
        // 0.1% mỗi model 10000 🍬. Giữa đoạn của mỗi model thì ra đúng model đó, theo thứ tự bảng.
        let l: Vec<_> = legendary().collect();
        let total: u32 = l.iter().map(|(_, w)| w).sum();
        assert_eq!(total, 34);
        let mut acc = 0;
        for (it, w) in &l {
            let mid = (acc as f64 + *w as f64 / 2.0) / total as f64;
            assert_eq!(pick_weighted(&l, mid).unwrap().id, it.id);
            acc += w;
        }
        assert_eq!(l.first().unwrap().0.id, "model.gothic-kawaii");
        assert_eq!(
            l.iter().map(|(_, p)| pct(*p)).collect::<Vec<_>>(),
            vec![0.4, 0.4, 0.4, 0.2, 0.2, 0.2, 0.1, 0.1, 0.1, 0.1, 0.4, 0.4, 0.4]
        );
    }

    #[test]
    fn legendary_pull_resets_both_pities_and_refunds_ten_percent() {
        let mut c = Collection::default();
        c.candy = 200;
        c.pity = 5;
        c.epic_pity = 30;
        // r = 0.995 → Huyền thoại; r2, chọn món = 0 → model đầu bảng (Gothic Kawaii, 5000 🍬).
        let got = pull_many(&mut c, 1, 1, &mut seq(&[0.995, 0.0, 0.0])).unwrap();
        assert_eq!(got[0].item.id, "model.gothic-kawaii");
        assert!(!got[0].dup && !got[0].pity_hit);
        assert_eq!((c.pity, c.epic_pity), (0, 0));
        let again = pull_many(&mut c, 1, 2, &mut seq(&[0.995, 0.0, 0.0])).unwrap();
        assert!(again[0].dup);
        assert_eq!(again[0].refund, 500);
        assert_eq!(c.candy, 500);
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
        assert_eq!(b[0].refund, refund(a[0].item));
        assert_eq!(c.owned.iter().filter(|o| *o == a[0].item.id).count(), 1);
        assert_eq!(c.candy, refund(a[0].item));
        assert_eq!(c.history.len(), 2);
        assert_eq!((c.history[0].t, c.history[0].dup), (2, true), "mới nhất lên đầu");
    }

    #[test]
    fn free_pull_once_a_day_costs_nothing() {
        let mut c = Collection::default();
        let day = 20_000;
        assert!(free_ready(&c, day));
        let p = pull_free(&mut c, day, 7, &mut seq(&[0.0])).unwrap();
        assert!(!p.dup);
        assert_eq!((c.candy, c.spent_total, c.pulls, c.pity), (0, 0, 1, 1), "không trừ kẹo, vẫn tính bảo hiểm");
        assert_eq!(c.history[0].id, p.item.id);
        assert!(!free_ready(&c, day));
        let before = c.clone();
        assert_eq!(pull_free(&mut c, day, 8, &mut seq(&[0.0])).unwrap_err(), BuyError::UsedToday);
        assert_eq!(c, before, "lượt thứ hai trong ngày không đổi gì");
        // Ngày mới: lại có lượt; trùng thì vẫn hoàn kẹo như lượt thường.
        assert!(free_ready(&c, day + 1));
        let q = pull_free(&mut c, day + 1, 9, &mut seq(&[0.0])).unwrap();
        assert!(q.dup);
        assert_eq!(c.candy, refund(q.item));
    }

    fn new_models(c: &Collection, before: &Collection) -> usize {
        legendary().filter(|(it, _)| owns(c, it) && !owns(before, it)).count()
    }

    #[test]
    fn gift_is_granted_once_in_a_lifetime() {
        let mut c = Collection::default();
        assert!(grant_gift(&mut c));
        assert_eq!(c.gift_spins, GIFT_SPINS);
        assert!(!grant_gift(&mut c), "lần nạp sau không tặng lại");
        assert_eq!(c.gift_spins, GIFT_SPINS);
        // Dùng hết rồi cũng không tặng lại.
        pull_gift(&mut c, 10, 0, &mut seq(&[0.0])).unwrap();
        pull_gift(&mut c, 10, 0, &mut seq(&[0.0])).unwrap();
        assert!(!grant_gift(&mut c));
        assert_eq!(c.gift_spins, 0);
        assert_eq!(pull_gift(&mut c, 1, 0, &mut seq(&[0.0])).unwrap_err(), BuyError::NoGift);
    }

    #[test]
    fn gift_guarantees_exactly_one_new_model_when_luck_never_gives_one() {
        let mut c = Collection::default();
        grant_gift(&mut c);
        let before = c.clone();
        // r = 0 → luôn Thường: tự nhiên không bao giờ ra Huyền thoại.
        let mut r = seq(&[0.0]);
        let a = pull_gift(&mut c, 10, 0, &mut r).unwrap();
        assert_eq!(a.len(), 10);
        assert!(gift_promise(&c), "chưa ra model thì vẫn còn bảo đảm");
        let b = pull_gift(&mut c, 10, 0, &mut r).unwrap();
        let last = b.last().unwrap();
        assert_eq!(last.item.rarity, Rarity::Legendary);
        assert!(!last.dup && last.pity_hit, "lượt quà cuối bị ép ra model mới");
        assert_eq!(new_models(&c, &before), 1);
        assert!(c.gift_model && !gift_promise(&c));
        // Không trừ kẹo: kẹo đang có đúng bằng tổng tiền hoàn của các lượt trùng.
        let refunds: u32 = a.iter().chain(b.iter()).map(|p| p.refund).sum();
        assert_eq!((c.candy, c.spent_total, c.pulls), (refunds, 0, 20), "quà không tốn kẹo, vẫn tính lượt");
    }

    #[test]
    fn gift_does_not_force_after_a_natural_new_model() {
        let mut c = Collection::default();
        grant_gift(&mut c);
        // Lượt đầu ra Huyền thoại tự nhiên (r gần 1 rơi vào phần Huyền thoại ở cuối bảng).
        let first = pull_gift(&mut c, 1, 0, &mut seq(&[0.9999, 0.0, 0.0])).unwrap();
        assert_eq!(first[0].item.rarity, Rarity::Legendary);
        assert!(c.gift_model);
        let before = c.clone();
        let rest = pull_gift(&mut c, 19, 0, &mut seq(&[0.0])).unwrap();
        assert_eq!(rest.len(), 19);
        assert!(rest.iter().all(|p| p.item.rarity != Rarity::Legendary), "không ép thêm model nào");
        assert_eq!(new_models(&c, &before), 0);
    }

    #[test]
    fn gift_with_every_model_owned_falls_back_to_normal_pulls() {
        let mut c = Collection::default();
        crate::collection::unlock_all(&mut c);
        grant_gift(&mut c);
        assert!(!gift_promise(&c));
        let got = pull_gift(&mut c, 20, 0, &mut seq(&[0.0])).unwrap();
        assert_eq!(got.len(), 20);
        assert!(got.iter().all(|p| p.dup));
    }

    #[test]
    fn gift_count_is_capped_by_spins_left() {
        let mut c = Collection::default();
        grant_gift(&mut c);
        pull_gift(&mut c, 10, 0, &mut seq(&[0.0])).unwrap();
        pull_gift(&mut c, 5, 0, &mut seq(&[0.0])).unwrap();
        assert_eq!(pull_gift(&mut c, 10, 0, &mut seq(&[0.0])).unwrap().len(), 5);
        assert_eq!(c.gift_spins, 0);
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
