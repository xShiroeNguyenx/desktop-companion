// Initial runtime globals the Live2D frontend expects. In the original
// extension these were injected by VS Code; here they are plain defaults that
// later phases will override from the persisted settings store.
(function () {
  // id → model3.json path (relative to the bundled /media tree). The list comes
  // from Rust (models.rs BUILTIN), the one place bundled models are declared.
  const builtin = Array.isArray(window.__BUILTIN_MODELS__) && window.__BUILTIN_MODELS__.length
    ? window.__BUILTIN_MODELS__
    : [{ id: 'hiyori', url: '/media/live2d/Hiyori/Hiyori.model3.json', name: 'Hiyori', description: '' }];
  const MODELS = {};
  for (const m of builtin) MODELS[m.id] = m.url;
  // Merge user's custom models (served via the petmodel:// scheme). A fresh
  // list from a recent "models-changed" reload (sessionStorage) wins over the
  // list injected at window-creation time.
  let custom = Array.isArray(window.__CUSTOM_MODELS__) ? window.__CUSTOM_MODELS__ : [];
  try {
    const cached = sessionStorage.getItem('customModels');
    if (cached) custom = JSON.parse(cached);
  } catch (_) { /* ignore */ }
  for (const m of custom) {
    // On Windows, Tauri maps a custom scheme to http://<scheme>.localhost/...
    // which `fetch()` can reach (the petmodel:// form cannot be fetched).
    MODELS[m.id] = 'http://petmodel.localhost/' + m.id + '/' + encodeURIComponent(m.entry);
  }
  window.__MODELS_MAP__ = MODELS;

  // Model id precedence: a pending choice from a setModel reload (sessionStorage)
  // wins over the startup-injected __INIT_MODEL__, since the injected script is
  // baked at window-build time and still holds the previous model after reload.
  let chosen = window.__INIT_MODEL__;
  try {
    const pending = sessionStorage.getItem('petModel');
    if (pending) { chosen = pending; sessionStorage.removeItem('petModel'); }
  } catch (_) { /* ignore */ }

  const initId = (chosen && MODELS[chosen]) ? chosen : 'hiyori';
  window.__INIT_MODEL__ = initId;

  window.__DESKTOP_PET_MODE__ = true;
  window.__MODEL_ID__ = initId;
  window.__MODEL_URL__ = MODELS[initId];
  let voiceLang = 'ja';
  try {
    const saved = localStorage.getItem('voiceLanguage');
    if (saved === 'ja' || saved === 'vi' || saved === 'en') voiceLang = saved;
  } catch (_) { /* ignore */ }
  window.__VOICE_LANGUAGE__ = voiceLang;
  window.__AUDIO_BASE_URL__ = '/media/audio/' + voiceLang;
  // Ngôn ngữ chữ: lựa chọn vừa đổi trong phiên này (sessionStorage, vì
  // initialization_script tiêm lúc dựng cửa sổ và không cập nhật khi nạp lại
  // trang) thắng giá trị đã lưu mà Rust tiêm vào.
  let messageLang = window.__INIT_MESSAGE_LANGUAGE__ || 'vi';
  try {
    const recent = sessionStorage.getItem('messageLanguage');
    if (recent) messageLang = recent;
  } catch (_) { /* ignore */ }
  window.__MESSAGE_LANGUAGE__ = ['vi', 'ja', 'en'].includes(messageLang) ? messageLang : 'vi';
  // Tắt tiếng: như ngôn ngữ chữ, lựa chọn trong phiên này (sessionStorage, trang
  // nạp lại khi đổi model) thắng giá trị đã lưu mà Rust tiêm vào lúc dựng cửa sổ.
  let muted = window.__INIT_MUTED__ === true;
  try {
    const recent = sessionStorage.getItem('audioMuted');
    if (recent === '1' || recent === '0') muted = recent === '1';
  } catch (_) { /* ignore */ }
  window.__AUDIO_MUTED__ = muted;
  window.__FOCUS_FOLLOW__ = true;
  window.__AMBIENT_PRESET__ = 'off';
  window.__AMBIENT_VOLUME__ = 0.5;
  window.__AMBIENT_TRACKS__ = [];
  window.__CLICK_THROUGH__ = false;
  window.__WEBVIEW_STRINGS__ = {
    menu: {
      voice: 'Giọng nói',
      messages: 'Ngôn ngữ chữ',
      ambient: 'Nhạc nền',
      mute: 'Tắt tiếng',
      model: 'Đổi model',
      focusFollow: 'Nhìn theo con trỏ',
      motion: 'Động tác',
      poke: 'Chọc nhẹ',
      chatQuick: 'Chat nhanh',
    },
  };
  window.__VISIBLE_MODELS__ = [
    ...builtin.map((m) => ({ id: m.id, name: m.name, description: m.description })),
    ...custom.map((m) => ({ id: m.id, name: m.name, description: 'Model tự thêm' })),
  ];
  // Bundled models still locked in the Collection (bridge.js fills this from Rust).
  window.__LOCKED_MODELS__ = [];
})();
