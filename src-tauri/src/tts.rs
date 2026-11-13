// Đọc thành tiếng lời của pet (TTS). Ba engine, chọn trong Cài đặt:
//   "windows" — WinRT SpeechSynthesizer: giọng có sẵn của Windows, offline.
//   "edge"    — giọng Neural của Microsoft Edge "Read aloud" qua WebSocket.
//               Dịch vụ KHÔNG chính thức: văn bản đi lên máy chủ Microsoft và
//               có thể ngừng hoạt động bất cứ lúc nào.
//   "server"  — server giọng nói do người dùng tự chạy, nói giao thức
//               POST /v1/audio/speech kiểu OpenAI (VoiceStudio, OmniVoice…).
//               Dùng cho giọng nhân bản: model quá nặng để nhúng vào app.
//
// Luồng (theo Open-LLM-VTuber): câu trả lời đang stream được cắt thành từng câu
// (SentenceSplitter) → lọc chữ không nên đọc (clean_for_tts) → một luồng worker
// tổng hợp tuần tự nên audio tới webview đúng thứ tự → webview phát và đo âm
// lượng để mấp máy miệng (ParamMouthOpenY).

use std::sync::{
    atomic::{AtomicU64, Ordering},
    mpsc, OnceLock,
};

use base64::Engine as _;
use serde_json::json;
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Emitter};

use crate::store;

/// Tạm tắt các engine giọng nói (Windows / Edge / server tự chạy): giọng có sẵn
/// không khớp nhân vật, còn giọng nhân bản cần server quá nặng với máy người dùng.
/// Clip thu sẵn (`say`) KHÔNG phụ thuộc cờ này và vẫn phát.
/// Bật lại = đổi thành true + bỏ style="display:none" của nhóm "Giọng nói" trong
/// app/settings.html. Khi tắt, setting tts_engine đã lưu bị bỏ qua.
/// Bật lại thì thêm vào bảng "Data sent over the network" của
/// docs/landing/privacy.html trước khi phát hành: giọng Edge (văn bản gửi lên máy
/// chủ Microsoft) và server giọng nói (văn bản gửi tới địa chỉ người dùng nhập).
pub const TTS_ENABLED: bool = false;

/// Địa chỉ mặc định của server giọng nói trên máy (cổng mặc định của VoiceStudio).
pub const DEFAULT_SERVER_URL: &str = "http://127.0.0.1:3900";

/// Giọng Edge cho người dùng chọn (id, nhãn hiển thị).
pub const EDGE_VOICES: &[(&str, &str)] = &[
    ("vi-VN-HoaiMyNeural", "Hoài My — nữ, tiếng Việt"),
    ("vi-VN-NamMinhNeural", "Nam Minh — nam, tiếng Việt"),
    ("ja-JP-NanamiNeural", "Nanami — nữ, tiếng Nhật"),
    ("ja-JP-AoiNeural", "Aoi — nữ, tiếng Nhật"),
    ("en-US-AnaNeural", "Ana — bé gái, tiếng Anh (Mỹ)"),
    ("en-US-AriaNeural", "Aria — nữ, tiếng Anh (Mỹ)"),
    ("en-US-JennyNeural", "Jenny — nữ, tiếng Anh (Mỹ)"),
    ("en-GB-SoniaNeural", "Sonia — nữ, tiếng Anh (Anh)"),
    ("zh-CN-XiaoyiNeural", "Xiaoyi — nữ, tiếng Trung"),
    ("ko-KR-SunHiNeural", "SunHi — nữ, tiếng Hàn"),
];

// ─── Lọc chữ trước khi đọc ───────────────────────────────────────────────────

/// Bỏ những thứ không nên đọc thành tiếng: *hành động*, nội dung trong ngoặc
/// (kể cả lồng nhau), emoji/ký hiệu, dấu markdown. Chỉ dùng cho TTS — chữ
/// hiển thị giữ nguyên.
pub fn clean_for_tts(s: &str) -> String {
    // Bước 1: bỏ *...* (chỉ khi có cặp; dấu * lẻ bị bỏ ở bước 3).
    let mut no_actions = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(start) = rest.find('*') {
        let after = &rest[start + 1..];
        let open_len = after.len() - after.trim_start_matches('*').len();
        let body = &after[open_len..];
        match body.find('*') {
            Some(end) => {
                no_actions.push_str(&rest[..start]);
                let close = &body[end..];
                rest = close.trim_start_matches('*');
            }
            None => break,
        }
    }
    no_actions.push_str(rest);

    // Bước 2: bỏ nội dung trong ngoặc, đếm độ sâu để xử lý ngoặc lồng.
    let mut depth = 0usize;
    let mut no_brackets = String::with_capacity(no_actions.len());
    for ch in no_actions.chars() {
        match ch {
            '(' | '[' | '{' | '<' | '（' | '【' => depth += 1,
            ')' | ']' | '}' | '>' | '）' | '】' if depth > 0 => depth -= 1,
            _ if depth == 0 => no_brackets.push(ch),
            _ => {}
        }
    }

    // Bước 3: chỉ giữ chữ, số, khoảng trắng, dấu câu đọc được và dấu thanh
    // dạng tổ hợp (U+0300–U+036F, khi model trả tiếng Việt chưa dựng sẵn NFC).
    const KEEP: &str = ".,!?;:'\"-…。、，！？「」%$/&+=";
    let filtered: String = no_brackets
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || KEEP.contains(c) || ('\u{0300}'..='\u{036F}').contains(&c) {
                c
            } else {
                ' '
            }
        })
        .collect();
    filtered.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Còn gì để đọc không (chỉ toàn dấu câu thì thôi).
pub fn speakable(s: &str) -> bool {
    s.chars().any(|c| c.is_alphanumeric())
}

// ─── Cắt câu khi đang stream ─────────────────────────────────────────────────

