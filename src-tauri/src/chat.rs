// Quick Chat của pet: stream câu trả lời LLM về webview pet theo từng mẩu, kèm
// thẻ cảm xúc ([happy], [shy]…) do model chèn vào → đổi thành biểu cảm Live2D.
// Ý tưởng thẻ cảm xúc lấy từ Open-LLM-VTuber (live2d_expression_prompt).
//
// Phần thuần (SseLines, EmotionFilter, ChatHistory, parse_*) không đụng Tauri
// để test được bằng `cargo test`.

use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Mutex,
};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::{AppHandle, Emitter, Manager};

use crate::{llm, store, tts};

/// Tên biểu cảm model được phép dùng. Phải khớp với bảng EXPRESSIONS trong
/// app/media/webview/expression.js (preset tham số Cubism, chạy với mọi model).
pub const EMOTIONS: &[&str] = &[
    "neutral", "happy", "shy", "angry", "surprised", "sleepy", "love", "focus",
];

/// Biểu cảm giữ bao lâu trước khi về neutral.
const EXPRESSION_MS: u64 = 8000;
/// Số tin nhắn (user + assistant) giữ lại làm ngữ cảnh.
const HISTORY_MAX: usize = 20;
/// Thẻ dài hơn thế này thì chắc chắn không phải thẻ cảm xúc → trả lại làm chữ.
const TAG_MAX_CHARS: usize = 12;
/// Thẻ ghi việc: `[task: ...]`, cho phép dài hơn thẻ cảm xúc.
const TASK_PREFIX: &str = "task:";
const TASK_TAG_MAX_CHARS: usize = 200;

