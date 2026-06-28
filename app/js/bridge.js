// Bridge shim: replaces the VS Code webview `acquireVsCodeApi()` /
// WebSocket transport the original frontend expected. The Live2D runtime
// (media/webview/*.js) reads `window.__VS_CODE_BRIDGE__` and posts commands
// through it, and listens for incoming `window` 'message' events. Here we route
// outgoing commands to the Rust `pet_command` handler via Tauri invoke, and
// re-dispatch backend `pet-message` events as `window` 'message' events so the
// existing runtime keeps working unchanged.
(function () {
  const TAURI = window.__TAURI__ || {};
  const invoke =
    (TAURI.core && TAURI.core.invoke) ||
    TAURI.invoke ||
    function () {
      return Promise.resolve();
    };

  window.__VS_CODE_BRIDGE__ = {
    postMessage(message) {
      try {
        // setModel switches the displayed Live2D model. Persist via Rust and
        // reload the pet page with the new model id baked into __INIT_MODEL__.
        if (message && message.command === 'setModel' && message.modelId) {
          const map = window.__MODELS_MAP__ || {};
          if (map[message.modelId]) {
            invoke('pet_command', { payload: message });
            window.__INIT_MODEL__ = message.modelId;
            sessionStorage.setItem('petModel', message.modelId);
            location.reload();
            return;
          }
        }
        // setVoiceLanguage: switch the audio folder (ja/vi/en) client-side and
        // persist it so SFX play in the chosen language.
        if (message && message.command === 'setVoiceLanguage' && message.voiceLanguage) {
          const lang = message.voiceLanguage;
          window.__VOICE_LANGUAGE__ = lang;
          window.__AUDIO_BASE_URL__ = '/media/audio/' + lang;
          try { localStorage.setItem('voiceLanguage', lang); } catch (_) {}
          invoke('pet_command', { payload: message });
          return;
        }
        invoke('pet_command', { payload: message });
      } catch (err) {
        console.error('[bridge] postMessage failed', err);
      }
    },
  };

  // Note: pet-config.js (loaded before this) consumes sessionStorage.petModel
  // and resolves __INIT_MODEL__, so the reload shows the newly chosen model.

  if (TAURI.event && TAURI.event.listen) {
    TAURI.event.listen('pet-message', (event) => {
      window.dispatchEvent(new MessageEvent('message', { data: event.payload }));
    });

    // Global cursor position (screen coords) for look-at-cursor. pet-lookat.js
    // converts these to client coords against the pet window.
    TAURI.event.listen('cursor-pos', (event) => {
      if (window.__onCursorPos__) window.__onCursorPos__(event.payload);
    });

    // A newer version is available → nudge the user with a bubble.
    TAURI.event.listen('update-available', (event) => {
      try {
        window.dispatchEvent(new MessageEvent('message', {
          data: { command: 'showMessage', text: '✨ Có bản mới v' + event.payload + '! Mở Cài đặt → Kiểm tra cập nhật nha~' },
        }));
      } catch (_) { /* ignore */ }
    });

    // Custom model list changed in Settings → refresh the cached list and reload
    // so the Appearance menu shows the new models.
    TAURI.event.listen('models-changed', async () => {
      try {
        const s = await invoke('settings_get');
        sessionStorage.setItem('customModels', JSON.stringify(s.custom_models || []));
      } catch (_) { /* ignore */ }
      location.reload();
    });
  }
})();
