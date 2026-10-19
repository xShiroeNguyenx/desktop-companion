# PLAN — Desktop Companion (Live2D Pet standalone + Translate / Quick Reply / Task)

> Phạm vi tài liệu này: **chỉ lên plan**. Việc code bắt đầu sau, từ Phase 0.

---

## 1. Bối cảnh (Context)

Hiện tại tính năng "desktop pet" nằm trong extension VS Code `anime-companion-vscode`. Phần
`desktop-pet/` chỉ là **một vỏ cửa sổ Tauri mỏng (thin sidecar)**: nó KHÔNG tự chứa gì cả — toàn
bộ model Live2D, audio, `main.js`, chat AI và state khởi tạo đều do **extension phục vụ qua
HTTP + WebSocket** (`ModelFileServer` + `DesktopPetBridge` trong extension). Cửa sổ Tauri chỉ làm:
trong suốt, luôn-trên-cùng, click-through, tray menu, kéo-thả, và nối WS về extension
(xem `desktop-pet/src/main.rs`, `desktop-pet/web/index.html`).

Mục tiêu: tách thành **một app desktop độc lập (standalone)** — khởi động cùng Windows, không cần
mở VS Code — và bổ sung các tính năng làm việc với văn bản bôi đen ở **bất kỳ ứng dụng nào**
(Teams, trình duyệt, Word...):

- **Dịch (Translate):** bôi đen một đoạn văn bản → hiện **biểu tượng bông hoa** cạnh con trỏ →
  click vào bông hoa → popup hiển thị bản dịch (kèm tuỳ chọn **thêm ngữ cảnh**).
- **Trả lời nhanh (Quick Reply):** bôi đen đoạn văn bản → chọn "Trả lời" → gõ **tiếng Việt** →
  tự động dịch sang **tiếng Anh hoặc tiếng Nhật** (theo ngôn ngữ của đoạn được chọn) để copy/gửi.
- **Lưu task (TODO):** bôi đen đoạn văn bản → lưu thành một việc cần làm (danh sách TODO cục bộ).
- **Right-click menu** giữ lại 3 chức năng: **Diện mạo** (đổi model Live2D), **Âm thanh**, **Chat AI**.
- **Pet nhìn theo con trỏ (look-at cursor):** mắt/đầu model dõi theo vị trí chuột trên **toàn màn hình**;
  bật/tắt được.

**Quyết định đã chốt với người dùng:**
- Framework: **Tauri 2** (giống source tham khảo, binary nhỏ, khởi động nhanh — phù hợp auto-start).
- Bắt selection: **cả hai** — tự động hiện bông hoa khi bôi đen (global mouse hook) **+** phím tắt dự phòng.

---

## 2. Nguyên tắc kiến trúc (chốt trước khi làm)

1. **Tái dùng FRONTEND, xây lại BACKEND.** Tài sản tái dùng được là phần *render Live2D* và *UI*
   (chạy trong webview): `media/webview/*.js`, `media/lib/*`, `media/live2d/*`, `media/audio/*`,
   `media/companion.css`. Phần *backend* mà extension từng cung cấp (server asset, state init,
   gọi LLM) **phải tự viết lại trong app**.

2. **Bỏ hẳn cầu nối WebSocket — KHÔNG port lại.** WS server + token + `ModelFileServer` trong
   reference tồn tại CHỈ để vượt ranh giới tiến trình *VS Code host ↔ sidecar*. App standalone
   không có ranh giới đó, nên:
   - Phục vụ asset bằng **localhost static server** (Rust, qua `tauri-plugin-localhost` hoặc một
     HTTP server nhỏ) để `main.js` fetch `/media/...` và `index.html` chạy gần như nguyên vẹn.
   - Thay `window.__VS_CODE_BRIDGE__.postMessage` + vòng WS bằng **Tauri `invoke`/`emit`**.
     Bộ "lệnh" mà shim phải hỗ trợ (`setModel`, chat, audio, `runCommand`...) chính là hợp đồng
     đang nằm trong `interaction.js`.
   - Gọi LLM/dịch **từ trong webview** (port code TS provider), đi ra ngoài qua **`tauri-plugin-http`**
     để né CORS — KHÔNG cần viết lại provider bằng Rust.

3. **Selection-capture là điểm sống-còn → spike trước, build UI sau.** (Chi tiết Phase 0.)

4. **Đa cửa sổ, không cướp focus.** Bông hoa và các popup là cửa sổ riêng, always-on-top, phải
   xuất hiện **mà không activate** (no-focus-steal) để không ngắt việc người dùng đang gõ ở app gốc.

---

## 3. Kiến trúc tổng thể

```
┌──────────────────────────────────────────────────────────────────────┐
│  Tauri App (1 tiến trình Rust + nhiều webview window)                  │
│                                                                        │
│  Rust core (main.rs + modules)                                         │
│   ├─ Local asset server (localhost:PORT) → phục vụ media/* , web/*     │
│   ├─ Global mouse hook (WH_MOUSE_LL)  → phát hiện kết thúc bôi đen     │
│   ├─ Global shortcut (hotkey)         → fallback bắt selection         │
│   ├─ Selection reader: lưu clipboard → gửi Ctrl+C → đọc → khôi phục    │
│   ├─ Window manager: pet / flower / popup / tasks / settings          │
│   ├─ Store: settings.json, tasks.json (tauri-plugin-store)            │
│   └─ Autostart (tauri-plugin-autostart)                               │
│                                                                        │
│  Webview windows (HTML/JS, tái dùng frontend)                          │
│   ├─ pet     : Live2D companion (main.js, interaction.js, ...)         │
│   ├─ flower  : nút bông hoa nổi cạnh con trỏ                           │
│   ├─ popup   : kết quả dịch / trả lời nhanh / lưu task                 │
│   ├─ tasks   : danh sách TODO                                          │
│   └─ settings: API key, model, ngôn ngữ, autostart...                  │
│                                                                        │
│  Ra ngoài: tauri-plugin-http → api.anthropic.com (dịch/chat/reply)     │
└──────────────────────────────────────────────────────────────────────┘
```

Luồng tính năng selection (cốt lõi):

```
Người dùng bôi đen text ở app khác
        │  (mouse-up sau drag, hoặc double-click, hoặc hotkey)
        ▼
Rust mouse hook phát hiện → lấy toạ độ con trỏ
        ▼
Selection reader: lưu clipboard cũ → SendInput Ctrl+C → đọc clipboard → khôi phục
        ▼
Nếu có text → show window `flower` tại (x,y), no-activate
        ▼
Click bông hoa → window `popup` mở: [Dịch] [Trả lời] [Lưu task]
        ▼
Dịch/Reply: webview popup gọi LLM qua tauri-plugin-http → render kết quả
Lưu task : ghi vào tasks.json (store)
```

---

## 4. Cấu trúc thư mục dự kiến (`desktop-companion/`)

```
desktop-companion/
├─ PLAN.md                         # file này
├─ package.json                    # scripts build/dev (npm run tauri ...)
├─ tsconfig.json
├─ src/                            # frontend TS/JS (build ra dist/)
│  ├─ providers/                   # PORT từ extension src/chat/*
│  │  ├─ anthropic.ts
│  │  ├─ llm-provider.ts
│  │  ├─ sse-parser.ts
│  │  └─ persona.ts
│  ├─ features/
│  │  ├─ translate.ts              # prompt dịch + ngữ cảnh
│  │  ├─ quick-reply.ts            # VI → EN/JP
│  │  └─ tasks.ts                  # CRUD TODO qua store
│  ├─ bridge.ts                    # shim thay __VS_CODE_BRIDGE__ bằng invoke/emit
│  └─ windows/                     # entry cho từng webview (flower/popup/tasks/settings)
├─ web/                            # HTML cho các webview
│  ├─ pet.html                     # phỏng theo desktop-pet/web/index.html (bỏ WS)
│  ├─ flower.html
│  ├─ popup.html
│  ├─ tasks.html
│  └─ settings.html
├─ media/                          # COPY từ anime-companion-vscode/media
│  ├─ webview/  (main.js, interaction.js, ui.js, audio.js, expression.js, rotation.js, core.js)
│  ├─ lib/      (live2dcubismcore.min.js, pixi.min.js, cubism4.min.js)
│  ├─ live2d/   (Hiyori bundled; Haru/Mao/Miara tải khi cần)
│  ├─ audio/    (ja/vi/en, sfx)
│  └─ companion.css, icon.png, character.png
└─ src-tauri/
   ├─ Cargo.toml                   # phỏng theo desktop-pet/Cargo.toml + plugin mới
   ├─ tauri.conf.json
   ├─ build.rs
   └─ src/
      ├─ main.rs                   # tạo window pet, tray, autostart, asset server
      ├─ windows.rs                # quản lý flower/popup/tasks/settings windows
      ├─ selection.rs              # mouse hook + đọc selection + khôi phục clipboard
      ├─ hotkey.rs                 # global shortcut fallback
      └─ assets.rs                 # localhost static server
```

---

## 5. Kế hoạch theo phase

### Phase 0 — Spike "bắt selection" (LÀM ĐẦU TIÊN, độc lập)
Đây là rủi ro lớn nhất; prototype riêng trước khi dựng UI lên trên.
- **Phát hiện kết thúc bôi đen:** cài `WH_MOUSE_LL` (windows-rs/`windows-sys`). Heuristic:
  *mouse-up sau khi giữ-kéo qua một ngưỡng khoảng cách* → ứng viên; *double-click* → ứng viên;
  *click đơn* → bỏ qua. (Không hoàn hảo — chấp nhận.)
- **Đọc text:** lưu clipboard hiện tại → `SendInput` Ctrl+C → đọc clipboard → **khôi phục clipboard cũ**.
  Ghi rõ nhược điểm: tạm ghi đè clipboard; vài app chặn copy. (UI Automation `TextPattern` né được
  clipboard nhưng KHÔNG ổn định đúng ở Teams/Electron/browser — để v2 cân nhắc, không dùng v1.)
- **Toạ độ bông hoa:** dùng toạ độ con trỏ lúc mouse-up (đường clipboard không cho bounding-box của
  vùng chọn — đặt bông hoa ngay cạnh con trỏ, đúng như ảnh tham khảo).
- **Fallback hotkey:** `tauri-plugin-global-shortcut` (vd `Ctrl+Shift+Space`) → chạy cùng selection reader.
- **Tiêu chí PASS:** lấy đúng text đã bôi đen ở Teams + Chrome + Notepad ≥ ~80% lần; clipboard được
  khôi phục; không cướp focus.

### Phase 1 — Khung app Tauri standalone
- Dựng project Tauri 2 mới; phỏng theo `desktop-pet/Cargo.toml` (đã có `windows-sys`,
  profile release tối ưu kích thước) và `desktop-pet/src/main.rs`.
- **Cửa sổ pet:** tái dùng nguyên cấu hình từ `main.rs`: `transparent(true)`, `decorations(false)`,
  `always_on_top(true)`, `skip_taskbar(true)`, `shadow(false)`; hàm `apply_click_through` (WS_EX_TRANSPARENT
  + `SetWindowPos` SWP_FRAMECHANGED) và lưu/khôi phục vị trí cửa sổ (`desktop-pet-window-position.json`).
  Bỏ phần `build_window_url()` đọc env `ANIME_PET_PORT/TOKEN` — thay bằng URL `http://127.0.0.1:{PORT}/pet.html`
  của asset server cục bộ.
- **Asset server cục bộ** (`assets.rs`): `tauri-plugin-localhost` hoặc HTTP server nhỏ phục vụ `media/` + `web/`.
- **Autostart:** `tauri-plugin-autostart` (đăng ký chạy cùng Windows; bật/tắt được trong Settings).
- **Bundle:** bật `bundle.active` (reference đang `false`) để xuất installer/exe độc lập.

### Phase 2 — Render Live2D (tái dùng frontend, thay bridge)
- Copy `media/webview/*`, `media/lib/*`, `media/live2d/*`, `media/companion.css` sang app.
- `web/pet.html`: phỏng theo `desktop-pet/web/index.html` nhưng **bỏ toàn bộ khối WebSocket/token**.
  Thay vào đó set trực tiếp các `window.__*__` (modelUrl, modelId, audioBaseUrl, voiceLanguage,
  messageLanguage, muted, visibleModels, webviewStrings...) từ store cục bộ, rồi `loadLive2DRuntime()`
  như cũ → nạp `media/webview/main.js` **không sửa**.
- **`bridge.ts` (shim):** cung cấp `window.__VS_CODE_BRIDGE__ = { postMessage(msg) {...} }` chuyển
  `msg` thành `invoke('handle_command', { msg })`; chiều ngược lại Rust `emit` → dispatch `MessageEvent`
  cho runtime (giống `ws.onmessage` cũ). Đối chiếu danh sách lệnh trong `interaction.js`/`main.js`
  (`setModel`, `runCommand`, audio, motion...) để shim phủ đủ.

### Phase 2B — Pet nhìn theo con trỏ (look-at cursor)
Tái dùng phần nhìn-theo-con-trỏ có sẵn trong frontend (`media/webview/rotation.js` + cờ `__FOCUS_FOLLOW__`),
**nhưng đổi nguồn toạ độ chuột** cho phù hợp app standalone.
- **Vấn đề riêng của standalone:** cửa sổ pet nhỏ, con trỏ thường nằm **ngoài** cửa sổ → webview KHÔNG
  nhận `mousemove`; và khi bật **click-through** thì webview càng không nhận sự kiện chuột nào. Reference
  trước đây theo con trỏ *trong* editor/webview, không dùng được nguyên.
- **Giải pháp:** dùng lại **global mouse hook ở Phase 0** để lấy toạ độ chuột toàn màn hình. Rust `emit`
  toạ độ `(x, y)` (throttle ~30–60fps) → `bridge.ts` chuyển thành `MessageEvent` cho `rotation.js`.
- **Quy đổi toạ độ:** `rotation.js` quy đổi điểm chuột màn hình sang góc nhìn **tương đối với tâm cửa sổ pet**
  (dùng vị trí + kích thước cửa sổ pet đang lưu) → điều khiển hướng mắt/đầu (và xoay nhẹ thân nếu model hỗ trợ).
- **Bật/tắt:** cờ `__FOCUS_FOLLOW__` từ store; toggle trong right-click menu (nhóm **Diện mạo**) và Settings.
  Vẫn theo dõi được cả khi click-through đang bật (vì nguồn là global hook, không phụ thuộc focus).

### Phase 3 — Right-click menu (Diện mạo / Âm thanh / Chat AI)
- Menu context đã có sẵn trong `media/webview/interaction.js` với nhiều nhóm. **Rút gọn còn 3 nhóm**:
  - **Diện mạo:** mở model panel (`showModelPanel()` đã có) → đổi Live2D model. Nguồn model lấy từ
    `window.__VISIBLE_MODELS__` (set từ store thay vì extension). Kèm toggle **Nhìn theo con trỏ** (Phase 2B).
  - **Âm thanh:** chọn ngôn ngữ giọng (ja/vi/en), ambient, mute — tái dùng `media/webview/audio.js`.
  - **Chat AI:** mở khung chat → gọi LLM (Phase tiếp). Bỏ các nhóm Git/Workflow/Agent/Desktop-mode.
- Đổi `vscode.postMessage(...)` thành shim `__VS_CODE_BRIDGE__.postMessage(...)` (đã có ở Phase 2).

### Phase 4 — Bông hoa selection + popup khung
- **Window `flower`** (`web/flower.html`): cửa sổ nhỏ, trong suốt, always-on-top, **no-activate**
  (`WS_EX_NOACTIVATE`), không skip nhận click. Rust hiện nó tại toạ độ con trỏ khi Phase 0 báo có
  selection; ẩn khi click ra ngoài / Esc / hết thời gian.
- Click bông hoa → mở **window `popup`** với 3 nút: **Dịch**, **Trả lời**, **Lưu task**, kèm sẵn
  đoạn text đã chọn (truyền qua state Rust hoặc query param).

### Phase 5 — Dịch (Translate) + ngữ cảnh
- **`features/translate.ts`** dựng prompt dịch, gọi `providers/anthropic.ts` (port từ
  `src/chat/providers/anthropic.ts` + `sse-parser.ts`) qua `tauri-plugin-http`.
- Tự nhận diện ngôn ngữ nguồn; mặc định dịch sang **tiếng Việt** (đổi được trong Settings).
- **Ngữ cảnh:** popup có ô "Ngữ cảnh" (vd: "đây là chat công việc về tính toán lương") → chèn vào
  system/prompt để dịch sát nghĩa hơn. Stream kết quả vào popup; nút **Copy**.
- Model mặc định: `claude-haiku-4-5-20251001` (nhanh/rẻ, đúng default reference) — đổi được trong Settings.

