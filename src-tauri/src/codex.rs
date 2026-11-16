// Nhà cung cấp "chatgpt": người dùng đăng nhập tài khoản ChatGPT rồi dùng hạn mức
// của gói đã đăng ký, không nhập API key.
//
// App không tự cầm token và không tự gọi API của OpenAI. Mọi thứ đi qua Codex CLI
// chính thức chạy ở chế độ `codex app-server` (JSON-RPC, mỗi dòng một JSON, qua
// stdin / stdout): đăng nhập (`account/login/start`, Codex tự lo OAuth và tự làm mới
// token), đọc tài khoản + hạn mức (`account/read`, `account/rateLimits/read`, không
// tốn token), và trả lời (`thread/start` + `turn/start`, chữ về qua
// `item/agentMessage/delta`). Đã chạy thật với codex-cli 0.163.
//
// Cách ly khỏi Codex người dùng tự dùng ở terminal:
// - CODEX_HOME riêng trong thư mục cấu hình của app → đăng nhập riêng, và không nạp
//   config.toml / MCP server / plugin của người dùng vào câu trả lời của pet.
// - Biến môi trường theo danh sách cho phép: OPENAI_API_KEY không lọt sang, kẻo Codex
//   lặng lẽ tính tiền theo key thay vì dùng gói.
// - Codex là agent biết chạy lệnh; pet chỉ cần chữ. Nên: tắt các nhóm tool, sandbox
//   chỉ đọc, thư mục làm việc rỗng, và từ chối mọi yêu cầu xin phép.
//
// Phần thuần (dò binary, env, đọc JSON, gộp sự kiện một lượt) không đụng Tauri để
// test bằng `cargo test`.

use std::collections::{HashMap, HashSet, VecDeque};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use serde::Serialize;
use serde_json::{json, Value};
use tauri::{AppHandle, Emitter};

use crate::chat::ChatMsg;

/// Gói npm chính thức; app tự cài vào thư mục riêng (nút "Cài Codex").
const NPM_PACKAGE: &str = "@openai/codex";

/// Tắt các nhóm tool của agent. Tên lấy từ `codex features list` (0.163). Bản khác
/// không nhận thì `acquire` chạy lại không kèm các cờ này.
const LOCKDOWN_FLAGS: &[&str] = &[
    "features.shell_tool=false",
    "features.unified_exec=false",
    "features.code_mode_host=false",
    "features.multi_agent=false",
    "features.apps=false",
    "features.plugins=false",
    "features.view_image=false",
    "features.image_generation=false",
    "features.browser_use=false",
    "features.computer_use=false",
    "features.sleep_tool=false",
    "features.goals=false",
];

const INIT_TIMEOUT: Duration = Duration::from_secs(15);
const CALL_TIMEOUT: Duration = Duration::from_secs(30);
/// Một lượt im lặng lâu hơn thế này (không chữ, không sự kiện) thì coi là treo.
const TURN_SILENCE: Duration = Duration::from_secs(120);
/// Không dùng ngần này thì tắt tiến trình Codex (nó ăn RAM khi ngồi không).
const IDLE_KILL: Duration = Duration::from_secs(10 * 60);
/// Chờ người dùng bấm xong trong trình duyệt tối đa bao lâu.
const LOGIN_TTL: Duration = Duration::from_secs(5 * 60);
const INSTALL_TIMEOUT: Duration = Duration::from_secs(5 * 60);
const STDERR_TAIL: usize = 20;

// ─── Dò binary (thuần) ───────────────────────────────────────────────────────

/// Tên file `codex` trên PATH. Windows: bản npm là shim `.cmd`, không phải exe.
fn exe_names(windows: bool) -> &'static [&'static str] {
    if windows {
        &["codex.exe", "codex.cmd", "codex.bat"]
    } else {
        &["codex"]
    }
}

/// Ứng viên trên PATH, theo thứ tự thử.
pub fn path_candidates(path_var: &str, windows: bool) -> Vec<PathBuf> {
    let sep = if windows { ';' } else { ':' };
    let mut out = Vec::new();
    for dir in path_var.split(sep) {
        let dir = dir.trim().trim_matches('"');
        if dir.is_empty() {
            continue;
        }
        for name in exe_names(windows) {
            out.push(Path::new(dir).join(name));
        }
    }
    out
}

/// Binary sau khi `npm install @openai/codex --prefix <root>`: nằm trong gói con
/// theo nền tảng. Trả nhiều ứng viên vì cách xếp thư mục đổi theo bản.
pub fn installed_candidates(root: &Path, windows: bool) -> Vec<PathBuf> {
    let exe = if windows { "codex.exe" } else { "codex" };
    let targets: &[(&str, &str)] = if windows {
        &[
            ("codex-win32-x64", "x86_64-pc-windows-msvc"),
            ("codex-win32-arm64", "aarch64-pc-windows-msvc"),
        ]
    } else if cfg!(target_os = "macos") {
        &[
            ("codex-darwin-arm64", "aarch64-apple-darwin"),
            ("codex-darwin-x64", "x86_64-apple-darwin"),
        ]
    } else {
        &[
            ("codex-linux-x64", "x86_64-unknown-linux-musl"),
            ("codex-linux-arm64", "aarch64-unknown-linux-musl"),
        ]
    };
    let nm = root.join("node_modules").join("@openai");
    let mut out = Vec::new();
    for (pkg, triple) in targets {
        let vendor = nm.join(pkg).join("vendor").join(triple);
        out.push(vendor.join("bin").join(exe));
        out.push(vendor.join("codex").join(exe));
    }
    out
}

/// Các chỗ hay gặp ngoài PATH. Windows: bản npm toàn cục; còn thư mục của Codex
/// desktop (mỗi bản một thư mục hash) phải duyệt lúc chạy, xem `desktop_binary`.
pub fn well_known_candidates(env: &HashMap<String, String>, windows: bool) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if windows {
        if let Some(appdata) = env.get("APPDATA").filter(|v| !v.is_empty()) {
            out.push(Path::new(appdata).join("npm").join("codex.cmd"));
        }
    } else {
        if let Some(home) = env.get("HOME").filter(|v| !v.is_empty()) {
            out.push(Path::new(home).join(".local").join("bin").join("codex"));
        }
        out.push(PathBuf::from("/usr/local/bin/codex"));
        out.push(PathBuf::from("/opt/homebrew/bin/codex"));
    }
    out
}

/// Chọn bản mới nhất trong các thư mục hash của Codex desktop. Thư mục không có exe
/// (bản cũ đã dọn nhưng thư mục còn lại) thì bỏ.
pub fn pick_newest(entries: &[(PathBuf, u64, bool)]) -> Option<PathBuf> {
    entries
        .iter()
        .filter(|(_, _, has_exe)| *has_exe)
        .max_by_key(|(_, mtime, _)| *mtime)
        .map(|(dir, _, _)| dir.join("codex.exe"))
}

/// Biến môi trường đưa sang Codex: chỉ những thứ nó cần để chạy. Không có
/// OPENAI_API_KEY (dùng gói, không dùng key) hay token nào khác của người dùng.
const KEEP_ENV: &[&str] = &[
    "PATH", "PATHEXT", "HOME", "USERPROFILE", "HOMEDRIVE", "HOMEPATH", "LOCALAPPDATA", "APPDATA",
    "PROGRAMDATA", "TEMP", "TMP", "TMPDIR", "SYSTEMROOT", "SYSTEMDRIVE", "COMSPEC", "WINDIR",
    "NUMBER_OF_PROCESSORS", "PROCESSOR_ARCHITECTURE", "USERNAME", "USERDOMAIN", "LANG", "LC_ALL",
    "LC_CTYPE", "HTTP_PROXY", "HTTPS_PROXY", "NO_PROXY", "ALL_PROXY", "SSL_CERT_FILE",
];

