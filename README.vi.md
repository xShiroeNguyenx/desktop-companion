# Desktop Companion

🌸 Thú cưng Live2D trên màn hình Windows (chạy độc lập), tích hợp sẵn **dịch AI**,
**trả lời nhanh** và **lưu công việc** — hoạt động trên mọi ứng dụng (Teams, trình
duyệt, Word…).

🌐 **[Trang giới thiệu (Website) →](https://xshiroenguyenx.github.io/desktop-companion/)**

**Ngôn ngữ:** [English](README.md) · **Tiếng Việt** · [日本語](README.ja.md)

![Live2D pet](src-tauri/icons/128x128.png)

---

## Tính năng

- **Pet Live2D trên desktop** — nhân vật trong suốt, luôn nổi trên cùng, kéo-thả
  được, khởi động cùng Windows. Chuột phải để mở menu.
- **Dịch** — bôi đen văn bản ở **bất kỳ** ứng dụng nào → bông hoa 🌸 hiện cạnh con
  trỏ → **click** để dịch ngay, hoặc **nhấn giữ** để mở bảng đầy đủ (dịch / trả
  lời / lưu task). Phía trên có icon 📋 — bấm hoặc nhấn **Ctrl+C** (khi bông hoa
  đang hiện) để sao chép đoạn vừa chọn.
- **Trả lời nhanh** — bôi đen tin nhắn, gõ ý trả lời bằng tiếng Việt, AI soạn câu
  trả lời tự nhiên theo ngôn ngữ của tin nhắn (Anh / Nhật / …).
- **Lưu công việc** — biến đoạn văn bản bất kỳ thành TODO; quản lý trong cửa sổ Tasks.
- **Ngữ cảnh dịch** — danh sách ngữ cảnh có sẵn trong Cài đặt (vd: "chat công việc
  trang trọng") để dịch nhanh đúng giọng điệu. Chọn một cái mặc định hoặc không.
- **Nhìn theo con trỏ** — đầu/mắt nhân vật dõi theo chuột khi chuột ở gần pet.
- **Phóng to / thu nhỏ** — `Ctrl` + lăn chuột để đổi kích thước; tự nhớ kích thước.
- **Model Live2D tự thêm** — nạp model `*.model3.json` của bạn từ thư mục bất kỳ.
- **Âm thanh** — hiệu ứng tương tác (chọc / vuốt đầu / …) bằng tiếng Nhật / Việt / Anh.
- **Nhà cung cấp AI** — Anthropic Claude hoặc Google Gemini (tự nhập API key của bạn).
  Có thể tải danh sách model trực tiếp từ nhà cung cấp.
- **Phím tắt tùy chỉnh** — đổi phím tắt bắt văn bản (mặc định `Ctrl+Shift+Space`).

---

## Cài đặt

Tải bộ cài mới nhất ở trang [Releases](../../releases):

- **`Desktop Companion_x.y.z_x64-setup.exe`** (NSIS) hoặc **`…_x64_en-US.msi`** (MSI)

Chạy file đó, app sẽ tự khởi động và nằm ở khay hệ thống (system tray).

> Hiện chỉ hỗ trợ Windows. Cần WebView2 runtime (đã có sẵn trên Windows 10/11).

---

## Bắt đầu

1. **Nhập API key** — icon tray → **Cài đặt** → chọn nhà cung cấp (Anthropic hoặc
   Gemini) → dán key → **Lưu**.
2. **Dịch** — bôi đen văn bản ở đâu cũng được → click bông hoa 🌸.
3. **Trả lời** — bôi đen tin nhắn → nhấn giữ bông hoa → tab **Trả lời** → gõ tiếng
   Việt → **Tạo câu trả lời**.
4. **Công việc** — tray → **Tasks**, hoặc lưu đoạn bôi đen qua menu nhấn-giữ bông hoa.

### Menu tray

Hiện/Ẩn pet · Công việc · Cài đặt · Bật/Tắt click-through · Thoát

### Chuột phải vào pet

Diện mạo (đổi model, nhìn theo con trỏ, động tác, chọc) · Âm thanh (giọng nói,
nhạc nền, tắt tiếng) · Chat AI · Công việc · Cài đặt

---

## Build từ mã nguồn

### Yêu cầu

- [Rust](https://rustup.rs) (stable) + MSVC C++ build tools
- [Node.js](https://nodejs.org) 18+
- Yêu cầu Tauri — xem <https://v2.tauri.app/start/prerequisites/>

### Lệnh

```bash
npm install
npm run dev      # chạy chế độ phát triển
npm run build    # tạo bộ cài trong src-tauri/target/release/bundle/
```

---

## Công nghệ

- **[Tauri 2](https://tauri.app)** (lõi Rust + WebView2)
- **Live2D Cubism 4** render bằng **[PIXI.js](https://pixijs.com)**
- Global mouse hook + giả lập copy để bắt văn bản xuyên ứng dụng (Windows)
- API HTTP của Anthropic / Gemini cho dịch & trả lời

---

## Ghi công

- Model mẫu Live2D © Live2D Inc. (dùng theo Free Material License).
- Xây trên runtime render của dự án Anime Companion.

## Giấy phép

MIT — xem [LICENSE](LICENSE).
