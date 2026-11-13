// Widget 🐠 Bể cá ghim trên desktop (cửa sổ "pin-aquarium", aquarium-pin.html).
// Cửa sổ, ghim, nhớ vị trí: pin_widget.rs. Cá, cho ăn: aquarium.rs. Vẽ và bơi:
// app/js/aquarium-pin.js + aquarium-core.js.
//
// GPU: cửa sổ KHÔNG dựng sẵn (không nằm trong windows::precreate); gỡ ghim thì
// pin_widget::set_pinned huỷ hẳn cửa sổ, nên không còn vòng vẽ nào.
// Vòng nền ở đây chỉ hỏi trạng thái máy khi cửa sổ đang hiện, để trang dừng vẽ
// lúc có app toàn màn hình / người dùng rời máy, và báo khi sang ngày mới.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};
use tauri::{AppHandle, Emitter, Manager};

use crate::collection::Category;
use crate::pin_widget::{PinState, Spec};
use crate::store::{self, Settings};

pub const LABEL: &str = "pin-aquarium";
const INTERVAL_SECS: u64 = 2;

/// Cỡ cửa sổ (logical px). ĐỒNG BỘ với aquarium-pin.html.
pub fn size(s: &Settings) -> (f64, f64) {
    (crate::pin_widget::WIDTH, if s.aquarium_big { 280.0 } else { 170.0 })
}

fn state(s: &Settings) -> PinState {
    s.aquarium_pin
}

fn set_state(s: &mut Settings, st: PinState) {
    s.aquarium_pin = st;
}

pub static SPEC: Spec = Spec {
    label: LABEL,
    html: "aquarium-pin.html",
    title: "Bể cá",
    slot: 6,
    size,
    state,
    set_state,
    broadcast,
};

fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

fn today() -> crate::bond::Day {
    crate::companion::today_and_hour(now_secs()).0
}

/// Trang dừng vẽ khi: app khác toàn màn hình, màn hình khoá, người dùng rời máy.
pub fn should_pause(snap: &crate::awareness::Snapshot) -> bool {
    snap.fullscreen || snap.away()
}

pub fn view(app: &AppHandle) -> Value {
    let s = store::load_settings(app);
    let a = crate::persona::address(&s);
    let c = store::load_collection(app);
    let day = today();
    // Lần đầu mở bể: tặng 2 cá vàng (ghi file một lần).
    let aq = store::update_aquarium(app, |aq| {
        crate::aquarium::ensure_starter(aq, day);
        aq.clone()
    })
    .unwrap_or_else(|e| {
        eprintln!("[aquarium] ghi aquarium.json lỗi: {e}");
        store::load_aquarium(app)
    });
    let owned_fish: Vec<&str> = crate::aquarium::SPECIES
        .iter()
        .copied()
        .filter(|sp| crate::collection::owned_aquarium_item(&c, Category::Fish, sp).is_some())
        .collect();
    let owned_decor: Vec<&str> = crate::collection::items()
        .filter(|it| it.cat == Category::TankDeco && crate::collection::owns(&c, it))
        .map(|it| it.id)
        .collect();
    json!({
        "pinTop": s.aquarium_pin.top,
        "big": s.aquarium_big,
        "full": s.aquarium_full,
        "lang": a.lang.code(),
        "skin": crate::collection::effective_skin(&c),
        "deco": crate::collection::effective_widget_deco(&c),
        "bg": crate::collection::effective_tank_bg(&c),
        "tank": crate::aquarium::view_json(&aq, day, s.fish_hunger, |id| {
            crate::collection::owned_aquarium_item(&c, Category::TankDeco, id)
        }),
        // Cửa sổ quản lý: loài đã mua (thả thêm), vật trang trí đã mua (đặt vào ô),
        // tên mọi món bể cá theo ngôn ngữ (lấy từ catalog, không lặp lại ở trang).
        "ownedFish": owned_fish,
        "ownedDecor": owned_decor,
        "names": crate::collection::items()
            .filter(|it| it.cat.aquarium())
            .map(|it| (it.id.to_string(), json!(it.name[crate::collection::lang_index(a.lang)])))
            .collect::<serde_json::Map<String, Value>>(),
        "paused": should_pause(&crate::awareness::snapshot()),
    })
}

/// Gửi lại cho widget và cửa sổ quản lý (cái nào đang mở).
pub fn broadcast(app: &AppHandle) {
    let open: Vec<&str> = [LABEL, MANAGE_LABEL, crate::aquarium_full::LABEL]
        .into_iter()
        .filter(|l| app.get_webview_window(l).is_some())
        .collect();
    if open.is_empty() {
        return;
    }
    let v = view(app);
    for l in open {
        let _ = app.emit_to(l, "aquarium-data", v.clone());
    }
}

/// Câu do người dùng vừa bấm: nói ngay, không qua trọng tài (như collection_buy).
fn say(app: &AppHandle, key: crate::lines::Key, vars: &[(&str, &str)], expression: &str) {
    let a = crate::persona::address(&store::load_settings(app));
    let line = crate::lines::line(key, &a, vars);
    let _ = app.emit_to("main", "pet-message", json!({ "command": "setExpression", "expression": expression, "duration": 6000 }));
    let _ = app.emit_to("main", "pet-message", json!({ "command": "showMessage", "text": line }));
    crate::tts::say(app, &line);
}

