// Anthropic Claude calls, server-side (Rust) so there is no browser CORS issue
// and the API key never reaches the webview.

use serde_json::{json, Value};

use crate::chat::{parse_anthropic_event, parse_gemini_chunk, ChatMsg, SseLines};
use crate::store::Settings;

const API_URL: &str = "https://api.anthropic.com/v1/messages";
const API_VERSION: &str = "2023-06-01";

// Provider-agnostic entry point used by the command layer. Picks Anthropic or
// Gemini based on settings.provider and routes the prompt accordingly.
pub async fn complete(
    settings: &Settings,
    system: &str,
    user: &str,
    max_tokens: u32,
) -> Result<String, String> {
    match settings.provider.as_str() {
        "gemini" => {
            gemini_complete(
                &settings.gemini_key,
                &settings.gemini_model,
                system,
                user,
                max_tokens,
            )
            .await
        }
        _ => {
            anthropic_complete(&settings.api_key, &settings.model, system, user, max_tokens).await
        }
    }
}

// Fetch the list of Gemini models available to this API key, keeping only the
// ones that support generateContent. Returns (id, display_name) pairs, newest
// first-ish (the API returns them grouped; we sort to surface 2.5 > 2.0 > 1.5).
pub async fn gemini_list_models(api_key: &str) -> Result<Vec<(String, String)>, String> {
    if api_key.trim().is_empty() {
        return Err("Chưa nhập Gemini API key.".to_string());
    }
    // Key đi qua header, không nằm trong URL: lỗi kết nối của reqwest in cả URL ra
    // và lỗi đó được hiện lên giao diện.
    let client = reqwest::Client::new();
    let resp = client
        .get("https://generativelanguage.googleapis.com/v1beta/models?pageSize=200")
        .header("x-goog-api-key", api_key)
        .send()
        .await
        .map_err(|e| format!("Lỗi kết nối: {e}"))?;
    let status = resp.status();
    let val: Value = resp
        .json()
        .await
        .map_err(|e| format!("Lỗi đọc phản hồi: {e}"))?;
    if !status.is_success() {
        let msg = val
            .get("error")
            .and_then(|e| e.get("message"))
            .and_then(|m| m.as_str())
            .unwrap_or("unknown error");
        return Err(format!("Gemini {status}: {msg}"));
    }

    let mut out: Vec<(String, String)> = Vec::new();
    if let Some(models) = val.get("models").and_then(|m| m.as_array()) {
        for m in models {
            let supports = m
                .get("supportedGenerationMethods")
                .and_then(|s| s.as_array())
                .map(|arr| {
                    arr.iter()
                        .any(|v| v.as_str() == Some("generateContent"))
                })
                .unwrap_or(false);
            if !supports {
                continue;
            }
            // name is "models/gemini-2.5-flash" → strip the prefix.
            let id = m
                .get("name")
                .and_then(|n| n.as_str())
                .map(|n| n.trim_start_matches("models/").to_string());
            let Some(id) = id else { continue };
            // Skip non-chat helpers (embeddings, aqa, etc.)
            if !id.starts_with("gemini") {
                continue;
            }
            let display = m
                .get("displayName")
                .and_then(|d| d.as_str())
                .unwrap_or(&id)
                .to_string();
            out.push((id, display));
        }
    }

    // Sort newest-major-version first, then by name; keep it stable & readable.
    out.sort_by(|a, b| b.0.cmp(&a.0));
    Ok(out)
}