### Phase 6 — Trả lời nhanh (Quick Reply: VI → EN/JP)
- **`features/quick-reply.ts`:** nhận đoạn gốc đã chọn + câu trả lời tiếng Việt người dùng gõ →
  prompt LLM: "người dùng đang trả lời tin nhắn [gốc], hãy dịch câu trả lời tiếng Việt sang
  [ngôn ngữ của đoạn gốc: EN hoặc JP], giữ giọng điệu phù hợp công việc."
- Ngôn ngữ đích = **ngôn ngữ phát hiện của đoạn được chọn** (gốc tiếng Nhật → trả lời tiếng Nhật;
  gốc tiếng Anh → tiếng Anh). Có thể override EN/JP thủ công.
- Kết quả: nút **Copy** (và tuỳ chọn auto-paste ở v2).

### Phase 7 — Lưu task (TODO)
- **`features/tasks.ts`:** model đơn giản (KHÔNG dùng event-sourcing như task-mind — quá nặng cho nhu cầu này):
  ```ts
  interface Task { id: string; text: string; sourceApp?: string; note?: string;
                   done: boolean; createdAt: string; }
  ```
- Lưu vào `tasks.json` qua **`tauri-plugin-store`**.
- **Window `tasks`:** danh sách TODO — thêm/sửa/đánh dấu xong/xoá; mở từ right-click menu hoặc tray.
- Từ popup selection: nút "Lưu task" tạo task với `text` = đoạn đã chọn (+ ô ghi chú tuỳ chọn).

### Phase 8 — Settings & API key
- **Window `settings`:** nhập **API key** (lưu an toàn — `tauri-plugin-stronghold` hoặc tối thiểu là
  file trong app_config_dir, ghi rõ đánh đổi bảo mật), chọn **model**, **ngôn ngữ dịch đích**,
  bật/tắt **autostart**, bật/tắt **auto-flower**, đổi **hotkey**, **mute** mặc định.
- Tray menu (phỏng `main.rs`): Show/Hide pet, Mở Tasks, Mở Settings, Toggle click-through, Quit.

---

## 6. Bản đồ tái dùng (Reuse map)

| Cần | Lấy từ | Ghi chú |
|-----|--------|---------|
| Render Live2D | `media/webview/main.js`, `core.js`, `expression.js`, `rotation.js` | dùng gần như nguyên |
| Nhìn theo con trỏ | `media/webview/rotation.js`, cờ `__FOCUS_FOLLOW__` | tái dùng; đổi nguồn toạ độ = global hook |
| Context menu + model panel | `media/webview/interaction.js` | rút gọn còn 3 nhóm |
| Audio/ambient | `media/webview/audio.js`, `media/audio/*` | dùng nguyên |
| Bubble + panels | `media/webview/ui.js`, `media/companion.css` | dùng nguyên |
| Lib Live2D | `media/lib/{live2dcubismcore,pixi,cubism4}.min.js` | copy |
| Model assets | `media/live2d/{Hiyori,Haru,Mao,Miara}` | Hiyori bundle, còn lại tải sau |
| Window/tray/click-through Rust | `desktop-pet/src/main.rs` | dùng `apply_click_through`, lưu vị trí; bỏ env WS |
| Bootstrap HTML | `desktop-pet/web/index.html` | bỏ WS, set `__*__` trực tiếp |
| LLM provider | `src/chat/providers/anthropic.ts`, `llm-provider.ts`, `sse-parser.ts`, `persona.ts` | port sang webview, gọi qua tauri-plugin-http |

(Đường dẫn trên tính từ gốc `anime-companion-vscode/`.)

---

## 7. Tech stack & phụ thuộc

- **Tauri 2.1** (Rust + webview), `serde`, `serde_json`, `url`, `windows-sys` (đã dùng trong reference).
- **Plugin Tauri:** `tauri-plugin-autostart`, `tauri-plugin-http`, `tauri-plugin-store`,
  `tauri-plugin-global-shortcut`, `tauri-plugin-clipboard-manager`, và `tauri-plugin-localhost`
  (hoặc HTTP server tự viết). Cân nhắc `tauri-plugin-stronghold` cho API key.
- **Rust native (windows):** `WH_MOUSE_LL` hook, `SendInput`, `WS_EX_TRANSPARENT`/`WS_EX_NOACTIVATE`,
  `SetWindowPos` — qua `windows-sys`/`windows` crate.
- **Frontend:** PIXI.js + Live2D Cubism 4 (đã bundle trong `media/lib`).
- **LLM:** Anthropic Claude (mặc định `claude-haiku-4-5-20251001`).
- **Build prereq:** Rust stable + MSVC C++ Build Tools (như `desktop-pet/README.md` mô tả).

---

## 8. Rủi ro & cách giảm thiểu

