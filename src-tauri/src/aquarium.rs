// 🐠 Bể cá (PLAN.md mục 18): cá trong bể, bể chờ, cho ăn mỗi ngày, cá lớn dần,
// vật trang trí ở đáy bể. Hàm thuần, có test. Đọc / ghi file:
// store::load_aquarium / update_aquarium (aquarium.json riêng, không nằm trong
// Settings). Widget + lệnh: aquarium_pin.rs. Loài / nền / trang trí mua ở Bộ sưu
// tập (collection.rs, nhóm Fish / TankBg / TankDeco).
//
// Nguyên tắc: cá không chết, không bỏ đi, không trừ kẹo; cho ăn không ra kẹo.
// Loài và id trang trí gửi ra webview luôn là chuỗi tĩnh lấy từ SPECIES / catalog,
// không phải chuỗi đọc từ file.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::bond::Day;

/// Cá trong bể tối đa.
pub const MAX_IN_TANK: usize = 8;
/// Mỗi loài tối đa (tính cả bể chờ).
pub const MAX_PER_SPECIES: usize = 3;
/// Số ô đặt vật trang trí ở đáy bể.
pub const DECOR_SLOTS: usize = 4;
/// Tên cá tối đa (ký tự).
pub const NAME_MAX: usize = 20;
/// Cá vàng tặng khi bể mới.
pub const STARTER: &str = "fish.goldfish";
pub const STARTER_COUNT: usize = 2;
/// Số ngày được cho ăn để lên cỡ nhỡ / lớn.
pub const STAGE_DAYS: [u32; 2] = [5, 15];

/// Mọi loài cá app biết vẽ (app/js/aquarium-core.js có shape cho từng id). Mỗi
/// loài là một món Fish trong catalog collection.rs (test fish_catalog_matches_aquarium_species).
pub const SPECIES: [&str; 9] = [
    "fish.goldfish",
    "fish.neon",
    "fish.guppy",
    "fish.molly",
    "fish.betta",
    "fish.angel",
    "fish.clown",
    "fish.koi",
    "fish.miu",
];

/// Chuỗi đọc từ file → id tĩnh. Loài lạ (catalog đổi id, file bị sửa) → None.
pub fn species_id(s: &str) -> Option<&'static str> {
    SPECIES.iter().copied().find(|id| *id == s)
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fish {
    #[serde(default)]
    pub uid: u32,
    #[serde(default)]
    pub species: String,
    /// Tên người dùng đặt; rỗng = gọi theo tên loài.
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub born: Day,
    /// Số ngày (khác nhau) đã được cho ăn khi đang ở trong bể.
    #[serde(default)]
    pub fed_days: u32,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Aquarium {
    /// Cá đang thả trong bể (≤ MAX_IN_TANK).
    #[serde(default)]
    pub fish: Vec<Fish>,
    /// Cá đã vớt ra (giữ tên, tuổi, cỡ), thả lại được.
    #[serde(default)]
    pub stored: Vec<Fish>,
    /// Vật trang trí theo ô (id tdeco.*; "" = ô trống). Vec chứ không phải mảng cố
    /// định: file sai độ dài vẫn đọc được, không làm hỏng cả file (mất cá).
    #[serde(default)]
    pub decor: Vec<String>,
    /// Ngày cho ăn gần nhất.
    #[serde(default)]
    pub last_fed: Option<Day>,
    #[serde(default)]
    pub next_uid: u32,
    /// Đã tặng cá vàng (một lần trong đời, kể cả khi người dùng vớt hết cá sau này).
    #[serde(default)]
    pub starter: bool,
    /// Ngày Miu đã nhắc "cá đói" (tối đa một lần mỗi ngày).
    #[serde(default)]
    pub hungry_said: Option<Day>,
}

// ─── Đói (PLAN.md 18.7) ──────────────────────────────────────────────────────
// Lâu không cho ăn thì cá bơi chậm lại và Miu nhắc (tối đa một lần / ngày). Cá
// KHÔNG chết, không bỏ đi, không nhỏ lại, không trừ kẹo: cho ăn là bơi lại như thường.

/// Không được ăn từ bấy nhiêu ngày: hơi đói (bơi chậm một chút).
pub const PECKISH_DAYS: Day = 2;
/// Từ bấy nhiêu ngày: đói (bơi chậm hẳn, Miu nhắc).
pub const HUNGRY_DAYS: Day = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hunger {
    Full,
    Peckish,
    Hungry,
}

impl Hunger {
    pub fn code(self) -> &'static str {
        match self {
            Hunger::Full => "full",
            Hunger::Peckish => "peckish",
            Hunger::Hungry => "hungry",
        }
    }
}