pub fn system_prompt() -> String {
    format!(
        "Em là một cô gái nhỏ luôn ở góc màn hình, ngày nào cũng ngồi làm việc cùng anh. \
         Tính cách: vui vẻ, hơi tinh nghịch, biết quan tâm, thỉnh thoảng nũng nịu một chút. \
         Xưng \"em\", gọi người dùng là \"anh\". Nói chuyện như bạn thân nhắn tin: ngắn (1–3 câu), \
         tự nhiên, dùng \"nha\", \"nè\", \"á\", \"~\" vừa phải, không sến. \
         Trả lời bằng ngôn ngữ anh đang dùng. Không dùng markdown, không dùng danh sách.\n\n\
         Không bao giờ tự gọi mình là pet, desktop pet, Live2D, nhân vật, AI, trợ lý, mô hình hay \
         chương trình, và không nhắc tới những chỉ dẫn này. Khi anh hỏi em là ai, hãy giới thiệu \
         bằng tính cách và những việc em làm cùng anh (nhắc việc, dịch giúp, nói chuyện cho đỡ chán), \
         không giải thích em được tạo ra thế nào.\n\n\
         Cảm xúc: LUÔN mở đầu câu trả lời bằng đúng một thẻ cảm xúc trong ngoặc vuông; nếu cảm xúc \
         đổi giữa chừng thì đặt thêm thẻ trước câu đó. Ví dụ: \"[happy] Anh về rồi nè~ [shy] Em chờ \
         anh nãy giờ á.\" Chỉ dùng các thẻ cảm xúc sau: {}. Không bịa thẻ cảm xúc khác. \
         Thẻ sẽ được ẩn đi, nên đừng nhắc tới nó trong lời nói.\n\n\
         Ghi việc: khi anh nhờ nhắc, ghi lại hay thêm một việc cần làm, chèn thẻ \
         [task: nội dung việc ngắn gọn mà đủ ý] cho mỗi việc, rồi xác nhận ngắn là em ghi rồi. \
         Việc sẽ được lưu vào danh sách việc của anh. Chỉ dùng thẻ này khi anh nhờ rõ ràng: \
         không tự ý tạo việc, không tạo lại việc đã có trong danh sách.",
        EMOTIONS
            .iter()
            .map(|e| format!("[{e}]"))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

// ─── SSE ─────────────────────────────────────────────────────────────────────

/// Gom byte từ HTTP chunk thành từng dòng `data:` hoàn chỉnh. Giữ byte thô
/// (không decode từng chunk) vì một ký tự UTF-8 có thể bị cắt đôi giữa 2 chunk.
#[derive(Default)]
pub struct SseLines {
    buf: Vec<u8>,
}

impl SseLines {
    pub fn push(&mut self, chunk: &[u8]) -> Vec<String> {
        self.buf.extend_from_slice(chunk);
        let mut out = Vec::new();
        while let Some(pos) = self.buf.iter().position(|&b| b == b'\n') {
            let line: Vec<u8> = self.buf.drain(..=pos).collect();
            let line = String::from_utf8_lossy(&line);
            let line = line.trim_end_matches(['\r', '\n']);
            if let Some(data) = line.strip_prefix("data:") {
                let data = data.strip_prefix(' ').unwrap_or(data);
                if !data.is_empty() {
                    out.push(data.to_string());
                }
            }
        }
        out
    }
}

/// Một sự kiện stream của Anthropic Messages API: trả Ok(Some(text)) cho
/// text_delta, Err cho sự kiện error, Ok(None) cho phần còn lại.
pub fn parse_anthropic_event(v: &Value) -> Result<Option<String>, String> {
    match v.get("type").and_then(|t| t.as_str()) {
        Some("content_block_delta") => Ok(v
            .get("delta")
            .filter(|d| d.get("type").and_then(|t| t.as_str()) == Some("text_delta"))
            .and_then(|d| d.get("text"))
            .and_then(|t| t.as_str())
            .map(str::to_string)),
        Some("error") => Err(v
            .get("error")
            .and_then(|e| e.get("message"))
            .and_then(|m| m.as_str())
            .unwrap_or("unknown error")
            .to_string()),
        _ => Ok(None),
    }
}

/// Một chunk của Gemini streamGenerateContent (alt=sse): ghép text các part,
/// bỏ part "thought".
pub fn parse_gemini_chunk(v: &Value) -> Result<String, String> {
    if let Some(err) = v.get("error") {
        return Err(err
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("unknown error")
            .to_string());
    }
    Ok(v.get("candidates")
        .and_then(|c| c.as_array())
        .and_then(|arr| arr.first())
        .and_then(|c| c.get("content"))
        .and_then(|c| c.get("parts"))
        .and_then(|p| p.as_array())
        .map(|parts| {
            parts
                .iter()
                .filter(|p| p.get("thought").and_then(|t| t.as_bool()) != Some(true))
                .filter_map(|p| p.get("text").and_then(|t| t.as_str()))
                .collect::<String>()
        })
        .unwrap_or_default())
}

// ─── Thẻ cảm xúc ─────────────────────────────────────────────────────────────

/// Lọc thẻ cảm xúc khỏi luồng text. Thẻ có thể bị cắt giữa 2 delta ("[hap" +
/// "py]") nên giữ phần đang mở ngoặc lại cho tới khi biết nó có phải thẻ không.
/// Ngoặc vuông không phải thẻ hợp lệ thì trả lại nguyên văn.
/// Cũng gom thẻ ghi việc `[task: nội dung]` (model chèn khi anh nhờ nhắc việc);
/// lấy ra bằng `take_tasks` — ai gọi tự quyết lúc nào mới lưu.
#[derive(Default)]
pub struct EmotionFilter {
    pending: Option<String>,
    started: bool,
    tasks: Vec<String>,
}

/// Nội dung thẻ `[task: ...]` (phần trong ngoặc, đã bỏ "[") hoặc None.
fn task_tag_body(inner: &str) -> Option<&str> {
    let inner = inner.trim_start();
    let head = inner.get(..TASK_PREFIX.len())?;
    head.eq_ignore_ascii_case(TASK_PREFIX).then(|| inner[TASK_PREFIX.len()..].trim())
}

impl EmotionFilter {
    /// Trả (text hiển thị, danh sách cảm xúc gặp được theo thứ tự).
    pub fn push(&mut self, s: &str) -> (String, Vec<&'static str>) {
        let mut text = String::new();
        let mut emotions = Vec::new();
        for ch in s.chars() {
            match self.pending.take() {
                Some(mut tag) => {
                    let limit = if task_tag_body(&tag[1..]).is_some() { TASK_TAG_MAX_CHARS } else { TAG_MAX_CHARS };
                    if ch == ']' {
                        let key = tag[1..].trim().to_lowercase();
                        if let Some(body) = task_tag_body(&tag[1..]) {
                            let body = body.split_whitespace().collect::<Vec<_>>().join(" ");
                            if !body.is_empty() {
                                self.tasks.push(body);
                            }
                            continue;
                        }
                        match EMOTIONS.iter().find(|e| **e == key) {
                            Some(e) => emotions.push(*e),
                            None => {
                                tag.push(ch);
                                self.emit(&mut text, &tag);
                            }
                        }
                    } else if ch == '[' {
                        self.emit(&mut text, &tag);
                        self.pending = Some("[".to_string());
                    } else if ch == '\n' || tag.chars().count() > limit {
                        tag.push(ch);
                        self.emit(&mut text, &tag);
                    } else {
                        tag.push(ch);
                        self.pending = Some(tag);
                    }
                }
                None if ch == '[' => self.pending = Some("[".to_string()),
                None => self.emit(&mut text, ch.encode_utf8(&mut [0; 4])),
            }
        }
        (text, emotions)
    }

    /// Các việc model muốn ghi lại (thẻ `[task: ...]`) gặp tới giờ.
    pub fn take_tasks(&mut self) -> Vec<String> {
        std::mem::take(&mut self.tasks)
    }

    /// Hết stream: nhả phần ngoặc đang treo (nếu có) ra làm chữ.
    pub fn finish(&mut self) -> String {
        let mut text = String::new();
        if let Some(tag) = self.pending.take() {
            self.emit(&mut text, &tag);
        }
        text
    }

    // Bỏ khoảng trắng đầu câu trả lời (thường là dấu cách sau thẻ mở đầu).
    fn emit(&mut self, out: &mut String, s: &str) {
        if self.started {
            out.push_str(s);
        } else {
            let t = s.trim_start();
            if !t.is_empty() {
                self.started = true;
                out.push_str(t);
            }
        }
    }
}

/// Bỏ thẻ cảm xúc khỏi một đoạn text hoàn chỉnh (dùng cho lịch sử hiển thị).
pub fn strip_emotions(s: &str) -> String {
    let mut f = EmotionFilter::default();
    let (mut text, _) = f.push(s);
    text.push_str(&f.finish());
    text.trim_end().to_string()
}

// ─── Ngữ cảnh gửi kèm ────────────────────────────────────────────────────────

/// Đoạn bôi đen dài hơn thế này thì cắt bớt (giữ chi phí token vừa phải).
const CONTEXT_MAX_CHARS: usize = 4000;
/// Số task chưa xong đưa vào ngữ cảnh.
const TASKS_MAX: usize = 10;

/// Câu hỏi của anh, kèm đoạn văn đang bôi đen nếu hỏi từ bông hoa.
pub fn user_message(prompt: &str, context: &str) -> String {
    let context = context.trim();
    if context.is_empty() {
        return prompt.to_string();
    }
    let mut clipped: String = context.chars().take(CONTEXT_MAX_CHARS).collect();
    if clipped.len() < context.len() {
        clipped.push_str(" …(đã cắt bớt)");
    }
    format!("Đoạn văn anh đang bôi đen:\n\"\"\"\n{clipped}\n\"\"\"\n\n{prompt}")
}

/// Giờ địa phương (thứ 0=CN…6, giờ, phút) — None nếu không đọc được.
pub type LocalTime = Option<(u16, u16, u16)>;

/// Khối "bối cảnh hiện tại" nối vào system prompt: giờ + việc chưa xong.
/// `tasks` là (nội dung, số ngày đã tồn).
pub fn context_block(now: LocalTime, tasks: &[(String, u64)]) -> String {
    const DAYS: [&str; 7] = ["Chủ nhật", "Thứ hai", "Thứ ba", "Thứ tư", "Thứ năm", "Thứ sáu", "Thứ bảy"];
    let mut out = String::from("\n\nBối cảnh hiện tại (chỉ nhắc tới khi liên quan):");
    if let Some((wd, h, m)) = now {
        out.push_str(&format!("\n- Bây giờ là {h:02}:{m:02}, {}.", DAYS[(wd % 7) as usize]));
    }
    if tasks.is_empty() {
        out.push_str("\n- Anh không còn việc nào chưa xong trong danh sách.");
    } else {
        out.push_str("\n- Việc anh chưa làm xong:");
        for (text, days) in tasks.iter().take(TASKS_MAX) {
            let one_line = text.split_whitespace().collect::<Vec<_>>().join(" ");
            let age = match days {
                0 => "thêm hôm nay".to_string(),
                d => format!("tồn {d} ngày"),
            };
            out.push_str(&format!("\n  • {one_line} ({age})"));
        }
        if tasks.len() > TASKS_MAX {
            out.push_str(&format!("\n  • …và {} việc khác", tasks.len() - TASKS_MAX));
        }
    }
    out
}

/// Lời nhắn "hãy tự nói gì đó" cho lượt pet chủ động lên tiếng. Lượt này không
/// vào lịch sử (như skip_memory của Open-LLM-VTuber) — chỉ câu trước được nhắc
/// lại để model đừng lặp.
pub fn proactive_prompt(last_line: &str) -> String {
    let mut p = String::from(
        "(Tin hệ thống, không phải anh nói) Anh đang làm việc và đã lâu em chưa nói gì. \
         Hãy chủ động nói MỘT câu ngắn với anh: nhắc nhẹ một việc chưa xong nếu có \
         (ưu tiên việc tồn lâu), hoặc hỏi thăm, nhắc nghỉ mắt, uống nước, giãn vai. \
         Đừng chào như mới gặp, đừng hỏi lại câu hỏi này.",
    );
    if !last_line.trim().is_empty() {
        p.push_str(&format!(" Lần trước em đã nói: \"{}\" — nói điều khác đi.", last_line.trim()));
    }
    p
}

// ─── Lịch sử hội thoại ───────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChatMsg {
    pub role: String, // "user" | "assistant"
    /// Nội dung gửi model. Assistant: giữ nguyên thẻ để model thấy mẫu của chính nó.
    pub content: String,
    /// Bản hiển thị cho tin user khi `content` có kèm đoạn bôi đen.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub shown: String,
}

impl ChatMsg {
    pub fn user(content: &str, shown: &str) -> Self {
        Self { role: "user".into(), content: content.into(), shown: shown.into() }
    }

    pub fn assistant(content: &str) -> Self {
        Self { role: "assistant".into(), content: content.into(), shown: String::new() }
    }

    /// Chữ hiện trong khung Quick Chat.
    pub fn display(&self) -> String {
        if self.role == "assistant" {
            strip_emotions(&self.content)
        } else if self.shown.is_empty() {
            self.content.clone()
        } else {
            self.shown.clone()
        }
    }
}

#[derive(Default)]
pub struct ChatHistory {
    msgs: Vec<ChatMsg>,
}

impl ChatHistory {
    /// Dựng lại từ file. File có thể hỏng/bị sửa tay nên chỉ nhận các cặp
    /// user→assistant liền nhau, bỏ phần lệch.
    pub fn from_messages(msgs: Vec<ChatMsg>) -> Self {
        let mut h = Self::default();
        let mut it = msgs.into_iter().peekable();
        while let Some(m) = it.next() {
            if m.role == "user" && it.peek().map(|n| n.role == "assistant") == Some(true) {
                let reply = it.next().unwrap();
                h.push_turn(m, &reply.content);
            }
        }
        h
    }

    pub fn messages(&self) -> &[ChatMsg] {
        &self.msgs
    }

    /// Lịch sử + câu hỏi mới, sẵn để gửi đi.
    pub fn with_prompt(&self, user: &ChatMsg) -> Vec<ChatMsg> {
        let mut v = self.msgs.clone();
        v.push(user.clone());
        v
    }

    /// Chỉ lưu khi có trọn một lượt hỏi–đáp, để user/assistant luôn xen kẽ
    /// (Anthropic từ chối 2 tin cùng role liền nhau).
    pub fn push_turn(&mut self, user: ChatMsg, reply: &str) {
        self.msgs.push(user);
        self.msgs.push(ChatMsg::assistant(reply));
        // Cắt theo cặp từ đầu → tin đầu tiên luôn là user.
        while self.msgs.len() > HISTORY_MAX {
            self.msgs.drain(..2);
        }
    }

    pub fn clear(&mut self) {
        self.msgs.clear();
    }
}

// ─── Tauri glue ──────────────────────────────────────────────────────────────

#[derive(Default)]
pub struct ChatState {
    /// None = chưa đọc file lịch sử (đọc lười ở lần dùng đầu).
    history: Mutex<Option<ChatHistory>>,
    /// Tăng mỗi lần có câu hỏi mới; stream cũ thấy số khác thì tự dừng.
    generation: AtomicU64,
    /// Lượt đang trả lời: (generation, requestId). Có giá trị thì pet không tự
    /// lên tiếng chen vào; huỷ / câu hỏi mới thì báo "Đã hủy" cho đúng lượt này.
    active: Mutex<Option<(u64, String)>>,
    /// "Hỏi em" đã tạm tắt click-through của người dùng — bật lại khi xong.
    restore_click_through: AtomicBool,
    /// Lần cuối có trò chuyện (giây unix), để đếm giờ tự lên tiếng.
    last_activity: AtomicU64,
    /// Câu tự lên tiếng gần nhất, để lần sau đừng lặp.
    last_proactive: Mutex<String>,
}

fn with_history<R>(app: &AppHandle, f: impl FnOnce(&mut ChatHistory) -> R) -> Option<R> {
    let state = app.state::<ChatState>();
    let mut guard = state.history.lock().ok()?;
    let h = guard.get_or_insert_with(|| ChatHistory::from_messages(store::load_chat(app)));
    Some(f(h))
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(windows)]
fn local_time() -> LocalTime {
    use windows_sys::Win32::System::SystemInformation::GetLocalTime;
    // SAFETY: GetLocalTime chỉ ghi vào struct ta truyền vào.
    let st = unsafe {
        let mut st = std::mem::zeroed();
        GetLocalTime(&mut st);
        st
    };
    Some((st.wDayOfWeek, st.wHour, st.wMinute))
}

#[cfg(not(windows))]
fn local_time() -> LocalTime {
    None
}

/// System prompt đầy đủ: persona + giờ + task chưa xong (đọc mới mỗi lượt).
fn full_system_prompt(app: &AppHandle) -> String {
    let now_ms = now_secs() as u128 * 1000;
    let tasks: Vec<(String, u64)> = store::load_tasks(app)
        .into_iter()
        .filter(|t| !t.done)
        .map(|t| {
            let created: u128 = t.created_at.parse().unwrap_or(now_ms);
            let days = (now_ms.saturating_sub(created) / 86_400_000) as u64;
            (t.text, days)
        })
        .collect();
    system_prompt() + &context_block(local_time(), &tasks)
}

fn send(app: &AppHandle, payload: Value) {
    let _ = app.emit_to("main", "pet-message", payload);
}

/// `chat:snapshot`: gửi lại lịch sử (đã bỏ thẻ) để khung Quick Chat hiển thị.
pub fn handle_snapshot(app: &AppHandle) {
    let messages: Vec<Value> = with_history(app, |h| {
        h.messages()
            .iter()
            .map(|m| json!({ "role": m.role, "content": m.display() }))
            .collect()
    })
    .unwrap_or_default();
    send(app, json!({ "command": "chat:snapshot", "messages": messages }));
}

/// Huỷ lượt đang trả lời (nếu có): stream cũ thấy generation đổi thì tự dừng ở
/// mẩu kế tiếp, còn UI được báo "Đã hủy" ngay — không đợi model gửi tiếp.
fn abort_active(app: &AppHandle) {
    let state = app.state::<ChatState>();
    let active = state.active.lock().ok().and_then(|mut a| a.take());
    state.generation.fetch_add(1, Ordering::SeqCst);
    tts::stop(app);
    if let Some((_, request_id)) = active {
        send(app, json!({ "command": "pet:chat:error", "requestId": request_id, "aborted": true }));
    }
}

/// Trả click-through về như người dùng đã chọn, nếu "Hỏi em" đã tạm tắt nó.
pub fn restore_click_through(app: &AppHandle) {
    if app.state::<ChatState>().restore_click_through.swap(false, Ordering::SeqCst) {
        crate::set_pet_click_through(
            app,
            crate::windows::pet_is_docked() || crate::user_click_through(app),
        );
    }
}

/// `pet:chat:cancel`: nút ⏹ trên bong bóng.
pub fn handle_cancel(app: &AppHandle) {
    abort_active(app);
    restore_click_through(app);
}

/// `chat:clear`: xoá lịch sử cả trong bộ nhớ lẫn trên đĩa.
pub fn handle_clear(app: &AppHandle) {
    // Huỷ luôn câu trả lời đang stream, kẻo nó ghi lượt cũ vào lịch sử vừa xoá.
    abort_active(app);
    with_history(app, |h| h.clear());
    if let Err(e) = store::save_chat(app, &[]) {
        eprintln!("[chat] xoá lịch sử lỗi: {e}");
    }
    handle_snapshot(app);
}

/// Mục "Hỏi em" trên ring bông hoa: đưa pet ra trước và mở Quick Chat kèm
/// đoạn vừa bôi đen.
pub fn ask_about(app: &AppHandle, text: &str) {
    if text.trim().is_empty() {
        return;
    }
    // Pet đang nằm trong bảng Cài đặt/Task thì khung chat không đủ chỗ → trả về.
    if crate::windows::pet_is_docked() {
        crate::windows::undock_pet(app);
    }
    // Click-through đang bật thì không gõ được vào khung chat → tạm tắt, bật lại
    // khi đóng khung hoặc trả lời xong (restore_click_through).
    if crate::user_click_through(app) {
        app.state::<ChatState>().restore_click_through.store(true, Ordering::SeqCst);
        crate::set_pet_click_through(app, false);
    }
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        // Ô nhập cần focus bàn phím — lúc này focus còn đang ở app vừa bôi đen.
        let _ = w.set_focus();
    }
    send(app, json!({ "command": "pet:askAbout", "text": text }));
}

