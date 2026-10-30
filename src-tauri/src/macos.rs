// Phần riêng macOS của những tính năng mà bản Windows dùng Win32: chuột / phím
// toàn cục (bắt chữ bôi đen), idle / khoá màn hình / toàn màn hình, CPU, RAM,
// ổ đĩa, mạng, pin, thời gian chạy, danh sách tiến trình, giọng có sẵn (`say`).
//
// FFI viết thẳng (CoreGraphics, CoreFoundation, IOKit, ApplicationServices,
// mach qua libc) để không kéo thêm crate. Mọi đối tượng CF tạo ra ("Create" /
// "Copy") đều được CFRelease qua `Cf`; đối tượng "Get" thì không.
//
// Toạ độ: CoreGraphics trả điểm (point) toàn cục, gốc góc trên-trái màn hình
// chính. Phần còn lại của app (như bản Windows) dùng pixel vật lý của Tauri, nên
// `to_physical` nhân với tỉ lệ của màn hình chứa điểm đó.

#![allow(non_upper_case_globals)]
// libc đánh dấu các hàm mach (mach_host_self, mach_timebase_info…) là deprecated
// để khuyên dùng crate mach2; hàm vẫn là API hệ thống ổn định, đủ dùng ở đây.
#![allow(deprecated)]

use std::ffi::{c_char, c_void, CStr, CString};
use std::sync::OnceLock;

type CFTypeRef = *const c_void;
type CFIndex = isize;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct CGPoint {
    x: f64,
    y: f64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct CGSize {
    width: f64,
    height: f64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct CGRect {
    origin: CGPoint,
    size: CGSize,
}

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    static kCFBooleanTrue: CFTypeRef;
    static kCFTypeDictionaryKeyCallBacks: [usize; 6];
    static kCFTypeDictionaryValueCallBacks: [usize; 5];
    static kCFURLVolumeLocalizedNameKey: CFTypeRef;
    fn CFRelease(cf: CFTypeRef);
    fn CFGetTypeID(cf: CFTypeRef) -> usize;
    fn CFStringGetTypeID() -> usize;
    fn CFNumberGetTypeID() -> usize;
    fn CFBooleanGetTypeID() -> usize;
    fn CFDictionaryGetTypeID() -> usize;
    fn CFArrayGetTypeID() -> usize;
    fn CFStringCreateWithBytes(alloc: CFTypeRef, bytes: *const u8, len: CFIndex, encoding: u32, external: u8) -> CFTypeRef;
    fn CFStringGetCString(s: CFTypeRef, buf: *mut c_char, size: CFIndex, encoding: u32) -> u8;
    fn CFNumberGetValue(n: CFTypeRef, ty: CFIndex, out: *mut c_void) -> u8;
    fn CFBooleanGetValue(b: CFTypeRef) -> u8;
    fn CFDictionaryGetValue(d: CFTypeRef, key: CFTypeRef) -> CFTypeRef;
    fn CFDictionaryCreate(
        alloc: CFTypeRef,
        keys: *const CFTypeRef,
        values: *const CFTypeRef,
        n: CFIndex,
        key_cb: *const c_void,
        value_cb: *const c_void,
    ) -> CFTypeRef;
    fn CFArrayGetCount(a: CFTypeRef) -> CFIndex;
    fn CFArrayGetValueAtIndex(a: CFTypeRef, i: CFIndex) -> CFTypeRef;
    fn CFURLCreateFromFileSystemRepresentation(alloc: CFTypeRef, path: *const u8, len: CFIndex, is_dir: u8) -> CFTypeRef;
    fn CFURLCopyResourcePropertyForKey(url: CFTypeRef, key: CFTypeRef, out: *mut CFTypeRef, err: *mut CFTypeRef) -> u8;
}

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGEventCreate(source: CFTypeRef) -> CFTypeRef;
    fn CGEventGetLocation(event: CFTypeRef) -> CGPoint;
    fn CGEventSourceCreate(state: i32) -> CFTypeRef;
    fn CGEventSourceButtonState(state: i32, button: u32) -> bool;
    fn CGEventSourceSecondsSinceLastEventType(state: i32, event_type: u32) -> f64;
    fn CGEventCreateKeyboardEvent(source: CFTypeRef, keycode: u16, keydown: bool) -> CFTypeRef;
    fn CGEventSetFlags(event: CFTypeRef, flags: u64);
    fn CGEventPost(tap: u32, event: CFTypeRef);
    fn CGGetDisplaysWithPoint(point: CGPoint, max: u32, displays: *mut u32, count: *mut u32) -> i32;
    fn CGGetActiveDisplayList(max: u32, displays: *mut u32, count: *mut u32) -> i32;
    fn CGDisplayBounds(display: u32) -> CGRect;
    fn CGDisplayCopyDisplayMode(display: u32) -> CFTypeRef;
    fn CGDisplayModeGetPixelWidth(mode: CFTypeRef) -> usize;
    fn CGDisplayModeGetWidth(mode: CFTypeRef) -> usize;
    fn CGDisplayModeRelease(mode: CFTypeRef);
    fn CGWindowListCopyWindowInfo(option: u32, relative_to: u32) -> CFTypeRef;
    fn CGSessionCopyCurrentDictionary() -> CFTypeRef;
}