/// Mức đói của bể hôm nay. `enabled` = công tắc trong Cài đặt (tắt = luôn no).
/// Chưa cho ăn lần nào thì tính từ ngày con cá lâu nhất được thả (bể mới không bị
/// coi là đói ngay). Đồng hồ lùi (ngày cho ăn ở tương lai) thì coi như vừa ăn.
pub fn hunger(a: &Aquarium, today: Day, enabled: bool) -> Hunger {
    if !enabled || a.fish.is_empty() {
        return Hunger::Full;
    }
    let since = a.last_fed.unwrap_or_else(|| a.fish.iter().map(|f| f.born).min().unwrap_or(today));
    match (today - since).max(0) {
        d if d >= HUNGRY_DAYS => Hunger::Hungry,
        d if d >= PECKISH_DAYS => Hunger::Peckish,
        _ => Hunger::Full,
    }
}

/// Miu có nên nhắc "cá đói" hôm nay không: đói thật, chưa nhắc hôm nay, và người
/// dùng đang dùng bể (đang ghim widget hoặc bật bể full; không thì nhắc là thừa).
pub fn should_nag(a: &Aquarium, today: Day, enabled: bool, in_use: bool) -> bool {
    in_use && hunger(a, today, enabled) == Hunger::Hungry && a.hungry_said != Some(today)
}

/// Cỡ cá: 0 bé · 1 nhỡ · 2 lớn.
pub fn stage(fed_days: u32) -> u8 {
    STAGE_DAYS.iter().filter(|d| fed_days >= **d).count() as u8
}

/// Bể mới: tặng 2 cá vàng, đúng một lần. Trả true nếu vừa tặng.
pub fn ensure_starter(a: &mut Aquarium, today: Day) -> bool {
    if a.starter {
        return false;
    }
    a.starter = true;
    for _ in 0..STARTER_COUNT {
        if add(a, STARTER, true, today).is_err() {
            break;
        }
    }
    true
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Feed {
    /// Lần cho ăn đầu tiên trong ngày; grew = uid các con vừa lên cỡ.
    Fed { grew: Vec<u32> },
    /// Hôm nay đã cho ăn rồi: không cộng gì (thức ăn vẫn rơi cho vui ở trang).
    Already,
}

pub fn fed_today(a: &Aquarium, today: Day) -> bool {
    a.last_fed == Some(today)
}

/// Cho ăn. Mỗi ngày cộng fed_days của mọi cá TRONG BỂ đúng một lần. Đồng hồ máy
/// lùi (last_fed ở tương lai) thì cũng không cộng, để không cày được bằng cách
/// chỉnh ngày.
pub fn feed(a: &mut Aquarium, today: Day) -> Feed {
    if a.last_fed.is_some_and(|d| d >= today) {
        return Feed::Already;
    }
    a.last_fed = Some(today);
    let mut grew = Vec::new();
    for f in &mut a.fish {
        let before = stage(f.fed_days);
        f.fed_days = f.fed_days.saturating_add(1);
        if stage(f.fed_days) > before {
            grew.push(f.uid);
        }
    }
    Feed::Fed { grew }
}

// ─── Thả / vớt / đổi tên / trang trí ────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AqError {
    /// Loài lạ / id trang trí lạ.
    NotFound,
    /// Chưa mua loài / món này.
    NotOwned,
    TankFull,
    SpeciesLimit,
    NoSuchFish,
    BadSlot,
}

impl AqError {
    /// Mã cho trang (js/aquarium-core.js errorText).
    pub fn code(self) -> &'static str {
        match self {
            AqError::NotFound => "aq_not_found",
            AqError::NotOwned => "aq_not_owned",
            AqError::TankFull => "aq_tank_full",
            AqError::SpeciesLimit => "aq_species_limit",
            AqError::NoSuchFish => "aq_no_fish",
            AqError::BadSlot => "aq_bad_slot",
        }
    }
}