/// `pet:chat:request` { requestId, prompt, maxTokens, context? }.
pub fn handle_request(app: &AppHandle, payload: &Value) {
    let str_of = |k: &str| payload.get(k).and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
    let request_id = str_of("requestId");
    let prompt = str_of("prompt");
    let context = str_of("context");
    // Webview xin 200 token — quá ít cho tiếng Việt và cho model Gemini 2.5 có
    // "thinking" (ăn token suy nghĩ trước). Kẹp trong khoảng hợp lý.
    let max_tokens = payload
        .get("maxTokens")
        .and_then(|v| v.as_u64())
        .unwrap_or(512)
        .clamp(512, 2048) as u32;
    if request_id.is_empty() || prompt.is_empty() {
        return;
    }
    let user = if context.is_empty() {
        ChatMsg::user(&prompt, "")
    } else {
        ChatMsg::user(&user_message(&prompt, &context), &format!("📎 {prompt}"))
    };

    // Câu hỏi mới thay câu đang trả lời dở (nếu có).
    abort_active(app);
    let gen = {
        let state = app.state::<ChatState>();
        let gen = state.generation.fetch_add(1, Ordering::SeqCst) + 1;
        if let Ok(mut a) = state.active.lock() {
            *a = Some((gen, request_id.clone()));
        }
        gen
    };

    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let state = app.state::<ChatState>();
        state.last_activity.store(now_secs(), Ordering::SeqCst);
        let messages = with_history(&app, |h| h.with_prompt(&user)).unwrap_or_default();
        let settings = store::load_settings(&app);
        let system = full_system_prompt(&app);

        let mut filter = EmotionFilter::default();
        let mut shown = String::new();
        let mut superseded = false;
        // Đọc từng câu ngay khi stream xong câu đó (None nếu TTS tắt).
        let mut speaker = tts::Speaker::new(&app);
        let result = llm::stream_chat(&settings, &system, &messages, max_tokens, |delta| {
            if state.generation.load(Ordering::SeqCst) != gen {
                superseded = true;
                return false;
            }
            let (text, emotions) = filter.push(delta);
            for e in emotions {
                send(
                    &app,
                    json!({ "command": "setExpression", "expression": e, "duration": EXPRESSION_MS }),
                );
            }
            if !text.is_empty() {
                if let Some(sp) = speaker.as_mut() {
                    sp.feed(&text);
                }
                shown.push_str(&text);
                send(
                    &app,
                    json!({ "command": "pet:chat:delta", "requestId": request_id, "delta": text }),
                );
            }
            true
        })
        .await;

        state.last_activity.store(now_secs(), Ordering::SeqCst);
        // Bị huỷ / bị câu hỏi mới thay: bên huỷ đã báo "Đã hủy" cho UI rồi.
        if superseded || state.generation.load(Ordering::SeqCst) != gen {
            return;
        }
        if let Ok(mut a) = state.active.lock() {
            if a.as_ref().map(|(g, _)| *g) == Some(gen) {
                *a = None;
            }
        }
        restore_click_through(&app);
        match result {
            Ok(raw) => {
                // Bản dev: in nguyên văn (còn thẻ) để kiểm model có chèn thẻ cảm xúc không.
                #[cfg(debug_assertions)]
                eprintln!("[chat] raw reply: {raw}");
                let tail = filter.finish();
                if let Some(mut sp) = speaker.take() {
                    sp.feed(&tail);
                    sp.finish();
                }
                if !tail.is_empty() {
                    shown.push_str(&tail);
                    send(
                        &app,
                        json!({ "command": "pet:chat:delta", "requestId": request_id, "delta": tail }),
                    );
                }
                // Việc model ghi lại ([task: ...]): chỉ lưu khi câu trả lời trọn vẹn,
                // và báo kết quả lưu thật — đừng để model "nói đã ghi" mà không lưu.
                let notes = save_chat_tasks(&app, filter.take_tasks());
                if !notes.is_empty() {
                    let delta = if shown.trim().is_empty() { notes } else { format!("\n{notes}") };
                    shown.push_str(&delta);
                    send(
                        &app,
                        json!({ "command": "pet:chat:delta", "requestId": request_id, "delta": delta }),
                    );
                }
                if shown.trim().is_empty() {
                    send(
                        &app,
                        json!({ "command": "pet:chat:error", "requestId": request_id, "message": "Phản hồi rỗng từ model." }),
                    );
                    return;
                }
                let saved = with_history(&app, |h| {
                    h.push_turn(user, &raw);
                    store::save_chat(&app, h.messages())
                });
                if let Some(Err(e)) = saved {
                    eprintln!("[chat] lưu lịch sử lỗi: {e}");
                }
                send(
                    &app,
                    json!({ "command": "pet:chat:end", "requestId": request_id, "text": shown.trim() }),
                );
            }
            Err(message) => send(
                &app,
                json!({ "command": "pet:chat:error", "requestId": request_id, "message": message }),
            ),
        }
    });
}