#[link(name = "IOKit", kind = "framework")]
extern "C" {
    fn IOPSCopyPowerSourcesInfo() -> CFTypeRef;
    fn IOPSCopyPowerSourcesList(blob: CFTypeRef) -> CFTypeRef;
    fn IOPSGetPowerSourceDescription(blob: CFTypeRef, ps: CFTypeRef) -> CFTypeRef;
}

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    static kAXTrustedCheckOptionPrompt: CFTypeRef;
    fn AXIsProcessTrusted() -> u8;
    fn AXIsProcessTrustedWithOptions(options: CFTypeRef) -> u8;
}

const UTF8: u32 = 0x0800_0100;
const NUMBER_SINT64: CFIndex = 4;
const NUMBER_FLOAT64: CFIndex = 6;

// CGEventSourceStateID
const STATE_PRIVATE: i32 = -1;
const STATE_HID: i32 = 1;
const ANY_INPUT_EVENT: u32 = !0;
const MOUSE_BUTTON_LEFT: u32 = 0;
const HID_EVENT_TAP: u32 = 0;
const FLAG_COMMAND: u64 = 0x0010_0000;
const KEY_C: u16 = 8; // kVK_ANSI_C

const WINDOW_LIST_ON_SCREEN_ONLY: u32 = 1;
const WINDOW_LIST_EXCLUDE_DESKTOP: u32 = 16;

// ─── CoreFoundation: tiện ích ────────────────────────────────────────────────

/// Đối tượng CF mình sở hữu (từ hàm Create / Copy): tự CFRelease.
struct Cf(CFTypeRef);

impl Cf {
    fn new(p: CFTypeRef) -> Option<Cf> {
        (!p.is_null()).then_some(Cf(p))
    }
}

impl Drop for Cf {
    fn drop(&mut self) {
        // SAFETY: con trỏ khác null, lấy từ hàm Create / Copy nên ta giữ một tham chiếu.
        unsafe { CFRelease(self.0) }
    }
}

fn cfstr(s: &str) -> Option<Cf> {
    // SAFETY: bytes hợp lệ trong suốt lời gọi; CF sao chép chúng.
    Cf::new(unsafe { CFStringCreateWithBytes(std::ptr::null(), s.as_ptr(), s.len() as CFIndex, UTF8, 0) })
}

fn is_type(v: CFTypeRef, id: usize) -> bool {
    // SAFETY: v là đối tượng CF hợp lệ (khác null).
    !v.is_null() && unsafe { CFGetTypeID(v) } == id
}

/// Giá trị của `key` trong dictionary (quy tắc "Get": không sở hữu).
fn dict_get(d: CFTypeRef, key: &str) -> CFTypeRef {
    // SAFETY: chỉ đọc; kiểm kiểu trước khi coi d là dictionary.
    unsafe {
        if !is_type(d, CFDictionaryGetTypeID()) {
            return std::ptr::null();
        }
        match cfstr(key) {
            Some(k) => CFDictionaryGetValue(d, k.0),
            None => std::ptr::null(),
        }
    }
}

fn num_i64(v: CFTypeRef) -> Option<i64> {
    let mut out = 0i64;
    // SAFETY: kiểm kiểu CFNumber trước; out đủ chỗ cho SInt64.
    unsafe { (is_type(v, CFNumberGetTypeID()) && CFNumberGetValue(v, NUMBER_SINT64, &mut out as *mut _ as *mut c_void) != 0).then_some(out) }
}