1. **Bắt selection không ổn định** (rủi ro #1): heuristic mouse-up dễ false-positive; vài app chặn
   Ctrl+C. → Spike Phase 0 trước; luôn có **hotkey fallback**; cho tắt auto-flower trong Settings.
2. **Ghi đè clipboard:** save/restore clipboard quanh mỗi lần đọc; cảnh báo người dùng trong Settings.
3. **Cướp focus:** mọi cửa sổ overlay đặt `WS_EX_NOACTIVATE` + không `set_focus()` khi hiện.
4. **CORS khi gọi LLM từ webview:** đi qua `tauri-plugin-http` (request từ Rust) thay vì `fetch` trình duyệt.
5. **Bảo mật API key:** ưu tiên stronghold; nếu dùng file thì để trong `app_config_dir`, nêu rõ đánh đổi.
6. **Frontend phụ thuộc protocol cũ:** phải đối chiếu kỹ tập lệnh trong `interaction.js`/`main.js` khi
   viết `bridge.ts` shim — rủi ro thiếu lệnh làm vỡ tính năng.
7. **Nhìn theo con trỏ tốn CPU / giật:** stream toạ độ chuột liên tục có thể nặng. → **throttle** (~30–60fps),
   chỉ gửi khi toạ độ đổi đủ ngưỡng; tạm dừng feed khi pet đang ẩn.

---

## 9. Kiểm thử (Verification)

- **Phase 0:** chạy thử bắt selection ở Teams / Chrome / Notepad → in ra text bắt được; xác nhận
  clipboard khôi phục và không cướp focus.
- **Pet:** `npm run tauri dev` → pet hiện, trong suốt, luôn-trên-cùng, kéo-thả, đổi model (Diện mạo),
  âm thanh/mute hoạt động, chat AI trả lời.
- **Nhìn theo con trỏ:** bật toggle → di chuột khắp màn hình (cả ngoài cửa sổ pet, cả khi click-through bật)
  → mắt/đầu model dõi theo đúng hướng; tắt toggle → model về idle; kiểm tra CPU không tăng bất thường.
- **Dịch:** bôi đen đoạn tiếng Nhật ở Teams → bông hoa hiện → click → popup ra bản dịch tiếng Việt;
  thêm ngữ cảnh → dịch lại sát hơn.
- **Quick reply:** bôi đen tin nhắn tiếng Nhật → "Trả lời" → gõ tiếng Việt → ra tiếng Nhật → Copy.
- **Task:** bôi đen đoạn bất kỳ → "Lưu task" → xuất hiện trong window Tasks → đánh dấu done/xoá; còn
  sau khi khởi động lại app.
- **Autostart:** bật trong Settings → khởi động lại Windows → app tự chạy.
- **Build:** `npm run tauri build` → ra installer/exe độc lập chạy không cần VS Code.

---

## 10. Lộ trình thực thi (gợi ý thứ tự)

1. **Phase 0** — spike bắt selection (rủi ro cao nhất, làm trước, độc lập).
2. **Phase 1 + 2 + 2B** — dựng khung Tauri standalone + render được Live2D (pet hiển thị) + nhìn theo con trỏ.
3. **Phase 3** — right-click menu 3 nhóm (Diện mạo / Âm thanh / Chat AI).
4. **Phase 4** — nối Phase 0 vào UI: bông hoa + popup khung.
5. **Phase 5 → 7** — Dịch → Trả lời nhanh → Lưu task.
6. **Phase 8** — Settings, API key, autostart, đóng gói (`tauri build`).

---

## 11. Việc để sau (Backlog)

### 11.1 Giọng nói khớp nhân vật (nhân bản giọng)

**Hiện trạng.** Khung TTS đã có nhưng đang **tắt**: `TTS_ENABLED = false` trong
`src-tauri/src/tts.rs`, nhóm "🔊 Giọng nói" trong `app/settings.html` đang ẩn
(`style="display:none"`). Hai engine sẵn có là giọng Windows (WinRT) và Edge Neural.
Lý do tắt: các giọng này không khớp giọng của nhân vật. Phần dùng lại được nguyên:
cắt câu khi stream, lọc chữ trước khi đọc, hàng đợi phát theo thứ tự, mấp máy miệng
theo âm lượng (`app/media/webview/tts-player.js`).

**Hướng làm.** Thêm một engine thứ ba gọi tới server nhân bản giọng (model quá nặng để
nhúng vào app, app chỉ gọi HTTP):

| | Server chạy trên máy (GPT-SoVITS, CosyVoice, F5-TTS…) | Dịch vụ trực tuyến (ElevenLabs, Fish Audio…) |
|---|---|---|
| Chi phí | Miễn phí | Trả theo lượng ký tự |
| Phần cứng | Nên có GPU NVIDIA; CPU thì chậm | Không cần |
| Cài đặt | Python + PyTorch, vài GB, tự chạy server | Chỉ cần API key |
| Riêng tư | Không gửi gì ra ngoài | Gửi giọng mẫu + câu nói lên máy chủ |

**Cần quyết trước khi làm:**
1. **Quyền dùng giọng.** Kiểm nguồn gốc các file trong `app/media/audio/{ja,vi,en}`. Nếu là
   giọng diễn viên thật, nhân bản rồi phát hành trong app public là rủi ro pháp lý và đạo đức.
   Giấy phép model mẫu Live2D không bao gồm quyền nhân bản giọng. Chỉ dùng cá nhân thì nhẹ hơn;
   muốn phát hành thì dùng giọng có giấy phép rõ hoặc giọng của người đồng ý.
2. **Tiếng Việt.** GPT-SoVITS, XTTS mạnh ja/zh/en/ko nhưng không có tiếng Việt chính thức;
   F5-TTS có bản cộng đồng cho tiếng Việt; dịch vụ trả phí như ElevenLabs hỗ trợ tiếng Việt.
   (Kiểm lại lúc làm — các model này thay đổi nhanh.) Giọng mẫu tiếng Nhật mà nói tiếng Việt
   là trường hợp khó đoán chất lượng nhất.
3. **Audio mẫu.** Cần ít nhất vài giây giọng sạch: không nhạc nền, không hiệu ứng, tốt nhất có
   lời thoại chép lại. Các clip SFX hiện có có thể quá ngắn hoặc lẫn tiếng cười/hiệu ứng.
4. **Máy có GPU NVIDIA không**, quyết định chạy server tại máy hay dùng dịch vụ.

**Các bước:**
1. Kiểm các file audio sẵn có: độ dài, độ sạch, nguồn gốc.
2. Chốt phương án (server trên máy / dịch vụ) theo mục "Cần quyết" ở trên.
3. Nếu chạy trên máy: viết hướng dẫn dựng server từng bước cho Windows; người dùng tự chạy và
   tự nghe đánh giá giọng.
4. Thêm engine vào `tts.rs` + Cài đặt: địa chỉ server (hoặc API key), file giọng mẫu, nút
   Nghe thử. Báo lỗi rõ khi server không chạy (đừng im lặng).
5. Bật lại `TTS_ENABLED`, hiện lại nhóm Giọng nói, thêm mục CHANGELOG.

**Việc liên quan nên làm cùng:** tự chọn giọng theo ngôn ngữ của câu trả lời (hiện một giọng
đọc mọi ngôn ngữ, nghe rất lạ); tinh chỉnh `MOUTH_GAIN` trong `tts-player.js` khi đã nghe thật;
câu pet tự bắt chuyện ngay sau khởi động có thể bị chặn autoplay nếu chưa bấm vào pet lần nào.

### 11.2 Việc nhỏ còn tồn

Đã làm xong (2026-10-13): nút ⏹ huỷ trả lời, "Hỏi em" khi bật Click-through, key Gemini
chuyển sang header, chat tạo task bằng thẻ `[task: ...]`.
Đã làm xong (2026-10-14): bộ đếm 30 giây "Không nhận được phản hồi" huỷ luôn request phía Rust.

Còn mở:
- Chat mới **tạo** được task; chưa đánh dấu xong / xoá task qua chat. (Gộp vào Phase 10, mục 12.5.)

---

## 12. Companion features: nhắc việc, nhận biết máy tính, thân thiết, mini game

> Lập kế hoạch ngày 2026-10-15, tính từ v0.4.1. Chưa code.

### 12.1 Đánh giá: tính năng nào làm được, nền sẵn có tới đâu

| # | Tính năng đề xuất | Nền đã có trong code | Còn thiếu | Độ khó | Ưu tiên |
|---|---|---|---|---|---|
| 5 | **Task & Reminder** | Danh sách task (`store.rs` `Task`, `app/tasks.html`); chat tạo task bằng thẻ `[task: …]`; pet tự bắt chuyện, có nhắc nghỉ mắt nhưng **do LLM tự chọn** (`chat.rs` `start_proactive_loop`) | Hạn chót cho task; nhắc sức khoẻ **theo giờ cố định, không cần API key**; khen thưởng khi xong việc; chat đánh dấu xong/xoá task | Vừa | **1** |
| 7 | **Desktop Awareness** | Biểu cảm `sleepy`, `focus` đã có trong `EMOTIONS`; vòng Pomodoro ring + lệnh `pomodoroStart/Break/Stop/Tick` còn trong `main.js`/`ui.js` (thừa hưởng từ extension, phía Rust chưa có); `windows-sys` đã bật `Win32_System_SystemInformation`, `Win32_System_Threading`, `Win32_UI_Input_KeyboardAndMouse` | Đo thời gian idle; chào khi quay lại; phiên Focus phía Rust; im khi đang trình chiếu / toàn màn hình; cảnh báo CPU/RAM (tuỳ chọn) | Vừa | **2** |
| 6 | **Daily Life & Relationship** | Pet biết giờ địa phương (`local_time`) và việc chưa xong (`context_block`); lịch sử chat lưu trên máy | Trạng thái thân thiết (điểm, cấp, chuỗi ngày); chào buổi sáng; sinh nhật, ngày kỷ niệm quen nhau | Vừa | **3** |
| 8 | **Mini Games** | Vuốt đầu (`headpat`), chọc (`poke`), bấm liên tục (`spamClick`) đã có phản ứng + âm thanh; radial menu dễ thêm mục | Oẳn tù tì (kéo-búa-bao), đoán số | Thấp | **4** |

Ghi chú: "oẳn tù tì" và "kéo-búa-bao" là **cùng một trò**, chỉ làm một. "Vuốt ve để nhận phản ứng"
đã có sẵn, chỉ cần nối nó vào điểm thân thiết (mục 12.7).

**Kết luận:** cả 4 nhóm đều thêm được. Thứ tự hợp lý là 5 → 7 → 6 → 8, vì:
- 5 cho giá trị dùng hằng ngày rõ nhất, và tái dùng nhiều nhất cái đã có.
- 7 cần cùng một hạ tầng với 5 (đo idle, biết khi nào được phép lên tiếng). Làm 5 trước thì 7
  chủ yếu là thêm nguồn sự kiện.
- 6 cộng điểm từ hành động của 5, 7, 8 (xong việc, xong phiên Focus, chơi game, vuốt đầu), nên
  làm sau khi đã có các nguồn đó.
- 8 độc lập, rẻ, làm lúc nào cũng được. Có thể kéo lên trước nếu muốn một bản phát hành nhỏ nhanh.

### 12.2 Nguyên tắc chung (chốt trước khi làm)

1. **Không cần API key vẫn chạy.** Nhắc nghỉ, nhắc hạn, khen thưởng, chào hỏi, mini game đều có
   **câu mẫu cục bộ** (`lines.rs`). Có API key thì LLM chỉ làm câu nói **tự nhiên hơn** (tuỳ chọn
   trong Settings), không phải điều kiện để tính năng hoạt động. Lý do: người chưa nhập key vẫn
   thấy app có ích ngay lần đầu. Câu mẫu có đủ **vi / ja / en** và dùng tên pet + xưng hô do
   người dùng chọn (12.4b), không viết cứng "anh/em".
2. **Một "trọng tài lời nói" duy nhất.** Mọi câu pet tự nói (nhắc nghỉ, hạn chót, chào lại, cảnh
   báo CPU, tự bắt chuyện) đi qua **một hàng đợi có ưu tiên + khoảng nghỉ tối thiểu**. Không có
   module nào tự gọi `showMessage` riêng. Nếu không, 4 tính năng cùng lúc sẽ biến pet thành máy
   spam.
3. **Quyền riêng tư của Desktop Awareness:** chỉ đọc **số liệu trạng thái** (số giây idle, máy
   có đang toàn màn hình hay không, % CPU/RAM). **Không** đọc tên cửa sổ, tên tiến trình, nội
   dung màn hình, phím gõ. Mọi thứ có công tắc trong Settings; CPU/RAM **mặc định tắt**.
4. **Không biến thành game nặng.** Không tiền ảo, không cửa hàng, không phạt khi bỏ lỡ ngày (không
   trừ điểm, không câu trách móc). Điểm thân thiết chỉ tăng, có trần mỗi ngày để không "cày" được.
5. **Logic quyết định là hàm thuần có test** (khi nào nhắc, mốc hạn chót, cộng điểm, phân xử ưu
   tiên). Vòng lặp nền và UI chỉ gọi các hàm đó. Test viết cạnh code theo kiểu `#[cfg(test)] mod
   tests` như `chat.rs`, `tts.rs` đang làm.
6. **Dữ liệu cũ đọc được.** Trường mới thêm vào `Task`/`Settings` đều có `#[serde(default)]`;
   file mới (`bond.json`) thiếu hoặc hỏng thì dùng mặc định, không làm hỏng app.

### 12.3 Kiến trúc

```
src-tauri/src/
├─ companion.rs   (MỚI) vòng tick 30s duy nhất + trọng tài lời nói
│                       thay cho start_proactive_loop hiện tại trong chat.rs
├─ awareness.rs   (MỚI) idle (GetLastInputInfo), toàn màn hình
│                       (SHQueryUserNotificationState), CPU/RAM (GetSystemTimes,
│                       GlobalMemoryStatusEx); chỉ trả số liệu, không lưu gì
├─ reminders.rs   (MỚI) hàm thuần: nhắc sức khoẻ tới hạn chưa, task đang ở mốc hạn nào
├─ focus.rs       (MỚI) phiên Focus/Pomodoro: trạng thái + chuyển pha (hàm thuần)
├─ bond.rs        (MỚI) điểm thân thiết, chuỗi ngày, cột mốc; đọc/ghi bond.json
├─ lines.rs       (MỚI) kho câu mẫu vi/ja/en theo loại sự kiện, có biến {me} {you}
│                       {name}…, chọn không lặp lại (12.4b)
├─ persona.rs     (MỚI) dựng prompt hệ thống + "Address" (tên pet, xưng hô, ngôn ngữ)
│                       từ Settings, thay chuỗi "em/anh" viết cứng trong chat.rs
├─ notify.rs      (MỚI) toast Windows cho hạn chót (tauri-plugin-notification)
├─ chat.rs        (SỬA) proactive_due/proactive_prompt giữ lại, gọi từ companion.rs;
│                       thêm thẻ [done: …] và hạn chót trong [task: …]
└─ store.rs       (SỬA) Task.due_at, Task.reminded; Settings thêm các công tắc mới

app/
├─ js/tasks.js        (SỬA) chọn hạn chót, hiện "còn 2 giờ" / "quá hạn"
├─ js/settings.js     (SỬA) nhóm "Nhắc nhở", "Nhận biết máy tính", "Thân thiết"
├─ media/webview/main.js, ui.js   (SỬA nhẹ) dùng lại Pomodoro ring + hiệu ứng achievement
└─ media/webview/games.js         (MỚI) oẳn tù tì, đoán số
```

**Luồng của `companion.rs` (mỗi 30 giây):**

```
đọc Settings + số liệu awareness (idle, fullscreen, cpu/ram)
        ▼
mỗi nguồn trả về 0..n "ý muốn nói" { loại, ưu tiên, câu, biểu cảm, motion? }
  reminders (sức khoẻ, hạn chót) · awareness (chào lại, buồn ngủ, CPU)
  focus (hết pha) · bond (chào sáng, cột mốc) · chat (tự bắt chuyện)
        ▼
trọng tài (hàm thuần, có test):
  - im lặng nếu: pet ẩn / đang nằm trong bảng / đang trả lời chat /
    "Đừng làm phiền" / đang toàn màn hình hoặc trình chiếu
  - trong phiên Focus chỉ cho qua: hết pha Focus, hạn chót tới giờ
  - cách câu tự nói trước ≥ N phút (mặc định 10), trừ ưu tiên cao nhất
  - chọn 1 câu ưu tiên cao nhất; câu bị hoãn thì giữ lại tick sau,
    câu hết hạn ý nghĩa (vd "chào lại" sau 5 phút) thì bỏ
        ▼
send setExpression / playMotion / showMessage (+ TTS nếu bật) như hiện nay
```

Thứ tự ưu tiên: hạn chót tới giờ > hết pha Focus > chào lại / chào sáng > nhắc sức khoẻ >
cột mốc thân thiết > cảnh báo CPU/RAM > tự bắt chuyện.

**"Đừng làm phiền":** mục mới trong tray và radial menu, tắt mọi câu tự nói trong 1 giờ (hoặc
tới khi bật lại). Lệnh người dùng chủ động (chat, game) vẫn chạy.

### 12.4 Phase 9: Nền tảng (trọng tài + idle)

Làm trước vì 3 phase sau đều cần.
- `awareness.rs`: `idle_secs()` bằng `GetLastInputInfo` + `GetTickCount` (đã có feature).
  `is_fullscreen_busy()` bằng `SHQueryUserNotificationState` (cần thêm feature
  `Win32_UI_Shell`): trả true khi đang trình chiếu / game toàn màn hình / chế độ yên lặng.
- `companion.rs`: vòng tick + hàm thuần `arbitrate(candidates, ctx) -> Option<Utterance>`.
- Chuyển `start_proactive_loop` thành một nguồn của trọng tài. Thêm điều kiện: **không tự bắt
  chuyện khi người dùng đang idle** (họ không ngồi đó thì nói cho ai nghe). Hiện tại pet vẫn nói
  khi máy bỏ trống.
- "Đừng làm phiền" trong tray + radial menu.
- **Test:** bảng tình huống cho `arbitrate` (ẩn, docked, đang trả lời, fullscreen, khoảng nghỉ,
  hai ứng viên cùng lúc, câu hết hạn ý nghĩa); `proactive_due` có idle.
- Không có thay đổi người dùng nhìn thấy ngoài "Đừng làm phiền" và việc pet thôi nói khi máy bỏ
  trống. Có thể gộp phát hành với Phase 10.

**Đã code (2026-10-15, chưa phát hành):**
- `awareness.rs` (`snapshot()`: idle, fullscreen, màn hình khoá), `companion.rs` (`arbitrate`,
  `blocked`, `prune`, vòng 30s, DND). `chat::start_proactive_loop` đã bỏ; phần hỏi model còn lại là
  `chat::proactive_line`.
- Khác plan: khoảng nghỉ giữa hai câu là hằng `QUIET_GAP_SECS` = 10 phút, chưa có trong Settings
  (thêm ô chỉnh ở nhóm ⏰ Nhắc nhở của Phase 10). `Kind` đã khai đủ các loại của Phase 10–12 để test
  thứ tự ưu tiên, các loại chưa có nguồn đánh dấu `#[allow(dead_code)]`.
- Khác hành vi cũ: tự bắt chuyện bị chặn (ẩn, rời máy, toàn màn hình, DND…) thì **chờ**, không bỏ
  lượt rồi đợi thêm cả chu kỳ như trước. Rời máy 2 tiếng quay lại thì pet có thể nói ngay; Phase 11
  thay bằng câu chào lại (ưu tiên cao hơn, và khoảng nghỉ sẽ chặn câu tự bắt chuyện ngay sau đó).
- DND: tray "🔕 Đừng làm phiền (1 giờ)" / "🔔 Bật lại lời nhắc", và một mục trên vòng menu tròn
  của pet. Chỉ giữ trong RAM: thoát app là mất.

### 12.4b Phase 9B: Tên pet, xưng hô, ngôn ngữ câu nói

Làm ngay sau Phase 9, **trước khi viết bất kỳ câu mẫu nào**, vì mọi câu mẫu đều phụ thuộc vào nó.

**Hiện trạng cần sửa:**
- Prompt hệ thống trong `chat.rs` viết cứng `Xưng "em", gọi người dùng là "anh"`, và các chuỗi
  `context_block`, `proactive_prompt`, `with_selection` cũng viết cứng "anh".
- "Ngôn ngữ chữ" (menu Âm thanh → 💬) chỉ đổi `window.__MESSAGE_LANGUAGE__` trong webview:
  không lưu lại (`pet-config.js` luôn đặt `'vi'`) và Rust không biết. Khởi động lại là mất.

**Settings mới:**

| Trường | Ý nghĩa | Mặc định |
|---|---|---|
| `pet_name` | Tên người dùng đặt cho pet; rỗng = chưa đặt | rỗng |
| `user_name` | Pet gọi người dùng bằng tên gì (tuỳ chọn) | rỗng |
| `vi_self` / `vi_user` | Cặp xưng hô tiếng Việt: pet tự xưng / gọi người dùng | `em` / `anh` |
| `ja_honorific` | Hậu tố khi gọi tên bằng tiếng Nhật: `さん` / `くん` / `ちゃん` / `様` / không | `さん` |
| `message_language` | `vi` / `ja` / `en`, nguồn duy nhất cho câu mẫu + ngôn ngữ trả lời chat | `vi` |

- Cặp xưng hô tiếng Việt chọn từ danh sách: em–anh, em–chị, mình–bạn, tớ–cậu, em–sếp, và ô "tự
  nhập" (cả hai phía, tối đa 12 ký tự). Tiếng Anh chỉ dùng tên (`user_name` hoặc bỏ trống). Tiếng
  Nhật dùng `user_name` + `ja_honorific`; pet xưng `わたし`.
- Tên pet là **một tên chung**, không theo từng model Live2D: người dùng đặt tên cho "bạn đồng
  hành", đổi ngoại hình vẫn là cô ấy. (Đổi được trong Settings.)
- Tên và xưng hô chỉ nằm trong `settings.json` trên máy, giống lịch sử chat.

**Luồng đặt tên:**
1. Chưa có `pet_name`: sau lần chào đầu tiên (hoặc lần đầu mở chat), pet nói một lần
   "Bạn chưa đặt tên cho em nè… đặt cho em một cái tên nha?" kèm nút **✏️ Đặt tên** trên bubble.
   Từ chối hoặc đóng thì không hỏi lại trong 7 ngày, tối đa 3 lần, sau đó chỉ còn trong Settings.
2. Bấm nút → ô nhập nhỏ ngay trên bubble (không mở cửa sổ mới). Tên được làm sạch: cắt khoảng trắng,
   1–20 ký tự, bỏ ký tự điều khiển và `[` `]` (để không lẫn với thẻ cảm xúc/task).
3. Lưu xong → biểu cảm `love`/`shy`, câu cảm ơn có tên mới ("Mây… em thích tên này lắm, cảm ơn anh
   nha~"), cộng điểm thân thiết khi Phase 12 đã có. Đây là cột mốc "được đặt tên", nói đúng một lần.
4. **Đặt tên qua chat:** "từ giờ em tên là Mây nhé" → model trả thẻ `[name: Mây]`, Rust kiểm như
   bước 2 rồi mới lưu. Bubble chỉ báo "đã nhận tên" **sau khi lưu thật**, lỗi thì nói lý do (đúng
   cách thẻ `[task: …]` đang làm).
5. Đổi tên lần sau (Settings hoặc chat) → câu "Từ giờ em là Mây nha", không lặp lại câu cảm ơn
   lần đầu.
6. Xưng hô chọn ở Settings, cũng hỏi nhẹ một lần ngay sau khi đặt tên: "Em nên gọi bạn là gì nè?"
   với các nút cặp xưng hô phổ biến.

**Kho câu mẫu đa ngôn ngữ (`lines.rs`):**
- Khoá theo loại sự kiện, mỗi loại có danh sách câu cho `vi`, `ja`, `en`. Mỗi câu là mẫu có biến:
  `{me}` (pet tự xưng), `{you}` (gọi người dùng), `{name}` (tên pet), `{task}`, `{when}`…
- Hàm thuần `render(template, &Address) -> String`. Tiếng Việt viết hoa chữ đầu câu khi `{you}`/
  `{me}` đứng đầu câu ("Anh ơi…" / "Chị ơi…"). Tên pet rỗng thì câu dùng biến thể không có tên.
- Câu tiếng Nhật và tiếng Anh viết riêng cho tự nhiên, **không dịch máy từng câu tiếng Việt**.
  Mỗi loại sự kiện tối thiểu 6 câu mỗi ngôn ngữ.
- Test bắt buộc: **mọi khoá có đủ 3 ngôn ngữ**, không câu nào còn sót `{` chưa thay, không mẫu nào
  dùng biến lạ. Test này chặn được việc thêm câu tiếng Việt rồi quên ja/en.

**Prompt hệ thống:** dựng từ Settings thay cho chuỗi cứng: tên pet, cặp xưng hô, ngôn ngữ trả lời.
`message_language` = `ja`/`en` thì yêu cầu model trả lời bằng ngôn ngữ đó. Các chuỗi
`context_block`/`proactive_prompt` gửi model vẫn viết bằng tiếng Việt cũng được (model hiểu), chỉ
phần **người dùng nhìn thấy** mới phải theo ngôn ngữ đã chọn. Sửa luôn các test hiện có đang chứa
"anh".

**Ngôn ngữ chữ được lưu:** menu 💬 gửi `setMessageLanguage` → Rust lưu vào `Settings`; lúc tạo cửa
sổ pet, Rust tiêm giá trị đã lưu (như đang làm với `__INIT_MODEL__`) thay cho `'vi'` cứng.

**Test:** làm sạch tên (rỗng, quá dài, ký tự điều khiển, dấu `[` `]`, emoji, chữ Nhật), parse thẻ
`[name: …]`, `render` cho cả 3 ngôn ngữ và các cặp xưng hô, kiểm đủ khoá 3 ngôn ngữ, dựng prompt
theo Settings.

**Đã code (2026-10-15, chưa phát hành):**
- `persona.rs` (Lang, Address, render, sanitize_name, name_ask_due, system_prompt), `lines.rs`
  (13 loại câu × vi/ja/en), `naming.rs` (luồng đặt tên + xưng hô), `app/media/webview/naming.js`
  (thẻ trên pet). Câu khen tờ note ghim và câu "Đừng làm phiền" đã chuyển sang `lines.rs`.
- Xin tên là một nguồn của trọng tài (`Kind::AskName`, ưu tiên sau Welcome). Không hỏi khi
  click-through đang bật (thẻ không bấm được) và không tính là một lần hỏi.
- Đặt tên từ Cài đặt khi pet đang nằm trong bảng: không mở thẻ hỏi xưng hô (pet bị khoá click,
  ô xưng hô có ngay trong bảng), đánh dấu là đã hỏi.
- **Chưa làm:** chữ trên giao diện (menu, bảng Cài đặt, bảng Công việc, tờ note) vẫn chỉ tiếng
  Việt; "Ngôn ngữ chữ" mới đổi câu pet nói + câu trả lời chat. Muốn dịch giao diện là một việc
  riêng (i18n toàn app).

### 12.4c Ghim task lên desktop (đã code 2026-10-15, chưa phát hành)

Yêu cầu thêm của người dùng: bấm 📌 ở một task → task thành **tờ note dán trên màn hình desktop**,
có **cây ghim** ở trên, đầu ghim là **mặt nhân vật**.

- `src-tauri/src/pins.rs`: mỗi task ghim là một cửa sổ trong suốt `pin-<id>` (`app/pin.html`,
  `app/js/pin.js`). Không luôn-nổi theo mặc định (nằm như widget, cửa sổ khác che được); nút 📍
  bật luôn-nổi cho từng tờ. Ẩn khỏi taskbar + Alt+Tab (`WS_EX_TOOLWINDOW`).
- `Task` thêm `pinned`, `pin_pos`, `pin_top` (`serde(default)`). Mở app thì các tờ đang ghim hiện
  lại đúng chỗ; màn hình phụ đã rút thì tờ note tự về màn hình chính.
- Kéo tờ note ở bất cứ đâu; vị trí ghi vào `tasks.json` sau khi đứng yên 400ms.
- ✓ Xong trên tờ note: đóng dấu "XONG!", tờ rơi xuống, task chuyển sang Đã làm, pet khen một câu.
  ✕ gỡ ghim (task vẫn còn). Nhấp đúp mở bảng Công việc. Alt+F4 = gỡ ghim.
- Đầu ghim: pet cắt phần đầu nhân vật từ canvas Live2D (128px) sau khi nạp model, chỉ lưu khi chưa
  có ảnh hoặc đổi model (`pin_avatar.png` + `.model` trong config dir). Chưa có ảnh thì dùng
  `media/character.png`.
- Tối đa 8 tờ. `tasks.json` giờ được đọc-sửa-ghi trong một khoá (`store::update_tasks`) vì
  nhiều cửa sổ cùng ghi.
- **Còn lại / giới hạn:** câu khen trong `pins.rs` (`PRAISES`) còn viết cứng "anh/em", chuyển
  sang `lines.rs` ở Phase 9B. Win+D (Show desktop) che cả tờ note (muốn tờ note nằm hẳn trong lớp
  desktop phải gắn vào cửa sổ `WorkerW` của Explorer, để sau). Khi có hạn chót (Phase 10), tờ
  note hiện thêm "còn 2 giờ" / "quá hạn".

### 12.5 Phase 10: Task & Reminder Companion

**a) Hạn chót cho task**
- `Task` thêm `due_at: String` (millis, rỗng = không có hạn) và `reminded: String` (mốc nhắc gần
  nhất đã nói, để không nhắc lặp). Cả hai `#[serde(default)]`.
- Lệnh mới `tasks_set_due(id, due_at)`. Bảng Tasks: nút 📅 mở ô chọn ngày + giờ (giờ tuỳ chọn,
  bỏ trống = cuối ngày 23:59). Dòng task hiện "còn 2 giờ", "hôm nay", "quá hạn 1 ngày" (màu nhạt
  dần theo mức gấp). Danh sách "Chưa xong" xếp theo hạn gần nhất trước, task không hạn xuống dưới.
- Hàm thuần `due_stage(now, due_at) -> None | DayBefore | HourBefore | Due | Overdue(days)`.
  Mỗi mốc chỉ nhắc **một lần** (so với `reminded`); quá hạn nhắc tối đa một lần mỗi ngày, ngừng
  sau 3 ngày để không thành cằn nhằn.
- Chat: thẻ `[task: gửi báo cáo | due: 2026-10-16 17:00]`. Prompt hệ thống đã có giờ hiện tại nên
  model quy đổi được "mai 5 giờ chiều". Ngày giờ không parse được thì vẫn lưu task, báo "📝 Đã lưu
  task (chưa hiểu hạn chót)" chứ không im lặng bỏ hạn.
- `context_block` gửi kèm hạn của từng task, để "hôm nay còn gì gấp không em?" trả lời đúng.

**b) Nhắc nghỉ mắt, uống nước, vận động**
- Settings nhóm "⏰ Nhắc nhở": 3 công tắc + chu kỳ (phút).
  Mặc định (đã chốt, 12.10): nghỉ mắt **bật, 45'** · uống nước **bật, 60'** · đứng dậy **bật, 60'**.