/// Tên hiển thị của một con: tên người dùng đặt, không thì tên loài theo ngôn ngữ.
fn fish_name(app: &AppHandle, f: &crate::aquarium::Fish) -> String {
    let name = crate::aquarium::clean_name(&f.name);
    if !name.is_empty() {
        return name;
    }
    let lang = crate::persona::address(&store::load_settings(app)).lang;
    crate::collection::item(&f.species)
        .map(|it| it.name[crate::collection::lang_index(lang)].to_string())
        .unwrap_or_default()
}

/// Cho ăn. Trả { fed: lần đầu trong ngày, grew: số con vừa lên cỡ }. Lần đầu
/// trong ngày Miu nói một câu (cá lớn thì khen con đó, không thì câu cho ăn).
pub fn feed(app: &AppHandle) -> Result<Value, String> {
    let day = today();
    let (out, aq) = store::update_aquarium(app, |aq| (crate::aquarium::feed(aq, day), aq.clone()))?;
    let v = match &out {
        crate::aquarium::Feed::Fed { grew } => {
            crate::collection::quest_fish_fed(app);
            match grew.first().and_then(|uid| aq.fish.iter().find(|f| f.uid == *uid)) {
                Some(f) => say(app, crate::lines::Key::FishGrown, &[("fish", &fish_name(app, f))], "happy"),
                None => say(app, crate::lines::Key::FishFed, &[], "happy"),
            }
            json!({ "fed": true, "grew": grew.len() })
        }
        crate::aquarium::Feed::Already => json!({ "fed": false, "grew": 0 }),
    };
    broadcast(app);
    // Bể full không bấm được (nằm dưới icon desktop): cho ăn từ chỗ khác thì thả
    // thức ăn ở đó luôn.
    let _ = app.emit_to(crate::aquarium_full::LABEL, "aquarium-food", ());
    Ok(v)
}

/// Sửa bể bằng hàm `f` (thả / vớt / đổi tên / trang trí), lỗi trả mã cho trang.
pub fn edit(
    app: &AppHandle,
    f: impl FnOnce(&mut crate::aquarium::Aquarium, &crate::collection::Collection, crate::bond::Day) -> Result<(), crate::aquarium::AqError>,
) -> Result<Value, String> {
    let c = store::load_collection(app);
    let day = today();
    store::update_aquarium(app, |aq| f(aq, &c, day))?.map_err(|e| e.code().to_string())?;
    broadcast(app);
    Ok(view(app))
}

/// Cửa sổ quản lý "🐠 Bể cá" (aquarium.html). Dựng lười, đóng là huỷ (không giữ GPU).
pub const MANAGE_LABEL: &str = "aquarium";

/// Đổi cỡ nhỏ / lớn: lưu Settings, đổi cỡ cửa sổ, gửi lại dữ liệu.
pub fn set_big(app: &AppHandle, big: bool) -> Result<(), String> {
    let mut s = store::load_settings(app);
    s.aquarium_big = big;
    store::save_settings(app, &s)?;
    crate::pin_widget::resize(app, &SPEC);
    broadcast(app);
    Ok(())
}

/// Vòng nền. Gọi một lần lúc setup. Không có cửa sổ bể thì chỉ ngủ.
pub fn start(app: AppHandle) {
    std::thread::spawn(move || {
        let mut last: Option<(bool, crate::bond::Day)> = None;
        loop {
            let visible = app
                .get_webview_window(LABEL)
                .map(|w| w.is_visible().unwrap_or(false))
                .unwrap_or(false);
            if visible {
                let paused = should_pause(&crate::awareness::snapshot());
                let day = today();
                match last {
                    Some((p, d)) if d != day => {
                        // Sang ngày mới: "đã cho ăn hôm nay" đổi, gửi lại cả bể.
                        broadcast(&app);
                        if p != paused {
                            let _ = app.emit_to(LABEL, "aquarium-pause", paused);
                        }
                    }
                    Some((p, _)) if p == paused => {}
                    _ => {
                        let _ = app.emit_to(LABEL, "aquarium-pause", paused);
                    }
                }
                last = Some((paused, day));
            } else {
                last = None;
            }
            std::thread::sleep(Duration::from_secs(INTERVAL_SECS));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::awareness::Snapshot;

    #[test]
    fn size_matches_shared_width_and_toggles_height() {
        let mut s = Settings::default();
        assert_eq!(size(&s), (crate::pin_widget::WIDTH, 170.0));
        s.aquarium_big = true;
        assert_eq!(size(&s), (crate::pin_widget::WIDTH, 280.0));
    }

    #[test]
    fn pauses_when_fullscreen_locked_or_away() {
        assert!(!should_pause(&Snapshot::default()));
        assert!(should_pause(&Snapshot { fullscreen: true, ..Default::default() }));
        assert!(should_pause(&Snapshot { locked: true, ..Default::default() }));
        assert!(should_pause(&Snapshot { idle_secs: crate::awareness::AWAY_SECS, ..Default::default() }));
    }

    #[test]
    fn not_precreated_and_not_a_task_note() {
        assert_eq!(crate::pins::id_from_label(LABEL), None);
        let src = include_str!("windows.rs");
        assert!(!src.contains("aquarium-pin.html"), "bể cá không được dựng sẵn (tốn GPU khi chưa ghim)");
    }
}