fn num_f64(v: CFTypeRef) -> Option<f64> {
    let mut out = 0f64;
    // SAFETY: như num_i64.
    unsafe { (is_type(v, CFNumberGetTypeID()) && CFNumberGetValue(v, NUMBER_FLOAT64, &mut out as *mut _ as *mut c_void) != 0).then_some(out) }
}

fn cf_bool(v: CFTypeRef) -> Option<bool> {
    // SAFETY: kiểm kiểu CFBoolean trước.
    unsafe { is_type(v, CFBooleanGetTypeID()).then(|| CFBooleanGetValue(v) != 0) }
}

fn cf_string(v: CFTypeRef) -> Option<String> {
    // SAFETY: kiểm kiểu CFString; bộ đệm đủ lớn cho tên ngắn, CF tự kết thúc bằng 0.
    unsafe {
        if !is_type(v, CFStringGetTypeID()) {
            return None;
        }
        let mut buf = [0 as c_char; 512];
        (CFStringGetCString(v, buf.as_mut_ptr(), buf.len() as CFIndex, UTF8) != 0)
            .then(|| CStr::from_ptr(buf.as_ptr()).to_string_lossy().into_owned())
    }
}

fn array_items(a: CFTypeRef) -> Vec<CFTypeRef> {
    // SAFETY: kiểm kiểu CFArray; chỉ đọc trong phạm vi count.
    unsafe {
        if !is_type(a, CFArrayGetTypeID()) {
            return Vec::new();
        }
        (0..CFArrayGetCount(a)).map(|i| CFArrayGetValueAtIndex(a, i)).collect()
    }
}

// ─── Màn hình, chuột, phím ───────────────────────────────────────────────────

/// Toạ độ con trỏ (point).
fn cursor_points() -> Option<CGPoint> {
    // SAFETY: CGEventCreate(NULL) tạo event rỗng mang vị trí chuột hiện tại.
    let ev = Cf::new(unsafe { CGEventCreate(std::ptr::null()) })?;
    Some(unsafe { CGEventGetLocation(ev.0) })
}

/// Tỉ lệ pixel / point của màn hình chứa điểm `p` (Retina = 2).
fn scale_at(p: CGPoint) -> f64 {
    let mut id = 0u32;
    let mut n = 0u32;
    // SAFETY: ghi tối đa 1 id vào biến cục bộ; mode được release ngay.
    unsafe {
        if CGGetDisplaysWithPoint(p, 1, &mut id, &mut n) != 0 || n == 0 {
            return 1.0;
        }
        let mode = CGDisplayCopyDisplayMode(id);
        if mode.is_null() {
            return 1.0;
        }
        let (px, pt) = (CGDisplayModeGetPixelWidth(mode), CGDisplayModeGetWidth(mode));
        CGDisplayModeRelease(mode);
        if pt == 0 {
            1.0
        } else {
            px as f64 / pt as f64
        }
    }
}

fn to_physical(p: CGPoint) -> (i32, i32) {
    let s = scale_at(p);
    ((p.x * s).round() as i32, (p.y * s).round() as i32)
}

/// Vị trí con trỏ theo pixel vật lý (cùng hệ với Tauri / GetCursorPos).
pub fn cursor_pos() -> (i32, i32) {
    cursor_points().map(to_physical).unwrap_or((0, 0))
}

/// Nút trái đang được giữ.
pub fn left_button_down() -> bool {
    // SAFETY: chỉ đọc trạng thái nút chuột.
    unsafe { CGEventSourceButtonState(STATE_HID, MOUSE_BUTTON_LEFT) }
}

/// Đã được cấp quyền Trợ năng (Accessibility). Không có quyền này thì phím giả
/// (Cmd+C) bị macOS bỏ đi mà KHÔNG báo lỗi, nên phải hỏi trước.
/// `prompt` = true: macOS hiện hộp thoại mời mở Cài đặt (chỉ lần đầu).
pub fn accessibility_trusted(prompt: bool) -> bool {
    // SAFETY: dictionary 1 cặp khoá/giá trị tạo bằng callback chuẩn, release sau khi dùng.
    unsafe {
        if !prompt {
            return AXIsProcessTrusted() != 0;
        }
        let keys = [kAXTrustedCheckOptionPrompt];
        let values = [kCFBooleanTrue];
        let opts = CFDictionaryCreate(
            std::ptr::null(),
            keys.as_ptr(),
            values.as_ptr(),
            1,
            &kCFTypeDictionaryKeyCallBacks as *const _ as *const c_void,
            &kCFTypeDictionaryValueCallBacks as *const _ as *const c_void,
        );
        let Some(opts) = Cf::new(opts) else {
            return AXIsProcessTrusted() != 0;
        };
        AXIsProcessTrustedWithOptions(opts.0) != 0
    }
}

