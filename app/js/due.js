// Mô tả hạn chót cho bảng Công việc và tờ note ghim: "còn 20 phút",
// "hôm nay 17:00", "quá hạn 1 ngày"… kèm mức gấp để tô màu.
// Hàm thuần, dùng được cả trong trình duyệt (window.DueFmt) lẫn node --test.
(function (root) {
  const MIN = 60_000;
  const HOUR = 60 * MIN;
  const DAY = 24 * HOUR;
  const p2 = (x) => String(x).padStart(2, '0');

  function sameDay(a, b) {
    return a.getFullYear() === b.getFullYear() && a.getMonth() === b.getMonth() && a.getDate() === b.getDate();
  }

  /**
   * @param {number|string} due  millis UTC (chuỗi rỗng / sai = không có hạn)
   * @param {number} now         millis UTC
   * @returns {{text: string, level: 'over'|'soon'|'today'|'later'} | null}
   */
  function describeDue(due, now) {
    const ms = Number(due);
    if (!due || !Number.isFinite(ms) || ms <= 0) return null;
    const d = new Date(ms);
    const n = new Date(now);
    const hhmm = `${p2(d.getHours())}:${p2(d.getMinutes())}`;
    const diff = ms - now;
    if (diff < 0) {
      const days = Math.floor(-diff / DAY);
      return { text: days >= 1 ? `⚠️ Quá hạn ${days} ngày` : `⚠️ Quá hạn (${hhmm})`, level: 'over' };
    }
    if (diff < HOUR) {
      return { text: `⏰ Còn ${Math.max(1, Math.ceil(diff / MIN))} phút`, level: 'soon' };
    }
    if (sameDay(d, n)) return { text: `⏰ Hôm nay ${hhmm}`, level: 'today' };
    const tomorrow = new Date(n.getFullYear(), n.getMonth(), n.getDate() + 1);
    if (sameDay(d, tomorrow)) return { text: `📅 Mai ${hhmm}`, level: 'later' };
    return { text: `📅 ${p2(d.getDate())}/${p2(d.getMonth() + 1)} ${hhmm}`, level: 'later' };
  }

  /** Giá trị cho <input type="datetime-local"> (giờ địa phương). */
  function toLocalInput(due) {
    const ms = Number(due);
    if (!due || !Number.isFinite(ms) || ms <= 0) return '';
    const d = new Date(ms);
    return `${d.getFullYear()}-${p2(d.getMonth() + 1)}-${p2(d.getDate())}T${p2(d.getHours())}:${p2(d.getMinutes())}`;
  }

  /** Ngược lại: chuỗi datetime-local → millis UTC, hoặc null nếu sai. */
  function fromLocalInput(value) {
    if (!/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}/.test(value || '')) return null;
    const ms = new Date(value).getTime();
    return Number.isFinite(ms) ? ms : null;
  }

  /** Sắp việc chưa xong: hạn gần nhất trước, việc không hạn giữ thứ tự cũ ở cuối. */
  function byDue(a, b) {
    const da = Number(a.due_at) || Infinity;
    const db = Number(b.due_at) || Infinity;
    return da === db ? 0 : da < db ? -1 : 1;
  }

  const api = { describeDue, toLocalInput, fromLocalInput, byDue };
  if (typeof module !== 'undefined' && module.exports) module.exports = api;
  else root.DueFmt = api;
})(typeof window !== 'undefined' ? window : globalThis);