/// Gom text đang stream, nhả ra từng câu hoàn chỉnh để đọc ngay. Câu đầu được
/// cắt sớm ở dấu phẩy (faster_first_response của Open-LLM-VTuber) để pet mở
/// miệng nhanh hơn.
#[derive(Default)]
pub struct SentenceSplitter {
    buf: String,
    emitted: bool,
}

const SENTENCE_ENDS: &[char] = &['.', '!', '?', '…', '。', '！', '？'];
const SOFT_BREAKS: &[char] = &[',', '，', '、', ';', ':'];
/// Câu đầu dài ít nhất ngần này ký tự mới cắt ở dấu phẩy.
const FIRST_CHUNK_MIN: usize = 8;
/// Câu dài quá mức này thì cắt ở dấu phẩy kế tiếp cho đỡ chờ.
const LONG_SENTENCE: usize = 160;

impl SentenceSplitter {
    pub fn push(&mut self, s: &str) -> Vec<String> {
        self.buf.push_str(s);
        let mut out = Vec::new();
        while let Some(cut) = self.find_cut() {
            let piece: String = self.buf.drain(..cut).collect();
            let piece = piece.trim();
            if !piece.is_empty() {
                out.push(piece.to_string());
                self.emitted = true;
            }
        }
        out
    }

    pub fn finish(&mut self) -> Option<String> {
        let rest = std::mem::take(&mut self.buf);
        let rest = rest.trim();
        (!rest.is_empty()).then(|| rest.to_string())
    }

    // Vị trí byte để cắt (sau dấu câu). Dấu câu chỉ tính là hết câu khi theo
    // sau là khoảng trắng — "3.5" hay "v0.3" vì thế không bị cắt, và cuối
    // buffer thì chờ thêm vì delta sau có thể là chữ số.
    fn find_cut(&self) -> Option<usize> {
        let mut count = 0usize;
        let mut it = self.buf.char_indices().peekable();
        while let Some((i, c)) = it.next() {
            count += 1;
            if c == '\n' {
                return Some(i + 1);
            }
            let Some(&(j, next)) = it.peek() else { break };
            if !next.is_whitespace() {
                continue;
            }
            if SENTENCE_ENDS.contains(&c) {
                return Some(j);
            }
            let soft_ok = (!self.emitted && count >= FIRST_CHUNK_MIN) || count >= LONG_SENTENCE;
            if SOFT_BREAKS.contains(&c) && soft_ok {
                return Some(j);
            }
        }
        None
    }
}

// ─── Ngày giờ (không kéo thêm crate) ─────────────────────────────────────────

/// (năm, tháng, ngày) từ số ngày kể từ 1970-01-01 (thuật toán của H. Hinnant).
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (yoe + era * 400 + i64::from(m <= 2), m, d)
}

fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let mp = (m as i64 + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

const WEEKDAYS: [&str; 7] = ["Thu", "Fri", "Sat", "Sun", "Mon", "Tue", "Wed"]; // 1970-01-01 là thứ Năm
const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

/// X-Timestamp kiểu JavaScript Date.toString() mà Edge gửi.
pub fn edge_timestamp(unix: u64) -> String {
    let days = (unix / 86_400) as i64;
    let secs = unix % 86_400;
    let (y, m, d) = civil_from_days(days);
    format!(
        "{} {} {:02} {} {:02}:{:02}:{:02} GMT+0000 (Coordinated Universal Time)",
        WEEKDAYS[(days % 7) as usize],
        MONTHS[(m - 1) as usize],
        d,
        y,
        secs / 3600,
        secs / 60 % 60,
        secs % 60
    )
}

/// Header HTTP Date ("Tue, 14 Oct 2026 03:04:05 GMT") → giây unix.
pub fn parse_http_date(s: &str) -> Option<u64> {
    let parts: Vec<&str> = s.split_whitespace().collect();
    let [_, d, mon, y, hms, ..] = parts.as_slice() else { return None };
    let m = MONTHS.iter().position(|x| x.eq_ignore_ascii_case(mon))? as u32 + 1;
    let mut t = hms.split(':').map(|p| p.parse::<u64>().ok());
    let (h, mi, sec) = (t.next()??, t.next()??, t.next()??);
    let days = days_from_civil(y.parse().ok()?, m, d.parse().ok()?);
    Some(days as u64 * 86_400 + h * 3600 + mi * 60 + sec)
}

// ─── Edge TTS ────────────────────────────────────────────────────────────────

const EDGE_TOKEN: &str = "6A5AA1D4EAFF4E9FB37E23D68491D6F4";
// Theo hằng số của thư viện edge-tts; máy chủ từ chối bản Chromium quá cũ.
const EDGE_CHROMIUM: &str = "143.0.3650.75";
const EDGE_WSS: &str = "wss://speech.platform.bing.com/consumer/speech/synthesize/readaloud/edge/v1";

/// Chữ ký Sec-MS-GEC: SHA-256 của (thời điểm theo đơn vị 100ns kể từ 1601,
/// làm tròn xuống 5 phút) + token, viết hoa.
pub fn edge_sec_ms_gec(unix: u64) -> String {
    let mut secs = unix + 11_644_473_600;
    secs -= secs % 300;
    let ticks = secs as u128 * 10_000_000;
    let digest = Sha256::digest(format!("{ticks}{EDGE_TOKEN}").as_bytes());
    digest.iter().map(|b| format!("{b:02X}")).collect()
}

pub fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

pub fn edge_ssml(voice: &str, text: &str) -> String {
    // "vi-VN-HoaiMyNeural" → "vi-VN"
    let lang: String = voice.splitn(3, '-').take(2).collect::<Vec<_>>().join("-");
    format!(
        "<speak version='1.0' xmlns='http://www.w3.org/2001/10/synthesis' xml:lang='{}'>\
         <voice name='{}'><prosody pitch='+0Hz' rate='+0%' volume='+0%'>{}</prosody></voice></speak>",
        xml_escape(&lang),
        xml_escape(voice),
        xml_escape(text)
    )
}

/// Frame nhị phân của Edge: 2 byte độ dài header (big-endian), header, rồi audio.
pub fn split_edge_binary(frame: &[u8]) -> Option<(&str, &[u8])> {
    if frame.len() < 2 {
        return None;
    }
    let n = u16::from_be_bytes([frame[0], frame[1]]) as usize;
    let headers = std::str::from_utf8(frame.get(2..2 + n)?).ok()?;
    Some((headers, &frame[2 + n..]))
}

fn random_hex() -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let digest = Sha256::digest(format!("{nanos}-{n}-{:p}", &n).as_bytes());
    digest[..16].iter().map(|b| format!("{b:02x}")).collect()
}

fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Lệch đồng hồ so với máy chủ Edge (giây), học từ lần bị 403.
static EDGE_SKEW: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(0);

enum EdgeError {
    /// 403: thường do đồng hồ máy lệch → chữ ký sai. Kèm giờ của máy chủ.
    Forbidden(Option<u64>),
    Other(String),
}

fn edge_synthesize(voice: &str, text: &str) -> Result<Vec<u8>, String> {
    match edge_once(voice, text) {
        Err(EdgeError::Forbidden(Some(server))) => {
            EDGE_SKEW.store(server as i64 - unix_now() as i64, Ordering::SeqCst);
            edge_once(voice, text).map_err(|e| match e {
                EdgeError::Forbidden(_) => "Edge TTS từ chối (403). Dịch vụ có thể đã đổi cách xác thực.".to_string(),
                EdgeError::Other(m) => m,
            })
        }
        Err(EdgeError::Forbidden(None)) => Err("Edge TTS từ chối (403).".to_string()),
        Err(EdgeError::Other(m)) => Err(m),
        Ok(audio) => Ok(audio),
    }
}

fn edge_once(voice: &str, text: &str) -> Result<Vec<u8>, EdgeError> {
    use tungstenite::{client::IntoClientRequest, http::HeaderValue, stream::MaybeTlsStream, Message};

    let other = |what: &str, e: &dyn std::fmt::Display| EdgeError::Other(format!("Edge TTS {what}: {e}"));
    let now = (unix_now() as i64 + EDGE_SKEW.load(Ordering::SeqCst)).max(0) as u64;
    let url = format!(
        "{EDGE_WSS}?TrustedClientToken={EDGE_TOKEN}&ConnectionId={}&Sec-MS-GEC={}&Sec-MS-GEC-Version=1-{EDGE_CHROMIUM}",
        random_hex(),
        edge_sec_ms_gec(now)
    );
    let mut req = url.into_client_request().map_err(|e| other("URL", &e))?;
    let major = EDGE_CHROMIUM.split('.').next().unwrap_or("143");
    let ua = format!(
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) \
         Chrome/{major}.0.0.0 Safari/537.36 Edg/{major}.0.0.0"
    );
    let h = req.headers_mut();
    h.insert("Pragma", HeaderValue::from_static("no-cache"));
    h.insert("Cache-Control", HeaderValue::from_static("no-cache"));
    h.insert("Origin", HeaderValue::from_static("chrome-extension://jdiccldimpdaibmpdkjnbmckianbfold"));
    h.insert("Accept-Language", HeaderValue::from_static("en-US,en;q=0.9"));
    h.insert("Accept-Encoding", HeaderValue::from_static("gzip, deflate, br, zstd"));
    // Cookie muid ngẫu nhiên như Edge thật gửi; thiếu thì máy chủ trả 403.
    if let Ok(v) = HeaderValue::from_str(&format!("muid={};", random_hex().to_uppercase())) {
        h.insert("Cookie", v);
    }
    if let Ok(v) = HeaderValue::from_str(&ua) {
        h.insert("User-Agent", v);
    }

    let (mut ws, _) = tungstenite::connect(req).map_err(|e| match e {
        tungstenite::Error::Http(resp) if resp.status() == 403 => EdgeError::Forbidden(
            resp.headers()
                .get("date")
                .and_then(|v| v.to_str().ok())
                .and_then(parse_http_date),
        ),
        e => other("kết nối", &e),
    })?;
    // Không để luồng TTS treo mãi nếu máy chủ im lặng.
    let timeout = Some(std::time::Duration::from_secs(15));
    match ws.get_mut() {
        MaybeTlsStream::Plain(s) => {
            let _ = s.set_read_timeout(timeout);
        }
        MaybeTlsStream::Rustls(s) => {
            let _ = s.get_mut().set_read_timeout(timeout);
        }
        _ => {}
    }

    let stamp = edge_timestamp(now);
    let config = format!(
        "X-Timestamp:{stamp}\r\nContent-Type:application/json; charset=utf-8\r\nPath:speech.config\r\n\r\n\
         {{\"context\":{{\"synthesis\":{{\"audio\":{{\"metadataoptions\":{{\"sentenceBoundaryEnabled\":\"false\",\
         \"wordBoundaryEnabled\":\"false\"}},\"outputFormat\":\"audio-24khz-48kbitrate-mono-mp3\"}}}}}}}}\r\n"
    );
    ws.send(Message::text(config)).map_err(|e| other("gửi cấu hình", &e))?;
    let ssml = format!(
        "X-RequestId:{}\r\nContent-Type:application/ssml+xml\r\nX-Timestamp:{stamp}Z\r\nPath:ssml\r\n\r\n{}",
        random_hex(),
        edge_ssml(voice, text)
    );
    ws.send(Message::text(ssml)).map_err(|e| other("gửi văn bản", &e))?;

    let mut audio = Vec::new();
    loop {
        match ws.read().map_err(|e| other("nhận audio", &e))? {
            Message::Text(t) if t.contains("Path:turn.end") => break,
            Message::Binary(b) => {
                if let Some((headers, data)) = split_edge_binary(&b) {
                    if headers.contains("Path:audio") {
                        audio.extend_from_slice(data);
                    }
                }
            }
            Message::Close(_) => break,
            _ => {}
        }
    }
    let _ = ws.close(None);
    if audio.is_empty() {
        return Err(EdgeError::Other(format!("Edge TTS không trả audio (giọng \"{voice}\" có đúng không?)")));
    }
    Ok(audio)
}