/// Gửi Cmd+C tới app đang được focus. Cờ đặt tường minh = chỉ Command, nên
/// phím người dùng còn đang giữ (vd Ctrl+Shift của phím tắt) không lẫn vào.
pub fn send_cmd_c() {
    // SAFETY: mọi event / source tạo ra đều được release khi ra khỏi phạm vi.
    unsafe {
        let src = Cf::new(CGEventSourceCreate(STATE_PRIVATE));
        let src_ptr = src.as_ref().map_or(std::ptr::null(), |s| s.0);
        for down in [true, false] {
            if let Some(ev) = Cf::new(CGEventCreateKeyboardEvent(src_ptr, KEY_C, down)) {
                CGEventSetFlags(ev.0, FLAG_COMMAND);
                CGEventPost(HID_EVENT_TAP, ev.0);
            }
        }
    }
}

// ─── Có mặt: idle, khoá màn hình, toàn màn hình ──────────────────────────────

pub fn idle_secs() -> u64 {
    // SAFETY: chỉ đọc.
    let s = unsafe { CGEventSourceSecondsSinceLastEventType(STATE_HID, ANY_INPUT_EVENT) };
    if s.is_finite() && s > 0.0 {
        s as u64
    } else {
        0
    }
}

pub fn screen_locked() -> bool {
    // SAFETY: dictionary "Copy" được release; chỉ đọc một khoá.
    let Some(d) = Cf::new(unsafe { CGSessionCopyCurrentDictionary() }) else {
        // Không có phiên đồ hoạ (đang ở màn hình đăng nhập / chuyển người dùng).
        return true;
    };
    cf_bool(dict_get(d.0, "CGSSessionScreenIsLocked")).unwrap_or(false)
}

fn display_rects() -> Vec<(f64, f64, f64, f64)> {
    let mut ids = [0u32; 16];
    let mut n = 0u32;
    // SAFETY: ghi tối đa 16 id vào mảng cục bộ.
    unsafe {
        if CGGetActiveDisplayList(ids.len() as u32, ids.as_mut_ptr(), &mut n) != 0 {
            return Vec::new();
        }
        ids[..n as usize]
            .iter()
            .map(|&id| {
                let r = CGDisplayBounds(id);
                (r.origin.x, r.origin.y, r.size.width, r.size.height)
            })
            .collect()
    }
}

/// Cửa sổ trước nhất (lớp ứng dụng bình thường) của app khác đang phủ kín một
/// màn hình: game / video / app toàn màn hình, trình chiếu.
/// Chỉ đọc lớp, chủ sở hữu (pid) và khung cửa sổ, không đọc tên hay nội dung.
pub fn frontmost_fullscreen() -> bool {
    // SAFETY: mảng "Copy" được release; phần tử là "Get".
    let Some(list) = Cf::new(unsafe {
        CGWindowListCopyWindowInfo(WINDOW_LIST_ON_SCREEN_ONLY | WINDOW_LIST_EXCLUDE_DESKTOP, 0)
    }) else {
        return false;
    };
    let me = std::process::id() as i64;
    for w in array_items(list.0) {
        if num_i64(dict_get(w, "kCGWindowLayer")) != Some(0) {
            continue;
        }
        if num_i64(dict_get(w, "kCGWindowOwnerPID")) == Some(me) {
            continue;
        }
        let b = dict_get(w, "kCGWindowBounds");
        let rect = (
            num_f64(dict_get(b, "X")),
            num_f64(dict_get(b, "Y")),
            num_f64(dict_get(b, "Width")),
            num_f64(dict_get(b, "Height")),
        );
        let (Some(x), Some(y), Some(wd), Some(h)) = rect else {
            return false;
        };
        // Danh sách xếp từ trước ra sau: cửa sổ ứng dụng đầu tiên là cửa sổ trên cùng.
        return crate::awareness::covers_display((x, y, wd, h), &display_rects());
    }
    false
}

// ─── CPU, RAM, ổ đĩa, mạng, pin, thời gian chạy ──────────────────────────────