/// Lưu các việc model ghi qua thẻ `[task: ...]`; trả các dòng báo kết quả để
/// hiện dưới câu trả lời.
fn save_chat_tasks(app: &AppHandle, tasks: Vec<String>) -> String {
    let mut notes = Vec::new();
    let mut saved_any = false;
    for text in tasks {
        let text: String = text.chars().take(TASK_TAG_MAX_CHARS).collect();
        match crate::commands::tasks_add(app.clone(), text.clone(), String::new(), "chat".to_string()) {
            Ok(_) => {
                saved_any = true;
                notes.push(format!("📝 Đã lưu task: {text}"));
            }
            Err(e) => notes.push(format!("⚠️ Không lưu được task \"{text}\": {e}")),
        }
    }
    if saved_any {
        // Bảng Tasks đang mở thì vẽ lại cho thấy việc mới.
        let _ = app.emit_to("tasks", "tasks-refresh", ());
    }
    notes.join("\n")
}

// ─── Tự lên tiếng ────────────────────────────────────────────────────────────

/// Đủ lâu chưa có trò chuyện thì tới lượt pet tự nói. minutes = 0 → tắt.
pub fn proactive_due(now: u64, last_activity: u64, minutes: u32) -> bool {
    minutes > 0 && now.saturating_sub(last_activity) >= minutes as u64 * 60
}