// ─── Server giọng nói (POST /v1/audio/speech kiểu OpenAI) ────────────────────

/// Địa chỉ người dùng nhập → URL đầy đủ của endpoint. Nhận cả địa chỉ gốc
/// ("http://127.0.0.1:3900"), dạng có "/v1" lẫn URL endpoint đầy đủ.
pub fn speech_endpoint(base: &str) -> Result<String, String> {
    let base = base.trim();
    let base = if base.is_empty() { DEFAULT_SERVER_URL } else { base };
    let lower = base.to_ascii_lowercase();
    if !(lower.starts_with("http://") || lower.starts_with("https://")) {
        return Err("Địa chỉ server giọng nói phải bắt đầu bằng http:// hoặc https://.".to_string());
    }
    let base = base.trim_end_matches('/');
    Ok(if base.ends_with("/v1/audio/speech") {
        base.to_string()
    } else if base.ends_with("/v1") {
        format!("{base}/audio/speech")
    } else {
        format!("{base}/v1/audio/speech")
    })
}

/// Thân request. "tts-1" là bí danh của engine đang bật trên VoiceStudio; giọng
/// rỗng → "default". Xin WAV vì server tự dựng không cần ffmpeg để xuất.
pub fn speech_body(voice: &str, text: &str) -> serde_json::Value {
    let voice = voice.trim();
    json!({
        "model": "tts-1",
        "input": text,
        "voice": if voice.is_empty() { "default" } else { voice },
        "response_format": "wav",
    })
}

/// Câu lỗi của server, rút gọn để hiện trong bong bóng.
pub fn short_detail(body: &str) -> String {
    let flat = body.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut out: String = flat.chars().take(160).collect();
    if flat.chars().count() > 160 {
        out.push('…');
    }
    out
}

fn server_synthesize(server: &str, voice: &str, text: &str) -> Result<Vec<u8>, String> {
    let url = speech_endpoint(server)?;
    let body = speech_body(voice, text);
    tauri::async_runtime::block_on(async move {
        let client = reqwest::Client::builder()
            .connect_timeout(std::time::Duration::from_secs(3))
            // Câu đầu sau khi server vừa bật có thể phải chờ nạp model.
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .map_err(|e| format!("Server giọng nói: {e}"))?;
        let resp = client.post(&url).json(&body).send().await.map_err(|e| {
            if e.is_connect() {
                format!("Không kết nối được server giọng nói ở {url}. Server đã chạy chưa?")
            } else if e.is_timeout() {
                "Server giọng nói không trả lời sau 120 giây.".to_string()
            } else {
                format!("Server giọng nói lỗi: {e}")
            }
        })?;
        let status = resp.status();
        if !status.is_success() {
            let detail = short_detail(&resp.text().await.unwrap_or_default());
            return Err(format!("Server giọng nói trả lỗi {}: {detail}", status.as_u16()));
        }
        let audio = resp.bytes().await.map_err(|e| format!("Server giọng nói: không đọc được audio: {e}"))?;
        // Server lạ có thể trả 200 kèm JSON/HTML: phát ra sẽ im lặng mà không báo gì.
        if audio.len() < 44 || &audio[..4] != b"RIFF" {
            return Err("Server giọng nói trả về dữ liệu không phải WAV.".to_string());
        }
        Ok(audio.to_vec())
    })
}

// ─── Windows (WinRT) ─────────────────────────────────────────────────────────

#[cfg(windows)]
fn windows_synthesize(voice: &str, text: &str) -> Result<Vec<u8>, String> {
    use windows::{core::HSTRING, Media::SpeechSynthesis::SpeechSynthesizer, Storage::Streams::DataReader};

    let run = || -> windows::core::Result<Vec<u8>> {
        let synth = SpeechSynthesizer::new()?;
        if !voice.is_empty() {
            for v in SpeechSynthesizer::AllVoices()? {
                if v.Id()? == voice {
                    synth.SetVoice(&v)?;
                    break;
                }
            }
        }
        let stream = synth.SynthesizeTextToStreamAsync(&HSTRING::from(text))?.get()?;
        let size = stream.Size()? as u32;
        let reader = DataReader::CreateDataReader(&stream.GetInputStreamAt(0)?)?;
        reader.LoadAsync(size)?.get()?;
        let mut buf = vec![0u8; size as usize];
        reader.ReadBytes(&mut buf)?;
        Ok(buf)
    };
    run().map_err(|e| format!("Giọng Windows lỗi: {}", e.message()))
}

/// macOS: engine "windows" (giọng có sẵn của hệ điều hành) dùng lệnh `say`.
#[cfg(target_os = "macos")]
fn windows_synthesize(voice: &str, text: &str) -> Result<Vec<u8>, String> {
    crate::macos::say_synthesize(voice, text)
}

#[cfg(not(any(windows, target_os = "macos")))]
fn windows_synthesize(_voice: &str, _text: &str) -> Result<Vec<u8>, String> {
    Err("Giọng Windows chỉ có trên Windows.".to_string())
}

#[cfg(windows)]
fn windows_voices() -> Result<Vec<(String, String)>, String> {
    use windows::Media::SpeechSynthesis::SpeechSynthesizer;
    let run = || -> windows::core::Result<Vec<(String, String)>> {
        let mut out = Vec::new();
        for v in SpeechSynthesizer::AllVoices()? {
            out.push((v.Id()?.to_string(), format!("{} ({})", v.DisplayName()?, v.Language()?)));
        }
        Ok(out)
    };
    run().map_err(|e| format!("Không đọc được danh sách giọng Windows: {}", e.message()))
}