// List Anthropic models available to this key (GET /v1/models).
pub async fn anthropic_list_models(api_key: &str) -> Result<Vec<(String, String)>, String> {
    if api_key.trim().is_empty() {
        return Err("Chưa nhập Anthropic API key.".to_string());
    }
    let client = reqwest::Client::new();
    let resp = client
        .get("https://api.anthropic.com/v1/models?limit=100")
        .header("x-api-key", api_key)
        .header("anthropic-version", API_VERSION)
        .send()
        .await
        .map_err(|e| format!("Lỗi kết nối: {e}"))?;
    let status = resp.status();
    let val: Value = resp
        .json()
        .await
        .map_err(|e| format!("Lỗi đọc phản hồi: {e}"))?;
    if !status.is_success() {
        let msg = val
            .get("error")
            .and_then(|e| e.get("message"))
            .and_then(|m| m.as_str())
            .unwrap_or("unknown error");
        return Err(format!("Anthropic {status}: {msg}"));
    }
    let mut out: Vec<(String, String)> = Vec::new();
    if let Some(arr) = val.get("data").and_then(|d| d.as_array()) {
        for m in arr {
            let id = m.get("id").and_then(|i| i.as_str());
            let Some(id) = id else { continue };
            let name = m
                .get("display_name")
                .and_then(|d| d.as_str())
                .unwrap_or(id)
                .to_string();
            out.push((id.to_string(), name));
        }
    }
    out.sort_by(|a, b| b.0.cmp(&a.0));
    Ok(out)
}

pub async fn gemini_complete(
    api_key: &str,
    model: &str,
    system: &str,
    user: &str,
    max_tokens: u32,
) -> Result<String, String> {
    if api_key.trim().is_empty() {
        return Err("Chưa cấu hình Gemini API key. Mở Cài đặt để nhập key.".to_string());
    }

    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent",
        model
    );

    let body = json!({
        "systemInstruction": { "parts": [{ "text": system }] },
        "contents": [{ "role": "user", "parts": [{ "text": user }] }],
        "generationConfig": { "maxOutputTokens": max_tokens }
    });

    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .header("x-goog-api-key", api_key)
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Lỗi kết nối: {e}"))?;

    let status = resp.status();
    let val: Value = resp
        .json()
        .await
        .map_err(|e| format!("Lỗi đọc phản hồi: {e}"))?;

    if !status.is_success() {
        let msg = val
            .get("error")
            .and_then(|e| e.get("message"))
            .and_then(|m| m.as_str())
            .unwrap_or("unknown error");
        return Err(format!("Gemini {status}: {msg}"));
    }

    // candidates[0].content.parts[*].text, skipping any "thought" parts.
    let text = val
        .get("candidates")
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
                .collect::<Vec<_>>()
                .join("")
        })
        .unwrap_or_default();

    if text.trim().is_empty() {
        return Err("Phản hồi rỗng từ Gemini.".to_string());
    }
    Ok(text)
}

pub async fn anthropic_complete(
    api_key: &str,
    model: &str,
    system: &str,
    user: &str,
    max_tokens: u32,
) -> Result<String, String> {
    if api_key.trim().is_empty() {
        return Err("Chưa cấu hình API key. Mở Cài đặt để nhập Anthropic API key.".to_string());
    }

    let body = json!({
        "model": model,
        "max_tokens": max_tokens,
        "system": system,
        "messages": [{ "role": "user", "content": user }],
    });

    let client = reqwest::Client::new();
    let resp = client
        .post(API_URL)
        .header("x-api-key", api_key)
        .header("anthropic-version", API_VERSION)
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Lỗi kết nối: {e}"))?;

    let status = resp.status();
    let val: Value = resp
        .json()
        .await
        .map_err(|e| format!("Lỗi đọc phản hồi: {e}"))?;

    if !status.is_success() {
        let msg = val
            .get("error")
            .and_then(|e| e.get("message"))
            .and_then(|m| m.as_str())
            .unwrap_or("unknown error");
        return Err(format!("API {status}: {msg}"));
    }

    let text = val
        .get("content")
        .and_then(|c| c.as_array())
        .map(|blocks| {
            blocks
                .iter()
                .filter_map(|b| b.get("text").and_then(|t| t.as_str()))
                .collect::<Vec<_>>()
                .join("")
        })
        .unwrap_or_default();

    if text.trim().is_empty() {
        return Err("Phản hồi rỗng từ model.".to_string());
    }
    Ok(text)
}

