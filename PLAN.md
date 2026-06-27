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