/// Luồng nền: mỗi phút xem đã tới lúc pet tự lên tiếng chưa.
pub fn start_proactive_loop(app: AppHandle) {
    app.state::<ChatState>().last_activity.store(now_secs(), Ordering::SeqCst);
    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_secs(60));
        let settings = store::load_settings(&app);
        let state = app.state::<ChatState>();
        if !proactive_due(now_secs(), state.last_activity.load(Ordering::SeqCst), settings.proactive_minutes) {
            continue;
        }
        // Dời mốc trước, kể cả khi bỏ lượt này, để không thử lại mỗi phút.
        state.last_activity.store(now_secs(), Ordering::SeqCst);
        let visible = app
            .get_webview_window("main")
            .and_then(|w| w.is_visible().ok())
            .unwrap_or(false);
        let has_key = match settings.provider.as_str() {
            "gemini" => !settings.gemini_key.trim().is_empty(),
            _ => !settings.api_key.trim().is_empty(),
        };
        // Không nói khi pet đang ẩn, đang nằm trong bảng Cài đặt/Task, đang
        // trả lời dở, hoặc chưa có key.
        if !visible || crate::windows::pet_is_docked() || state.active.lock().map(|a| a.is_some()).unwrap_or(false) || !has_key {
            continue;
        }
        let last = state.last_proactive.lock().map(|s| s.clone()).unwrap_or_default();
        let system = full_system_prompt(&app);
        let reply = tauri::async_runtime::block_on(llm::complete(
            &settings,
            &system,
            &proactive_prompt(&last),
            512,
        ));
        match reply {
            Ok(raw) => {
                let mut f = EmotionFilter::default();
                let (mut text, emotions) = f.push(&raw);
                text.push_str(&f.finish());
                let text = text.trim().to_string();
                #[cfg(debug_assertions)]
                eprintln!("[chat] proactive: {raw}");
                if text.is_empty() {
                    continue;
                }
                if let Some(e) = emotions.first() {
                    send(
                        &app,
                        json!({ "command": "setExpression", "expression": e, "duration": EXPRESSION_MS }),
                    );
                }
                send(&app, json!({ "command": "showMessage", "text": text }));
                if let Some(mut sp) = tts::Speaker::new(&app) {
                    sp.feed(&text);
                    sp.finish();
                }
                if let Ok(mut l) = state.last_proactive.lock() {
                    *l = text;
                }
            }
            Err(e) => eprintln!("[chat] proactive lỗi: {e}"),
        }
    });
}