// Multi-turn streaming chat (pet Quick Chat). `on_delta` gets each raw text
// chunk as it arrives and returns false to stop early. Returns the full raw text.
pub async fn stream_chat<F: FnMut(&str) -> bool>(
    settings: &Settings,
    system: &str,
    messages: &[ChatMsg],
    max_tokens: u32,
    mut on_delta: F,
) -> Result<String, String> {
    let gemini = settings.provider == "gemini";
    let client = reqwest::Client::new();
    let req = if gemini {
        if settings.gemini_key.trim().is_empty() {
            return Err("Chưa cấu hình Gemini API key. Mở Cài đặt để nhập key.".to_string());
        }
        let contents: Vec<Value> = messages
            .iter()
            .map(|m| {
                let role = if m.role == "assistant" { "model" } else { "user" };
                json!({ "role": role, "parts": [{ "text": m.content }] })
            })
            .collect();
        // Key đi qua header chứ không nằm trong URL: lỗi kết nối của reqwest
        // in cả URL ra, và lỗi đó được hiện lên bong bóng chat.
        client
            .post(format!(
                "https://generativelanguage.googleapis.com/v1beta/models/{}:streamGenerateContent?alt=sse",
                settings.gemini_model
            ))
            .header("x-goog-api-key", &settings.gemini_key)
            .json(&json!({
                "systemInstruction": { "parts": [{ "text": system }] },
                "contents": contents,
                "generationConfig": { "maxOutputTokens": max_tokens }
            }))
    } else {
        if settings.api_key.trim().is_empty() {
            return Err("Chưa cấu hình API key. Mở Cài đặt để nhập Anthropic API key.".to_string());
        }
        let msgs: Vec<Value> = messages
            .iter()
            .map(|m| json!({ "role": m.role, "content": m.content }))
            .collect();
        client
            .post(API_URL)
            .header("x-api-key", &settings.api_key)
            .header("anthropic-version", API_VERSION)
            .json(&json!({
                "model": settings.model,
                "max_tokens": max_tokens,
                "system": system,
                "messages": msgs,
                "stream": true,
            }))
    };

    let mut resp = req.send().await.map_err(|e| format!("Lỗi kết nối: {e}"))?;
    let status = resp.status();
    if !status.is_success() {
        let val: Value = resp.json().await.unwrap_or(Value::Null);
        let msg = val
            .get("error")
            .and_then(|e| e.get("message"))
            .and_then(|m| m.as_str())
            .unwrap_or("unknown error");
        let who = if gemini { "Gemini" } else { "API" };
        return Err(format!("{who} {status}: {msg}"));
    }

    let mut lines = SseLines::default();
    let mut full = String::new();
    while let Some(chunk) = resp
        .chunk()
        .await
        .map_err(|e| format!("Lỗi đọc phản hồi: {e}"))?
    {
        for data in lines.push(&chunk) {
            let Ok(val) = serde_json::from_str::<Value>(&data) else {
                continue; // e.g. "[DONE]" sentinels
            };
            let delta = if gemini {
                parse_gemini_chunk(&val)?
            } else {
                parse_anthropic_event(&val)?.unwrap_or_default()
            };
            if delta.is_empty() {
                continue;
            }
            full.push_str(&delta);
            if !on_delta(&delta) {
                return Ok(full);
            }
        }
    }
    Ok(full)
}

// Cheap script heuristic so quick-reply can default its target language to the
// language of the selected text (Japanese vs. everything-else → English).
pub fn detect_lang(text: &str) -> &'static str {
    for ch in text.chars() {
        let c = ch as u32;
        // Hiragana, Katakana, CJK Unified Ideographs.
        if (0x3040..=0x30FF).contains(&c) || (0x4E00..=0x9FFF).contains(&c) {
            return "ja";
        }
    }
    "en"
}