#[cfg(target_os = "macos")]
fn windows_voices() -> Result<Vec<(String, String)>, String> {
    crate::macos::say_voices()
}

#[cfg(not(any(windows, target_os = "macos")))]
fn windows_voices() -> Result<Vec<(String, String)>, String> {
    Ok(Vec::new())
}

/// Đọc output của `say -v '?'`, mỗi dòng dạng
/// "Eddy (English (UK)) en_GB    # Hello! My name is Eddy."
/// → (id = tên giọng, nhãn "Tên (en_GB)"). Dòng không đúng dạng thì bỏ.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub fn parse_say_voices(out: &str) -> Vec<(String, String)> {
    out.lines()
        .filter_map(|line| {
            let head = line.split(" # ").next()?.trim_end();
            let (name, locale) = head.rsplit_once(char::is_whitespace)?;
            let name = name.trim();
            let locale_ok = locale.len() >= 2 && locale.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
            (!name.is_empty() && locale_ok).then(|| (name.to_string(), format!("{name} ({locale})")))
        })
        .collect()
}

// ─── API cho phần còn lại của app ────────────────────────────────────────────

/// Tổng hợp một đoạn (blocking). Trả (audio, mime). `server` chỉ dùng cho
/// engine "server" (địa chỉ server giọng nói).
pub fn synthesize(engine: &str, voice: &str, server: &str, text: &str) -> Result<(Vec<u8>, &'static str), String> {
    match engine {
        "edge" => edge_synthesize(voice, text).map(|a| (a, "audio/mpeg")),
        "windows" => windows_synthesize(voice, text).map(|a| (a, "audio/wav")),
        "server" => server_synthesize(server, voice, text).map(|a| (a, "audio/wav")),
        other => Err(format!("Engine giọng nói không rõ: {other}")),
    }
}

pub fn list_voices(engine: &str) -> Result<Vec<(String, String)>, String> {
    match engine {
        "edge" => Ok(EDGE_VOICES.iter().map(|(id, name)| (id.to_string(), name.to_string())).collect()),
        "windows" => windows_voices(),
        _ => Ok(Vec::new()),
    }
}

pub fn encode_audio(audio: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(audio)
}

struct Job {
    gen: u64,
    seq: u32,
    engine: String,
    voice: String,
    server: String,
    text: String,
}

static QUEUE: OnceLock<mpsc::Sender<Job>> = OnceLock::new();
/// Lượt đọc hiện hành. Mỗi Speaker mới tăng số này → câu của lượt cũ bị bỏ.
static CURRENT_GEN: AtomicU64 = AtomicU64::new(0);

/// Khởi động luồng worker TTS (gọi một lần lúc setup).
pub fn start(app: AppHandle) {
    let (tx, rx) = mpsc::channel::<Job>();
    if QUEUE.set(tx).is_err() {
        return;
    }
    std::thread::spawn(move || {
        // Báo lỗi một lần mỗi lượt đọc, đừng spam từng câu.
        let mut reported_gen = 0u64;
        for job in rx {
            if job.gen != CURRENT_GEN.load(Ordering::SeqCst) {
                continue;
            }
            match synthesize(&job.engine, &job.voice, &job.server, &job.text) {
                Ok((audio, mime)) => {
                    let _ = app.emit_to(
                        "main",
                        "pet-message",
                        json!({ "command": "pet:tts:audio", "gen": job.gen, "seq": job.seq,
                                "mime": mime, "data": encode_audio(&audio) }),
                    );
                }
                Err(message) => {
                    eprintln!("[tts] {message}");
                    if reported_gen != job.gen {
                        reported_gen = job.gen;
                        let _ = app.emit_to(
                            "main",
                            "pet-message",
                            json!({ "command": "pet:tts:error", "gen": job.gen, "message": message }),
                        );
                    }
                }
            }
        }
    });
}

// ─── Clip thu sẵn cho câu mẫu ────────────────────────────────────────────────
//
// Câu mẫu tiếng Việt không có biến lúc chạy (lines.rs) được đọc sẵn bằng giọng
// nhân vật và đóng gói trong app: `app/media/audio/<giọng>/lines/<mã>.mp3`, với
// <mã> tính từ chính câu đã dựng (clip_file). Máy người dùng không cần model hay
// server. Sinh lại clip: xem test `dump_fixed_vi_lines`.
//
// Có hai giọng tiếng Việt, cùng tên file: "vi" (OmniVoice, bản nhân bản) và
// "vi-el" (ElevenLabs, giọng gốc). Người dùng chọn giọng ở menu của pet, lựa chọn
// đó chỉ webview biết — nên ở đây chỉ gửi tên file, webview tự lấy đúng thư mục.

/// Các thư mục clip trong frontendDist, mỗi giọng tiếng Việt một thư mục.
const CLIP_DIRS: &[&str] = &["media/audio/vi/lines", "media/audio/vi-el/lines"];

/// Tên file clip của một câu: 16 ký tự hex đầu của SHA-256 trên câu đã lọc
/// (clean_for_tts), nên emoji hay dấu "~" đổi không làm lệch mã. None nếu câu
/// không có gì để đọc.
pub fn clip_file(text: &str) -> Option<String> {
    let clean = clean_for_tts(text);
    if !speakable(&clean) {
        return None;
    }
    let digest = Sha256::digest(clean.as_bytes());
    let hex: String = digest[..8].iter().map(|b| format!("{b:02x}")).collect();
    Some(format!("{hex}.mp3"))
}