/// (idle, kernel, user) theo đơn vị 100ns, cùng quy ước GetSystemTimes:
/// `kernel` đã gồm `idle`. Tick của mach = 1/100 giây = 100 000 × 100ns.
pub fn cpu_times() -> Option<(u64, u64, u64)> {
    const TICK_100NS: u64 = 100_000;
    let mut ncpu: libc::natural_t = 0;
    let mut info: libc::processor_info_array_t = std::ptr::null_mut();
    let mut count: libc::mach_msg_type_number_t = 0;
    // SAFETY: kernel cấp phát mảng `count` integer_t; đọc trong phạm vi rồi trả lại bằng vm_deallocate.
    unsafe {
        let kr = libc::host_processor_info(
            libc::mach_host_self(),
            libc::PROCESSOR_CPU_LOAD_INFO,
            &mut ncpu,
            &mut info,
            &mut count,
        );
        if kr != libc::KERN_SUCCESS || info.is_null() {
            return None;
        }
        let ticks = std::slice::from_raw_parts(info as *const u32, count as usize);
        let (mut user, mut system, mut idle) = (0u64, 0u64, 0u64);
        for cpu in ticks.chunks_exact(4) {
            user += cpu[libc::CPU_STATE_USER as usize] as u64 + cpu[libc::CPU_STATE_NICE as usize] as u64;
            system += cpu[libc::CPU_STATE_SYSTEM as usize] as u64;
            idle += cpu[libc::CPU_STATE_IDLE as usize] as u64;
        }
        libc::vm_deallocate(
            libc::mach_task_self(),
            info as libc::vm_address_t,
            count as usize * std::mem::size_of::<libc::integer_t>(),
        );
        Some((idle * TICK_100NS, (system + idle) * TICK_100NS, user * TICK_100NS))
    }
}

fn sysctl_u64(name: &str) -> Option<u64> {
    let c = CString::new(name).ok()?;
    let mut v = 0u64;
    let mut len = std::mem::size_of::<u64>();
    // SAFETY: v đủ 8 byte, len báo đúng cỡ.
    let ok = unsafe { libc::sysctlbyname(c.as_ptr(), &mut v as *mut _ as *mut c_void, &mut len, std::ptr::null_mut(), 0) } == 0;
    ok.then_some(v)
}

/// RAM: (đang dùng, tổng) byte. "Đang dùng" như Activity Monitor: bộ nhớ app
/// (internal − purgeable) + wired + bị nén.
pub fn ram_bytes() -> Option<(u64, u64)> {
    let total = sysctl_u64("hw.memsize")?;
    let mut vm: libc::vm_statistics64 = unsafe { std::mem::zeroed() };
    let mut count = libc::HOST_VM_INFO64_COUNT;
    // SAFETY: struct đúng cỡ HOST_VM_INFO64_COUNT; hàm chỉ ghi vào nó.
    let kr = unsafe {
        libc::host_statistics64(libc::mach_host_self(), libc::HOST_VM_INFO64, &mut vm as *mut _ as libc::host_info64_t, &mut count)
    };
    if kr != libc::KERN_SUCCESS {
        return None;
    }
    // SAFETY: không tham số.
    let page = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
    if page <= 0 {
        return None;
    }
    let pages = (vm.internal_page_count as u64).saturating_sub(vm.purgeable_count as u64)
        + vm.wire_count as u64
        + vm.compressor_page_count as u64;
    Some(((pages * page as u64).min(total), total))
}

pub fn ram_percent() -> Option<u32> {
    let (used, total) = ram_bytes()?;
    crate::sysmon_pin::used_pct(used, total)
}

/// Tên ổ khởi động (vd "Macintosh HD"), đọc một lần.
fn volume_name() -> String {
    static NAME: OnceLock<String> = OnceLock::new();
    NAME.get_or_init(|| {
        // SAFETY: url và giá trị "Copy" đều được release.
        unsafe {
            let path = b"/";
            let Some(url) = Cf::new(CFURLCreateFromFileSystemRepresentation(std::ptr::null(), path.as_ptr(), 1, 1)) else {
                return "Macintosh HD".to_string();
            };
            let mut out: CFTypeRef = std::ptr::null();
            if CFURLCopyResourcePropertyForKey(url.0, kCFURLVolumeLocalizedNameKey, &mut out, std::ptr::null_mut()) == 0 {
                return "Macintosh HD".to_string();
            }
            let Some(v) = Cf::new(out) else { return "Macintosh HD".to_string() };
            cf_string(v.0).filter(|s| !s.is_empty()).unwrap_or_else(|| "Macintosh HD".to_string())
        }
    })
    .clone()
}