/// Lọc env của app thành env cho Codex, rồi ghi CODEX_HOME của app đè lên.
/// So tên không phân biệt hoa thường (Windows có cả `Path` lẫn `PATH`).
pub fn child_env(src: impl IntoIterator<Item = (String, String)>, home: &Path) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = src
        .into_iter()
        .filter(|(k, v)| !v.is_empty() && KEEP_ENV.contains(&k.to_ascii_uppercase().as_str()))
        .collect();
    out.push(("CODEX_HOME".into(), home.to_string_lossy().into_owned()));
    out
}

// ─── Đọc JSON của app-server (thuần) ─────────────────────────────────────────

/// Một dòng từ app-server. Phân biệt theo `id` và `method`, đúng chuẩn JSON-RPC
/// (app-server không gửi trường "jsonrpc").
#[derive(Debug, PartialEq)]
pub enum Incoming {
    /// Trả lời cho yêu cầu ta đã gửi.
    Response { id: u64, result: Result<Value, String> },
    /// Codex hỏi ngược và chờ trả lời (xin phép chạy lệnh, sửa file…).
    Request { id: Value, method: String },
    Notification { method: String, params: Value },
    Unknown,
}

pub fn classify(line: &str) -> Incoming {
    let Ok(Value::Object(mut m)) = serde_json::from_str::<Value>(line) else {
        return Incoming::Unknown;
    };
    let id = m.remove("id").filter(|v| v.is_number() || v.is_string());
    let method = m.remove("method").and_then(|v| v.as_str().map(str::to_string));
    match (id, method) {
        (Some(id), Some(method)) => Incoming::Request { id, method },
        (None, Some(method)) => Incoming::Notification { method, params: m.remove("params").unwrap_or(Value::Null) },
        (Some(id), None) => {
            let Some(id) = id.as_u64() else { return Incoming::Unknown };
            let result = match m.remove("error") {
                Some(e) if !e.is_null() => Err(e
                    .get("message")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.trim().is_empty())
                    .unwrap_or("lỗi không rõ")
                    .to_string()),
                _ => Ok(m.remove("result").unwrap_or(Value::Null)),
            };
            Incoming::Response { id, result }
        }
        (None, None) => Incoming::Unknown,
    }
}