/// Đọc một câu pet vừa nói (câu mẫu hoặc câu tự bắt chuyện). Thứ tự:
/// 1. Đang bật engine giọng nói → engine đọc (một giọng thống nhất cho mọi câu).
/// 2. Không thì báo webview phát clip thu sẵn nếu câu này có clip ở ít nhất một
///    giọng; webview chỉ phát khi người dùng đang chọn một giọng tiếng Việt và
///    giọng đó có clip này (tts-player.js).
/// 3. Không có cả hai thì im lặng như trước.
pub fn say(app: &AppHandle, text: &str) {
    if let Some(mut sp) = Speaker::new(app) {
        sp.feed(text);
        sp.finish();
        return;
    }
    if store::load_settings(app).muted {
        return;
    }
    let Some(file) = clip_file(text) else { return };
    if !CLIP_DIRS.iter().any(|dir| app.asset_resolver().get(format!("{dir}/{file}")).is_some()) {
        return;
    }
    let gen = CURRENT_GEN.fetch_add(1, Ordering::SeqCst) + 1;
    let _ = app.emit_to("main", "pet-message", json!({ "command": "pet:tts:stop", "gen": gen }));
    let _ = app.emit_to("main", "pet-message", json!({ "command": "pet:tts:clip", "gen": gen, "file": file }));
}

/// Dừng mọi thứ đang/sắp đọc.
pub fn stop(app: &AppHandle) {
    let gen = CURRENT_GEN.fetch_add(1, Ordering::SeqCst) + 1;
    let _ = app.emit_to("main", "pet-message", json!({ "command": "pet:tts:stop", "gen": gen }));
}

/// Một lượt đọc: nhận text theo từng mẩu, cắt câu rồi xếp hàng tổng hợp.
pub struct Speaker {
    gen: u64,
    seq: u32,
    engine: String,
    voice: String,
    server: String,
    splitter: SentenceSplitter,
}

impl Speaker {
    /// None khi TTS đang tắt. Tạo Speaker mới là ngắt lượt đọc trước đó.
    pub fn new(app: &AppHandle) -> Option<Self> {
        if !TTS_ENABLED {
            return None;
        }
        let s = store::load_settings(app);
        // Đang tắt tiếng: tts-player.js cũng bỏ audio, nhưng đừng tốn công tổng hợp.
        if s.muted {
            return None;
        }
        let voice = match s.tts_engine.as_str() {
            "edge" => s.tts_voice_edge,
            "windows" => s.tts_voice_windows,
            "server" => s.tts_voice_server,
            _ => return None,
        };
        let gen = CURRENT_GEN.fetch_add(1, Ordering::SeqCst) + 1;
        // Câu cũ còn đang phát thì im ngay, đừng đợi audio của lượt mới tới.
        let _ = app.emit_to("main", "pet-message", json!({ "command": "pet:tts:stop", "gen": gen }));
        Some(Self {
            gen,
            seq: 0,
            engine: s.tts_engine,
            voice,
            server: s.tts_server_url,
            splitter: SentenceSplitter::default(),
        })
    }

    pub fn feed(&mut self, text: &str) {
        for sentence in self.splitter.push(text) {
            self.enqueue(&sentence);
        }
    }

    pub fn finish(mut self) {
        if let Some(rest) = self.splitter.finish() {
            self.enqueue(&rest);
        }
    }

    fn enqueue(&mut self, sentence: &str) {
        let text = clean_for_tts(sentence);
        if !speakable(&text) {
            return;
        }
        if let Some(q) = QUEUE.get() {
            let _ = q.send(Job {
                gen: self.gen,
                seq: self.seq,
                engine: self.engine.clone(),
                voice: self.voice.clone(),
                server: self.server.clone(),
                text,
            });
            self.seq += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_say_voices_reads_names_with_spaces() {
        let out = "Alex                en_US    # Most people recognize me by my voice.
                   Eddy (English (UK)) en_GB    # Hello! My name is Eddy.
                   Linh                vi_VN    # Xin chào, tên tôi là Linh.
                   
                   garbage
";
        let v = parse_say_voices(out);
        assert_eq!(
            v,
            vec![
                ("Alex".to_string(), "Alex (en_US)".to_string()),
                ("Eddy (English (UK))".to_string(), "Eddy (English (UK)) (en_GB)".to_string()),
                ("Linh".to_string(), "Linh (vi_VN)".to_string()),
            ]
        );
    }

    #[test]
    fn clean_strips_actions_brackets_and_emoji() {
        assert_eq!(clean_for_tts("*cười khúc khích* Chào anh~ 💖"), "Chào anh");
        assert_eq!(clean_for_tts("Em (thật ra (rất)) vui! <3"), "Em vui!");
        assert_eq!(clean_for_tts("**Đậm** và `code` # tiêu đề"), "và code tiêu đề");
        assert_eq!(clean_for_tts("Giá 3.5$ nha, 50% thôi"), "Giá 3.5$ nha, 50% thôi");
        assert_eq!(clean_for_tts("một * lẻ"), "một lẻ");
        // Dấu thanh dạng tổ hợp (NFD) không được rơi mất.
        assert_eq!(clean_for_tts("Cha\u{0300}o anh"), "Cha\u{0300}o anh");
        assert!(!speakable(&clean_for_tts("✨💖~")));
    }

    fn split_all(chunks: &[&str]) -> Vec<String> {
        let mut s = SentenceSplitter::default();
        let mut out: Vec<String> = chunks.iter().flat_map(|c| s.push(c)).collect();
        out.extend(s.finish());
        out
    }

    #[test]
    fn splitter_streams_sentences() {
        assert_eq!(
            split_all(&["Chào anh nha, hôm nay ", "em vui lắm. Anh khỏe", " không? Phiên bản v0.3", ".2 rồi đó"]),
            ["Chào anh nha,", "hôm nay em vui lắm.", "Anh khỏe không?", "Phiên bản v0.3.2 rồi đó"]
        );
    }

    #[test]
    fn splitter_only_early_flushes_first_comma() {
        // Câu đầu quá ngắn → không cắt ở phẩy; sau câu đầu, phẩy không cắt nữa.
        assert_eq!(
            split_all(&["Dạ, em đây. Một, hai, ba nha"]),
            ["Dạ, em đây.", "Một, hai, ba nha"]
        );
        assert_eq!(split_all(&["dòng một\ndòng hai"]), ["dòng một", "dòng hai"]);
        assert_eq!(split_all(&["Ơ...", " thật hả?!", " Ừ"]), ["Ơ...", "thật hả?!", "Ừ"]);
    }

    #[test]
    fn dates_roundtrip() {
        // 2026-10-13 là thứ Ba.
        let t = parse_http_date("Tue, 13 Oct 2026 03:04:05 GMT").unwrap();
        assert_eq!(edge_timestamp(t), "Tue Oct 13 2026 03:04:05 GMT+0000 (Coordinated Universal Time)");
        assert_eq!(edge_timestamp(0), "Thu Jan 01 1970 00:00:00 GMT+0000 (Coordinated Universal Time)");
        assert_eq!(parse_http_date("Thu, 29 Feb 2024 23:59:59 GMT"), Some(1_709_251_199));
        assert_eq!(parse_http_date("rác"), None);
    }

    #[test]
    fn gec_is_stable_within_five_minutes() {
        let base = 1_791_849_600; // chia hết cho 300
        let a = edge_sec_ms_gec(base);
        assert_eq!(a.len(), 64);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_lowercase()));
        assert_eq!(a, edge_sec_ms_gec(base + 299));
        assert_ne!(a, edge_sec_ms_gec(base + 300));
    }

