// Ring cho đoạn văn vừa bôi đen. Dùng đúng createRadialMenu của menu chuột
// phải, chỉ khác bộ entry và dispatcher: mỗi entry gọi thẳng một lệnh Rust có
// sẵn, không thêm logic LLM/clipboard nào ở đây.
import { createRadialMenu } from '/media/webview/radial-menu.js';

const { invoke, listen, log } = window.DC;

// Cửa sổ ring không có focus (WS_EX_NOACTIVATE) nên phím Escape của
// radial-menu.js không tới nơi — đóng bằng click ra ngoài, rời chuột, và một
// hẹn giờ nhàn rỗi, để ring không bao giờ kẹt lại trên màn hình.
const IDLE_MS = 6000;
let idleTimer = null;

function scheduleClose(ms) {
  clearTimeout(idleTimer);
  idleTimer = setTimeout(() => close(), ms);
}

function close() {
  clearTimeout(idleTimer);
  invoke('close_flower_menu').catch((e) => log('close_flower_menu: ' + e));
}

const menu = createRadialMenu({
  getItems: () => [
    { icon: '🌐', label: 'Dịch nhanh', action: 'translate' },
    { icon: '📋', label: 'Sao chép', action: 'copy' },
    { icon: '💬', label: 'Trả lời', action: 'reply' },
    { icon: '💭', label: 'Hỏi em', action: 'ask' },
    { icon: '📝', label: 'Lưu vào task', action: 'task' },
    { icon: '🗂️', label: 'Mở đầy đủ', action: 'full' },
  ],
  onAction: (action) => {
    clearTimeout(idleTimer);
    if (action === 'copy') {
      // Sao chép xong thì tự đóng ring; lỗi cũng đóng, chỉ ghi log.
      invoke('copy_selection')
        .catch((e) => log('copy_selection: ' + e))
        .finally(() => close());
      return;
    }
    // translate / reply / ask / task / full đều do Rust mở cửa sổ tương ứng rồi ẩn ring.
    invoke('flower_menu_action', { action }).catch((e) => log('flower_menu_action: ' + e));
  },
  labels: { title: 'Văn bản', back: 'Quay lại', close: 'Đóng' },
  // Ring này chỉ có 6 mục và luôn bung ngay cạnh con trỏ nên để gọn hơn ring
  // của menu chuột phải (92px); 62px vẫn đủ chỗ cho pill "Văn bản" ở giữa.
  radius: 62,
});

// Mỗi lần Rust hiện lại cửa sổ thì mở ring vào giữa khung.
function openCentered() {
  menu.close();
  menu.openAt(window.innerWidth / 2, window.innerHeight / 2);
  scheduleClose(IDLE_MS);
  const items = document.querySelectorAll('.companion-radial-item').length;
  log(`flower-menu ring: ${items} mục, khung ${window.innerWidth}x${window.innerHeight}`);
}

listen('flower-menu-shown', openCentered);

// force_repaint bên Rust nhích cửa sổ 1px để ép WebView2 vẽ lại; ring neo theo
// window.innerWidth/2 nên phải đặt lại tâm sau mỗi lần khung đổi kích thước.
window.addEventListener('resize', () => { if (menu.isOpen()) openCentered(); });

// Click vào khoảng trống trong cửa sổ (ngoài ring) → đóng.
window.addEventListener('click', (e) => {
  if (menu.contains(e.target)) { scheduleClose(IDLE_MS); return; }
  close();
}, true);

// Chuột còn rê trên ring thì gia hạn. Nền có pointer-events:none nên chỉ các
// nút mới sinh sự kiện — vừa đúng ý: rê ngoài ring không giữ cửa sổ sống mãi.
document.addEventListener('mousemove', () => scheduleClose(IDLE_MS), true);

// KHÔNG mở ring ở đây: script này chạy lúc precreate, khi cửa sổ còn ẩn. Mở
// sẵn sẽ hẹn giờ đóng ngay từ lúc khởi động và requestAnimationFrame của
// radial-menu.js không chạy trong cửa sổ ẩn. Ring chỉ mở khi Rust báo đã hiện.
log('flower-menu.js ready');