- Cả 3 cùng bật nên dễ rơi trùng lúc: hai loại tới hạn cách nhau < 10 phút thì **gộp thành một
  câu** ("uống ngụm nước rồi đứng dậy vươn vai chút nha"), có câu mẫu riêng cho các cặp gộp. Hàm
  thuần `merge_health(due_list)`.
- Hàm thuần `health_due(now, last_fired, last_return_from_idle, interval)`. Điểm quan trọng: đồng
  hồ **đếm thời gian ngồi máy liên tục**, không đếm giờ đồng hồ. Người dùng idle ≥ 5 phút được
  tính là đã nghỉ, đặt lại bộ đếm nghỉ mắt + đứng dậy. Không nhắc khi đang idle.
- Câu mẫu trong `lines.rs` (mỗi loại ~8 câu, chọn không lặp câu vừa nói). Kèm biểu cảm/motion.
- Nhắc nghỉ mắt kèm gợi ý 20-20-20 (nhìn xa 6 mét trong 20 giây).

**c) Hoàn thành việc → khen thưởng**
- Khi `tasks_set_done(id, true)` (từ bảng Tasks hoặc từ chat): pet đổi biểu cảm `happy`, chạy một
  motion vui, nói một câu khen từ `lines.rs`. Không cần LLM, phản hồi ngay lập tức.
- Thưởng lớn hơn khi: xong **việc cuối cùng** trong danh sách ("hết sạch việc rồi!"), xong việc
  **trước hạn**, xong việc thứ 3/5/10 trong ngày. Dùng lại `showAchievementUnlockEffect` (đã có
  trong `interaction.js`) cho mốc lớn.
- "Mở khoá": mỗi cấp thân thiết (Phase 12) mở thêm một bộ câu khen và motion. Trước Phase 12 thì
  chỉ có bộ câu cơ bản.
- Khen thưởng **không** đi qua khoảng nghỉ tối thiểu của trọng tài (người dùng vừa thao tác, đây
  là phản hồi chứ không phải câu tự nói), nhưng vẫn im khi "Đừng làm phiền" bật.

**d) Chat đánh dấu xong / xoá task** (việc tồn ở 11.2)
- Thẻ `[done: <id hoặc đoạn đầu tên task>]`, `[delete: …]`. `context_block` gửi kèm id ngắn của
  task. Khớp không chắc chắn (nhiều task gần giống) thì **không làm gì**, bubble hỏi lại. Xoá qua
  chat thì báo rõ "🗑 Đã xoá task: …".

**e) Thông báo Windows (toast) cho hạn chót** (đã chốt, 12.10)
- Thêm `tauri-plugin-notification` (khai trong `src-tauri/Cargo.toml`, đăng ký trong `main.rs`,
  thêm quyền vào `capabilities/default.json`).
- **Khi nào gửi toast:** mốc hạn chót (`HourBefore`, `Due`, `Overdue`) tới lúc mà pet **không nói
  được**: pet đang ẩn, hoặc đang trình chiếu / toàn màn hình. Pet đang hiện thì chỉ nói bằng
  bubble, không gửi toast trùng. Nhắc sức khoẻ, chào hỏi **không** gửi toast (không đáng làm phiền
  ở mức hệ điều hành).
- Nội dung toast theo `message_language` + tên pet làm tiêu đề ("Mây 🌸: còn 1 giờ nữa là tới hạn
  'gửi báo cáo'"). Không chứa ghi chú của task, chỉ tên task (toast có thể hiện trên màn hình khoá).
- Bấm toast → hiện pet và mở bảng Tasks, cuộn tới task đó.
- Mốc đã báo bằng toast được ghi vào `reminded` như bubble, nên lúc pet hiện lại **không nhắc lần
  nữa**.
- Settings: công tắc "Thông báo Windows khi tới hạn" (mặc định **bật**), nằm trong nhóm ⏰.
- **API im lặng:** Windows có thể chặn thông báo (Focus Assist, tắt thông báo cho app) mà plugin
  vẫn báo thành công. Settings có nút **Gửi thử** + dòng hướng dẫn "không thấy thông báo? kiểm tra
  Cài đặt Windows → Thông báo". Lỗi gửi toast thì ghi log và rơi về bubble khi pet hiện lại (không
  đánh dấu `reminded`).
- Bản dev (`tauri dev`) có thể không hiện toast vì chưa cài app (thiếu AppUserModelID). Phải thử
  bằng bản build đã cài.

**Test:** `due_stage` (biên giờ, đổi ngày, task không hạn, đồng hồ lùi), parse `due:` trong thẻ,
`health_due` (idle đặt lại bộ đếm, không nhắc lúc idle), `merge_health`, chọn câu không lặp, khớp
`[done: …]`, hàm chọn kênh `deliver_channel(pet_visible, fullscreen, kind) -> Bubble|Toast|Hold`.

**Đã code (2026-10-15, chưa phát hành):**
- `reminders.rs` (lịch, mốc hạn chót, `parse_due`, `HealthClock`, `reward_kind`), `taskref.rs`
  (mã ngắn `#4821`, khớp thẻ done/delete), `rewards.rs` (đánh dấu xong + khen, dùng chung cho bảng
  Công việc / tờ note / chat), `notify.rs` (toast). `companion.rs` thêm nguồn Deadline + Health và
  đường toast. `app/js/due.js` + `due.test.js` (test JS đầu tiên, `npm test`). CI chạy thêm
  `cargo test` + `npm test` (trước đây chỉ `cargo check`).
- **Khác plan:** mốc "trước 1 ngày" đổi thành **"hôm nay tới hạn" lúc 7:00 sáng** ngày đó (task
  chỉ có ngày lưu hạn 23:59, nên "trước 24 giờ" rơi vào gần nửa đêm hôm trước). Mốc còn lại:
  còn < 1 giờ, tới giờ, quá hạn 1/2/3 ngày. Mốc bắt đầu trước lúc tạo task thì bỏ qua.
- **Khác plan:** `Task.reminded` là số (rank của mốc), không phải chuỗi.
- **Chưa làm:** bấm vào toast để mở bảng Công việc (plugin thông báo trên Windows chưa cho bắt sự
  kiện bấm một cách đơn giản). DND giờ chặn bubble nhưng hạn chót vẫn đi bằng toast (Windows có chế
  độ Không làm phiền riêng nếu muốn chặn cả toast).
- **Chưa làm:** hiệu ứng `showAchievementUnlockEffect` cho mốc lớn (dữ liệu nó cần là định dạng của
  extension). Mốc lớn hiện chỉ đổi biểu cảm `love` + câu khen riêng.

### 12.6 Phase 11: Desktop Awareness

- **Buồn ngủ:** idle ≥ 10 phút (chỉnh được) → biểu cảm `sleepy`, chậm motion idle. **Không
  nói** (không ai ở đó để nghe), chỉ đổi dáng.
- **Chào khi quay lại:** idle ≥ 10 phút rồi có input → một câu chào lại, ưu tiên cao. Câu khác
  nhau theo thời gian vắng (10 phút / >1 giờ / qua đêm) và theo giờ trong ngày. Lần đầu trong
  ngày thì để Phase 12 thay bằng **chào buổi sáng**.
- **Phiên Focus (Pomodoro):** `focus.rs` giữ trạng thái `{ phase: Focus|Break, started, length }`.
  - Mở từ radial menu "🍅 Tập trung" và tray. Mặc định 25' Focus / 5' nghỉ, chỉnh trong Settings.
  - Dùng lại `pomodoroStart/Tick/Break/Stop` + vòng ring đã có trong `main.js`/`ui.js`, chỉ
    viết phía Rust phát các lệnh này. Tick mỗi giây chỉ gửi khi pet đang hiện.
  - Trong Focus: biểu cảm `focus`, trọng tài chặn mọi câu tự nói trừ hạn chót tới giờ.
  - Hết Focus → khen + gợi ý nghỉ (thay luôn cho nhắc nghỉ mắt của chu kỳ đó). Nghỉ mà idle thì
    tính là nghỉ thật.
- **Im khi toàn màn hình / trình chiếu:** `is_fullscreen_busy()`; câu bị hoãn tới khi thoát.
  Đây là điều kiện chặn, không phải sự kiện.
- **CPU / RAM cao (mặc định TẮT):** lấy mẫu mỗi 30s bằng `GetSystemTimes` + `GlobalMemoryStatusEx`.
  Nhắc khi **liên tục ≥ 2 phút** vượt ngưỡng (CPU 90%, RAM 90%, chỉnh được), mỗi loại tối đa một
  lần / 30 phút. Câu nhắc nhẹ ("máy đang thở dốc đó anh"), **không** kể tên tiến trình (chính sách
  12.2.3). Không lấy mẫu khi công tắc tắt.
- Settings nhóm "🖥️ Nhận biết máy tính": buồn ngủ khi idle, chào khi quay lại, im khi toàn màn
  hình, cảnh báo CPU/RAM. Có một dòng nói rõ app **đọc gì và không đọc gì**.

**Test:** chuyển trạng thái idle → về (ngưỡng, nhiều lần liên tiếp), máy trạng thái Focus (hết
pha, dừng giữa chừng, máy ngủ rồi thức dậy làm nhảy giờ), cửa sổ "liên tục ≥ 2 phút" của CPU,
phép tính % CPU từ hai mẫu `GetSystemTimes`.

**Đã code (2026-10-15, chưa phát hành, dự kiến v0.6.0):**
- `awareness.rs` thêm `Presence` (sự kiện Sleepy / Wake / Back{away_secs}), `cpu_percent` +
  `cpu_times` (GetSystemTimes), `ram_percent` (GlobalMemoryStatusEx), `Sustain` (vượt ngưỡng liên
  tục ≥ 2 phút, nguội 30 phút). `focus.rs` (Session Focus → Break → Idle; máy ngủ qua cả phiên thì
  về Idle im lặng). `companion.rs`: vòng 1 giây cho phiên Tập trung (vẽ vòng đếm bằng lệnh
  `pomodoroTick` có sẵn trong `ui.js`), `awareness_candidates` (chào lại, cảnh báo máy),
  `welcome_key`, `kick` (chạy trọng tài ngay khi hết pha). Ctx.focus chặn mọi câu trừ hạn chót.
- Không dùng 3 lệnh cũ `pomodoroStart/Break/Stop` trong `main.js` (câu viết cứng "Onii-chan");
  câu nói đi qua `lines.rs`, nét mặt giữ bằng `setExpression` không thời hạn.
- Phiên Tập trung chỉ giữ trong RAM (thoát app là mất phiên).
- **Khác plan:** hết giờ nghỉ thì phiên kết thúc, không tự chạy phiên mới (người dùng tự bấm 🍅).
- **Còn hạn chế:** câu trả lời chat có biểu cảm tạm (8 giây) sẽ đưa nét mặt về neutral sau đó, làm
  mất nét "focus" / "sleepy" đang giữ cho tới lần đổi pha kế tiếp.

### 12.7 Phase 12: Daily Life & Relationship

- **`bond.json`** (trong `app_config_dir`, cạnh `tasks.json`):
  ```json
  { "first_seen": "2026-10-15", "points": 0, "active_days": 0,
    "last_active_day": "", "streak": 0, "today": { "day": "", "earned": {} },
    "milestones_shown": [], "last_greeting_day": "" }
  ```
- **Cộng điểm** (hàm thuần `award(state, event, today) -> state`), mỗi loại có trần/ngày:
  có mặt trong ngày +5 · xong task +2 (trần 10) · xong phiên Focus +3 (trần 9) · chat +1 (trần 5)
  · vuốt đầu +1 (trần 3) · chơi game +1 (trần 3). Không bao giờ trừ điểm.