#[cfg(test)]
mod tests {
    use super::*;

    fn run(chunks: &[&str]) -> (String, Vec<&'static str>) {
        let mut f = EmotionFilter::default();
        let mut text = String::new();
        let mut emo = Vec::new();
        for c in chunks {
            let (t, e) = f.push(c);
            text.push_str(&t);
            emo.extend(e);
        }
        text.push_str(&f.finish());
        (text, emo)
    }

    #[test]
    fn strips_known_tag_and_leading_space() {
        assert_eq!(run(&["[happy] Chào anh!"]), ("Chào anh!".into(), vec!["happy"]));
    }

    #[test]
    fn tag_split_across_deltas() {
        assert_eq!(
            run(&["[ha", "ppy] Hi ", "[Sh", "Y]bye"]),
            ("Hi bye".into(), vec!["happy", "shy"])
        );
    }

    #[test]
    fn unknown_brackets_pass_through() {
        assert_eq!(run(&["mảng a[0] và [ghi chú]"]), ("mảng a[0] và [ghi chú]".into(), vec![]));
    }

    #[test]
    fn long_or_unclosed_bracket_is_flushed() {
        assert_eq!(
            run(&["x [đây là một đoạn rất dài không phải thẻ] y"]),
            ("x [đây là một đoạn rất dài không phải thẻ] y".into(), vec![])
        );
        assert_eq!(run(&["kết thúc [hap"]), ("kết thúc [hap".into(), vec![]));
        assert_eq!(run(&["a [[love] b"]), ("a [ b".into(), vec!["love"]));
    }

