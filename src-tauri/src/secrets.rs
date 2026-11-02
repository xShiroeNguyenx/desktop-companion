// API key AI nằm trong két của hệ điều hành (Windows Credential Manager /
// Keychain macOS), không nằm trong settings.json (PLAN.md 15.1 A2).
//
// - `Settings.api_key` / `gemini_key` chỉ có trong RAM: store::load_settings điền
//   từ đây, serde không ghi chúng ra file và settings_get không gửi chúng cho UI
//   (UI chỉ nhận cờ has_api_key / has_gemini_key).
// - Bản cũ để key dạng chữ trong settings.json: lần đọc đầu chuyển sang két, đọc
//   lại cho chắc két giữ thật, rồi store::migrate_secrets ghi lại file không còn key.
//   Két không nhận thì để key ở chỗ cũ (save_settings ghi lại), không làm mất key.

use std::collections::HashMap;
use std::sync::Mutex;

/// Tên dịch vụ trong két ("Generic credential" trên Windows).
const SERVICE: &str = "desktop-companion";

/// (trường trong Settings, tên mục trong két).
pub const FIELDS: [(&str, &str); 2] = [("api_key", "anthropic_api_key"), ("gemini_key", "gemini_api_key")];

/// Kho bí mật; bản thật là két hệ điều hành, test dùng kho trong RAM.
pub trait Vault {
    fn get(&self, name: &str) -> Result<Option<String>, String>;
    fn set(&self, name: &str, value: &str) -> Result<(), String>;
    fn delete(&self, name: &str) -> Result<(), String>;
}

struct OsVault;

