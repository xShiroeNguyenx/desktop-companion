// Chữ giao diện theo ngôn ngữ chữ của pet (Settings.message_language: vi / ja / en).
// Mỗi khu vực có một bảng ở js/strings/<khu>.js gọi I18n.add({ vi, ja, en }).
//   HTML: data-i18n="key" (chữ), data-i18n-title / -placeholder / -aria-label (thuộc tính).
//         Giữ sẵn chữ tiếng Việt trong HTML: thiếu khoá thì vẫn hiện tiếng Việt.
//   JS:   I18n.t('key', { n: 3 }) — biến {n} như các bảng khác trong app.
// Đổi ngôn ngữ (Cài đặt hoặc menu 💬) → Rust phát "ui-lang" → mọi cửa sổ dịch lại
// và gọi các hàm đăng ký bằng I18n.onChange (để vẽ lại chữ dựng bằng JS).
// Nạp SAU tauri-api.js (nếu có) và TRƯỚC các file js/strings/*.js.
(function (root) {
  const LANGS = ['vi', 'ja', 'en'];
  const dict = { vi: {}, ja: {}, en: {} };
  const subs = [];
  let lang = 'vi';

  function add(table) {
    for (const l of LANGS) Object.assign(dict[l], (table && table[l]) || {});
  }

  function fill(s, vars) {
    return String(s).replace(/\{(\w+)\}/g, (m, k) => (vars && k in vars ? String(vars[k]) : m));
  }

  /** Chữ của `key` theo ngôn ngữ hiện tại; thiếu thì tiếng Việt, thiếu nữa thì chính key. */
  function t(key, vars) {
    const s = dict[lang][key] ?? dict.vi[key] ?? key;
    return fill(s, vars);
  }

  const ATTRS = [
    ['data-i18n-title', 'title'],
    ['data-i18n-placeholder', 'placeholder'],
    ['data-i18n-aria-label', 'aria-label'],
  ];

  function apply(scope) {
    const base = scope || (typeof document !== 'undefined' ? document : null);
    if (!base || !base.querySelectorAll) return;
    for (const el of base.querySelectorAll('[data-i18n]')) el.textContent = t(el.getAttribute('data-i18n'));
    for (const [data, attr] of ATTRS) {
      for (const el of base.querySelectorAll('[' + data + ']')) el.setAttribute(attr, t(el.getAttribute(data)));
    }
  }

  function setLang(next) {
    lang = LANGS.includes(next) ? next : 'vi';
    if (typeof document !== 'undefined') {
      document.documentElement.lang = lang;
      apply();
    }
    for (const fn of subs) {
      try {
        fn(lang);
      } catch (e) {
        console.error('[i18n]', e);
      }
    }
  }

  function onChange(fn) {
    subs.push(fn);
  }

  // Cửa sổ phụ có window.DC (tauri-api.js); cửa sổ pet thì gọi thẳng __TAURI__.
  function tauri() {
    if (root.DC) return { invoke: root.DC.invoke, listen: root.DC.listen };
    const T = root.__TAURI__ || {};
    const invoke = (T.core && T.core.invoke) || T.invoke;
    const listen = T.event && T.event.listen;
    return invoke && listen ? { invoke, listen } : null;
  }

  function init() {
    const api = tauri();
    if (!api) return;
    api.invoke('ui_lang').then(setLang).catch(() => setLang('vi'));
    api.listen('ui-lang', (e) => setLang(e && e.payload));
  }

  const api = { LANGS, dict, add, fill, t, apply, setLang, onChange, lang: () => lang };
  if (typeof module !== 'undefined' && module.exports) {
    module.exports = api;
  } else {
    root.I18n = api;
    if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', init);
    else init();
  }
})(typeof window !== 'undefined' ? window : globalThis);