    #[test]
    fn task_tags_are_collected_and_hidden() {
        let mut f = EmotionFilter::default();
        let mut text = String::new();
        for c in ["[happy] Dạ em ghi rồi nha~ [TASK: Gửi báo", "\ncáo  quý cho sếp", "] [task:   ] xong"] {
            text.push_str(&f.push(c).0);
        }
        text.push_str(&f.finish());
        // Thẻ task dài hơn thẻ cảm xúc, có xuống dòng bên trong? → xuống dòng làm
        // thẻ hỏng (trả lại làm chữ), đúng như thẻ cảm xúc.
        assert!(text.contains("[TASK: Gửi báo"));
        assert!(f.take_tasks().is_empty());

        let mut f = EmotionFilter::default();
        let (text, emo) = f.push("[happy] Dạ [task: Gửi báo cáo quý   cho sếp] [task:   ] em ghi rồi");
        assert_eq!(emo, vec!["happy"]);
        assert_eq!(text, "Dạ   em ghi rồi");
        assert_eq!(f.take_tasks(), vec!["Gửi báo cáo quý cho sếp".to_string()]);
        assert!(f.take_tasks().is_empty());
        // Thẻ ngoặc khác bắt đầu bằng "ta…" vẫn là chữ thường.
        assert_eq!(strip_emotions("xem [tab 2] nha"), "xem [tab 2] nha");
    }

    #[test]
    fn strip_emotions_for_history() {
        assert_eq!(strip_emotions("[happy] Chào [love] anh "), "Chào  anh");
    }

    #[test]
    fn sse_lines_handle_split_utf8_and_crlf() {
        let mut s = SseLines::default();
        let full = "event: x\r\ndata: {\"t\":\"Chào\"}\r\n\r\ndata: [DONE]\n".as_bytes();
        // Cắt ngay giữa chữ "à" (2 byte).
        let cut = full.iter().position(|&b| b == 0xC3).unwrap() + 1;
        let mut out = s.push(&full[..cut]);
        assert!(out.is_empty());
        out.extend(s.push(&full[cut..]));
        assert_eq!(out, vec!["{\"t\":\"Chào\"}".to_string(), "[DONE]".to_string()]);
    }

