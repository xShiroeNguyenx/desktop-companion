// Initial runtime globals the Live2D frontend expects. In the original
// extension these were injected by VS Code; here they are plain defaults that
// later phases will override from the persisted settings store.
(function () {
  // id → model3.json path (relative to the bundled /media tree).
  const MODELS = {
    hiyori: '/media/live2d/Hiyori/Hiyori.model3.json',
    haru: '/media/live2d/Haru/haru_greeter_t05.model3.json',
    mao: '/media/live2d/Mao/mao_pro.model3.json',
    miara: '/media/live2d/Miara/miara_pro_t03.model3.json',
  };
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
  window.__MESSAGE_LANGUAGE__ = 'vi';
  window.__AUDIO_MUTED__ = false;
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
    { id: 'hiyori', name: 'Hiyori', description: 'Cô gái học sinh dễ thương' },
    { id: 'haru', name: 'Haru', description: 'Cô gái chào đón' },
    { id: 'mao', name: 'Mao', description: 'Cô gái tai mèo' },
    { id: 'miara', name: 'Miara', description: 'Phép thuật thiếu nữ' },
    ...custom.map((m) => ({ id: m.id, name: m.name, description: 'Model tự thêm' })),
  ];
})();