    #[test]
    fn ssml_escapes_and_picks_lang() {
        let s = edge_ssml("vi-VN-HoaiMyNeural", "A & B <x> 'q'");
        assert!(s.contains("xml:lang='vi-VN'"));
        assert!(s.contains("A &amp; B &lt;x&gt; &apos;q&apos;"));
    }

    // Gọi engine thật (mạng / giọng của máy): `cargo test -- --ignored live_`.
    #[test]
    #[ignore]
    fn live_windows_voice() {
        let voices = list_voices("windows").unwrap();
        eprintln!("giọng Windows: {voices:?}");
        let (audio, mime) = synthesize("windows", "", "", "Xin chào, hello").unwrap();
        assert_eq!(mime, "audio/wav");
        assert_eq!(&audio[..4], b"RIFF");
    }

    #[test]
    #[ignore]
    fn live_edge_voice() {
        let (audio, mime) = synthesize("edge", "vi-VN-HoaiMyNeural", "", "Xin chào anh").unwrap();
        assert_eq!(mime, "audio/mpeg");
        assert!(audio.len() > 1000, "audio quá ngắn: {} byte", audio.len());
    }

    // Cần server giọng nói đang chạy ở địa chỉ mặc định.
    #[test]
    #[ignore]
    fn live_server_voice() {
        let (audio, mime) = synthesize("server", "", "", "Xin chào anh").unwrap();
        assert_eq!(mime, "audio/wav");
        assert_eq!(&audio[..4], b"RIFF");
    }

    /// Câu đưa cho model đọc khi thu clip: như clean_for_tts nhưng giữ chỗ ngắt
    /// ở dấu "~" (thành dấu phẩy), không thì hai vế bị đọc dính vào nhau.
    fn speak_text(text: &str) -> String {
        let s = clean_for_tts(&text.replace('~', ", "));
        let s = s.replace(" ,", ",").replace(",,", ",").replace("!,", "!").replace("?,", "?").replace(".,", ".");
        s.trim_end_matches([',', ' ']).to_string()
    }

    /// Mọi câu mẫu tiếng Việt chỉ dùng biến xưng hô, dựng theo từng cặp xưng hô
    /// trong `pairs`: (tên file clip, câu để model đọc).
    fn fixed_vi_lines(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
        use crate::lines::{templates, Key};
        use crate::persona::{render, Address, Lang};
        let mut out: Vec<(String, String)> = Vec::new();
        for &(me, you) in pairs {
            let a = Address { lang: Lang::Vi, me: me.into(), you: you.into(), pet: String::new() };
            for &key in Key::ALL {
                for t in templates(key, Lang::Vi) {
                    let only_address = t
                        .split('{')
                        .skip(1)
                        .filter_map(|p| p.split_once('}'))
                        .all(|(name, _)| matches!(name, "me" | "you" | "Me" | "You"));
                    if !only_address {
                        continue;
                    }
                    let text = render(t, &a, &[]);
                    if let Some(file) = clip_file(&text) {
                        if !out.iter().any(|(f, _)| *f == file) {
                            out.push((file, speak_text(&text)));
                        }
                    }
                }
            }
        }
        out
    }

    fn clip_dirs() -> Vec<std::path::PathBuf> {
        let app = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("app");
        CLIP_DIRS.iter().map(|d| app.join(d)).collect()
    }