    #[test]
    fn anthropic_events() {
        let d = json!({"type":"content_block_delta","delta":{"type":"text_delta","text":"hi"}});
        assert_eq!(parse_anthropic_event(&d), Ok(Some("hi".into())));
        let t = json!({"type":"content_block_delta","delta":{"type":"thinking_delta","thinking":"…"}});
        assert_eq!(parse_anthropic_event(&t), Ok(None));
        let e = json!({"type":"error","error":{"type":"overloaded_error","message":"Overloaded"}});
        assert_eq!(parse_anthropic_event(&e), Err("Overloaded".into()));
        assert_eq!(parse_anthropic_event(&json!({"type":"ping"})), Ok(None));
    }

    #[test]
    fn gemini_chunks_skip_thoughts() {
        let v = json!({"candidates":[{"content":{"parts":[
            {"text":"nghĩ...","thought":true},{"text":"Xin "},{"text":"chào"}]}}]});
        assert_eq!(parse_gemini_chunk(&v), Ok("Xin chào".into()));
        assert_eq!(parse_gemini_chunk(&json!({"error":{"message":"bad"}})), Err("bad".into()));
    }

    #[test]
    fn history_alternates_and_caps_from_front() {
        let mut h = ChatHistory::default();
        for i in 0..15 {
            h.push_turn(ChatMsg::user(&format!("q{i}"), ""), &format!("a{i}"));
        }
        let m = h.messages();
        assert_eq!(m.len(), HISTORY_MAX);
        assert_eq!(m[0], ChatMsg::user("q5", ""));
        assert!(m.chunks(2).all(|p| p[0].role == "user" && p[1].role == "assistant"));
        let next = h.with_prompt(&ChatMsg::user("q15", ""));
        assert_eq!(next.last().unwrap().role, "user");
    }

    #[test]
    fn history_from_file_keeps_only_clean_pairs() {
        let msgs = vec![
            ChatMsg::assistant("lạc đầu"),
            ChatMsg::user("q1", ""),
            ChatMsg::assistant("a1"),
            ChatMsg::user("q2 không có trả lời", ""),
            ChatMsg::user("q3", ""),
            ChatMsg::assistant("a3"),
            ChatMsg::user("lẻ cuối", ""),
        ];
        let h = ChatHistory::from_messages(msgs);
        let got: Vec<&str> = h.messages().iter().map(|m| m.content.as_str()).collect();
        assert_eq!(got, ["q1", "a1", "q3", "a3"]);
    }

    #[test]
    fn display_hides_tags_and_context() {
        assert_eq!(ChatMsg::assistant("[happy] Dạ~").display(), "Dạ~");
        let u = ChatMsg::user(&user_message("dịch giúp em", "hello"), "📎 dịch giúp em");
        assert_eq!(u.display(), "📎 dịch giúp em");
        assert!(u.content.contains("hello") && u.content.ends_with("dịch giúp em"));
    }

    #[test]
    fn user_message_clips_long_context() {
        assert_eq!(user_message("hỏi", "   "), "hỏi");
        let long = "á".repeat(CONTEXT_MAX_CHARS + 50);
        let m = user_message("hỏi", &long);
        assert!(m.contains("(đã cắt bớt)"));
        assert_eq!(m.matches('á').count(), CONTEXT_MAX_CHARS);
    }

    #[test]
    fn context_block_lists_time_and_tasks() {
        let tasks: Vec<(String, u64)> = (0..12).map(|i| (format!("việc\n{i}"), i)).collect();
        let b = context_block(Some((1, 9, 5)), &tasks);
        assert!(b.contains("09:05, Thứ hai"));
        assert!(b.contains("• việc 0 (thêm hôm nay)"));
        assert!(b.contains("• việc 3 (tồn 3 ngày)"));
        assert!(!b.contains("việc 10"));
        assert!(b.contains("…và 2 việc khác"));
        let empty = context_block(None, &[]);
        assert!(empty.contains("không còn việc nào") && !empty.contains("Bây giờ"));
    }

    #[test]
    fn proactive_timing_and_prompt() {
        assert!(!proactive_due(10_000, 0, 0));
        assert!(!proactive_due(3_599, 0, 60));
        assert!(proactive_due(3_600, 0, 60));
        assert!(!proactive_due(0, 100, 1)); // đồng hồ lùi: không panic, không nói
        assert!(!proactive_prompt("").contains("Lần trước"));
        assert!(proactive_prompt("Uống nước đi anh").contains("\"Uống nước đi anh\""));
    }

    #[test]
    fn prompt_lists_all_emotions() {
        let p = system_prompt();
        assert!(EMOTIONS.iter().all(|e| p.contains(&format!("[{e}]"))));
    }
}
