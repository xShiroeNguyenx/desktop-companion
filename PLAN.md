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
| **v0.5.0** ✅ (chuẩn bị phát hành 2026-10-15) | Ghim task (12.4c) + Phase 9 + 9B + Phase 10, cộng các việc phát sinh: link bấm được trong task, Bunny Lounge (nhân vật bám mép bảng Cài đặt / Công việc / Trợ lý văn bản, mặc định tắt), giao diện Trợ lý văn bản theo phong cách Cài đặt, sửa lỗi thu nhỏ bảng để pet nằm trơ | Xem CHANGELOG [0.5.0] |
| **v0.6.0** ✅ (chuẩn bị phát hành 2026-10-15) | Phase 11 + 12 + 13 gộp một bản, cộng: "🗂️ Mở đầy đủ" trên menu chuột phải (trợ lý văn bản ở chế độ tự gõ), biểu tượng game trên đầu pet, nhóm menu 💞 Đồng hành | Xem CHANGELOG [0.6.0] |

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