/// Số con của một loài (trong bể + bể chờ).
pub fn species_count(a: &Aquarium, species: &str) -> usize {
    a.fish.iter().chain(&a.stored).filter(|f| f.species == species).count()
}

/// uid lớn nhất đang có: file sửa tay / cũ có next_uid nhỏ hơn thì không cấp trùng.
fn max_uid(a: &Aquarium) -> u32 {
    a.fish.iter().chain(&a.stored).map(|f| f.uid).max().unwrap_or(0)
}

/// Thả một con mới vào bể. owned = đã mua loài này chưa (collection).
pub fn add(a: &mut Aquarium, species: &str, owned: bool, today: Day) -> Result<u32, AqError> {
    let id = species_id(species).ok_or(AqError::NotFound)?;
    if !owned {
        return Err(AqError::NotOwned);
    }
    if species_count(a, id) >= MAX_PER_SPECIES {
        return Err(AqError::SpeciesLimit);
    }
    if a.fish.len() >= MAX_IN_TANK {
        return Err(AqError::TankFull);
    }
    a.next_uid = a.next_uid.max(max_uid(a)) + 1;
    a.fish.push(Fish { uid: a.next_uid, species: id.into(), name: String::new(), born: today, fed_days: 0 });
    Ok(a.next_uid)
}

/// Vớt một con ra bể chờ.
pub fn to_storage(a: &mut Aquarium, uid: u32) -> Result<(), AqError> {
    let i = a.fish.iter().position(|f| f.uid == uid).ok_or(AqError::NoSuchFish)?;
    let f = a.fish.remove(i);
    a.stored.push(f);
    Ok(())
}

/// Thả lại một con từ bể chờ.
pub fn to_tank(a: &mut Aquarium, uid: u32) -> Result<(), AqError> {
    let i = a.stored.iter().position(|f| f.uid == uid).ok_or(AqError::NoSuchFish)?;
    if a.fish.len() >= MAX_IN_TANK {
        return Err(AqError::TankFull);
    }
    let f = a.stored.remove(i);
    a.fish.push(f);
    Ok(())
}

/// Đặt tên (rỗng = gọi theo tên loài). Cá ở bể hay bể chờ đều được.
pub fn rename(a: &mut Aquarium, uid: u32, name: &str) -> Result<(), AqError> {
    let f = a.fish.iter_mut().chain(a.stored.iter_mut()).find(|f| f.uid == uid).ok_or(AqError::NoSuchFish)?;
    f.name = clean_name(name);
    Ok(())
}

/// Đặt vật trang trí vào ô slot (None = dọn ô). owned = đã mua món này.
/// Một món chỉ nằm ở một ô: đặt sang ô khác thì ô cũ trống.
pub fn set_decor(a: &mut Aquarium, slot: usize, id: Option<&'static str>, owned: bool) -> Result<(), AqError> {
    if slot >= DECOR_SLOTS {
        return Err(AqError::BadSlot);
    }
    if id.is_some() && !owned {
        return Err(AqError::NotOwned);
    }
    a.decor.resize(DECOR_SLOTS.max(a.decor.len()), String::new());
    a.decor.truncate(DECOR_SLOTS);
    if let Some(id) = id {
        for d in a.decor.iter_mut().filter(|d| d.as_str() == id) {
            d.clear();
        }
    }
    a.decor[slot] = id.unwrap_or("").to_string();
    Ok(())
}

fn fish_json(f: &Fish, today: Day) -> Option<Value> {
    let species = species_id(&f.species)?;
    Some(json!({
        "uid": f.uid,
        "species": species,
        "name": clean_name(&f.name),
        "stage": stage(f.fed_days),
        "fedDays": f.fed_days,
        "age": (today - f.born).max(0),
    }))
}