    #[test]
    fn clip_file_ignores_emoji_and_is_stable() {
        let a = clip_file("Dạ, em đây~ 💕").unwrap();
        assert_eq!(a, clip_file("Dạ, em đây").unwrap());
        assert_ne!(a, clip_file("Dạ, anh đây").unwrap());
        assert_eq!(a.len(), 16 + 4);
        assert!(a.ends_with(".mp3") && a[..16].chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(clip_file("✨~"), None);
    }

    #[test]
    fn speak_text_keeps_a_pause_where_the_tilde_was() {
        assert_eq!(speak_text("Dừng rồi nha~ cần thì gọi em! 💕"), "Dừng rồi nha, cần thì gọi em!");
        assert_eq!(speak_text("Em đây nè~ 🎀"), "Em đây nè");
        assert_eq!(speak_text("Thật hả?~ Vui ghê~"), "Thật hả? Vui ghê");
    }

    // Clip còn sót của một câu đã sửa/xoá sẽ không bao giờ được phát mà vẫn làm
    // nặng bộ cài: mọi file trong thư mục clip phải khớp một câu mẫu hiện có.
    #[test]
    fn every_bundled_clip_belongs_to_a_current_line() {
        let known: Vec<String> = fixed_vi_lines(crate::persona::VI_PAIRS).into_iter().map(|(f, _)| f).collect();
        for dir in clip_dirs() {
            let Ok(entries) = std::fs::read_dir(&dir) else { continue };
            let orphans: Vec<String> = entries
                .filter_map(|e| e.ok())
                .map(|e| e.file_name().to_string_lossy().to_string())
                .filter(|n| n.ends_with(".mp3") && !known.contains(n))
                .collect();
            assert!(orphans.is_empty(), "clip không còn câu mẫu tương ứng trong {dir:?}: {orphans:?}");
        }
    }

    // Xuất danh sách câu cần thu sẵn cho script sinh clip:
    //   $env:CLIP_LINES_OUT = "d:\tmp\lines.json"; cargo test dump_fixed_vi_lines -- --ignored
    // CLIP_LINES_PAIRS=all để lấy mọi cặp xưng hô (mặc định chỉ cặp đầu: em / anh).
    #[test]
    #[ignore]
    fn dump_fixed_vi_lines() {
        let out = std::env::var("CLIP_LINES_OUT").expect("đặt CLIP_LINES_OUT = đường dẫn file json");
        let all = std::env::var("CLIP_LINES_PAIRS").map(|v| v == "all").unwrap_or(false);
        let pairs = if all { crate::persona::VI_PAIRS } else { &crate::persona::VI_PAIRS[..1] };
        let list: Vec<serde_json::Value> =
            fixed_vi_lines(pairs).into_iter().map(|(file, text)| json!({ "file": file, "text": text })).collect();
        std::fs::write(&out, serde_json::to_string_pretty(&list).unwrap()).unwrap();
        eprintln!("{} câu → {out}", list.len());
    }

    #[test]
    fn speech_endpoint_accepts_base_v1_and_full_url() {
        let full = "http://127.0.0.1:3900/v1/audio/speech";
        assert_eq!(speech_endpoint("").unwrap(), full);
        assert_eq!(speech_endpoint("  http://127.0.0.1:3900/ ").unwrap(), full);
        assert_eq!(speech_endpoint("http://127.0.0.1:3900/v1").unwrap(), full);
        assert_eq!(speech_endpoint(full).unwrap(), full);
        assert_eq!(speech_endpoint("HTTPS://example.com/tts/").unwrap(), "HTTPS://example.com/tts/v1/audio/speech");
        assert!(speech_endpoint("127.0.0.1:3900").is_err());
        assert!(speech_endpoint("file:///c:/x").is_err());
    }

    #[test]
    fn speech_body_defaults_voice_and_asks_for_wav() {
        let b = speech_body("  ", "Chào \"anh\"");
        assert_eq!(b["voice"], "default");
        assert_eq!(b["input"], "Chào \"anh\"");
        assert_eq!(b["response_format"], "wav");
        assert_eq!(speech_body(" pet ", "x")["voice"], "pet");
    }

    #[test]
    fn short_detail_flattens_and_truncates_on_char_boundary() {
        assert_eq!(short_detail("  lỗi\n  gì đó "), "lỗi gì đó");
        let long = "ế".repeat(200);
        let s = short_detail(&long);
        assert_eq!(s.chars().count(), 161);
        assert!(s.ends_with('…'));
    }

    #[test]
    fn server_engine_reports_unreachable_server() {
        // Cổng ephemeral vừa đóng: chắc chắn không ai nghe.
        let port = {
            let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            l.local_addr().unwrap().port()
        };
        let err = synthesize("server", "", &format!("http://127.0.0.1:{port}"), "xin chào").unwrap_err();
        assert!(err.contains("Không kết nối được"), "{err}");
    }

    #[test]
    fn server_engine_plays_wav_and_rejects_non_wav() {
        use std::io::{Read, Write};
        // Server giả: trả lần lượt một WAV hợp lệ rồi một trang JSON kèm mã 200.
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let mut wav = b"RIFF".to_vec();
        wav.extend_from_slice(&[0u8; 60]);
        let bodies: Vec<Vec<u8>> = vec![wav.clone(), b"{\"ok\":true}".to_vec()];
        let server = std::thread::spawn(move || {
            let mut seen = Vec::new();
            for body in bodies {
                let (mut s, _) = listener.accept().unwrap();
                let mut req = Vec::new();
                let mut buf = [0u8; 2048];
                // Đọc tới khi thấy đủ thân JSON (kết thúc bằng '}').
                loop {
                    let n = s.read(&mut buf).unwrap();
                    req.extend_from_slice(&buf[..n]);
                    if n == 0 || (req.windows(4).any(|w| w == b"\r\n\r\n") && req.ends_with(b"}")) {
                        break;
                    }
                }
                seen.push(String::from_utf8_lossy(&req).to_string());
                let head = format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len());
                s.write_all(head.as_bytes()).unwrap();
                s.write_all(&body).unwrap();
            }
            seen
        });
        let url = format!("http://127.0.0.1:{port}");
        let (audio, mime) = synthesize("server", "pet", &url, "xin chào").unwrap();
        assert_eq!((audio, mime), (wav, "audio/wav"));
        let err = synthesize("server", "pet", &url, "xin chào").unwrap_err();
        assert!(err.contains("không phải WAV"), "{err}");
        let seen = server.join().unwrap();
        assert!(seen[0].starts_with("POST /v1/audio/speech "), "{}", seen[0]);
        assert!(seen[0].contains("\"voice\":\"pet\""), "{}", seen[0]);
    }

    #[test]
    fn edge_binary_frame() {
        let mut f = vec![0u8, 10];
        f.extend_from_slice(b"Path:audio");
        f.extend_from_slice(&[1, 2, 3]);
        assert_eq!(split_edge_binary(&f), Some(("Path:audio", &[1u8, 2, 3][..])));
        assert_eq!(split_edge_binary(&[0, 50, 1]), None);
    }
}