/// Trả lời cho yêu cầu Codex hỏi ngược: luôn từ chối. Pet không cho agent chạy lệnh
/// hay sửa file, và không ai ngồi duyệt. Không trả lời thì lượt treo mãi.
pub fn refuse(id: &Value, method: &str) -> Value {
    let lower = method.to_ascii_lowercase();
    if lower == "execcommandapproval" || lower == "applypatchapproval" {
        // Giao thức v1.
        json!({ "id": id, "result": { "decision": "denied" } })
    } else if lower.contains("approval") {
        json!({ "id": id, "result": { "decision": "decline" } })
    } else {
        json!({ "id": id, "error": { "code": -32601, "message": "not supported by this client" } })
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Account {
    /// Rỗng khi Codex không cho biết.
    pub email: String,
    pub plan: String,
}

/// Kết quả `account/read`. None = chưa đăng nhập bằng tài khoản ChatGPT. Đăng nhập
/// kiểu API key cũng tính là chưa: cả tính năng này là để dùng gói.
pub fn parse_account(v: &Value) -> Option<Account> {
    let acc = v.get("account")?;
    if acc.get("type").and_then(|t| t.as_str()) != Some("chatgpt") {
        return None;
    }
    let s = |k: &str| acc.get(k).and_then(|v| v.as_str()).unwrap_or("").to_string();
    Some(Account { email: s("email"), plan: s("planType") })
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct RateLimit {
    pub used_percent: u32,
    /// Giây unix lúc hạn mức tính lại.
    pub resets_at: Option<i64>,
    pub reached: bool,
}

/// Kết quả `account/rateLimits/read`: lấy cửa sổ chính. None = không đọc được, đừng
/// hiện 0% cho thứ mình không biết.
pub fn parse_rate_limit(v: &Value) -> Option<RateLimit> {
    let rl = v.get("rateLimits")?;
    let primary = rl.get("primary")?;
    let used = primary.get("usedPercent")?.as_f64()?;
    Some(RateLimit {
        used_percent: used.clamp(0.0, 100.0).round() as u32,
        resets_at: primary.get("resetsAt").and_then(|v| v.as_i64()),
        reached: rl.get("rateLimitReachedType").is_some_and(|v| v.is_string()),
    })
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Model {
    pub id: String,
    pub name: String,
    pub efforts: Vec<String>,
    pub is_default: bool,
}

/// Kết quả `model/list`.
pub fn parse_models(v: &Value) -> Vec<Model> {
    let Some(arr) = v.get("data").and_then(|d| d.as_array()) else { return Vec::new() };
    arr.iter()
        .filter_map(|m| {
            let id = m.get("id").and_then(|v| v.as_str()).filter(|s| !s.is_empty())?.to_string();
            let name = m.get("displayName").and_then(|v| v.as_str()).filter(|s| !s.is_empty()).unwrap_or(&id).to_string();
            let efforts = m
                .get("supportedReasoningEfforts")
                .and_then(|v| v.as_array())
                .map(|a| a.iter().filter_map(|e| e.get("reasoningEffort").and_then(|v| v.as_str()).map(str::to_string)).collect())
                .unwrap_or_default();
            Some(Model { id, name, efforts, is_default: m.get("isDefault").and_then(|v| v.as_bool()) == Some(true) })
        })
        .collect()
}

/// Mức suy luận cho câu trả lời của pet: thấp nhất model nhận, để trả lời nhanh và
/// đỡ tốn hạn mức. `wanted` rỗng = model mặc định. None = không biết thì không gửi,
/// Codex tự dùng mặc định của nó.
pub fn pick_effort(models: &[Model], wanted: &str) -> Option<String> {
    let model = if wanted.trim().is_empty() {
        models.iter().find(|m| m.is_default)
    } else {
        models.iter().find(|m| m.id == wanted.trim())
    }?;
    ["low", "minimal", "medium"].iter().find(|e| model.efforts.iter().any(|x| x == *e)).map(|e| e.to_string())
}

/// Lịch sử trò chuyện + câu mới thành một tin nhắn. Mỗi lượt mở một thread Codex mới
/// (system prompt đổi theo giờ và danh sách việc), nên ngữ cảnh đi kèm ở đây; lịch sử
/// thật vẫn do app giữ.
pub fn flatten_history(messages: &[ChatMsg]) -> String {
    let Some((last, earlier)) = messages.split_last() else { return String::new() };
    if earlier.is_empty() {
        return last.content.clone();
    }
    let mut out = String::from("Earlier conversation, for context only (do not repeat it):\n");
    for m in earlier {
        let who = if m.role == "assistant" { "Assistant" } else { "User" };
        out.push_str(&format!("{who}: {}\n", m.content.trim()));
    }
    out.push_str("\nNew message from the user:\n");
    out.push_str(&last.content);
    out
}

/// Lỗi này có phải do chưa đăng nhập / phiên hết hạn không. App-server không có mã
/// riêng cho ca này nên so theo chuỗi.
pub fn looks_like_auth_error(text: &str) -> bool {
    let t = text.to_ascii_lowercase();
    [
        "unauthorized", "401", "not logged in", "not authenticated", "please log in", "please login",
        "codex login", "login required", "invalid_grant", "token expired", "refresh token", "sign in again",
    ]
    .iter()
    .any(|p| t.contains(p))
}

/// Lỗi của Codex thành câu cho người dùng.
pub fn friendly_error(msg: &str) -> String {
    if looks_like_auth_error(msg) {
        "Chưa đăng nhập ChatGPT hoặc phiên đã hết hạn. Mở Cài đặt → 🤖 để đăng nhập lại.".to_string()
    } else {
        format!("ChatGPT: {}", msg.trim())
    }
}

#[derive(Debug, PartialEq)]
pub enum TurnEvent {
    Delta(String),
    Done,
    Failed(String),
    Ignore,
}

/// Gộp các thông báo của MỘT lượt thành chữ + kết thúc.
#[derive(Default)]
pub struct TurnReducer {
    /// Tin nhắn đã có chữ chạy về (để `item/completed` không phát lại lần nữa).
    streamed: HashSet<String>,
    last_item: Option<String>,
    emitted: bool,
    /// Lỗi báo giữa lượt mà Codex không tự thử lại.
    error: Option<String>,
}

impl TurnReducer {
    /// Model có thể nói hai tin nhắn trong một lượt: ngắt đoạn giữa chúng.
    fn text(&mut self, item: &str, text: &str) -> TurnEvent {
        if text.is_empty() {
            return TurnEvent::Ignore;
        }
        let new_item = self.last_item.as_deref() != Some(item);
        let sep = if new_item && self.emitted { "\n\n" } else { "" };
        self.last_item = Some(item.to_string());
        self.emitted = true;
        TurnEvent::Delta(format!("{sep}{text}"))
    }

    pub fn feed(&mut self, method: &str, params: &Value) -> TurnEvent {
        let s = |v: &Value, k: &str| v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string();
        match method {
            "item/agentMessage/delta" => {
                let id = s(params, "itemId");
                self.streamed.insert(id.clone());
                self.text(&id, &s(params, "delta"))
            }
            // Chữ đầy đủ về một lần: chỉ dùng khi tin nhắn này chưa có mẩu nào chạy về.
            "item/completed" => {
                let item = params.get("item").unwrap_or(&Value::Null);
                let id = s(item, "id");
                if s(item, "type") != "agentMessage" || self.streamed.contains(&id) {
                    return TurnEvent::Ignore;
                }
                self.text(&id, &s(item, "text"))
            }
            "error" => {
                if params.get("willRetry").and_then(|v| v.as_bool()) != Some(true) {
                    let msg = s(params.get("error").unwrap_or(&Value::Null), "message");
                    if !msg.is_empty() {
                        self.error = Some(msg);
                    }
                }
                TurnEvent::Ignore
            }
            // Lượt hỏng về qua `turn/completed` với status "failed" (không có `turn/failed`).
            "turn/completed" => {
                let turn = params.get("turn").unwrap_or(&Value::Null);
                if s(turn, "status") == "failed" {
                    let msg = s(turn.get("error").unwrap_or(&Value::Null), "message");
                    let msg = if msg.is_empty() { self.error.take().unwrap_or_default() } else { msg };
                    TurnEvent::Failed(if msg.is_empty() { "lượt trả lời thất bại".to_string() } else { msg })
                } else {
                    TurnEvent::Done
                }
            }
            "thread/status/changed" => {
                let st = params.get("status").unwrap_or(&Value::Null);
                if s(st, "type") == "systemError" {
                    let msg = s(st, "message");
                    TurnEvent::Failed(if msg.is_empty() { "lỗi hệ thống của Codex".to_string() } else { msg })
                } else {
                    TurnEvent::Ignore
                }
            }
            _ => TurnEvent::Ignore,
        }
    }
}

// ─── Tiến trình app-server ───────────────────────────────────────────────────

pub enum Event {
    LoginCompleted { ok: bool, error: Option<String> },
    AccountChanged,
}

type EventFn = Box<dyn Fn(Event) + Send + Sync>;
type Reply = Result<Value, String>;

pub struct Server {
    child: Mutex<Child>,
    /// None = đã đóng (đang tắt).
    stdin: Mutex<Option<ChildStdin>>,
    next_id: AtomicU64,
    pending: Mutex<HashMap<u64, mpsc::Sender<Reply>>>,
    /// threadId → nơi nhận thông báo của lượt đang chạy trên thread đó.
    threads: Mutex<HashMap<String, mpsc::Sender<(String, Value)>>>,
    stderr_tail: Mutex<VecDeque<String>>,
    dead: AtomicBool,
    last_used: Mutex<Instant>,
    busy: AtomicUsize,
    models: Mutex<Option<Vec<Model>>>,
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// Lệnh chạy một binary Codex: không cửa sổ console, env đã lọc.
fn command(binary: &Path, home: &Path) -> Command {
    let mut cmd = Command::new(binary);
    cmd.env_clear().envs(child_env(std::env::vars(), home));
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    cmd
}

impl Server {
    /// Chạy `codex app-server` và bắt tay. `lockdown` = kèm các cờ tắt tool.
    pub fn spawn(binary: &Path, home: &Path, cwd: &Path, lockdown: bool, on_event: EventFn) -> Result<Arc<Server>, String> {
        let mut cmd = command(binary, home);
        cmd.arg("app-server");
        if lockdown {
            for flag in LOCKDOWN_FLAGS {
                cmd.args(["-c", *flag]);
            }
        }
        let mut child = cmd
            .current_dir(cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Không chạy được Codex ({}): {e}", binary.display()))?;
        let stdin = child.stdin.take();
        let stdout = child.stdout.take().ok_or("Codex không có stdout")?;
        let stderr = child.stderr.take().ok_or("Codex không có stderr")?;
        let server = Arc::new(Server {
            child: Mutex::new(child),
            stdin: Mutex::new(stdin),
            next_id: AtomicU64::new(1),
            pending: Mutex::new(HashMap::new()),
            threads: Mutex::new(HashMap::new()),
            stderr_tail: Mutex::new(VecDeque::new()),
            dead: AtomicBool::new(false),
            last_used: Mutex::new(Instant::now()),
            busy: AtomicUsize::new(0),
            models: Mutex::new(None),
        });

        let s = server.clone();
        std::thread::spawn(move || s.read_stdout(stdout, on_event));
        // stderr là nhật ký của Codex, không phải giao thức: giữ đuôi để nói được vì sao nó dừng.
        let s = server.clone();
        std::thread::spawn(move || {
            for line in BufReader::new(stderr).split(b'\n').map_while(Result::ok) {
                let line = String::from_utf8_lossy(&line).trim().to_string();
                if line.is_empty() {
                    continue;
                }
                let mut tail = lock(&s.stderr_tail);
                if tail.len() == STDERR_TAIL {
                    tail.pop_front();
                }
                tail.push_back(line);
            }
        });

        // App-server từ chối mọi yêu cầu khác cho tới khi bắt tay xong. Khai đúng tên
        // app: đây là thứ OpenAI thấy trong user agent.
        let init = server.request(
            "initialize",
            json!({
                "clientInfo": { "name": "desktop-companion", "title": "Desktop Companion", "version": env!("CARGO_PKG_VERSION") },
                // Pet không hiện phần suy luận: khỏi nhận mấy thông báo dày đặc đó.
                "capabilities": { "experimentalApi": false, "optOutNotificationMethods": ["item/reasoning/textDelta", "item/reasoning/summaryTextDelta"] },
            }),
            INIT_TIMEOUT,
        );
        match init {
            Ok(v) if v.is_object() => {
                server.write(&json!({ "method": "initialized", "params": {} }))?;
                Ok(server)
            }
            Ok(_) => {
                server.shutdown();
                Err("Codex trả lời bắt tay không đọc được (bản quá cũ?).".to_string())
            }
            Err(e) => {
                server.shutdown();
                Err(e)
            }
        }
    }

    fn read_stdout(&self, stdout: std::process::ChildStdout, on_event: EventFn) {
        // Đọc theo byte rồi mới giải mã: một dòng UTF-8 hỏng không được làm dừng cả luồng.
        for line in BufReader::new(stdout).split(b'\n').map_while(Result::ok) {
            let line = String::from_utf8_lossy(&line);
            match classify(line.trim()) {
                Incoming::Response { id, result } => {
                    if let Some(tx) = lock(&self.pending).remove(&id) {
                        let _ = tx.send(result);
                    }
                }
                Incoming::Request { id, method } => {
                    let _ = self.write(&refuse(&id, &method));
                }
                Incoming::Notification { method, params } => match method.as_str() {
                    "account/login/completed" => on_event(Event::LoginCompleted {
                        ok: params.get("success").and_then(|v| v.as_bool()) == Some(true),
                        error: params.get("error").and_then(|v| v.as_str()).filter(|s| !s.is_empty()).map(str::to_string),
                    }),
                    "account/updated" => on_event(Event::AccountChanged),
                    _ => {
                        let thread = params.get("threadId").and_then(|v| v.as_str()).unwrap_or("");
                        if let Some(tx) = lock(&self.threads).get(thread) {
                            let _ = tx.send((method, params));
                        }
                    }
                },
                Incoming::Unknown => {}
            }
        }
        // Codex đã thoát: đánh thức mọi chỗ đang chờ, kẻo chúng đợi tới hết giờ.
        self.dead.store(true, Ordering::SeqCst);
        lock(&self.pending).clear();
        lock(&self.threads).clear();
    }

    fn write(&self, msg: &Value) -> Result<(), String> {
        let mut guard = lock(&self.stdin);
        let stdin = guard.as_mut().ok_or_else(|| self.exit_reason())?;
        writeln!(stdin, "{msg}").and_then(|_| stdin.flush()).map_err(|_| self.exit_reason())
    }

    pub fn request(&self, method: &str, params: Value, timeout: Duration) -> Reply {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let (tx, rx) = mpsc::channel();
        lock(&self.pending).insert(id, tx);
        let msg = if params.is_null() { json!({ "id": id, "method": method }) } else { json!({ "id": id, "method": method, "params": params }) };
        if let Err(e) = self.write(&msg) {
            lock(&self.pending).remove(&id);
            return Err(e);
        }
        self.touch();
        match rx.recv_timeout(timeout) {
            Ok(r) => r,
            Err(mpsc::RecvTimeoutError::Timeout) => {
                lock(&self.pending).remove(&id);
                Err(format!("Codex không trả lời ({method})."))
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => Err(self.exit_reason()),
        }
    }

    fn exit_reason(&self) -> String {
        let tail = lock(&self.stderr_tail);
        let tail: Vec<&str> = tail.iter().rev().take(3).rev().map(String::as_str).collect();
        if tail.is_empty() {
            "Codex đã dừng.".to_string()
        } else {
            format!("Codex đã dừng: {}", tail.join(" | "))
        }
    }

    pub fn is_dead(&self) -> bool {
        self.dead.load(Ordering::SeqCst)
    }

    fn touch(&self) {
        *lock(&self.last_used) = Instant::now();
    }

    fn idle_for(&self) -> Duration {
        lock(&self.last_used).elapsed()
    }

    /// Đóng stdin cho Codex tự thoát; quá 1,5 giây chưa thoát thì diệt.
    pub fn shutdown(&self) {
        lock(&self.stdin).take();
        let mut child = lock(&self.child);
        let deadline = Instant::now() + Duration::from_millis(1500);
        while Instant::now() < deadline {
            if matches!(child.try_wait(), Ok(Some(_)) | Err(_)) {
                return;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        let _ = child.kill();
        let _ = child.wait();
    }

    pub fn account(&self) -> Result<Option<Account>, String> {
        Ok(parse_account(&self.request("account/read", json!({}), CALL_TIMEOUT)?))
    }

    pub fn models(&self) -> Result<Vec<Model>, String> {
        if let Some(m) = lock(&self.models).clone() {
            return Ok(m);
        }
        let list = parse_models(&self.request("model/list", json!({ "includeHidden": false }), CALL_TIMEOUT)?);
        if !list.is_empty() {
            *lock(&self.models) = Some(list.clone());
        }
        Ok(list)
    }

    /// Một lượt hỏi-đáp trên thread mới. `on_delta` nhận từng mẩu chữ. `cancel` bật
    /// lên thì ngắt lượt và trả phần đã có. `model` rỗng = mặc định của Codex.
    pub fn turn(
        &self,
        system: &str,
        input: &str,
        model: &str,
        cwd: &Path,
        cancel: &AtomicBool,
        mut on_delta: impl FnMut(&str),
    ) -> Result<String, String> {
        self.busy.fetch_add(1, Ordering::SeqCst);
        let _busy = BusyGuard(&self.busy);
        let model = model.trim();

        let mut start = json!({
            "baseInstructions": system,
            // Không lưu transcript: lịch sử do app giữ.
            "ephemeral": true,
            "sandbox": "read-only",
            "approvalPolicy": "never",
            "cwd": cwd.to_string_lossy(),
        });
        if !model.is_empty() {
            start["model"] = model.into();
        }
        let started = self.request("thread/start", start, CALL_TIMEOUT)?;
        let thread = started
            .get("thread")
            .and_then(|t| t.get("id"))
            .and_then(|v| v.as_str())
            .ok_or("Codex không trả về thread.")?
            .to_string();

        let (tx, rx) = mpsc::channel();
        lock(&self.threads).insert(thread.clone(), tx);
        let result = self.run_turn(&thread, input, model, cancel, &rx, &mut on_delta);
        lock(&self.threads).remove(&thread);
        // Trả thread lại cho Codex dọn. Bản cũ không có lệnh này thì thôi.
        let _ = self.request("thread/unsubscribe", json!({ "threadId": thread }), Duration::from_secs(3));
        result
    }

    fn run_turn(
        &self,
        thread: &str,
        input: &str,
        model: &str,
        cancel: &AtomicBool,
        rx: &mpsc::Receiver<(String, Value)>,
        on_delta: &mut impl FnMut(&str),
    ) -> Result<String, String> {
        let mut params = json!({ "threadId": thread, "input": [{ "type": "text", "text": input }] });
        if let Some(effort) = pick_effort(&self.models().unwrap_or_default(), model) {
            params["effort"] = effort.into();
        }
        let turn = self.request("turn/start", params, CALL_TIMEOUT)?;
        let turn_id = turn.get("turn").and_then(|t| t.get("id")).and_then(|v| v.as_str()).unwrap_or("").to_string();
        let interrupt = || {
            let _ = self.request("turn/interrupt", json!({ "threadId": thread, "turnId": turn_id }), Duration::from_secs(5));
        };

        let mut reducer = TurnReducer::default();
        let mut full = String::new();
        let mut last_event = Instant::now();
        loop {
            if cancel.load(Ordering::SeqCst) {
                interrupt();
                return Ok(full);
            }
            match rx.recv_timeout(Duration::from_millis(250)) {
                Ok((method, params)) => {
                    last_event = Instant::now();
                    self.touch();
                    match reducer.feed(&method, &params) {
                        TurnEvent::Delta(text) => {
                            full.push_str(&text);
                            on_delta(&text);
                        }
                        TurnEvent::Done => return Ok(full),
                        TurnEvent::Failed(msg) => return Err(friendly_error(&msg)),
                        TurnEvent::Ignore => {}
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    if last_event.elapsed() > TURN_SILENCE {
                        interrupt();
                        return Err("ChatGPT không trả lời (quá 2 phút).".to_string());
                    }
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => return Err(self.exit_reason()),
            }
        }
    }
}

struct BusyGuard<'a>(&'a AtomicUsize);

impl Drop for BusyGuard<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

// ─── Tauri glue ──────────────────────────────────────────────────────────────

struct Ctx {
    app: AppHandle,
    /// CODEX_HOME riêng của app (đăng nhập nằm ở đây).
    home: PathBuf,
    /// Thư mục làm việc rỗng cho Codex.
    work: PathBuf,
    /// Nơi app tự cài Codex CLI.
    install_root: PathBuf,
}

struct Login {
    id: Option<String>,
    deadline: Instant,
}

static CTX: OnceLock<Ctx> = OnceLock::new();
static SERVER: Mutex<Option<Arc<Server>>> = Mutex::new(None);
static LOGIN: Mutex<Option<Login>> = Mutex::new(None);

const NOT_INSTALLED: &str = "Chưa có Codex trên máy. Mở Cài đặt → 🤖 để cài.";

fn ctx() -> Result<&'static Ctx, String> {
    CTX.get().ok_or_else(|| "Codex chưa sẵn sàng.".to_string())
}

/// Gọi một lần lúc khởi động: ghi nhớ thư mục và chạy luồng dọn tiến trình ngồi không.
pub fn init(app: &AppHandle) {
    let dir = |name: &str| crate::store::config_file(app, name);
    let (Some(home), Some(work), Some(install_root)) = (dir("codex-home"), dir("codex-work"), dir("codex-cli")) else {
        return;
    };
    if CTX.set(Ctx { app: app.clone(), home, work, install_root }).is_err() {
        return;
    }
    std::thread::spawn(|| loop {
        std::thread::sleep(Duration::from_secs(30));
        reap();
    });
}

fn reap() {
    // Đăng nhập bỏ dở trong trình duyệt: báo hết giờ, đừng để nút quay mãi.
    let expired = {
        let mut login = lock(&LOGIN);
        match login.as_ref() {
            Some(l) if Instant::now() >= l.deadline => login.take(),
            _ => None,
        }
    };
    if let Some(l) = expired {
        cancel_login_on_server(l.id);
        emit_login(false, Some("Hết thời gian chờ đăng nhập.".to_string()));
    }
    if lock(&LOGIN).is_some() {
        return;
    }
    let mut server = lock(&SERVER);
    let stop = server.as_ref().is_some_and(|s| s.is_dead() || (s.busy.load(Ordering::SeqCst) == 0 && s.idle_for() > IDLE_KILL));
    if stop {
        if let Some(s) = server.take() {
            s.shutdown();
        }
    }
}

fn emit_login(ok: bool, error: Option<String>) {
    if let Ok(c) = ctx() {
        let _ = c.app.emit("codex-login", json!({ "ok": ok, "error": error }));
    }
}

/// Ghi nhớ đã đăng nhập hay chưa (chat::has_key đọc cờ này, khỏi phải hỏi Codex).
fn set_signed_in(value: bool) {
    let Ok(c) = ctx() else { return };
    let mut settings = crate::store::load_settings(&c.app);
    if settings.chatgpt_signed_in != value {
        settings.chatgpt_signed_in = value;
        if let Err(e) = crate::store::save_settings(&c.app, &settings) {
            eprintln!("[codex] không ghi được trạng thái đăng nhập: {e}");
        }
    }
}

fn on_event(ev: Event) {
    match ev {
        Event::LoginCompleted { ok, error } => {
            lock(&LOGIN).take();
            if ok {
                set_signed_in(true);
            }
            emit_login(ok, error);
        }
        Event::AccountChanged => {
            if let Ok(c) = ctx() {
                let _ = c.app.emit("codex-account", ());
            }
        }
    }
}

fn is_file(p: &Path) -> bool {
    p.is_file()
}

/// Bản mới nhất trong thư mục của Codex desktop trên Windows
/// (%LOCALAPPDATA%\OpenAI\Codex\bin\<hash>\codex.exe, không có trên PATH).
fn desktop_binary(env: &HashMap<String, String>) -> Option<PathBuf> {
    let root = Path::new(env.get("LOCALAPPDATA")?).join("OpenAI").join("Codex").join("bin");
    let entries: Vec<(PathBuf, u64, bool)> = std::fs::read_dir(root)
        .ok()?
        .flatten()
        .filter(|e| e.path().is_dir())
        .map(|e| {
            let dir = e.path();
            let mtime = e
                .metadata()
                .and_then(|m| m.modified())
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let has_exe = is_file(&dir.join("codex.exe"));
            (dir, mtime, has_exe)
        })
        .collect();
    pick_newest(&entries)
}

/// Tìm binary Codex: bản app tự cài → PATH → các chỗ hay gặp. Trả (binary, số chỗ đã tìm).
fn detect(install_root: &Path) -> (Option<PathBuf>, usize) {
    let windows = cfg!(windows);
    let env: HashMap<String, String> = std::env::vars().collect();
    let path_var = env.iter().find(|(k, _)| k.eq_ignore_ascii_case("PATH")).map(|(_, v)| v.as_str()).unwrap_or("");
    let mut candidates = installed_candidates(install_root, windows);
    candidates.extend(path_candidates(path_var, windows));
    candidates.extend(well_known_candidates(&env, windows));
    let searched = candidates.len() + usize::from(windows);
    let found = candidates.into_iter().find(|p| is_file(p)).or_else(|| if windows { desktop_binary(&env) } else { None });
    (found, searched)
}

/// `codex --version`, tối đa 5 giây. Chỉ để hiển thị.
fn version_of(binary: &Path, home: &Path) -> Option<String> {
    let mut child = command(binary, home).arg("--version").stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn().ok()?;
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => break,
            Ok(Some(_)) | Err(_) => return None,
            Ok(None) if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(30)),
        }
    }
    let mut out = String::new();
    std::io::Read::read_to_string(child.stdout.as_mut()?, &mut out).ok()?;
    out.lines().next().map(|l| l.trim().to_string()).filter(|l| !l.is_empty())
}

/// Tiến trình Codex dùng chung; chưa có (hoặc đã chết) thì chạy mới.
fn acquire() -> Result<Arc<Server>, String> {
    let c = ctx()?;
    let mut guard = lock(&SERVER);
    if let Some(s) = guard.as_ref().filter(|s| !s.is_dead()) {
        s.touch();
        return Ok(s.clone());
    }
    let (binary, _) = detect(&c.install_root);
    let binary = binary.ok_or(NOT_INSTALLED)?;
    for dir in [&c.home, &c.work] {
        std::fs::create_dir_all(dir).map_err(|e| format!("Không tạo được thư mục cho Codex: {e}"))?;
    }
    let server = match Server::spawn(&binary, &c.home, &c.work, true, Box::new(on_event)) {
        Ok(s) => s,
        // Bản Codex khác có thể không nhận các cờ tắt tool: thử lại không kèm, vẫn còn
        // sandbox chỉ đọc + từ chối mọi yêu cầu xin phép.
        Err(first) => Server::spawn(&binary, &c.home, &c.work, false, Box::new(on_event)).map_err(|_| first)?,
    };
    *guard = Some(server.clone());
    Ok(server)
}

fn stop_server() {
    if let Some(s) = lock(&SERVER).take() {
        s.shutdown();
    }
}

fn cancel_login_on_server(id: Option<String>) {
    let server = lock(&SERVER).clone();
    if let (Some(server), Some(id)) = (server, id) {
        let _ = server.request("account/login/cancel", json!({ "loginId": id }), Duration::from_secs(5));
    }
}

enum Chunk {
    Delta(String),
    Done(Result<String, String>),
}

/// Trả lời có stream, dùng trong async: lượt chạy ở luồng riêng (đọc pipe là việc
/// chặn), chữ về qua kênh. `on_delta` trả false = dừng sớm.
pub async fn stream<F: FnMut(&str) -> bool>(system: &str, input: &str, model: &str, mut on_delta: F) -> Result<String, String> {
    let work = ctx()?.work.clone();
    let (tx, mut rx) = tauri::async_runtime::channel::<Chunk>(64);
    let cancel = Arc::new(AtomicBool::new(false));
    let (system, input, model, flag) = (system.to_string(), input.to_string(), model.to_string(), cancel.clone());
    std::thread::spawn(move || {
        let result = acquire().and_then(|server| {
            server.turn(&system, &input, &model, &work, &flag, |delta| {
                let _ = tx.blocking_send(Chunk::Delta(delta.to_string()));
            })
        });
        let _ = tx.blocking_send(Chunk::Done(result));
    });
    let mut full = String::new();
    while let Some(chunk) = rx.recv().await {
        match chunk {
            Chunk::Delta(delta) => {
                full.push_str(&delta);
                if !on_delta(&delta) {
                    cancel.store(true, Ordering::SeqCst);
                    return Ok(full);
                }
            }
            Chunk::Done(result) => return result,
        }
    }
    Err("Codex đã dừng giữa chừng.".to_string())
}

/// Một câu hỏi, một câu trả lời (dịch, trả lời nhanh, pet tự lên tiếng).
pub async fn complete(system: &str, user: &str, model: &str) -> Result<String, String> {
    let text = stream(system, user, model, |_| true).await?;
    if text.trim().is_empty() {
        return Err("Phản hồi rỗng từ ChatGPT.".to_string());
    }
    Ok(text)
}

// ─── Lệnh cho trang Cài đặt ──────────────────────────────────────────────────

#[derive(Serialize, Default)]
pub struct Status {
    /// "missing" | "broken" | "signed_out" | "ready"
    state: &'static str,
    /// Lý do khi "broken".
    detail: String,
    version: String,
    email: String,
    plan: String,
    used_percent: Option<u32>,
    resets_at: Option<i64>,
    limit_reached: bool,
    login_pending: bool,
    /// Số chỗ đã tìm khi "missing".
    searched: usize,
}

fn status_blocking() -> Status {
    let Ok(c) = ctx() else {
        return Status { state: "broken", detail: "Codex chưa sẵn sàng.".into(), ..Default::default() };
    };
    let (binary, searched) = detect(&c.install_root);
    let Some(binary) = binary else {
        return Status { state: "missing", searched, ..Default::default() };
    };
    let mut st = Status {
        version: version_of(&binary, &c.home).unwrap_or_default(),
        login_pending: lock(&LOGIN).is_some(),
        ..Default::default()
    };
    let account = acquire().and_then(|server| server.account().map(|a| (server, a)));
    match account {
        Err(e) => {
            st.state = "broken";
            st.detail = e;
        }
        Ok((_, None)) => {
            st.state = "signed_out";
            set_signed_in(false);
        }
        Ok((server, Some(acc))) => {
            st.state = "ready";
            st.email = acc.email;
            st.plan = acc.plan;
            set_signed_in(true);
            // Hạn mức là thông tin phụ: không đọc được thì thiếu một dòng, không báo lỗi.
            if let Some(rl) = server.request("account/rateLimits/read", json!({}), CALL_TIMEOUT).ok().and_then(|v| parse_rate_limit(&v)) {
                st.used_percent = Some(rl.used_percent);
                st.resets_at = rl.resets_at;
                st.limit_reached = rl.reached;
            }
        }
    }
    st
}

async fn blocking<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(f).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn codex_status() -> Result<Status, String> {
    blocking(status_blocking).await
}

#[derive(Serialize)]
pub struct LoginStart {
    /// Đã mở trình duyệt chưa. Chưa mở được thì `auth_url` có link để người dùng tự mở.
    opened: bool,
    auth_url: String,
}

/// Bắt đầu đăng nhập: Codex trả link, app mở trình duyệt. Kết quả về sau qua sự
/// kiện "codex-login" { ok, error }.
#[tauri::command]
pub async fn codex_login(app: AppHandle) -> Result<LoginStart, String> {
    let url = blocking(|| -> Result<String, String> {
        let server = acquire()?;
        // Bấm lại khi lượt trước còn treo (lỡ đóng tab): bỏ lượt cũ, mở lượt mới.
        if let Some(old) = lock(&LOGIN).take() {
            let _ = server.request("account/login/cancel", json!({ "loginId": old.id }), Duration::from_secs(5));
        }
        let res = server.request("account/login/start", json!({ "type": "chatgpt" }), CALL_TIMEOUT)?;
        let url = res.get("authUrl").and_then(|v| v.as_str()).filter(|u| u.starts_with("https://")).ok_or("Codex không trả về link đăng nhập.")?.to_string();
        let id = res.get("loginId").and_then(|v| v.as_str()).map(str::to_string);
        *lock(&LOGIN) = Some(Login { id, deadline: Instant::now() + LOGIN_TTL });
        Ok(url)
    })
    .await??;
    // App-server không tự mở trình duyệt: nó chỉ đưa link rồi chờ ở localhost.
    use tauri_plugin_opener::OpenerExt;
    let opened = app.opener().open_url(url.clone(), None::<&str>).is_ok();
    Ok(LoginStart { opened, auth_url: if opened { String::new() } else { url } })
}

#[tauri::command]
pub async fn codex_login_cancel() -> Result<(), String> {
    blocking(|| {
        if let Some(l) = lock(&LOGIN).take() {
            cancel_login_on_server(l.id);
        }
    })
    .await
}

#[tauri::command]
pub async fn codex_logout() -> Result<(), String> {
    blocking(|| -> Result<(), String> {
        acquire()?.request("account/logout", Value::Null, CALL_TIMEOUT)?;
        set_signed_in(false);
        Ok(())
    })
    .await?
}

/// Model gói này dùng được: (id, tên hiển thị).
#[tauri::command]
pub async fn codex_models() -> Result<Vec<(String, String)>, String> {
    blocking(|| -> Result<Vec<(String, String)>, String> {
        let server = acquire()?;
        *lock(&server.models) = None; // nút ↻: hỏi lại, không dùng bản đã nhớ
        Ok(server.models()?.into_iter().map(|m| (m.id, m.name)).collect())
    })
    .await?
}

/// Cài / cập nhật Codex CLI vào thư mục của app bằng npm (không `npm -g`: không đụng
/// bản `codex` người dùng gõ ở terminal). Trả về version đã cài.
#[tauri::command]
pub async fn codex_install(app: AppHandle) -> Result<String, String> {
    blocking(move || -> Result<String, String> {
        let c = ctx()?;
        std::fs::create_dir_all(&c.install_root).map_err(|e| format!("Không tạo được thư mục cài: {e}"))?;
        let line = |text: &str| {
            let _ = app.emit("codex-install", text);
        };
        line(&format!("npm install {NPM_PACKAGE}@latest …"));
        // npm cần env của người dùng (registry, proxy, chứng chỉ) nên không lọc như Codex.
        let mut cmd = Command::new(if cfg!(windows) { "npm.cmd" } else { "npm" });
        cmd.args(["install", &format!("{NPM_PACKAGE}@latest"), "--prefix"])
            .arg(&c.install_root)
            .args(["--no-save", "--no-audit", "--no-fund", "--loglevel", "error"])
            .current_dir(&c.install_root)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x0800_0000);
        }
        let mut child = cmd.spawn().map_err(|e| format!("Không chạy được npm ({e}). Máy cần có Node.js để app tự cài Codex."))?;
        let errors = Arc::new(Mutex::new(Vec::<String>::new()));
        if let Some(stderr) = child.stderr.take() {
            let (errors, app) = (errors.clone(), app.clone());
            std::thread::spawn(move || {
                for l in BufReader::new(stderr).split(b'\n').map_while(Result::ok) {
                    let l = String::from_utf8_lossy(&l).trim().to_string();
                    if !l.is_empty() {
                        let _ = app.emit("codex-install", l.as_str());
                        lock(&errors).push(l);
                    }
                }
            });
        }
        let deadline = Instant::now() + INSTALL_TIMEOUT;
        let status = loop {
            match child.try_wait().map_err(|e| e.to_string())? {
                Some(s) => break s,
                None if Instant::now() >= deadline => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err("Cài Codex quá 5 phút, đã huỷ.".to_string());
                }
                None => std::thread::sleep(Duration::from_millis(200)),
            }
        };
        if !status.success() {
            let last = lock(&errors).last().cloned().unwrap_or_default();
            return Err(format!("npm install thất bại ({status}). {last}"));
        }
        // npm báo xong chưa chắc có binary chạy được: tìm và chạy thử.
        let binary = installed_candidates(&c.install_root, cfg!(windows))
            .into_iter()
            .find(|p| is_file(p))
            .ok_or("npm báo đã cài nhưng không thấy binary Codex trong thư mục vừa cài.")?;
        let version = version_of(&binary, &c.home).ok_or("Đã tải xong nhưng binary Codex không chạy được.")?;
        // Lần gọi sau dùng bản vừa cài.
        stop_server();
        Ok(version)
    })
    .await?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_candidates_try_exe_and_shims() {
        let got = path_candidates(r#"C:\a; "C:\b c" ;;"#, true);
        let names: Vec<String> = got.iter().map(|p| p.file_name().unwrap().to_string_lossy().into_owned()).collect();
        assert_eq!(names, ["codex.exe", "codex.cmd", "codex.bat", "codex.exe", "codex.cmd", "codex.bat"]);
        assert_eq!(got[3], Path::new(r"C:\b c").join("codex.exe"));
        assert_eq!(path_candidates("/usr/bin:/opt/x", false), [Path::new("/usr/bin").join("codex"), Path::new("/opt/x").join("codex")]);
        assert!(path_candidates("", true).is_empty());
    }

    #[test]
    fn installed_binary_is_in_the_platform_package() {
        let root = Path::new("root");
        let got = installed_candidates(root, true);
        let want = root.join("node_modules").join("@openai").join("codex-win32-x64").join("vendor").join("x86_64-pc-windows-msvc").join("bin").join("codex.exe");
        assert_eq!(got[0], want);
        assert!(got.iter().all(|p| p.ends_with("codex.exe")));
        assert!(installed_candidates(root, false).iter().all(|p| p.ends_with("codex")));
    }

    #[test]
    fn newest_desktop_build_with_an_exe_wins() {
        let e = |name: &str, mtime, has_exe| (PathBuf::from(name), mtime, has_exe);
        // Thư mục mới nhất mà rỗng thì bỏ qua.
        let got = pick_newest(&[e("old", 10, true), e("mid", 20, true), e("empty", 30, false)]);
        assert_eq!(got, Some(Path::new("mid").join("codex.exe")));
        assert_eq!(pick_newest(&[e("empty", 30, false)]), None);
    }

    #[test]
    fn env_is_allowlisted_and_home_is_ours() {
        let src = [
            ("Path", "C:\\bin"),
            ("OPENAI_API_KEY", "sk-x"),
            ("ANTHROPIC_API_KEY", "sk-ant-x"),
            ("CODEX_HOME", "C:\\Users\\deploy\\.codex"),
            ("USERPROFILE", "C:\\Users\\deploy"),
            ("TEMP", ""),
        ]
        .map(|(k, v)| (k.to_string(), v.to_string()));
        let env = child_env(src, Path::new("apphome"));
        let keys: Vec<&str> = env.iter().map(|(k, _)| k.as_str()).collect();
        assert_eq!(keys, ["Path", "USERPROFILE", "CODEX_HOME"]);
        assert_eq!(env.last().unwrap().1, "apphome");
    }

    #[test]
    fn classify_tells_the_three_message_kinds_apart() {
        assert_eq!(classify(r#"{"id":3,"result":{"a":1}}"#), Incoming::Response { id: 3, result: Ok(json!({"a":1})) });
        assert_eq!(
            classify(r#"{"id":4,"error":{"code":-1,"message":"boom"}}"#),
            Incoming::Response { id: 4, result: Err("boom".into()) }
        );
        assert_eq!(
            classify(r#"{"id":"srv-1","method":"item/commandExecution/requestApproval","params":{}}"#),
            Incoming::Request { id: json!("srv-1"), method: "item/commandExecution/requestApproval".into() }
        );
        assert_eq!(
            classify(r#"{"method":"turn/started","params":{"threadId":"t"}}"#),
            Incoming::Notification { method: "turn/started".into(), params: json!({"threadId":"t"}) }
        );
        assert_eq!(classify("not json"), Incoming::Unknown);
        assert_eq!(classify("[1]"), Incoming::Unknown);
        assert_eq!(classify("{}"), Incoming::Unknown);
    }

    #[test]
    fn server_requests_are_always_refused() {
        let id = json!(7);
        assert_eq!(refuse(&id, "item/commandExecution/requestApproval")["result"]["decision"], "decline");
        assert_eq!(refuse(&id, "item/fileChange/requestApproval")["result"]["decision"], "decline");
        assert_eq!(refuse(&id, "execCommandApproval")["result"]["decision"], "denied");
        let other = refuse(&id, "item/tool/requestUserInput");
        assert!(other.get("result").is_none() && other["error"]["code"] == -32601);
        assert_eq!(other["id"], 7);
    }

    #[test]
    fn account_needs_a_chatgpt_login() {
        let v = json!({"account":{"type":"chatgpt","email":"user@example.com","planType":"plus"},"requiresOpenaiAuth":true});
        assert_eq!(parse_account(&v), Some(Account { email: "user@example.com".into(), plan: "plus".into() }));
        assert_eq!(parse_account(&json!({"account":null,"requiresOpenaiAuth":true})), None);
        assert_eq!(parse_account(&json!({"account":{"type":"apiKey"}})), None);
        assert_eq!(
            parse_account(&json!({"account":{"type":"chatgpt","email":null,"planType":"go"}})),
            Some(Account { email: String::new(), plan: "go".into() })
        );
    }

    #[test]
    fn rate_limit_reads_the_primary_window() {
        let v = json!({"rateLimits":{"primary":{"usedPercent":41,"windowDurationMins":43200,"resetsAt":1797387187},"secondary":null,"rateLimitReachedType":null}});
        assert_eq!(parse_rate_limit(&v), Some(RateLimit { used_percent: 41, resets_at: Some(1797387187), reached: false }));
        let hit = json!({"rateLimits":{"primary":{"usedPercent":100},"rateLimitReachedType":"primary"}});
        assert_eq!(parse_rate_limit(&hit), Some(RateLimit { used_percent: 100, resets_at: None, reached: true }));
        assert_eq!(parse_rate_limit(&json!({"rateLimits":{"primary":null}})), None);
        assert_eq!(parse_rate_limit(&json!({})), None);
    }

    fn models() -> Vec<Model> {
        parse_models(&json!({"data":[
            {"id":"m-big","displayName":"Big","isDefault":true,"supportedReasoningEfforts":[{"reasoningEffort":"medium","description":""},{"reasoningEffort":"low","description":""}]},
            {"id":"m-odd","isDefault":false,"supportedReasoningEfforts":[{"reasoningEffort":"high","description":""}]},
            {"displayName":"không có id"}
        ]}))
    }

    #[test]
    fn models_and_lowest_effort() {
        let m = models();
        assert_eq!(m.len(), 2);
        assert_eq!((m[0].name.as_str(), m[1].name.as_str()), ("Big", "m-odd"));
        assert_eq!(pick_effort(&m, ""), Some("low".into()));
        assert_eq!(pick_effort(&m, " m-big "), Some("low".into()));
        // Model không có mức thấp, hoặc không biết model: không gửi gì.
        assert_eq!(pick_effort(&m, "m-odd"), None);
        assert_eq!(pick_effort(&m, "m-unknown"), None);
        assert_eq!(pick_effort(&[], ""), None);
    }

    #[test]
    fn history_is_flattened_around_the_new_message() {
        assert_eq!(flatten_history(&[]), "");
        assert_eq!(flatten_history(&[ChatMsg::user("chào", "")]), "chào");
        let msgs = [ChatMsg::user("q1", ""), ChatMsg::assistant("[happy] a1 "), ChatMsg::user("q2", "")];
        let got = flatten_history(&msgs);
        assert!(got.contains("User: q1\nAssistant: [happy] a1\n"), "{got}");
        assert!(got.ends_with("New message from the user:\nq2"), "{got}");
    }

    #[test]
    fn auth_errors_point_to_settings() {
        assert!(friendly_error("401 Unauthorized").contains("đăng nhập"));
        assert!(friendly_error("Your refresh token was revoked").contains("đăng nhập"));
        assert_eq!(friendly_error(" You've hit your usage limit. "), "ChatGPT: You've hit your usage limit.");
        // 503 không phải lỗi đăng nhập.
        assert!(!looks_like_auth_error("HTTP error: 503 Service Unavailable"));
    }

    fn feed_all(events: &[(&str, Value)]) -> (String, Option<TurnEvent>) {
        let mut r = TurnReducer::default();
        let mut text = String::new();
        for (method, params) in events {
            match r.feed(method, params) {
                TurnEvent::Delta(d) => text.push_str(&d),
                TurnEvent::Ignore => {}
                end => return (text, Some(end)),
            }
        }
        (text, None)
    }

    #[test]
    fn turn_streams_deltas_then_finishes() {
        let (text, end) = feed_all(&[
            ("turn/started", json!({"turn":{"id":"t1"}})),
            ("item/started", json!({"item":{"id":"u","type":"userMessage"}})),
            ("item/completed", json!({"item":{"id":"r","type":"reasoning"}})),
            ("item/agentMessage/delta", json!({"itemId":"a","delta":"[happy] Chào"})),
            ("item/agentMessage/delta", json!({"itemId":"a","delta":" anh"})),
            // Đã stream rồi: chữ đầy đủ ở đây không được phát lại.
            ("item/completed", json!({"item":{"id":"a","type":"agentMessage","text":"[happy] Chào anh"}})),
            ("turn/completed", json!({"turn":{"id":"t1","status":"completed"}})),
        ]);
        assert_eq!((text.as_str(), end), ("[happy] Chào anh", Some(TurnEvent::Done)));
    }

    #[test]
    fn turn_uses_completed_text_when_nothing_streamed() {
        let (text, end) = feed_all(&[
            ("item/completed", json!({"item":{"id":"a","type":"agentMessage","text":"một"}})),
            ("item/agentMessage/delta", json!({"itemId":"b","delta":"hai"})),
            ("turn/completed", json!({"turn":{"status":"interrupted"}})),
        ]);
        assert_eq!((text.as_str(), end), ("một\n\nhai", Some(TurnEvent::Done)));
    }

    #[test]
    fn turn_failure_carries_the_reason() {
        let (_, end) = feed_all(&[("turn/completed", json!({"turn":{"status":"failed","error":{"message":"usage limit"}}}))]);
        assert_eq!(end, Some(TurnEvent::Failed("usage limit".into())));
        // Lý do chỉ có ở thông báo `error` trước đó.
        let (_, end) = feed_all(&[
            ("error", json!({"error":{"message":"đang thử lại"},"willRetry":true})),
            ("error", json!({"error":{"message":"model not found"},"willRetry":false})),
            ("turn/completed", json!({"turn":{"status":"failed"}})),
        ]);
        assert_eq!(end, Some(TurnEvent::Failed("model not found".into())));
        let (_, end) = feed_all(&[("thread/status/changed", json!({"status":{"type":"systemError"}}))]);
        assert!(matches!(end, Some(TurnEvent::Failed(_))));
        let (_, end) = feed_all(&[("thread/status/changed", json!({"status":{"type":"idle"}}))]);
        assert_eq!(end, None);
    }

    /// Chạy thật với Codex trên máy (không gọi model, không tốn hạn mức): bắt tay,
    /// hỏi tài khoản, hỏi danh sách model. `cargo test codex_live -- --ignored`.
    #[test]
    #[ignore]
    fn codex_live_handshake() {
        let tmp = std::env::temp_dir().join(format!("dc-codex-live-{}", std::process::id()));
        let (home, work) = (tmp.join("home"), tmp.join("work"));
        std::fs::create_dir_all(&home).unwrap();
        std::fs::create_dir_all(&work).unwrap();
        let (binary, searched) = detect(&tmp.join("cli"));
        let binary = binary.unwrap_or_else(|| panic!("không thấy codex (đã tìm {searched} chỗ)"));
        eprintln!("codex: {} ({:?})", binary.display(), version_of(&binary, &home));
        let server = Server::spawn(&binary, &home, &work, true, Box::new(|_| {})).expect("spawn + initialize");
        // CODEX_HOME mới tinh: chưa đăng nhập.
        assert_eq!(server.account().unwrap(), None);
        let login = server.request("account/login/start", json!({ "type": "chatgpt" }), CALL_TIMEOUT).unwrap();
        assert!(login["authUrl"].as_str().unwrap().starts_with("https://"), "{login}");
        server.request("account/login/cancel", json!({ "loginId": login["loginId"] }), CALL_TIMEOUT).unwrap();
        server.shutdown();
        std::thread::sleep(Duration::from_millis(300));
        assert!(server.is_dead());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    /// Một lượt hỏi-đáp thật: TỐN một ít hạn mức của tài khoản đang đăng nhập trong
    /// CODEX_HOME được chỉ định. `DC_CODEX_HOME=<thư mục đã đăng nhập> cargo test
    /// codex_live_turn -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn codex_live_turn() {
        let home = PathBuf::from(std::env::var("DC_CODEX_HOME").expect("đặt DC_CODEX_HOME"));
        let work = std::env::temp_dir().join(format!("dc-codex-turn-{}", std::process::id()));
        std::fs::create_dir_all(&work).unwrap();
        let (binary, _) = detect(&work.join("cli"));
        let server = Server::spawn(&binary.expect("không thấy codex"), &home, &work, true, Box::new(|_| {})).expect("spawn");
        assert!(server.account().unwrap().is_some(), "CODEX_HOME này chưa đăng nhập");
        let mut chunks = 0;
        let reply = server
            .turn(
                "You are a tiny desktop pet. Always answer with exactly one short sentence that starts with [happy].",
                &flatten_history(&[ChatMsg::user("Tên em là Miu nhé", ""), ChatMsg::assistant("[happy] Dạ!"), ChatMsg::user("Em tên gì?", "")]),
                "",
                &work,
                &AtomicBool::new(false),
                |_| chunks += 1,
            )
            .expect("turn");
        eprintln!("reply ({chunks} mẩu): {reply}");
        assert!(reply.contains("[happy]") && reply.contains("Miu"), "{reply}");
        assert!(chunks > 1, "chữ phải về từng mẩu");
        server.shutdown();
        let _ = std::fs::remove_dir_all(&work);
    }
}