/// Dữ liệu cho widget và cửa sổ Bể cá. Cá loài lạ bị ẩn nhưng KHÔNG xoá khỏi file.
/// decor_ok(id) → id tĩnh nếu món có thật và đã sở hữu
/// (collection::owned_aquarium_item); ô có món lạ / chưa sở hữu hiện trống.
pub fn view_json(a: &Aquarium, today: Day, hunger_on: bool, decor_ok: impl Fn(&str) -> Option<&'static str>) -> Value {
    let fish: Vec<Value> = a.fish.iter().take(MAX_IN_TANK).filter_map(|f| fish_json(f, today)).collect();
    let stored: Vec<Value> = a.stored.iter().filter_map(|f| fish_json(f, today)).collect();
    let decor: Vec<&str> = (0..DECOR_SLOTS)
        .map(|i| a.decor.get(i).filter(|d| !d.is_empty()).and_then(|d| decor_ok(d)).unwrap_or(""))
        .collect();
    json!({
        "fish": fish,
        "stored": stored,
        "decor": decor,
        "fedToday": fed_today(a, today),
        "hunger": hunger(a, today, hunger_on).code(),
        "max": MAX_IN_TANK,
        "perSpecies": MAX_PER_SPECIES,
    })
}

/// Tên đọc từ file / người dùng gõ: bỏ ký tự điều khiển, tối đa NAME_MAX ký tự
/// (trang vẫn hiện bằng textContent).
pub fn clean_name(s: &str) -> String {
    s.chars().filter(|c| !c.is_control()).take(NAME_MAX).collect::<String>().trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stage_thresholds() {
        assert_eq!(stage(0), 0);
        assert_eq!(stage(4), 0);
        assert_eq!(stage(5), 1);
        assert_eq!(stage(14), 1);
        assert_eq!(stage(15), 2);
        assert_eq!(stage(u32::MAX), 2);
    }

    #[test]
    fn starter_given_once() {
        let mut a = Aquarium::default();
        assert!(ensure_starter(&mut a, 100));
        assert_eq!(a.fish.len(), STARTER_COUNT);
        assert!(a.fish.iter().all(|f| f.species == STARTER && f.born == 100));
        assert_ne!(a.fish[0].uid, a.fish[1].uid);
        // Vớt hết cá rồi mở lại: không tặng lần nữa.
        a.fish.clear();
        assert!(!ensure_starter(&mut a, 101));
        assert!(a.fish.is_empty());
    }

    #[test]
    fn starter_is_a_known_species() {
        assert_eq!(species_id(STARTER), Some(STARTER));
        assert_eq!(species_id("fish.shark"), None);
        let mut ids = SPECIES.to_vec();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), SPECIES.len(), "id loài không trùng");
    }

    #[test]
    fn feed_once_per_day_and_grow() {
        let mut a = Aquarium::default();
        ensure_starter(&mut a, 0);
        assert_eq!(feed(&mut a, 1), Feed::Fed { grew: vec![] });
        assert_eq!(feed(&mut a, 1), Feed::Already);
        assert!(fed_today(&a, 1));
        assert!(!fed_today(&a, 2));
        assert_eq!(a.fish[0].fed_days, 1);
        for d in 2..=4 {
            assert_eq!(feed(&mut a, d), Feed::Fed { grew: vec![] });
        }
        // Ngày thứ 5 được ăn → cả hai con lên cỡ nhỡ.
        let uids: Vec<u32> = a.fish.iter().map(|f| f.uid).collect();
        assert_eq!(feed(&mut a, 5), Feed::Fed { grew: uids });
        // Bỏ đói nhiều ngày không trừ gì, cá vẫn còn.
        assert_eq!(feed(&mut a, 30), Feed::Fed { grew: vec![] });
        assert_eq!(a.fish.len(), 2);
        assert_eq!(a.fish[0].fed_days, 6);
    }

    #[test]
    fn clock_going_back_does_not_feed_again() {
        let mut a = Aquarium::default();
        ensure_starter(&mut a, 0);
        feed(&mut a, 10);
        assert_eq!(feed(&mut a, 9), Feed::Already);
        assert_eq!(a.fish[0].fed_days, 1);
    }

    #[test]
    fn fish_in_storage_are_not_fed() {
        let mut a = Aquarium::default();
        ensure_starter(&mut a, 0);
        let uid = a.fish[0].uid;
        to_storage(&mut a, uid).unwrap();
        feed(&mut a, 1);
        assert_eq!(a.stored[0].fed_days, 0);
        assert_eq!(a.fish[0].fed_days, 1);
    }

    #[test]
    fn add_checks_owner_species_limit_and_tank_size() {
        let mut a = Aquarium::default();
        assert_eq!(add(&mut a, "fish.shark", true, 0), Err(AqError::NotFound));
        assert_eq!(add(&mut a, "fish.koi", false, 0), Err(AqError::NotOwned));
        for _ in 0..MAX_PER_SPECIES {
            add(&mut a, "fish.koi", true, 0).unwrap();
        }
        assert_eq!(add(&mut a, "fish.koi", true, 0), Err(AqError::SpeciesLimit));
        // Vớt ra bể chờ vẫn tính vào giới hạn loài (không thả vô hạn).
        let uid = a.fish[0].uid;
        to_storage(&mut a, uid).unwrap();
        assert_eq!(add(&mut a, "fish.koi", true, 0), Err(AqError::SpeciesLimit));
        for sp in ["fish.neon", "fish.neon", "fish.neon", "fish.guppy", "fish.guppy", "fish.guppy"] {
            add(&mut a, sp, true, 0).unwrap();
        }
        assert_eq!(a.fish.len(), MAX_IN_TANK);
        assert_eq!(add(&mut a, "fish.betta", true, 0), Err(AqError::TankFull));
        assert_eq!(to_tank(&mut a, uid), Err(AqError::TankFull));
        // uid không trùng, kể cả khi next_uid trong file bị lùi.
        let mut uids: Vec<u32> = a.fish.iter().chain(&a.stored).map(|f| f.uid).collect();
        uids.sort();
        uids.dedup();
        assert_eq!(uids.len(), a.fish.len() + a.stored.len());
        a.next_uid = 0;
        let fresh = Aquarium { fish: vec![], ..a.clone() };
        let mut b = fresh;
        let n = add(&mut b, "fish.betta", true, 0).unwrap();
        assert!(a.stored.iter().all(|f| f.uid != n));
    }

    #[test]
    fn storage_round_trip_keeps_name_and_growth() {
        let mut a = Aquarium::default();
        ensure_starter(&mut a, 0);
        let uid = a.fish[0].uid;
        a.fish[0].fed_days = 7;
        rename(&mut a, uid, "  Mập  ").unwrap();
        to_storage(&mut a, uid).unwrap();
        assert_eq!(a.fish.len(), 1);
        rename(&mut a, uid, "Mập Ú").unwrap();
        to_tank(&mut a, uid).unwrap();
        let f = a.fish.iter().find(|f| f.uid == uid).unwrap();
        assert_eq!((f.name.as_str(), f.fed_days), ("Mập Ú", 7));
        assert_eq!(to_storage(&mut a, 999), Err(AqError::NoSuchFish));
        assert_eq!(rename(&mut a, 999, "x"), Err(AqError::NoSuchFish));
    }

    #[test]
    fn decor_slots() {
        let mut a = Aquarium::default();
        assert_eq!(set_decor(&mut a, DECOR_SLOTS, Some("tdeco.castle"), true), Err(AqError::BadSlot));
        assert_eq!(set_decor(&mut a, 0, Some("tdeco.castle"), false), Err(AqError::NotOwned));
        set_decor(&mut a, 0, Some("tdeco.castle"), true).unwrap();
        assert_eq!(a.decor, vec!["tdeco.castle", "", "", ""]);
        // Đặt cùng món sang ô khác: ô cũ trống.
        set_decor(&mut a, 2, Some("tdeco.castle"), true).unwrap();
        assert_eq!(a.decor, vec!["", "", "tdeco.castle", ""]);
        set_decor(&mut a, 2, None, false).unwrap();
        assert!(a.decor.iter().all(|d| d.is_empty()));
        // File sai độ dài vẫn dùng được.
        a.decor = vec!["x".into(); 9];
        set_decor(&mut a, 1, None, false).unwrap();
        assert_eq!(a.decor.len(), DECOR_SLOTS);
    }

    #[test]
    fn view_hides_unknown_species_and_unowned_decor_without_deleting() {
        let mut a = Aquarium::default();
        ensure_starter(&mut a, 10);
        a.fish.push(Fish { uid: 99, species: "fish.removed".into(), born: 10, ..Default::default() });
        a.fish[0].name = "Bé\u{7}Vàng".into();
        a.fish[0].fed_days = 5;
        a.decor = vec!["tdeco.castle".into(), "tdeco.hacked".into()];
        let v = view_json(&a, 12, true, |id| (id == "tdeco.castle").then_some("tdeco.castle"));
        let list = v["fish"].as_array().unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0]["species"], "fish.goldfish");
        assert_eq!(list[0]["name"], "BéVàng");
        assert_eq!(list[0]["stage"], 1);
        assert_eq!(list[0]["age"], 2);
        assert_eq!(v["fedToday"], false);
        assert_eq!(v["hunger"], "peckish", "thả từ ngày 10, chưa ăn lần nào, hôm nay ngày 12");
        assert_eq!(v["decor"], json!(["tdeco.castle", "", "", ""]));
        assert_eq!(a.fish.len(), 3, "cá loài lạ vẫn nằm trong file");
    }

    #[test]
    fn hunger_grows_with_days_unfed_and_never_harms_the_fish() {
        let mut a = Aquarium::default();
        ensure_starter(&mut a, 100);
        // Bể mới, chưa ăn lần nào: tính từ ngày thả.
        assert_eq!(hunger(&a, 100, true), Hunger::Full);
        assert_eq!(hunger(&a, 100 + PECKISH_DAYS, true), Hunger::Peckish);
        assert_eq!(hunger(&a, 100 + HUNGRY_DAYS, true), Hunger::Hungry);
        feed(&mut a, 110);
        assert_eq!(hunger(&a, 110, true), Hunger::Full);
        assert_eq!(hunger(&a, 111, true), Hunger::Full);
        assert_eq!(hunger(&a, 112, true), Hunger::Peckish);
        assert_eq!(hunger(&a, 200, true), Hunger::Hungry);
        // Tắt công tắc: luôn no. Bể trống: không có ai đói.
        assert_eq!(hunger(&a, 200, false), Hunger::Full);
        assert_eq!(hunger(&Aquarium::default(), 200, true), Hunger::Full);
        // Đồng hồ lùi: coi như vừa ăn.
        assert_eq!(hunger(&a, 105, true), Hunger::Full);
        // Đói bao lâu cũng không mất cá, không nhỏ lại; cho ăn là no lại.
        let fed_days = a.fish[0].fed_days;
        assert_eq!(a.fish.len(), 2);
        feed(&mut a, 200);
        assert_eq!(hunger(&a, 200, true), Hunger::Full);
        assert_eq!(a.fish[0].fed_days, fed_days + 1);
    }

    #[test]
    fn nag_at_most_once_a_day_and_only_when_the_tank_is_used() {
        let mut a = Aquarium::default();
        ensure_starter(&mut a, 0);
        let day = HUNGRY_DAYS;
        assert!(should_nag(&a, day, true, true));
        assert!(!should_nag(&a, day, true, false), "không ghim bể / không bật bể full: không nhắc");
        assert!(!should_nag(&a, day, false, true), "tắt công tắc: không nhắc");
        assert!(!should_nag(&a, day - 1, true, true), "mới hơi đói: chưa nhắc");
        a.hungry_said = Some(day);
        assert!(!should_nag(&a, day, true, true), "hôm nay nhắc rồi");
        assert!(should_nag(&a, day + 1, true, true), "hôm sau vẫn đói thì nhắc lại");
        // File cũ chưa có trường mới vẫn đọc được.
        let old: Aquarium = serde_json::from_str(r#"{"fish":[],"last_fed":3}"#).unwrap();
        assert_eq!(old.hungry_said, None);
    }

    #[test]
    fn clean_name_limits_length() {
        assert_eq!(clean_name("  Mập  "), "Mập");
        assert_eq!(clean_name(&"á".repeat(30)).chars().count(), NAME_MAX);
        assert_eq!(clean_name("a\nb\tc"), "abc");
    }

    #[test]
    fn corrupt_or_old_file_reads_as_default() {
        let a: Aquarium = serde_json::from_str("{}").unwrap();
        assert_eq!(a, Aquarium::default());
        // File của Phase 27 (chưa có stored / decor) vẫn đọc được, giữ cá.
        let a: Aquarium = serde_json::from_str(r#"{"fish":[{"uid":1,"species":"fish.koi"}],"starter":true,"extra":1}"#).unwrap();
        assert_eq!(a.fish[0].species, "fish.koi");
        assert!(a.stored.is_empty() && a.decor.is_empty());
    }
}