/// Ổ khởi động: (tên, đang dùng, tổng) byte. Trên APFS, ổ dữ liệu
/// (/System/Volumes/Data) mới là nơi chứa file của người dùng.
pub fn system_disk() -> Option<(String, u64, u64)> {
    for path in ["/System/Volumes/Data", "/"] {
        let c = CString::new(path).ok()?;
        let mut st: libc::statfs = unsafe { std::mem::zeroed() };
        // SAFETY: chuỗi kết thúc 0; struct đúng kiểu.
        if unsafe { libc::statfs(c.as_ptr(), &mut st) } != 0 {
            continue;
        }
        let bs = st.f_bsize as u64;
        let total = st.f_blocks * bs;
        if total == 0 {
            continue;
        }
        let used = total.saturating_sub(st.f_bfree * bs);
        return Some((volume_name(), used, total));
    }
    None
}

/// Tổng byte mạng của các card thật (en0, en1…): bỏ loopback, VPN (utun),
/// AirDrop (awdl, llw), cầu nối ảo, để không đếm một gói hai lần.
/// Bộ đếm của getifaddrs là 32 bit, quay vòng sau 4 GB: net_rate trả 0 cho lần đó.
pub fn net_totals() -> Option<(u64, u64)> {
    let mut head: *mut libc::ifaddrs = std::ptr::null_mut();
    // SAFETY: duyệt danh sách liên kết do getifaddrs cấp phát rồi freeifaddrs.
    unsafe {
        if libc::getifaddrs(&mut head) != 0 {
            return None;
        }
        let (mut rx, mut tx) = (0u64, 0u64);
        let mut p = head;
        while !p.is_null() {
            let a = &*p;
            p = a.ifa_next;
            if a.ifa_addr.is_null() || a.ifa_data.is_null() || (*a.ifa_addr).sa_family as i32 != libc::AF_LINK {
                continue;
            }
            let flags = a.ifa_flags as i32;
            let up = flags & libc::IFF_UP != 0 && flags & libc::IFF_RUNNING != 0;
            let name = CStr::from_ptr(a.ifa_name).to_string_lossy();
            if !up || flags & libc::IFF_LOOPBACK != 0 || !name.starts_with("en") {
                continue;
            }
            let d = &*(a.ifa_data as *const libc::if_data);
            rx += d.ifi_ibytes as u64;
            tx += d.ifi_obytes as u64;
        }
        libc::freeifaddrs(head);
        Some((rx, tx))
    }
}

/// Pin trong máy: (phần trăm, đang cắm sạc). None = máy bàn / không đọc được.
pub fn battery() -> Option<(u32, bool)> {
    // SAFETY: blob và list là "Copy" (release qua Cf); mô tả từng nguồn là "Get".
    unsafe {
        let blob = Cf::new(IOPSCopyPowerSourcesInfo())?;
        let list = Cf::new(IOPSCopyPowerSourcesList(blob.0))?;
        for ps in array_items(list.0) {
            let d = IOPSGetPowerSourceDescription(blob.0, ps);
            if cf_string(dict_get(d, "Type")).as_deref() != Some("InternalBattery") {
                continue;
            }
            let cur = num_i64(dict_get(d, "Current Capacity"))?;
            let max = num_i64(dict_get(d, "Max Capacity")).filter(|&m| m > 0)?;
            let ac = cf_string(dict_get(d, "Power Source State")).as_deref() == Some("AC Power");
            let pct = ((cur as f64 * 100.0 / max as f64).round() as i64).clamp(0, 100) as u32;
            return Some((pct, ac));
        }
        None
    }
}