impl Vault for OsVault {
    fn get(&self, name: &str) -> Result<Option<String>, String> {
        let entry = keyring::Entry::new(SERVICE, name).map_err(|e| e.to_string())?;
        match entry.get_password() {
            Ok(v) => Ok(Some(v)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }
    fn set(&self, name: &str, value: &str) -> Result<(), String> {
        let entry = keyring::Entry::new(SERVICE, name).map_err(|e| e.to_string())?;
        entry.set_password(value).map_err(|e| e.to_string())
    }
    fn delete(&self, name: &str) -> Result<(), String> {
        let entry = keyring::Entry::new(SERVICE, name).map_err(|e| e.to_string())?;
        match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    }
}

/// Một key đã đọc. `plaintext` = két không nhận, key vẫn phải nằm trong settings.json.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Slot {
    pub value: String,
    pub plaintext: bool,
}

/// Ghi rồi đọc lại: két có thể báo thành công mà không giữ (vd kho giả của keyring).
fn put(vault: &dyn Vault, name: &str, value: &str) -> Result<(), String> {
    vault.set(name, value)?;
    match vault.get(name)? {
        Some(v) if v == value => Ok(()),
        _ => Err("két báo đã lưu nhưng đọc lại không thấy key".into()),
    }
}

/// Đọc một key. `legacy` = giá trị dạng chữ còn trong settings.json (bản cũ):
/// có thì chuyển sang két; két lỗi thì giữ nó ở chỗ cũ.
pub fn resolve(vault: &dyn Vault, name: &str, legacy: &str) -> Slot {
    let legacy = legacy.trim();
    if !legacy.is_empty() {
        return match put(vault, name, legacy) {
            Ok(()) => Slot { value: legacy.to_string(), plaintext: false },
            Err(e) => {
                eprintln!("[secrets] chưa chuyển được {name} vào két, giữ trong settings.json: {e}");
                Slot { value: legacy.to_string(), plaintext: true }
            }
        };
    }
    match vault.get(name) {
        Ok(v) => Slot { value: v.unwrap_or_default(), plaintext: false },
        Err(e) => {
            eprintln!("[secrets] không đọc được {name} từ két: {e}");
            Slot::default()
        }
    }
}

/// Lưu key người dùng vừa nhập; rỗng = xoá. Không có đường lùi về settings.json:
/// két lỗi thì báo lỗi cho người dùng.
pub fn write(vault: &dyn Vault, name: &str, value: &str) -> Result<Slot, String> {
    let value = value.trim();
    if value.is_empty() {
        vault.delete(name)?;
        if vault.get(name)?.is_some() {
            return Err("két báo đã xoá nhưng key vẫn còn".into());
        }
    } else {
        put(vault, name, value)?;
    }
    Ok(Slot { value: value.to_string(), plaintext: false })
}

// load_settings chạy rất thường (vòng nhắc nhở, chat…): đọc két một lần, giữ ở
// đây. Chỉ app này ghi vào các mục này nên bản giữ luôn khớp két.
static CACHE: Mutex<Option<HashMap<&'static str, Slot>>> = Mutex::new(None);

/// Key theo trường Settings. `legacy(field)` = giá trị dạng chữ đọc được từ file.
pub fn get(field: &str, legacy: &str) -> Slot {
    let mut cache = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    let map = cache.get_or_insert_with(HashMap::new);
    let Some(&(f, name)) = FIELDS.iter().find(|(f, _)| *f == field) else {
        return Slot::default();
    };
    match map.get(f) {
        // Key dạng chữ mới xuất hiện trong file (người dùng sửa tay) thì chuyển lại.
        Some(slot) if legacy.trim().is_empty() || legacy.trim() == slot.value => slot.clone(),
        _ => {
            let slot = resolve(&OsVault, name, legacy);
            map.insert(f, slot.clone());
            slot
        }
    }
}

/// Lưu key theo trường Settings (settings_set); rỗng = xoá.
pub fn set(field: &str, value: &str) -> Result<(), String> {
    let &(f, name) = FIELDS.iter().find(|(f, _)| *f == field).ok_or("trường key lạ")?;
    let slot = write(&OsVault, name, value).map_err(|e| format!("Không lưu được API key vào két của hệ điều hành: {e}"))?;
    let mut cache = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    cache.get_or_insert_with(HashMap::new).insert(f, slot);
    Ok(())
}

/// Key két không nhận, phải ghi lại vào settings.json để khỏi mất.
pub fn plaintext_fallback() -> Vec<(&'static str, String)> {
    let cache = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    let Some(map) = cache.as_ref() else { return Vec::new() };
    FIELDS
        .iter()
        .filter_map(|(f, _)| map.get(f).filter(|s| s.plaintext).map(|s| (*f, s.value.clone())))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    #[derive(Default)]
    struct MemVault {
        items: RefCell<HashMap<String, String>>,
        /// Ghi báo thành công nhưng không giữ gì.
        forgetful: bool,
        broken: bool,
    }

    impl Vault for MemVault {
        fn get(&self, name: &str) -> Result<Option<String>, String> {
            if self.broken {
                return Err("hỏng".into());
            }
            Ok(self.items.borrow().get(name).cloned())
        }
        fn set(&self, name: &str, value: &str) -> Result<(), String> {
            if self.broken {
                return Err("hỏng".into());
            }
            if !self.forgetful {
                self.items.borrow_mut().insert(name.into(), value.into());
            }
            Ok(())
        }
        fn delete(&self, name: &str) -> Result<(), String> {
            if self.broken {
                return Err("hỏng".into());
            }
            self.items.borrow_mut().remove(name);
            Ok(())
        }
    }

    #[test]
    fn legacy_key_moves_into_vault() {
        let v = MemVault::default();
        let s = resolve(&v, "k", " sk-ant-x ");
        assert_eq!(s, Slot { value: "sk-ant-x".into(), plaintext: false });
        assert_eq!(v.get("k").unwrap().as_deref(), Some("sk-ant-x"));
    }

    #[test]
    fn legacy_key_stays_in_file_when_vault_fails() {
        let s = resolve(&MemVault { broken: true, ..Default::default() }, "k", "sk-ant-x");
        assert_eq!(s, Slot { value: "sk-ant-x".into(), plaintext: true });
        let s = resolve(&MemVault { forgetful: true, ..Default::default() }, "k", "sk-ant-x");
        assert!(s.plaintext, "két báo thành công mà không giữ thì vẫn giữ key trong file");
    }

    #[test]
    fn no_legacy_reads_vault() {
        let v = MemVault::default();
        v.set("k", "AIza").unwrap();
        assert_eq!(resolve(&v, "k", "").value, "AIza");
        assert_eq!(resolve(&MemVault::default(), "k", ""), Slot::default());
        assert_eq!(resolve(&MemVault { broken: true, ..Default::default() }, "k", ""), Slot::default());
    }

    /// Két thật của máy đang chạy (bật feature nền tảng chưa): ghi, đọc lại, xoá
    /// một mục thử. `cargo test os_vault -- --ignored`.
    #[test]
    #[ignore]
    fn os_vault_round_trip() {
        let name = "test_probe";
        assert_eq!(write(&OsVault, name, "probe-value").unwrap().value, "probe-value");
        assert_eq!(OsVault.get(name).unwrap().as_deref(), Some("probe-value"));
        write(&OsVault, name, "").unwrap();
        assert_eq!(OsVault.get(name).unwrap(), None);
    }

    #[test]
    fn write_saves_clears_and_reports_errors() {
        let v = MemVault::default();
        assert_eq!(write(&v, "k", " new ").unwrap().value, "new");
        assert_eq!(v.get("k").unwrap().as_deref(), Some("new"));
        assert_eq!(write(&v, "k", "  ").unwrap().value, "");
        assert_eq!(v.get("k").unwrap(), None);
        assert!(write(&MemVault { broken: true, ..Default::default() }, "k", "x").is_err());
        assert!(write(&MemVault { forgetful: true, ..Default::default() }, "k", "x").is_err());
    }
}