- **Cấp:** Mới quen (0) → Quen biết (50) → Thân thiết (200) → Tri kỷ (500) → Không thể thiếu
  (1000). Khoảng 1 tháng dùng đều đặn lên cấp 3 (con số cụ thể chỉnh sau khi dùng thật). Cấp ảnh
  hưởng tới: câu mẫu được mở, độ thân mật trong prompt hệ thống (một dòng "mức thân thiết hiện
  tại: …"), motion khen thưởng.
- **Chào buổi sáng:** input đầu tiên trong ngày (sau 5:00) → chào kèm tóm tắt: số việc hạn hôm
  nay, việc quá hạn, chuỗi ngày. Có key thì LLM viết câu chào theo ngữ cảnh, không có thì câu mẫu.
- **Cột mốc:** 7 / 30 / 100 / 365 ngày kể từ `first_seen`, mỗi lần lên cấp, chuỗi 7 ngày. Mỗi
  cột mốc nói **đúng một lần** (`milestones_shown`), dùng hiệu ứng achievement.
- **Sinh nhật:** ô "Ngày sinh của bạn" (chỉ ngày/tháng, không năm, tuỳ chọn) trong Settings. Ngày
  đó: câu chúc riêng + motion. Ngày kỷ niệm quen nhau = `first_seen` hằng năm.
- **Hội thoại theo ngày:** `context_block` gửi thêm thứ trong tuần, cấp thân thiết, chuỗi ngày.
  Câu mẫu có biến thể thứ Hai / thứ Sáu / cuối tuần.
- Bảng nhỏ "💗 Thân thiết" (mở từ radial menu): cấp, thanh tiến độ, số ngày quen nhau, chuỗi ngày,
  các cột mốc đã đạt. Không có bảng xếp hạng, không có nhiệm vụ hằng ngày.

**Test:** `award` (trần/ngày, qua ngày mới, chuỗi đứt rồi nối lại, đồng hồ lùi), tính cấp, chọn
cột mốc chưa nói, đọc `bond.json` hỏng → mặc định.

**Đã code (2026-10-15, chưa phát hành, dự kiến cùng v0.6.0 với Phase 11):**
- `bond.rs` (thuần): `Bond`, `award` (trần/ngày, chuỗi ngày, không trừ điểm), `level` / `progress`
  / `level_name` (vi/ja/en), `milestones_due`, `daily_greeting_due`, `parse_birthday`.
  `store.rs`: `load_bond` / `update_bond` (khoá; chỉ ghi file khi dữ liệu đổi).
- Nguồn điểm đã nối: có mặt (lượt 30 giây của trọng tài, khi không vắng), xong việc
  (`rewards::complete`), xong phiên Tập trung, mỗi câu hỏi chat, vuốt đầu (lệnh `headpat` pet đã
  gửi sẵn), được đặt tên lần đầu. `Event::Game` chờ Phase 13.
- `companion.rs`: `bond_sync` (chào đầu ngày `Kind::Welcome` tag "daily" thay cho câu chào lại
  cùng lúc, cột mốc `Kind::Milestone` tag "ms:<tag>"), `show_bond_card`, `bond_context` (một dòng
  gửi model: cấp, số ngày quen nhau, chuỗi ngày — prompt dặn cấp càng cao nói càng thân mật).
- Bảng 💗 trên pet: `app/media/webview/bond.js`. Sinh nhật: hai ô ngày / tháng trong Cài đặt →
  *Bạn đồng hành*, lưu "MM-DD", `settings_set` kiểm hợp lệ.
- **Khác plan:** câu chào đầu ngày không đổi giọng theo cấp, và chưa có bộ câu khen "mở khoá" theo
  cấp (mục 12.5c) — cấp mới chỉ ảnh hưởng prompt chat và câu chúc mừng lên cấp. Thêm sau nếu cần.
- **Cần xem:** vòng menu chuột phải giờ có 8 mục ở vòng đầu (2 nhóm + Chat, Công việc, Tập trung,
  Thân thiết, Đừng làm phiền, Cài đặt); nếu chật quá thì gom bớt vào một nhóm.

### 12.8 Phase 13: Mini Games

- Radial menu thêm mục **🎮 Chơi**, mở một panel nhỏ cạnh pet (cùng kiểu panel chat).
- **Oẳn tù tì:** 3 nút ✊✌️✋, pet ra tay ngẫu nhiên (`crypto.getRandomValues`), biểu cảm thắng/
  thua/hoà, đếm tỉ số trong phiên. Chơi tới 3 thắng thì kết thúc ván.
- **Đoán số:** pet nghĩ số 1–100, tối đa 7 lượt, trả lời "cao hơn / thấp hơn", biểu cảm đổi dần
  khi người dùng gần đúng.
- Một ván ≤ 1–2 phút, đóng panel lúc nào cũng được, không lưu ván dở.
- Kết quả ván gửi về Rust (`pet:game:result`) chỉ để cộng điểm thân thiết.
- Logic (luật thắng thua, so số) tách thành hàm thuần trong `app/media/webview/games-core.js`, test
  bằng `node --test` (có sẵn trong Node, không thêm dependency). Thêm script `"test"` vào
  `package.json`. Đây sẽ là test JS đầu tiên của repo.
- Để sau (không làm bây giờ): trò "bắt chuột" cần cửa sổ phủ màn hình, trò dùng LLM (đố chữ).

**Đã code (2026-10-15, chưa phát hành, dự kiến cùng v0.6.0):**
- `app/js/games-core.js` (thuần, `window.GamesCore`): `randInt`, `rpsPick` / `rpsRound` /
  `rpsScore` (ai đủ 3 trận thắng trước), `guessNew` / `guess` (1–100, 7 lượt, nhập sai không mất
  lượt), `warmth`. Test: `games-core.test.js` (`npm test`).
- `app/media/webview/games.js`: bảng chơi trên pet, chữ vi/ja/en, nét mặt pet đổi theo từng lượt,
  ngẫu nhiên bằng `crypto.getRandomValues`. Hết ván → `game:result` → `bond::Event::Game`.
- Vòng menu: 🍅 Tập trung, 💗 Thân thiết, 🎮 Chơi gom vào nhóm **💞 Đồng hành** (vòng đầu còn 7 mục).
- **Chưa làm:** câu pet nói trong game là chữ trên bảng (không qua bubble / kho câu Rust), nên chưa
  theo cặp xưng hô tiếng Việt — dùng tên pet (hoặc "Em") và "Bạn". Trò "bắt chuột" và trò dùng AI
  vẫn để sau như plan.

**Làm lại ở v0.7.0 (2026-10-16):** bảng chơi trên pet (`games.js`) bị bỏ, thay bằng cửa sổ riêng:
- **Trang Mini Games** `arcade.html` + `js/arcade.js` (cửa sổ `arcade`, 726×475): 3 thẻ game
  (ảnh ở `media/arcade/`), game của ngày (`GamesCore.dailyPick`), thành tích đọc từ
  localStorage (`rpsStats`, `guessStats`, `minesStats`), điểm thân thiết qua `game_info`.
  "Chơi ngay" → lệnh `open_mini_game(kind)` ẩn trang và mở cửa sổ game.
- **Oẳn tù tì** `game.html` + `js/game-rps.js`, **Đoán số** `guess.html` + `js/game-guess.js`
  (648×404), **Gỡ bom** `mines.html` + `js/game-mines.js` (728×448, luật thuần ở
  `js/mines-core.js`, có test). Hết ván → lệnh `game_result` → `bond::Event::Game`.
- Ảnh biểu cảm Miu dùng chung ở `media/game/` (wink / think / cheer / pout / draw).
- Câu pet nói trong game đã theo cặp xưng hô (`game_info` trả `you` / `me`).
- **Chưa làm:** Gỡ bom dùng lại 5 ảnh của oẳn tù tì (nhân vật trong ảnh thiết kế không tách nền
  sạch được); CSS chung giữa các cửa sổ game chưa gom thành một file.

### 12.9 Settings mới (tổng hợp)

| Nhóm | Trường (`Settings`) | Mặc định |
|---|---|---|
| 🌸 Bạn đồng hành | `pet_name`, `user_name`, `vi_self`, `vi_user`, `ja_honorific`, `message_language` (chi tiết 12.4b) | rỗng / rỗng / em / anh / さん / vi |
| ⏰ Nhắc nhở | `remind_eyes_min`, `remind_water_min`, `remind_stand_min` (0 = tắt) | 45 / 60 / 60 |
| | `due_toast` (thông báo Windows khi tới hạn mà pet ẩn) | bật |
| | `quiet_gap_min` (khoảng nghỉ giữa hai câu tự nói) | 10 |
| | `llm_lines` (dùng LLM viết câu nhắc khi có key) | tắt |
| 🍅 Tập trung | `focus_min`, `break_min` | 25 / 5 |
| 🖥️ Nhận biết | `sleepy_idle_min` (0 = tắt), `welcome_back`, `quiet_fullscreen` | 10 / bật / bật |
| | `sysmon_enabled`, `cpu_warn_pct`, `ram_warn_pct` | **tắt** / 90 / 90 |
| 💗 Thân thiết | `birthday` (`MM-DD`, rỗng = không có) | rỗng |

Tất cả `#[serde(default = …)]`. "Đừng làm phiền" là trạng thái tạm (trong RAM), không lưu vào
Settings.

### 12.10 Quyết định đã chốt (2026-10-15)

1. **Nhắc nhở bật cả 3 loại theo mặc định:** nghỉ mắt 45', uống nước 60', đứng dậy 60'. Mỗi loại
   tắt riêng được. (Chu kỳ 60' của uống nước / đứng dậy là đề xuất, đổi được trong Settings.)
2. **Có dùng thông báo Windows (toast)** cho hạn chót khi pet đang ẩn (chi tiết 12.5e).
3. **Có Đặt tên + Xưng hô.** Pet xin người dùng đặt tên, cảm ơn khi được đặt tên. Làm **trước**
   kho câu mẫu, ở Phase 9B (12.4b). Mọi câu mẫu và prompt dùng biến, không viết cứng "anh/em".
4. **Câu mẫu đủ 3 thứ tiếng vi / ja / en**, chọn theo "Ngôn ngữ chữ" (Phase 9B lưu lựa chọn này
   lại và gửi sang Rust).

### 12.11 Lộ trình phát hành đề xuất

| Bản | Nội dung | Người dùng thấy gì |
|---|---|---|
| **v0.5.0** ✅ (phát hành 2026-10-15) | Ghim task (12.4c) + Phase 9 + 9B + Phase 10, cộng các việc phát sinh: link bấm được trong task, Bunny Lounge (nhân vật bám mép bảng Cài đặt / Công việc / Trợ lý văn bản, mặc định tắt), giao diện Trợ lý văn bản theo phong cách Cài đặt, sửa lỗi thu nhỏ bảng để pet nằm trơ | Xem CHANGELOG [0.5.0] |
| **v0.6.0** ✅ (phát hành 2026-10-15) | Phase 11 + 12 + 13 gộp một bản, cộng: "🗂️ Mở đầy đủ" trên menu chuột phải (trợ lý văn bản ở chế độ tự gõ), biểu tượng game trên đầu pet, nhóm menu 💞 Đồng hành | Xem CHANGELOG [0.6.0] |
| **v0.7.0** ✅ (phát hành 2026-10-16) | Trang Mini Games, oẳn tù tì và đoán số thành cửa sổ riêng, game mới Gỡ bom, ⚙ Cài đặt xuống ô dưới cùng của menu tròn | Xem CHANGELOG [0.7.0] |
| **v0.8.0** ✅ (phát hành 2026-10-16) | Mục 13 (Phase 14 + 15 + 16): hiệu ứng thời tiết toàn màn hình, theo mùa & giờ, thời tiết thật (Open-Meteo), thẻ dự báo, widget ghim, pet nói về thời tiết + bản tin sáng; nhóm ☁️ Thời tiết trên menu tròn; Bunny Lounge thêm thẻ thời tiết + trang Mini Games | Xem CHANGELOG [0.8.0] |
| **v0.9.0** ✅ (phát hành 2026-10-16) | Lịch tháng ghim trên desktop: âm lịch Việt Nam (múi +7), ngày lễ, chấm hạn chót task, thêm việc theo ngày, Miu mini; mở từ khay, 💞 → 📅, Cài đặt. Sửa pet nằm ngoài màn hình khi rút màn hình phụ, autostart trỏ nhầm bản dev; ảnh mới cho nhân vật thẻ thời tiết + Mini Games | Xem CHANGELOG [0.9.0] |
| **v0.9.1** ✅ (phát hành 2026-10-16) | Pet nói về lịch (ngày lễ, Rằm, mùng 1, nhắc tối hôm trước; lunar.rs, calendar_talk.json); sinh nhật 🎂 trên lịch; đồng hồ ghim có giờ thế giới (tối đa 2 múi) + hàng 🍅 Tập trung; khung widget chung pin_widget.rs; nhóm 🧩 Widget trong Cài đặt + Xếp gọn; 4 biểu cảm Miu mới; test dịch thử mọi file JS | Xem CHANGELOG [0.9.1] |
| **v0.9.2** (chuẩn bị phát hành 2026-10-19) | Mục 14 Phase 17: 🍬 kẹo (collection.json, cộng song song điểm thân thiết), cửa sổ Bộ sưu tập, 5 skin widget, 3 hiệu ứng (pháo hoa, mưa hoa anh đào, pháo giấy), đổi bộ nhân vật + bộ mặt Miu lúc chạy. Nội dung Phase 18: Bunny Lounge còn 2 cô thỏ; bộ nhân vật 🐱 Neko Lounge, 🦊 Hồ ly, ❄️ Xứ Tuyết; bộ Miu ❄️ tuyết, 🦇 Gothic (miu_sets.rs); giao diện ❄️ băng tuyết / 🦇 Gothic Lolita áp cho mọi khung (theme-apply.js, theme-snow/gothic/pages.css); nhân vật bám mép cả Bộ sưu tập; Miu trong game theo bộ đã chọn | Xem CHANGELOG [0.9.2] |

Toàn bộ mục 12 (Phase 9 → 13) đã xong ở v0.5.0 + v0.6.0. Việc còn treo nằm ở ghi chú "Chưa làm"
/ "Còn hạn chế" của từng phase, và mục 11 (giọng nói nhân bản, việc nhỏ còn tồn).

Mỗi bản: `cargo test` + `npm test` + `cargo build` xanh (CI chạy `cargo check`, `cargo test`,
`npm test`), thêm mục CHANGELOG ở đúng version sẽ ship, bump version ở `package.json`,
`package-lock.json`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`.

### 12.12 Kiểm thử bằng tay (ngoài unit test)

- Chỉnh chu kỳ nhắc về 1 phút để thử nhanh: nhắc đúng lúc; bỏ máy 5 phút → quay lại thì được chào
  và bộ đếm nghỉ mắt đã đặt lại; không có hai câu tự nói cách nhau dưới `quiet_gap_min`.
- Task hạn sau 2 phút → nhắc đúng một lần ở mốc "tới giờ", khởi động lại app không nhắc lại.
- Mở PowerPoint trình chiếu / video toàn màn hình → pet im; thoát ra → câu bị hoãn mới nói.
- Phiên Focus: ring chạy, không có câu nhắc chen ngang, hết pha thì khen + gợi ý nghỉ.
- Tắt cảnh báo CPU → xác nhận không còn lấy mẫu (CPU của app không đổi).
- Xoá `bond.json` / sửa nó thành JSON hỏng → app vẫn chạy, bắt đầu lại từ cấp 0.
- Không nhập API key: mọi tính năng ở mục 12 vẫn chạy bằng câu mẫu.
- Cài mới (xoá `settings.json`): pet xin đặt tên đúng một lần; đặt tên → câu cảm ơn có tên; đóng
  không đặt → 7 ngày sau mới hỏi lại. Đặt tên qua chat "từ giờ em tên là …" → Settings cập nhật.
- Đổi cặp xưng hô sang "mình–bạn" → câu nhắc, câu khen, câu chat đều đổi theo, không còn "anh".
- Đổi "Ngôn ngữ chữ" sang ja rồi en → câu nhắc + chat trả lời đúng ngôn ngữ; khởi động lại app
  vẫn giữ lựa chọn.
- Bản build đã cài: ẩn pet, đặt task hạn sau 2 phút → toast hiện, bấm toast mở pet + bảng Tasks;
  hiện pet lại thì không nhắc lặp. Tắt thông báo của app trong Windows → nút Gửi thử báo được
  hướng dẫn kiểm tra.

## 13. Thời tiết: hiệu ứng toàn màn hình + dự báo

> Lập kế hoạch ngày 2026-10-16, tính từ v0.7.0. Phase 14 → 16 đã code, phát hành chung ở **v0.8.0**.

### 13.1 Mục tiêu

- **Hiệu ứng thời tiết phủ toàn màn hình:** mưa, tuyết, gió (lá / cánh hoa anh đào), sương mù,
  nắng, sao đêm, sấm chớp. Hiệu ứng nằm **trên desktop và các app khác nhưng dưới pet và các bảng
  của app**, và **không bao giờ chặn chuột / bàn phím**.
- **Pet sống cùng thời tiết:** đổi nét mặt, nói câu hợp thời tiết ("Mưa rồi, {you} nhớ mang ô
  nha~"), có câu báo thời tiết buổi sáng.
- **Thẻ dự báo là một cửa sổ riêng** (đã chốt 2026-10-16), giống thẻ Widgets của Windows: nhiệt
  độ hiện tại, cao / thấp, dự báo 5–7 ngày, biểu đồ nhiệt độ theo giờ kèm khả năng mưa (%), đổi
  °C / °F, "cập nhật lúc …".
- **Ghim thời tiết lên desktop** (đã chốt 2026-10-16), giống ghim task: một widget nhỏ nằm trên
  màn hình, kéo đi được, nhớ vị trí, tự cập nhật (13.7b).

### 13.2 Nguồn dữ liệu: Open-Meteo

| Lựa chọn | Kết luận |
|---|---|
| MSN Weather (thẻ Widgets của Windows) | **Không dùng.** Không có API công khai; các endpoint ẩn `api.msn.com/weather/...` dùng key nhúng của Microsoft, không tài liệu, có thể bị chặn, vi phạm điều khoản. |
| Azure Maps Weather | Chính thức nhưng cần subscription key. Key không giấu được trong app desktop phát hành → mỗi người phải tự tạo key hoặc phải có server trung gian. Không đáng. |
| **Open-Meteo** | **Chọn.** Miễn phí, **không cần key**, có hiện tại + theo giờ + theo ngày, khả năng mưa, gió, mã thời tiết WMO, kèm API tìm toạ độ theo tên thành phố. |

- Dự báo: `https://api.open-meteo.com/v1/forecast` với `current`, `hourly`
  (`temperature_2m`, `precipitation_probability`, `weather_code`), `daily`
  (`temperature_2m_max/min`, `weather_code`), `timezone=auto`.
- Tìm thành phố: `https://geocoding-api.open-meteo.com/v1/search?name=…&language=vi|ja|en`.
- **Ghi công bắt buộc** (dữ liệu CC BY 4.0): dòng nhỏ "Dữ liệu thời tiết: Open-Meteo.com" trên thẻ
  dự báo + trong README.
- Gói free dành cho dùng phi thương mại, có giới hạn lượt gọi mỗi ngày. App hỏi **30 phút / lần**
  (chỉ khi tính năng bật) nên dư sức. Nếu sau này thương mại hoá thì xem lại gói trả phí.
- Gọi HTTP từ **Rust** (`reqwest` đã có trong `Cargo.toml`), không gọi từ WebView: một chỗ cache,
  một chỗ xử lý lỗi, các cửa sổ chỉ nhận kết quả.

### 13.3 Nguyên tắc (chốt trước khi làm)

1. **Mặc định TẮT**, giống Bunny Lounge. Bật trong Cài đặt → *Thời tiết* hoặc tray.
2. **Riêng tư:** **không tự dò vị trí theo IP / GPS.** Người dùng tự gõ tên thành phố, chọn từ
   danh sách gợi ý; chỉ lưu tên + toạ độ trong `settings.json` trên máy. Thứ duy nhất gửi đi là
   toạ độ thành phố tới Open-Meteo. Như mọi dữ liệu người dùng: không đưa thành phố thật vào repo /
   test / ví dụ (test dùng toạ độ giả).
3. **Không chặn máy:** lớp phủ luôn xuyên chuột, không nhận focus, không hiện trên taskbar. Luôn
   có **lối thoát khẩn**: phím tắt toàn cục bật / tắt hiệu ứng + mục tray.
4. **Nhẹ:** giới hạn số hạt, ~30 khung hình/giây, **tự dừng** khi có app toàn màn hình
   (`awareness.rs` đã có), khoá máy / rời máy lâu, chạy pin (tuỳ chọn), cửa sổ phủ bị ẩn.
5. **Không cần mạng vẫn chạy:** chế độ chọn tay và theo mùa không gọi mạng. Mất mạng ở chế độ
   thời tiết thật → giữ cache tối đa 3 giờ, quá hạn thì tắt hiệu ứng và thẻ ghi "chưa cập nhật
   được", không báo lỗi ồn ào.
6. **An toàn ánh sáng:** sấm chớp có công tắc riêng, **mặc định tắt**; khi bật thì tối đa một lần
   chớp mỗi 8 giây, độ sáng thấp, không nhấp nháy liên tục. Tôn trọng `prefers-reduced-motion`
   (giảm hạt, bỏ chớp).
7. **Logic quyết định là hàm thuần có test:** mã WMO → loại hiệu ứng + cường độ, hiệu ứng theo
   mùa / giờ, khi nào hỏi lại API, khi nào pet được nói câu thời tiết, tham số hạt theo cường độ.

### 13.4 Ba chế độ

| Chế độ | Nguồn | Mạng |
|---|---|---|
| **Chọn tay** | Người dùng chọn: mưa / tuyết / gió / hoa anh đào / sương / nắng / sao / tắt | Không |
| **Theo mùa & giờ** | Tháng + giờ địa phương: xuân hoa anh đào, hè nắng, thu lá bay, đông tuyết; đêm có sao | Không |
| **Thời tiết thật** | Open-Meteo theo thành phố đã chọn, hỏi lại mỗi 30 phút | Có |

Đổi mã WMO (rút gọn): 0–1 quang (đêm: sao) · 2–3 mây · 45, 48 sương mù · 51–57 mưa phùn (mưa nhẹ)
· 61–67, 80–82 mưa (nhẹ / vừa / to theo mã) · 71–77, 85–86 tuyết · 95–99 dông (mưa to + chớp nếu
bật). Gió ≥ 30 km/h: thêm lá bay, mưa / tuyết rơi xiên.

### 13.5 Kiến trúc

```
src-tauri/src/
  weather.rs      (mới, thuần + test) mã WMO → WeatherKind + Intensity; chế độ mùa/giờ;
                  parse JSON Open-Meteo → Forecast; cache + khi nào hỏi lại; khi nào pet được nói
  weather_net.rs  (mới) gọi Open-Meteo bằng reqwest (timeout 10 s), vòng nền 30 phút, geocoding
  power.rs        (mới, nhỏ) đang cắm sạc hay chạy pin (GetSystemPowerStatus)
  windows.rs      cửa sổ phủ "fx-<n>" mỗi màn hình + cửa sổ thẻ dự báo "weather"
  pins.rs         thêm widget ghim thời tiết "pin-weather" (dùng chung cách tạo / kéo / nhớ vị trí
                  với tờ note task; khớp sẵn glob "pin-*" trong capabilities)
  lines.rs        câu thời tiết vi/ja/en (biến xưng hô như mọi câu khác)
  companion.rs    nối WeatherKind vào trọng tài lời nói + nét mặt pet
app/
  fx.html + js/fx.js            canvas hạt, nhận "fx-set {kind, intensity, wind, lightning}"
  js/fx-core.js (+ test)        tham số hạt thuần: số hạt, tốc độ, góc rơi theo gió, chu kỳ chớp
  weather.html + js/weather.js  thẻ dự báo
  weather-pin.html + js/weather-pin.js  widget ghim thời tiết
```

**Cửa sổ phủ (`fx-<n>`):**
- **Mỗi màn hình một cửa sổ** (không trải qua cả màn hình ảo: hai màn khác DPI sẽ lệch toạ độ).
  Theo dõi cắm / rút màn phụ để dựng lại, không để cửa sổ mồ côi.
- Trong suốt, không viền, không bóng, `always_on_top`, `skip_taskbar`, không focus được,
  **`set_ignore_cursor_events(true)`** ngay khi tạo và **kiểm lại tường minh** sau khi hiện (API
  thất bại im lặng → không đặt được thì không hiện lớp phủ, log lý do, báo trong Cài đặt).
- Phủ cả màn hình (kể cả taskbar); "mặt đất" cho tuyết đọng / mưa bắn tóe lấy theo work area.
- **Thứ tự lớp:** pet, menu tròn, bảng Cài đặt / Công việc / game phải nằm **trên** lớp phủ. Trên
  Windows các cửa sổ topmost xếp theo lần kích hoạt cuối → sau khi hiện lớp phủ thì nâng lại pet và
  các bảng đang mở. **Đây là spike đầu tiên của Phase 14**, làm trước khi viết hiệu ứng.
- Canvas 2D, `requestAnimationFrame` khoá ~30 fps; ẩn cửa sổ là dừng hẳn vòng vẽ.

**Hiệu ứng (`fx.js`):**
- Mưa: vệt mảnh xiên theo gió, bắn tóe nhỏ ở mặt đất; mưa to thêm một lớp mưa xa mờ hơn.
- Tuyết: bông to nhỏ lắc lư, đọng thành dải mỏng ở mặt đất rồi tan dần (giới hạn chiều cao).
- Thu / gió: lá bay ngang có xoay. Xuân: cánh hoa anh đào.
- Sương mù: vài lớp gradient trôi chậm, độ mờ thấp.
- Nắng: tia sáng nhẹ ở một góc. Đêm quang: vài ngôi sao lấp lánh ở mép trên.
- Dông: mưa to + chớp (nếu bật) + tiếng sấm nhỏ (theo công tắc âm thanh).
- Cường độ Nhẹ / Vừa / Dày nhân số hạt (ví dụ mưa 80 / 160 / 280 hạt trên màn 1080p).

### 13.6 Pet phản ứng theo thời tiết

- Kind mới `Weather` trong trọng tài (ưu tiên thấp, dưới nhắc hạn chót / sức khoẻ), tuân
  `quiet_gap_min`, Đừng làm phiền, toàn màn hình, phiên Focus như mọi câu tự nói.
- Nói khi **thời tiết đổi loại** (quang → mưa, mưa → tạnh), tối đa một câu mỗi 2 giờ cho cùng
  loại. Chế độ chọn tay: một câu ngay khi người dùng bật hiệu ứng.
- **Bản tin buổi sáng** (chế độ thời tiết thật): gộp vào câu chào đầu ngày đã có — "Hôm nay
  {min}–{max}°, chiều mưa {p}%, {you} nhớ mang ô nha~". Một lần mỗi ngày.
- Nét mặt: mưa → `sad` ngắn, tuyết → `surprised`, nắng đẹp → `happy`.
- Không cần API key: câu mẫu `lines.rs` vi/ja/en (test kiểm đủ ngôn ngữ như các nhóm khác). Có key
  + bật `llm_lines` thì LLM viết lại cho tự nhiên.
- Để sau: pet cầm ô / mặc áo ấm (cần thêm motion / parts trong model Live2D).

### 13.7 Thẻ dự báo (`weather.html`)

- Mở từ menu tròn 💞 Đồng hành → ☁️ Thời tiết và từ tray. Cửa sổ nhỏ cùng phong cách các cửa sổ
  game (vẽ khổ lớn rồi `zoom`).
- Nội dung (bám theo thẻ Widgets): tên thành phố + giờ địa phương, "cập nhật … phút trước",
  °F / °C, icon + nhiệt độ hiện tại + mô tả + cao / thấp, dải 5–7 ngày (icon + cao / thấp), biểu
  đồ nhiệt độ 24 giờ tới với khả năng mưa (%) dưới mỗi mốc, dòng ghi công Open-Meteo.
- Phần riêng của app: Miu nhỏ ở góc nói một câu theo thời tiết; nút "Bật hiệu ứng như ngoài trời"
  nếu hiệu ứng đang tắt.
- Chưa chọn thành phố → thẻ hiện ô tìm thành phố (gợi ý từ geocoding, chọn một là lưu).
- Biểu đồ vẽ bằng SVG tự viết, không thêm thư viện biểu đồ.

### 13.7b Ghim thời tiết lên desktop (`pin-weather`)

Làm theo đúng cách ghim task (12.4c) để người dùng thấy quen và tái dùng code `pins.rs`:
- **Ghim từ đâu:** nút 📌 trên thẻ dự báo, mục tray "📌 Ghim thời tiết", và menu chuột phải của
  widget để gỡ. Chưa chọn thành phố thì nút ghim mờ, kèm gợi ý "Chọn thành phố trước nha~".
- **Cửa sổ:** trong suốt, không viền, ẩn khỏi taskbar + Alt+Tab (`WS_EX_TOOLWINDOW`), **không
  luôn-nổi theo mặc định** (nằm như widget, cửa sổ khác che được); nút 📍 bật luôn-nổi. Kéo ở bất
  cứ đâu; vị trí lưu sau khi đứng yên 400 ms. Mở app thì widget hiện lại đúng chỗ; màn hình phụ đã
  rút thì về màn hình chính.
- ~~Đầu ghim là mặt nhân vật~~ — đổi theo người dùng (2026-10-16): không có đầu ghim, Miu
  (`media/game/rps-wink.png`, cùng hình trên thẻ dự báo) đứng ở góc phải hàng nhiệt độ.
- **Hai cỡ** (đổi bằng nhấp vào góc widget):
  - *Nhỏ:* icon + nhiệt độ hiện tại + cao / thấp + khả năng mưa giờ tới.
  - *Lớn:* thêm tên thành phố, "cập nhật … phút trước", 3 mốc giờ tới hoặc 3 ngày tới.
- **Bấm đúp** mở thẻ dự báo đầy đủ. ✕ gỡ ghim.
- **Cập nhật:** Rust `emit_to("pin-weather", "weather-data", …)` mỗi lần có dữ liệu mới; widget
  không tự gọi mạng. Mất mạng → hiện số liệu cũ, mờ đi, kèm "chưa cập nhật được".
- Một widget thời tiết (một thành phố), **không tính** vào giới hạn 8 tờ note task.
- Màu nền đổi nhẹ theo thời tiết (nắng ấm, mưa xanh xám, đêm tím) nhưng vẫn cùng phong cách hồng
  của app; chữ đủ tương phản trên nền desktop sáng lẫn tối.
- **Vòng hỏi Open-Meteo chạy khi:** chế độ `live`, **hoặc** widget đang ghim, **hoặc** thẻ dự báo
  đang mở. Nghĩa là ghim widget không bắt buộc bật hiệu ứng toàn màn hình.
- Giới hạn như tờ note task: Win+D (Show desktop) che cả widget.
- Lưu ý khi code: nhãn `pin-weather` khớp glob `pin-*` (capability dùng chung được), còn
  `pins::id_from_label` chỉ nhận id toàn chữ số nên không nhầm nó là task (thêm test cho
  `"pin-weather"`).

### 13.8 Settings mới (nhóm ☁️ Thời tiết)

| Trường (`Settings`) | Giá trị | Mặc định |
|---|---|---|
| `weather_mode` | `off` / `manual` / `season` / `live` | `off` |
| `weather_manual` | `rain` / `snow` / `wind` / `sakura` / `fog` / `sun` / `stars` | `rain` |
| `weather_intensity` | `light` / `medium` / `heavy` | `medium` |
| `weather_city`, `weather_lat`, `weather_lon` | tên + toạ độ người dùng chọn | rỗng |
| `weather_units` | `c` / `f` | `c` |
| `weather_lightning` | sấm chớp | **tắt** |
| `weather_pause_battery` | dừng hiệu ứng khi chạy pin | bật |
| `weather_talk` | pet nói câu thời tiết + bản tin sáng | bật |
| `weather_pin`, `weather_pin_pos`, `weather_pin_top`, `weather_pin_size` | widget ghim: đang ghim, vị trí, luôn-nổi, cỡ `small` / `large` | tắt / rỗng / tắt / `small` |
| `weather_shortcut` | phím tắt bật / tắt nhanh hiệu ứng | `Ctrl+Alt+W` (đăng ký thất bại / trùng thì báo trong Cài đặt) |

Tất cả `#[serde(default = …)]`. Nhãn mới có đủ vi / ja / en.

### 13.9 Các phase

| Phase | Nội dung | Ước lượng |
|---|---|---|
| **14 — Lớp phủ + hiệu ứng chọn tay** | Spike thứ tự lớp + xuyên chuột trên 1 và 2 màn hình; cửa sổ `fx-<n>`; `fx.js` mưa / tuyết / gió / hoa; cường độ; tự dừng khi toàn màn hình / khoá máy / rời máy / chạy pin; phím tắt + tray; nhóm Cài đặt *Thời tiết* (tắt / chọn tay) | ~2 ngày |
| **15 — Pet phản ứng + theo mùa** | `weather.rs` phần mùa / giờ (thuần + test); sương / nắng / sao; Kind `Weather` trong trọng tài; câu `lines.rs` 3 thứ tiếng; nét mặt | ~1 ngày |
| **16 — Thời tiết thật + thẻ dự báo + ghim** | `weather_net.rs` + geocoding; parse + cache + đổi mã WMO (thuần + test); chế độ `live` + dông; bản tin sáng; cửa sổ riêng `weather.html` (dự báo ngày, biểu đồ giờ, °C / °F, ghi công); widget ghim `pin-weather` (13.7b) | ~3 ngày |

**Phase 14 — đã code (2026-10-16, chưa phát hành):**
- `src-tauri/src/fx.rs`: phần thuần có test (`pause_reason`, `overlay_rect` thấp hơn màn 1px,
  `ground` theo vùng làm việc, `holes` theo từng màn + DPI, `wants_hole`, làm sạch giá trị cài
  đặt) + vòng nền 500 ms: dựng / gỡ `fx-<n>` theo danh sách màn hình, kiểm cờ
  `WS_EX_TRANSPARENT` sau `set_ignore_cursor_events` (không có thì không hiện, báo trong Cài
  đặt, thôi thử lại tới lần đổi cài đặt kế tiếp), hiện bằng `SW_SHOWNOACTIVATE`, nâng pet lên trên
  lớp phủ bằng `SetWindowPos(HWND_TOPMOST)` mỗi nhịp, dừng theo toàn màn hình / rời máy / pin
  (`GetSystemPowerStatus`, feature `Win32_System_Power`).
- Phím tắt `weather_shortcut` (mặc định Ctrl+Alt+W) đăng ký lại sau mỗi lần `unregister_all` của
  phím chụp chữ; tray "🌦 Bật / Tắt hiệu ứng thời tiết"; menu tròn 💞 → 🌦.
- `app/fx.html` + `js/fx.js` (canvas, ~30 fps, dừng hẳn khi không active, khoét lỗ bằng
  `clearRect`), `js/fx-core.js` + test (số hạt theo diện tích / cường độ có trần, sinh hạt, bước
  rơi, tuyết đọng, lỗ). Mưa bắn tóe + tuyết đọng ở mép trên taskbar.
- Cài đặt: `weather_mode` (`off` / `manual`), `weather_manual`, `weather_intensity`,
  `weather_pause_battery`, `weather_shortcut`; nhóm 🌦 Thời tiết áp dụng ngay khi đổi.
- **Chưa kiểm trên máy thật:** thứ tự lớp (pet / menu tròn / bảng có thật sự trên lớp phủ),
  xuyên chuột, hai màn hình khác DPI, mức CPU / GPU. Nếu menu tròn hoặc bông hoa (topmost, hiện
  sau lớp phủ) bị lớp phủ đè thì chúng vẫn được khoét lỗ nên không bị mưa vẽ lên.

**Phase 15 — đã code (2026-10-16, chưa phát hành):**
- `src-tauri/src/weather.rs` (thuần + test): `KINDS` (7 loại, đồng bộ `FxCore.KINDS`),
  `season_kind(tháng, giờ)` (xuân hoa anh đào, hè nắng, thu lá, đông tuyết cả đêm, đêm 19:00–6:00
  có sao), `expression` (chỉ nét có trong `expression.js`, không có "sad" nên mưa dùng `shy`),
  `Talk::observe` (nói khi đổi loại hoặc vừa bật; cùng loại cách ≥ 2 giờ; lần quan sát đầu sau
  khi mở app chỉ ghi nhận, không nói).
- `fx.rs`: `Mode::Season`, `current_kind()` cho trọng tài, phím tắt / tray bật lại đúng chế độ
  trước lúc tắt (`weather_last_mode`).
- Trọng tài: Kind `Weather` (dưới `SysWarn`, trên `Chatter`), ttl 10 phút, câu
  `Key::Weather*` 3 thứ tiếng × 3 biến thể; công tắc `weather_talk`.
- `fx-core.js` / `fx.js`: sương mù (mảng gradient trôi ngang), nắng (quầng sáng + 7 tia ở góc
  trên phải), sao (lấp lánh ở nửa trên màn hình).
- Cài đặt: ô chế độ Tắt / Chọn hiệu ứng / Theo mùa & giờ; ô hiệu ứng chỉ bật khi chọn tay.
- **Chưa làm (để Phase 16):** dông + chớp (cần thời tiết thật), bản tin buổi sáng.

**Phase 16 — đã code (2026-10-16, chưa phát hành):**
- `weather.rs` thêm phần thuần có test: `wmo_effect` (mã WMO → loại / cường độ / dông; trời nhiều
  mây không vẽ; gió ≥ 30 km/h mà trời khô thì lá bay), `parse_forecast` / `parse_places` (bỏ giờ /
  ngày thiếu số, từ chối toạ độ sai), `forecast_url` / `geocode_url` / `url_encode`,
  `fetch_due` (30 phút; lỗi thì 5 phút sau thử lại), `is_stale` (> 3 giờ), `brief_due`,
  `brief_rainy`, `fmt_temp`. Đã thử URL thật với toạ độ giả: đúng cấu trúc JSON.
- `weather_net.rs`: cache `weather.json` trong thư mục cấu hình; vòng nền 2 giây chỉ gọi mạng khi
  có thành phố VÀ (chế độ live hoặc widget đang ghim hoặc thẻ dự báo đang mở); `live_effect` cho
  fx.rs (dữ liệu quá cũ → None, tắt hiệu ứng); đổi thành phố thì bỏ cache cũ ngay; `take_brief`
  cho trọng tài; widget ghim `pin-weather` (dựng / vị trí / luôn nổi / cỡ / Alt+F4 = gỡ ghim).
- `fx.rs`: `Mode::Live`, `Show { kind, intensity, lightning, thunder }`; cường độ live theo trời
  thật; chớp chỉ khi dông VÀ bật `weather_lightning`. `fx.js`: chớp nhẹ (đỉnh 0.18, tắt dần
  0.35 s, cách ≥ 8 s, bỏ khi reduced-motion).
- Trọng tài: câu "dông" (`storm`), bản tin sáng `WeatherBrief` / `WeatherBriefRain` (biến
  {min} {max} {rain}, ttl 30 phút, một lần / ngày 6–11 giờ, chỉ khi dữ liệu còn mới).
- Thẻ dự báo `weather.html` + `js/weather.js` (cửa sổ dựng sẵn "weather", 475×396), chữ / icon /
  biểu đồ thuần trong `js/weather-core.js` + test. Widget `weather-pin.html` + `js/weather-pin.js`.
- Mở thẻ: tray "☁️ Dự báo thời tiết", menu tròn 💞 → ☁️, nút trong Cài đặt. Cài đặt thêm chế độ
  "Theo thời tiết thật", công tắc chớp; mỗi chế độ có một dòng giải thích ngắn.
- Sửa sau khi người dùng thử (2026-10-16): các lệnh thời tiết đụng cửa sổ chuyển sang async (lệnh
  đồng bộ dựng / gỡ widget làm treo WebView2); kéo thẻ ở bất cứ đâu bằng JS `startDragging`; nút
  × ẩn bằng lệnh Rust `weather_close`; widget mới hiện ngay cạnh thẻ (`weather_net::beside`);
  widget bỏ đầu ghim, có Miu ở góc; ô tìm thành phố nằm ngay trong thẻ (không phủ cả thẻ); cô cáo
  steampunk `media/side/weather-girl.png` bám mép trái thẻ (sidegirl `side_weather`, theo công
  tắc Bunny Lounge); menu tròn: nhóm ☁️ Thời tiết (Dự báo / Hiệu ứng / Ghim) thay ô 🔕, 🔕 vào
  nhóm 💞 — vòng vẫn 8 ô nên ⚙ vẫn ở đáy. Lệnh `weather:pin` từ pet chạy ở luồng riêng.
- **Chưa làm / giới hạn:** tiếng sấm (chưa có file âm thanh); mưa / tuyết xiên theo gió (chỉ có lá
  bay khi gió mạnh); Win+D che cả widget như tờ note task.

### 13.10 Rủi ro

| Rủi ro | Cách giảm |
|---|---|
| Lớp phủ trong suốt toàn màn hình tốn GPU / CPU (Windows phải ghép hình liên tục) | Giới hạn hạt + 30 fps; dừng hẳn vòng vẽ khi ẩn; tự dừng theo pin / toàn màn hình / rời máy; đo trong Task Manager ở 3 mức cường độ trước khi phát hành |
| Xuyên chuột hỏng → cả màn hình "đơ" | Kiểm tường minh sau khi đặt; không đặt được thì không hiện; phím tắt toàn cục + tray để tắt ngay |
| Lớp phủ đè lên pet / bảng của app | Spike ở đầu Phase 14; nâng lại cửa sổ của app sau khi hiện lớp phủ |
| Nhiều màn hình, DPI khác nhau, cắm / rút màn | Mỗi màn một cửa sổ; dựng lại khi danh sách màn hình đổi |
| Không phủ được game chạy fullscreen độc quyền | Chấp nhận: lúc đó vốn phải tắt hiệu ứng |
| Sấm chớp gây khó chịu | Mặc định tắt, giới hạn tần suất + độ sáng, tôn trọng reduced-motion |
| Open-Meteo đổi API / giới hạn lượt | Gói lời gọi trong `weather_net.rs`; cache; lỗi thì tắt êm chế độ `live`, các chế độ khác vẫn chạy |

### 13.11 Kiểm thử bằng tay

- Bật mưa mức Dày: click, kéo cửa sổ, gõ phím, cuộn ở app bên dưới đều bình thường; pet, menu tròn,
  bảng Cài đặt vẫn nằm **trên** lớp mưa và bấm được.
- Hai màn hình khác DPI: phủ đúng cả hai, không lệch, không hở; rút màn phụ → không còn cửa sổ
  phủ mồ côi.
- Video / trình chiếu toàn màn hình → hiệu ứng dừng, thoát ra thì chạy lại. Rút sạc (khi bật dừng
  theo pin) → dừng.
- Phím tắt bật / tắt hiệu ứng chạy cả khi app khác đang focus.
- Chế độ `live`: chọn thành phố → thẻ dự báo khớp số liệu Open-Meteo; ngắt mạng → thẻ ghi chưa cập
  nhật được, không báo lỗi liên tục; có mạng lại → tự cập nhật.
- Ghim thời tiết: widget hiện đúng số liệu như thẻ dự báo; kéo đi, thoát app, mở lại → đúng chỗ
  cũ; rút màn phụ → widget về màn chính; 📍 bật luôn-nổi hoạt động; đổi cỡ nhỏ / lớn được lưu; ghim
  khi chế độ hiệu ứng đang tắt vẫn tự cập nhật mỗi 30 phút; gỡ ghim + đóng thẻ dự báo + chế độ khác
  `live` → không còn gọi mạng.
- Thời tiết đổi (giả lập bằng dữ liệu test) → pet nói đúng một câu, không lặp trong 2 giờ; bản
  tin sáng một lần mỗi ngày; Đừng làm phiền / Focus → pet im.

### 13.12 Câu hỏi cần chốt trước khi code

1. ~~Phase 14 làm hiệu ứng nào trước?~~ **Đã chốt:** mưa, tuyết, hoa anh đào, lá bay; sương / nắng /
   sao ở Phase 15, dông ở Phase 16.
2. ~~Phím tắt khẩn?~~ **Đã chốt:** `Ctrl+Alt+W` (đổi được trong Cài đặt).
3. ~~Thẻ dự báo: cửa sổ riêng hay tab trong bảng Công việc?~~ **Đã chốt (2026-10-16): cửa sổ
   riêng, kèm ghim thời tiết lên desktop** (13.7b).
4. ~~Chia bản phát hành?~~ **Đã chốt:** gộp cả ba phase vào **v0.8.0**.

---

## 14. 🍬 Kẹo · Bộ sưu tập · Gacha (Phase 17–19)

> Kế hoạch duyệt ngày 2026-10-17. **Phase 17 đã code (2026-10-18), phát hành ở v0.9.2**, kèm 2 món đầu của Phase 18 (bộ nhân vật ❄️ Xứ Tuyết, 🐱 Neko Lounge (kèm giao diện Cài đặt Gothic Lolita) và 🦊 Hồ ly tách từ Bunny Lounge, bộ ❄️ Miu tuyết, bộ 🦇 Miu Gothic). Phase 19 (gacha) chưa làm. Thêm bộ Miu mới: xem hướng dẫn đầu file `src-tauri/src/miu_sets.rs`.

### 14.1 Mục tiêu & nguyên tắc

App đã có vòng lặp thưởng: **điểm thân thiết** (`bond.rs`) cộng khi có mặt, xong việc, xong phiên
Tập trung, chat, vuốt đầu, thắng game, đặt tên; có trần mỗi ngày; lên cấp 50/200/500/1000. Nhưng
điểm này **không tiêu được**. Đồng thời đồ trang trí đã có (Bunny Lounge, 9 biểu cảm Miu, 3 widget
cùng màu tím) mà **chưa chọn được**: ảnh nhân vật bám mép nằm cứng trong `sidegirl::SPECS`, màu
widget nằm cứng trong CSS.

Mục tiêu: lớp "làm việc → được thưởng → mở đồ mới" đặt lên trên những gì đã có. Ba nguyên tắc:

1. **Chỉ mở khoá đồ trang trí.** Lịch, đồng hồ, nhắc hạn, dịch… luôn miễn phí. Gacha chỉ vui khi
   thứ quay ra là thứ *thích có*, không phải thứ *cần có*.
2. **Kẹo đi song song điểm thân thiết**, cùng nguồn, cùng trần mỗi ngày. Không có nguồn kẹo mới
   tạo động cơ cày.
3. **Không bao giờ bán bằng tiền thật.** Gacha trả tiền vướng luật nhiều nước (Bỉ, Hà Lan cấm loot
   box; Nhật cấm kompu gacha). Miễn phí hoàn toàn thì không vướng.

Thứ tự: **Phase 17** kẹo + bộ sưu tập (đổi thẳng) + hạ tầng chọn bộ → **Phase 18** nội dung (ảnh
mới) → **Phase 19** gacha. Gacha đặt cuối vì chỉ vui khi có ≥ 15–20 món.

### 14.2 Kinh tế kẹo

**Nguồn.** `companion::bond_award` hiện gọi `bond::award` rồi vứt số điểm thực cộng. Sửa để lấy
`gain` và cộng kẹo `gain × candy_mult(ev)` (bảng hệ số, mặc định ×1). Trần kế thừa từ bond. Chỉ
đụng file khi `gain > 0` (sự kiện Có mặt bắn mỗi 30 giây).

| `bond::Event` | Điểm/lần · trần/ngày | Kẹo tối đa/ngày |
|---|---|---|
| Present | 5 · 5 | 5 |
| TaskDone | 2 · 10 | 10 |
| FocusDone | 3 · 9 | 9 |
| Chat | 1 · 5 | 5 |
| Headpat | 1 · 3 | 3 |
| Game (thắng) | 1 · 3 | 3 |
| Named (một lần) | 10 · 10 | 10 |
| **Tổng ngày thường** | | **35** (thực tế 15–25) |

Thưởng thêm chỉ bằng kẹo: mỗi **cột mốc** (`bond::Milestone`) **+20 🍬**, móc ở
`companion::milestone_line`. Không thưởng chuỗi ngày riêng (tránh áp lực mở app mỗi ngày).

**Giá** (người dùng ~20 🍬/ngày): Thường **60** (~3 ngày, skin) · Hiếm **150** (~1 tuần, hiệu ứng,
bộ mặt Miu, bộ câu) · Sử thi **300** (~2 tuần, bộ nhân vật 5 ảnh). 20 món đầu ≈ 3000 🍬 ≈ 5 tháng.
Test chặn giá vô lý: `price ≤ 30 × max_daily_candy()`.

**Không làm:** bán kẹo, quảng cáo, nhiệm vụ "mở app N lần", trừ kẹo khi bỏ bê, khoá chức năng.

### 14.3 Danh mục món (catalog tĩnh trong `collection.rs`)

Mỗi món: `id`, `cat` (Skin · SideSet · MiuSet · Effect · LinePack), `rarity` (Common · Rare ·
Epic), `price`, tên `[vi, ja, en]`, `asset`. Test: id không trùng; mỗi nhóm Skin/SideSet/MiuSet có
**đúng một** món giá 0; giá khớp hạng; mọi skin có khối `[data-skin="…"]` trong `app/skins.css`;
mọi bộ Miu có đủ 9 file.

**Phase 17 — không cần ảnh mới (7 món mua được + 3 mặc định):**

| id | Nhóm | Hạng · giá | Ghi chú |
|---|---|---|---|
| `skin.lavender` | Skin | mặc định | tím hiện tại |
| `skin.sakura` · `skin.night` · `skin.mint` · `skin.sunset` · `skin.ocean` | Skin | Thường 60 | hồng đào · chàm/tím than · bạc hà · hoàng hôn · biển |
| `fx.fireworks` | Effect | Hiếm 150 | pháo hoa khi xong hết việc trong ngày (`Reward::AllDone`) |
| `fx.confetti` | Effect | Hiếm 150 | pháo giấy khi xong phiên Tập trung |
| `side.bunny` | SideSet | mặc định | Bunny Lounge, 5 ảnh đang có |
| `miu.classic` | MiuSet | mặc định | 9 biểu cảm đang có (`media/game/`) |

Skin áp cho **cả 3 widget** một lúc. Hiệu ứng mở rồi bật/tắt được từng cái.

**Phase 18 — cần ảnh / nội dung:** `side.<tên>` (Sử thi 300, 5 ảnh; gợi ý Neko Café ☕, Fox Shrine
⛩️, Winter Lodge ❄️, Beach Day 🏖️; bộ thiếu ảnh bảng nào thì bảng đó dùng tạm ảnh Bunny) ·
`miu.<tên>` (Hiếm 150, 9 ảnh; gợi ý áo dài, yukata, pyjama) · `lines.praise` (Hiếm 150, +3 biến
thể vi/ja/en cho `TaskDone`/`FocusDone`/`DailyGreeting`, cần cờ cho `lines::line` nên để Phase 18).
Gacha cần ≥ 15 món: tối thiểu 2 bộ nhân vật + 2 bộ mặt + 1 bộ câu, thiếu thì thêm skin.

**Dung lượng:** `media/side` 2,7 MB, `media/game` 4,3 MB hiện tại. Mỗi bộ nhân vật ≈ 2,5–3 MB; bộ
mặt Miu ≈ 1,5 MB (300px) / 4,5 MB (520px). **Trần đóng gói: 2 bộ nhân vật + 2 bộ mặt** (≈ +8 MB);
vượt thì tải khi mở khoá (GitHub Release assets + checksum), chỉ làm khi cần.

### 14.4 Khuôn ảnh (Phase 18, người dùng tự tạo)

**Bộ nhân vật bám mép** — 5 ảnh, `media/side/<set-id>/{settings,tasks,popup,weather,arcade}.png`:
- **1024 × 1536** PNG, nền trong suốt, không bóng đổ xuống sàn; ≤ 650 KB/ảnh.
- Từ hông trở lên hoặc toàn thân; **đỉnh đầu ở 8–15% chiều cao** (y ≈ 120–230); chân/vạt áo chạm đáy.
- **Một tay vươn hẳn sang một bên** như tựa mép bảng; ghi rõ từng ảnh tay vươn trái hay phải →
  `hand_right`. Thân nằm trong **35–65% chiều ngang**; mép bảng cắt ở ~60% (`edge_frac`).
- Anime pastel, viền mềm, cùng tông bộ hiện có; mỗi bảng một nhân vật/trang phục cùng chủ đề.
- Ảnh tạo riêng cho app; repo có thể public nên **không** dùng nhân vật từ game/anime.
- Prompt gợi ý: *anime girl, pastel soft shading, [theme outfit], upper body, leaning to the right
  with one arm stretched out resting on an invisible ledge, looking at viewer, smiling, transparent
  background, no ground shadow, full character visible, 2:3 portrait*.
- Có ảnh rồi: chạy script đo cột pixel đặc để tính `edge_frac` thật, chỉnh lại bằng ảnh chụp.

**Bộ mặt Miu mini** — 9 ảnh, `media/miu/<set-id>/rps-<face>.png`:
- Cao **520 px** (game hiện tới ~360 px; màn 150% cần ~540), rộng 440–620, PNG trong suốt, ≤ 600 KB.
- Chibi từ ngực trở lên, mặt ≈ 45% chiều cao. Cùng trang phục, tóc, phụ kiện cả 9 ảnh.
- Đúng 9 tên: `wink` nháy mắt · `cheer` reo · `think` chống cằm · `draw` bối rối · `pout` phụng
  phịu · `sleepy` ngáp · `surprised` giật mình · `shy` ngại · `fight` nắm tay cố lên.
- Một sheet 3×3 nền trong suốt cũng được, cắt tự động như lần cắt 4 biểu cảm.

### 14.5 Kiến trúc

**Lưu trữ: `collection.json` riêng, KHÔNG nằm trong Settings.** `commands::settings_set` chép lại
các trường "không có trên form" từ bản cũ; thêm trường mà quên là mất đồ đã mở. File riêng theo đúng
khuôn `store::update_bond`: `COLLECTION_LOCK`, hỏng → mặc định, không đổi thì không ghi (`PartialEq`).

```rust
pub struct Collection {          // mọi trường #[serde(default)]
    pub candy: u32, pub earned_total: u32, pub spent_total: u32,
    pub owned: Vec<String>,      // id đã mua; món giá 0 KHÔNG lưu
    pub skin: String,            // default "skin.lavender"
    pub side_set: String,        // default "side.bunny"
    pub miu_set: String,         // default "miu.classic"
    pub effects_on: Vec<String>,
    pub pity: u32, pub fragments: u32,   // Phase 19, vô hại từ bây giờ
}
```

Hàm thuần có test: `candy_mult`, `earn`, `item`, `owns`, `can_afford`,
`buy → Result<&Item, BuyError{NotFound|AlreadyOwned|NotEnough{need}}>`,
`select(cat, id) → Result<(), SelectError{NotFound|WrongCategory|NotOwned}>`, `set_effect`,
`effective_skin / effective_miu_dir / effective_side_set` (id lạ hoặc chưa sở hữu → mặc định;
**không bao giờ** đẩy chuỗi từ file JSON thẳng ra webview, chỉ `&'static str` từ catalog),
`max_daily_candy()`. `Settings.side_girls` vẫn là công tắc bật/tắt; *chọn bộ nào* ở `collection.json`.

**Luồng kẹo & lời pet.** `bond_award` → `gain > 0` → `collection::earn_from` →
`collection::changed` (phát `collection-changed` toàn app). Pet **không nói** khi được kẹo. Nói khi
**mở khoá**: `Key::ItemUnlocked` (biến `{item}`, 3 ngôn ngữ, ≥ 2 biến thể), gửi thẳng `showMessage`
vì do người dùng bấm (như `say_now`). `show_bond_card` và `game_info` thêm `"candy"`; `bond.js` thêm
ô 🍬; ô 🎁 trên trang Mini Games hiện "🍬 N".

**Nhân vật bám mép đổi được lúc chạy (`sidegirl.rs`).** Hiện ảnh nạp một lần qua
`initialization_script("window.__SIDE_IMG__ = …")`. Tách `SPECS` thành `HOSTS: [Host{label, host}; 5]`
(cửa sổ) và `SETS: &[Set{id, girls: &[Girl{host, img, edge_frac, hand_right}]}]` (ảnh; `SETS[0]` =
bunny); `SET_IDX: AtomicUsize` như `ENABLED`; `girl_for(host)` lấy từ bộ đang chọn, thiếu thì rơi về
bunny. `sync` phát `side-layout {flip, edge, img}`; `side.html` đổi `img.src` khi khác (trước khi
thêm `.ready`). Test `every_panel_has_one_girl` viết lại: `SETS[0]` đủ 5 host; mọi girl trỏ host có
thật; `edge_frac` ∈ 0..=1; fallback bunny.

**Skin widget: biến CSS dùng chung, trời vẫn hiện.** File mới `app/skins.css` nạp sau style của 3
trang ghim. Màu cứng (`#ff73ab`, `#ffd27a`, `#c9b6d6`, `rgba(255,255,255,.08)`, gradient nền) đổi
thành biến `--w-bg-a/--w-bg-b/--w-ink/--w-ink-soft/--w-accent/--w-gold/--w-chip/--w-line/--w-sky`
khai trên `.card`; `body[data-skin="…"] .card` ghi đè. Widget thời tiết: `.sky-*` **không đặt
`background`** nữa, chỉ đặt `--w-sky` làm lớp tint xếp lên gradient skin
(`background: linear-gradient(var(--w-sky), var(--w-sky)), linear-gradient(160deg, var(--w-bg-a), var(--w-bg-b))`).
Không làm vậy thì `body[data-skin] .card` (0,2,1) luôn thắng `.card.sky-rain` (0,2,0). Rust:
`calendar_pin::view`, `clock_pin::view`, `weather_net::view` thêm `"skin"` và `"miuDir"`; trang đặt
`document.body.dataset.skin` trong `receive()`.

**Bộ mặt Miu.** Ba chỗ chọn ảnh (`calendar-pin.js`, `clock-pin.js`, `weather-pin.js` `setFace`) thành
`${data.miuDir || 'media/game'}/rps-${face}.png`. Tên mặt giữ nguyên nên test hiện có vẫn xanh. Game
(`<img>` tĩnh) ngoài phạm vi Phase 17.

**Hiệu ứng trên pet.** Pet đã có `createSparkle` (ui.js). Thêm `app/media/webview/fx-pet.js`:
`playFx('confetti' | 'fireworks')`, tự xoá, tôn trọng `prefers-reduced-motion`; CSS trong
`companion.css`; `main.js` thêm `case 'fx'`. Rust `collection::fx(app, kind)`: đã mua **và** đang bật
**và** pet đang hiện → `emit_to("main","pet-message",{command:"fx",kind})`. Kích hoạt:
`rewards::react` khi `Reward::AllDone`; `focus_loop` nhánh `FocusDone`. `collection_preview_fx`
(không kiểm sở hữu) cho nút "Xem thử".

**Cửa sổ "🍬 Bộ sưu tập".** `windows.rs`: `COLLECTION_W/H` 726×475 + `html{zoom:.66}` như arcade;
`precreate("collection")`; `open_collection` chép từ `open_arcade`. `capabilities/default.json` thêm
`"collection"`. Lệnh (`commands.rs`, async khi đụng cửa sổ): `collection_get`, `collection_buy{id}`,
`collection_select{category,id}`, `collection_toggle_effect{id,on}`, `collection_preview_fx{kind}`,
`open_collection`, `collection_close`. `collection::changed`: `app.emit("collection-changed")` + áp
dụng ngay (skin/miu → `calendar_pin/clock_pin/weather_net::broadcast`; side_set →
`sidegirl::set_side_set`). Lối vào: menu tròn 💞 `{🍬 Bộ sưu tập, 'show-shop'}` → `shop:show` →
`pet_command` → `windows::open_collection`; ô 🎁 `#tileReward` trên Mini Games (đang trống); nút
trong Cài đặt → 🎨 Diện mạo. Front-end: `app/collection.html` (bảng màu arcade; tab Skin · Nhân vật ·
Miu · Hiệu ứng; đầu trang 🍬 số dư + hôm nay + cách kiếm), `app/js/collection.js`,
`app/js/collection-core.js` thuần (`T` vi/ja/en, `groupByCategory`, `isOwned`, `canAfford`,
`priceLabel`, `rarityClass`, `sortItems`) + test.

### 14.6 Gacha (Phase 19) — `gacha.rs` thuần, `rand` truyền vào

- 1 lượt **30 🍬** (rẻ hơn đổi thẳng 2–10 lần, đổi lại may rủi). Đổi thẳng vẫn luôn có.
- Tỉ lệ Thường 70 · Hiếm 25 · Sử thi 5; quay trong toàn bộ catalog trừ món giá 0.
- Trùng → **mảnh** 5 / 15 / 30 theo hạng; **100 mảnh = chọn bất kỳ món** (`craft`). Không bao giờ
  nhận trùng vào `owned`.
- Bảo hiểm: **10 lượt** không ra Hiếm+ → lượt kế chắc chắn Hiếm+; `pity` về 0 khi ra Hiếm+.
- Quay 10 lượt trừ `10 × 30` một lần, tất cả hoặc không.
- API: `pull(&mut c, &rules, rand) -> Result<Pull{item, dup, fragments, pity_hit}, BuyError>`,
  `pull_many`, `craft`. Test với `rand` cố định như `games-core.js`: ngưỡng tích luỹ đơn điệu;
  `rand=0.0` luôn ra món đầu nhóm thấp nhất; bảo hiểm nổ đúng lượt 10 rồi reset; trùng → mảnh;
  `NotEnough` không đổi `candy/pity`.
- Hoạt cảnh: máy capsule, Miu cạnh máy `think → surprised → cheer`, trùng → `pout`. Tỉ lệ hiện
  công khai ngay trên máy. Chỉ bật khi catalog ≥ 15 (hằng số có test).

### 14.7 Các phase & thứ tự Phase 17

| Phase | Nội dung | Bản (đề xuất) | Cỡ |
|---|---|---|---|
| **17** | Kẹo, `collection.json`, catalog 7 món + 3 mặc định, cửa sổ Bộ sưu tập (đổi thẳng), hạ tầng chọn bộ nhân vật / bộ mặt, skin 3 widget, 2 hiệu ứng, kẹo trên thẻ 💗, câu "mở khoá" | v0.10.0 | ≈ bản lịch ghim |
| **18** | 2 bộ nhân vật + 2 bộ mặt Miu theo khuôn 14.4, đo `edge_frac`; bộ câu hiếm; (tuỳ) tải bộ khi mở khoá | v0.10.x | phần lớn là làm ảnh |
| **19** | Máy kẹo "Candy Capsule": quay 1 / 10 lượt, mảnh, bảo hiểm, hoạt cảnh | v0.11.0 | nhỏ–vừa |

**Điều kiện hoàn thành** (khi nào một phase được coi là xong):

- **Phase 17:** người dùng kiếm được kẹo, tiêu được, và món đã mua **được áp dụng ổn định** (skin
  đổi màu widget, hiệu ứng chạy đúng lúc); kẹo + đồ đã mua **giữ nguyên qua lần khởi động lại** và
  qua lần Lưu Cài đặt; trần kẹo mỗi ngày đúng bảng 14.2. Chưa cần gacha. Về bộ nhân vật / bộ mặt
  Miu: chỉ có bộ mặc định để chọn, nên điều kiện là **hạ tầng chọn chạy được** (đổi bộ không cần
  khởi động lại), nội dung để Phase 18.
- **Phase 18:** catalog đủ ≥ 15 món; **mọi món mua được và trang bị đúng** (mỗi bộ nhân vật đủ 5
  bảng hoặc rơi về Bunny đúng chỗ, mỗi bộ mặt đủ 9 file, test catalog xanh); dung lượng bộ cài trong
  trần 14.3 hoặc đã có tải khi mở khoá.
- **Phase 19:** tỉ lệ đúng và hiện công khai; **không trừ kẹo sai** (quay 10 lượt là tất cả hoặc
  không); **không nhận trùng** vào `owned`, trùng luôn thành mảnh; bảo hiểm nổ đúng lượt; trạng
  thái `pity`/`fragments` **khôi phục đúng sau khi khởi động lại** và sau lỗi ghi file.

Thứ tự Phase 17 (mỗi bước `cargo test` + `npm test` xanh):

1. `collection.rs` (kiểu, catalog, hàm thuần, test) + `mod collection;` +
   `store::load_collection/update_collection`; `app/skins.css` khung với `[data-skin]` cho mọi skin.
2. Móc kẹo vào `companion::bond_award`; +20 ở `milestone_line`; `"candy"` trong
   `show_bond_card`/`game_info`; `bond.js` ô 🍬; `arcade.js` ô 🎁 hiện kẹo.
3. `lines.rs` `Key::ItemUnlocked` (enum, `ALL`, `vars`, 3 ngôn ngữ).
4. Widget: `skin` + `miuDir` trong 3 `view()`; màu cứng → biến CSS; `sky-*` → `--w-sky`; trang đặt
   `data-skin` và ghép đường dẫn Miu (mặc định → không đổi hình thức).
5. `sidegirl.rs` `HOSTS/SETS/SET_IDX`, `side-layout.img`, `side.html` đổi ảnh, viết lại test.
6. Hiệu ứng: `fx-pet.js`, `companion.css`, `main.js` `case 'fx'`; Rust `collection::fx` + kích hoạt.
7. Cửa sổ + lệnh: `windows.rs`; `capabilities`; 7 lệnh + `invoke_handler`; `pet_command`
   `shop:show`; `collection.html/.js/-core.js(+test)`.
8. Lối vào: menu tròn, ô 🎁 arcade + `listen('collection-changed')`, nút trong Cài đặt.
9. Tài liệu: CHANGELOG, bump version, cập nhật bảng bản phát hành ở mục 12.

**Nơi phải đăng ký thứ mới:** `mod collection;` · `invoke_handler` (+7) · `pet_command "shop:show"` ·
`capabilities/default.json` windows · `windows::precreate` + `open_collection` · `main.js case 'fx'` ·
menu tròn. **`settings_set` không đụng** (thêm chú thích: trạng thái mở khoá cố ý không nằm trong
Settings).

**Bẫy:** `Present` mỗi 30 giây → chặn `gain > 0` trước khi đụng file; mọi chuỗi ra webview qua
`effective_*`; lệnh show/hide cửa sổ phải `async` (WebView2 treo); `app.emit` trên `AppHandle` khi
phát toàn app; `precreate` đọc collection.json **trước** `sidegirl::precreate` để `__SIDE_IMG__` khớp
`SET_IDX`; skin thời tiết phải đổi `sky-*` thành tint; chữ trợ giúp `rewardHelp` trên arcade (giờ có
kẹo) đủ 3 ngôn ngữ.

### 14.8 Rủi ro

| Rủi ro | Giảm bằng |
|---|---|
| Hết đồ sớm, gacha nhàm | Gacha chỉ bật khi catalog ≥ 15 món (test) |
| Bộ cài phình | Trần 2+2 bộ đóng gói; vượt thì tải khi mở khoá |
| Mất đồ đã mở khi lưu Cài đặt | File riêng có khoá, không nằm trong Settings |
| Cảm giác bị "dụ" | Không tiền thật, đổi thẳng luôn có, giá và tỉ lệ công khai |
| Ảnh lệch mép | Đo tự động + chỉnh bằng ảnh chụp như Bunny Lounge |
| Câu thoại thiếu ngôn ngữ | `every_key_has_all_languages` bắt |
| Bộ Miu thiếu ảnh | Test catalog kiểm đủ 9 file theo `CARGO_MANIFEST_DIR` |

### 14.9 Kiểm thử

- `cargo test`: catalog hợp lệ; `earn` theo hệ số và `gain=0`; `buy` → `owned` → `AlreadyOwned`;
  `NotEnough` không đổi gì; `select` từ chối sai nhóm / chưa có; `effective_*` rơi về mặc định với
  dữ liệu rác; sidegirl fallback; (19) phân phối + bảo hiểm với rand cố định.
- `npm test`: `collection-core.js`; test cú pháp bắt trang mới; test "mọi biểu cảm có file" mở rộng
  theo `miuDir`.
- Bằng tay (`npm run dev`): tick xong một việc → thẻ 💗 tăng 🍬; đổi skin → 3 widget đổi màu ngay,
  widget thời tiết lúc mưa vẫn ám xanh; chọn bộ nhân vật → mở Cài đặt thấy nhân vật mới không cần
  khởi động lại; bật pháo hoa → tick việc cuối ngày thấy hiệu ứng; Alt+F4 cửa sổ Bộ sưu tập chỉ ẩn;
  Lưu Cài đặt không làm mất đồ đã mở.

### 14.10 Câu hỏi cần chốt trước khi code

1. Bắt đầu Phase 17 khi nào, và có ra bản ổn định (dùng thật, sửa lỗi) trước không?
2. Chủ đề cho 2 bộ nhân vật đầu tiên của Phase 18 (để làm ảnh theo khuôn 14.4).
3. Có làm "tải bộ khi mở khoá" ở Phase 18 không, hay chấp nhận bộ cài nặng thêm ≈ 8 MB?