pub fn uptime_secs() -> u64 {
    let c = c"kern.boottime";
    let mut tv: libc::timeval = unsafe { std::mem::zeroed() };
    let mut len = std::mem::size_of::<libc::timeval>();
    // SAFETY: tv đúng kiểu sysctl trả về, len báo đúng cỡ.
    let ok = unsafe { libc::sysctlbyname(c.as_ptr(), &mut tv as *mut _ as *mut c_void, &mut len, std::ptr::null_mut(), 0) } == 0;
    if !ok {
        return 0;
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    now.saturating_sub(tv.tv_sec as u64)
}

// ─── Tiến trình ──────────────────────────────────────────────────────────────

/// (pid, tên, thời gian CPU 100ns, bộ nhớ byte). Tiến trình của người dùng khác
/// / của hệ thống không đọc được nếu không có quyền root: bỏ qua, như bản Windows
/// bỏ tiến trình được bảo vệ. Bộ nhớ = "phys_footprint" (cột Bộ nhớ của Activity Monitor).
pub fn processes() -> Vec<(u32, String, u64, u64)> {
    let mut out = Vec::new();
    let mut tb = libc::mach_timebase_info { numer: 0, denom: 0 };
    // SAFETY: mọi bộ đệm cục bộ đúng cỡ báo cho API.
    unsafe {
        if libc::mach_timebase_info(&mut tb) != 0 || tb.denom == 0 {
            return out;
        }
        let n = libc::proc_listallpids(std::ptr::null_mut(), 0);
        if n <= 0 {
            return out;
        }
        let mut pids = vec![0i32; n as usize + 64];
        let bytes = (pids.len() * std::mem::size_of::<i32>()) as i32;
        let n = libc::proc_listallpids(pids.as_mut_ptr() as *mut c_void, bytes);
        if n <= 0 {
            return out;
        }
        for &pid in &pids[..(n as usize).min(pids.len())] {
            if pid <= 0 {
                continue;
            }
            let mut ti: libc::proc_taskinfo = std::mem::zeroed();
            let size = std::mem::size_of::<libc::proc_taskinfo>() as i32;
            if libc::proc_pidinfo(pid, libc::PROC_PIDTASKINFO, 0, &mut ti as *mut _ as *mut c_void, size) != size {
                continue;
            }
            let mut name = [0 as c_char; 256];
            if libc::proc_name(pid, name.as_mut_ptr() as *mut c_void, name.len() as u32) <= 0 {
                continue;
            }
            let name = CStr::from_ptr(name.as_ptr()).to_string_lossy().into_owned();
            let mach = ti.pti_total_user + ti.pti_total_system;
            let ns = (mach as u128 * tb.numer as u128 / tb.denom as u128) as u64;
            let mut ri: libc::rusage_info_v2 = std::mem::zeroed();
            let mem = if libc::proc_pid_rusage(pid, libc::RUSAGE_INFO_V2, &mut ri as *mut _ as *mut libc::rusage_info_t) == 0 {
                ri.ri_phys_footprint
            } else {
                ti.pti_resident_size
            };
            out.push((pid as u32, name, ns / 100, mem));
        }
    }
    out
}

// ─── Giọng có sẵn của macOS (`say`) ──────────────────────────────────────────

/// Danh sách giọng: (tên = id, nhãn "Tên (vi_VN)").
pub fn say_voices() -> Result<Vec<(String, String)>, String> {
    let out = std::process::Command::new("/usr/bin/say")
        .args(["-v", "?"])
        .output()
        .map_err(|e| format!("Không chạy được lệnh say: {e}"))?;
    if !out.status.success() {
        return Err("Không đọc được danh sách giọng của macOS.".to_string());
    }
    Ok(crate::tts::parse_say_voices(&String::from_utf8_lossy(&out.stdout)))
}

/// Đọc `text` bằng giọng `voice` (rỗng = giọng mặc định của máy) ra WAV.
/// Văn bản đi qua stdin, không lên dòng lệnh.
pub fn say_synthesize(voice: &str, text: &str) -> Result<Vec<u8>, String> {
    use std::io::Write;
    static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("dc-say-{}-{n}.wav", std::process::id()));
    let mut cmd = std::process::Command::new("/usr/bin/say");
    if !voice.is_empty() {
        cmd.args(["-v", voice]);
    }
    cmd.arg("-o")
        .arg(&path)
        .args(["--file-format=WAVE", "--data-format=LEI16@22050", "-f", "-"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped());
    let mut child = cmd.spawn().map_err(|e| format!("Không chạy được lệnh say: {e}"))?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(text.as_bytes()).map_err(|e| format!("Giọng macOS lỗi: {e}"))?;
    }
    let out = child.wait_with_output().map_err(|e| format!("Giọng macOS lỗi: {e}"))?;
    let audio = std::fs::read(&path);
    let _ = std::fs::remove_file(&path);
    if !out.status.success() {
        let why = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(format!("Giọng macOS lỗi: {why}"));
    }
    audio.map_err(|e| format!("Giọng macOS không tạo được file âm thanh: {e}"))
}
