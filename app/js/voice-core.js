// 🔊 Giọng của pet: chọn giọng lúc mở app và quyết định có mời người dùng đổi
// sang giọng tiếng Việt hay không. Hàm thuần, dùng trong trang pet
// (window.VoiceCore) và node --test.
//
// Giọng = tên thư mục trong media/audio/: 'ja', 'en', 'vi' (tiếng Việt, bản
// nhân bản OmniVoice), 'vi-el' (tiếng Việt, giọng gốc ElevenLabs).
(function (root) {
  const VOICES = ['ja', 'vi', 'vi-el', 'en'];

  /** Giọng hợp với ngôn ngữ chữ. Tiếng Việt lấy giọng gốc (đủ mọi câu). */
  function forLanguage(textLang) {
    if (textLang === 'vi') return 'vi-el';
    if (textLang === 'en') return 'en';
    return 'ja';
  }

  /**
   * Giọng dùng khi trang pet mở.
   * - Đã từng chọn (saved hợp lệ): giữ nguyên.
   * - Cài mới (fresh): theo ngôn ngữ chữ, và `persist` = true để ghi lại ngay —
   *   lần mở sau không còn là "cài mới" nữa.
   * - Người dùng cũ chưa từng chọn: giữ mặc định cũ 'ja' (không tự đổi giọng của
   *   ai; họ được mời một lần, xem offerDue).
   */
  function initial({ saved, fresh, textLang }) {
    if (VOICES.includes(saved)) return { voice: saved, persist: false };
    if (fresh) return { voice: forLanguage(textLang), persist: true };
    return { voice: 'ja', persist: false };
  }

  /** Đang dùng một giọng tiếng Việt? */
  function isVietnamese(voice) {
    return voice === 'vi' || voice === 'vi-el';
  }

  /**
   * Có mời "đổi sang giọng tiếng Việt" không: chữ đang là tiếng Việt mà giọng
   * thì không, chưa mời lần nào, và không đang tắt tiếng (mời nói lúc đang tắt
   * tiếng là vô nghĩa; để lần mở app sau).
   */
  function offerDue({ textLang, voice, offered, muted }) {
    return textLang === 'vi' && !isVietnamese(voice) && !offered && !muted;
  }

  const api = { VOICES, forLanguage, initial, isVietnamese, offerDue };
  if (typeof module !== 'undefined' && module.exports) module.exports = api;
  else root.VoiceCore = api;
})(typeof window !== 'undefined' ? window : globalThis);
