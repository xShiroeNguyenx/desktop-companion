// Chữ trên menu khay (tray) theo ngôn ngữ chữ của pet (vi / ja / en). Đổi
// ngôn ngữ thì `relabel` viết lại mọi mục; mục có trạng thái (Đừng làm phiền,
// Tập trung, hiệu ứng thời tiết) tự lấy chữ ở đây mỗi lần đổi trạng thái.

use std::sync::Mutex;

use tauri::menu::MenuItem;
use tauri::{AppHandle, Wry};

use crate::persona::Lang;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tray {
    ToggleShow,
    Tasks,
    Notes,
    Settings,
    ClickThrough,
    DndOff,
    DndOn,
    FocusStart,
    FocusStop,
    WeatherOn,
    WeatherOff,
    Forecast,
    Calendar,
    Sysmon,
    Monitor,
    Quit,
}

impl Tray {
    #[cfg(test)]
    const ALL: [Tray; 16] = [
        Tray::ToggleShow,
        Tray::Tasks,
        Tray::Notes,
        Tray::Settings,
        Tray::ClickThrough,
        Tray::DndOff,
        Tray::DndOn,
        Tray::FocusStart,
        Tray::FocusStop,
        Tray::WeatherOn,
        Tray::WeatherOff,
        Tray::Forecast,
        Tray::Calendar,
        Tray::Sysmon,
        Tray::Monitor,
        Tray::Quit,
    ];
}

pub fn text(t: Tray, lang: Lang) -> &'static str {
    use Lang::*;
    use Tray::*;
    match (t, lang) {
        (ToggleShow, Vi) => "Hiện / Ẩn pet",
        (ToggleShow, Ja) => "ペットを表示 / 非表示",
        (ToggleShow, En) => "Show / hide the pet",
        (Tasks, Vi) => "Công việc (Tasks)",
        (Tasks, Ja) => "タスク",
        (Tasks, En) => "Tasks",
        (Notes, Vi) => "🗒️ Ghi chú",
        (Notes, Ja) => "🗒️ メモ",
        (Notes, En) => "🗒️ Notes",
        (Settings, Vi) => "Cài đặt",
        (Settings, Ja) => "設定",
        (Settings, En) => "Settings",
        (ClickThrough, Vi) => "Bật / Tắt click-through",
        (ClickThrough, Ja) => "クリック透過 オン / オフ",
        (ClickThrough, En) => "Click-through on / off",
        (DndOff, Vi) => "🔕 Đừng làm phiền (1 giờ)",
        (DndOff, Ja) => "🔕 おやすみモード（1時間）",
        (DndOff, En) => "🔕 Do not disturb (1 hour)",
        (DndOn, Vi) => "🔔 Bật lại lời nhắc",
        (DndOn, Ja) => "🔔 お知らせを再開",
        (DndOn, En) => "🔔 Turn reminders back on",
        (FocusStart, Vi) => "🍅 Bắt đầu tập trung",
        (FocusStart, Ja) => "🍅 集中を始める",
        (FocusStart, En) => "🍅 Start focusing",
        (FocusStop, Vi) => "⏹ Dừng tập trung",
        (FocusStop, Ja) => "⏹ 集中をやめる",
        (FocusStop, En) => "⏹ Stop focusing",
        (WeatherOn, Vi) => "🌦 Bật hiệu ứng thời tiết",
        (WeatherOn, Ja) => "🌦 天気エフェクトをオン",
        (WeatherOn, En) => "🌦 Turn on weather effects",
        (WeatherOff, Vi) => "🌦 Tắt hiệu ứng thời tiết",
        (WeatherOff, Ja) => "🌦 天気エフェクトをオフ",
        (WeatherOff, En) => "🌦 Turn off weather effects",
        (Forecast, Vi) => "☁️ Dự báo thời tiết",
        (Forecast, Ja) => "☁️ 天気予報",
        (Forecast, En) => "☁️ Weather forecast",
        (Calendar, Vi) => "📅 Ghim / gỡ lịch tháng",
        (Calendar, Ja) => "📅 月カレンダーを固定 / 解除",
        (Calendar, En) => "📅 Pin / unpin the calendar",
        (Sysmon, Vi) => "📊 Ghim / gỡ thông số máy",
        (Sysmon, Ja) => "📊 システム情報を固定 / 解除",
        (Sysmon, En) => "📊 Pin / unpin system stats",
        (Monitor, Vi) => "🛰 Ghim / gỡ theo dõi website",
        (Monitor, Ja) => "🛰 サイト監視を固定 / 解除",
        (Monitor, En) => "🛰 Pin / unpin website monitor",
        (Quit, Vi) => "Thoát",
        (Quit, Ja) => "終了",
        (Quit, En) => "Quit",
    }
}

/// Ngôn ngữ đang dùng (Settings.message_language).
pub fn lang(app: &AppHandle) -> Lang {
    Lang::from_code(&crate::store::load_settings(app).message_language)
}

/// Mục cố định chữ (không đổi theo trạng thái), để viết lại khi đổi ngôn ngữ.
static FIXED: Mutex<Vec<(Tray, MenuItem<Wry>)>> = Mutex::new(Vec::new());

pub fn remember(t: Tray, item: &MenuItem<Wry>) {
    FIXED.lock().unwrap_or_else(|e| e.into_inner()).push((t, item.clone()));
}

/// Đổi ngôn ngữ: viết lại mục cố định, rồi các mục có trạng thái.
pub fn relabel(app: &AppHandle) {
    let l = lang(app);
    for (t, item) in FIXED.lock().unwrap_or_else(|e| e.into_inner()).iter() {
        let _ = item.set_text(text(*t, l));
    }
    crate::companion::sync_tray_labels(app);
    crate::fx::refresh_tray(app);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_entry_has_all_languages() {
        for t in Tray::ALL {
            for l in Lang::ALL {
                assert!(!text(t, l).trim().is_empty(), "{t:?}/{l:?}");
            }
            assert_ne!(text(t, Lang::Ja), text(t, Lang::Vi), "{t:?}: chưa dịch tiếng Nhật");
        }
    }
}
