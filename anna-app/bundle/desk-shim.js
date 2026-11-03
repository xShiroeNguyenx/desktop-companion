// Runs the desktop app's game pages (desk/game.html, desk/guess.html, copied
// by scripts/sync-assets.mjs) as their own page of the Anna app: the panel
// (games.js → app.js playDesk) navigates here, and the page's ← button goes
// back to index.html on the Games tab. Anna blocks frames, hence a page switch.
// Loaded in place of js/tauri-api.js, standing in for the Tauri commands:
//   game_info   → language from the URL
//   game_result → wins counted and handed back in the return URL (candy)
//   close / hide of the window (← and "Quit") → back to the panel
(function () {
  const q = new URLSearchParams(location.search);
  // `glang`: the game language. `lang`, `wid`, `t`… are Anna's and go back untouched.
  const info = { lang: q.get('glang') || 'en', pet: '', you: '', me: '' };
  let wins = 0;

  function invoke(cmd, args) {
    if (cmd === 'game_info') return Promise.resolve(info);
    if (cmd === 'game_result') {
      if (args && args.won) wins += 1;
      return Promise.resolve();
    }
    return Promise.reject(new Error('not available in Anna: ' + cmd));
  }
  window.DC = { invoke, listen: () => Promise.resolve(() => {}), log: (m) => console.log(m) };

  const close = () => {
    // Same rule as pageQuery in core.js: keep Anna's keys, latest token, then ours.
    const back = new URLSearchParams(location.search);
    back.delete('glang');
    const token = window.DC_HOST && window.DC_HOST.token;
    if (token) back.set('t', token);
    back.set('view', 'main');
    back.set('tab', 'games');
    back.set('wins', String(wins));
    location.replace('../index.html?' + back);
    return Promise.resolve();
  };
  window.__TAURI__ = { window: { getCurrentWindow: () => ({ close, hide: close }) } };
  // Sounds: the pages pick the voice folder from this key (default Japanese).
  try {
    if (!localStorage.getItem('voiceLanguage')) localStorage.setItem('voiceLanguage', info.lang);
  } catch (_) { /* ignore */ }

  // The pages are drawn at 900×560 and shrunk with `html { zoom }`: fit the window instead.
  const fit = () => {
    document.documentElement.style.zoom = String(Math.max(0.3, Math.min(innerWidth / 900, innerHeight / 560)));
  };
  addEventListener('resize', fit);
  fit();
  // The page's × becomes the way back to the panel.
  const btn = document.getElementById('closeBtn');
  if (btn) {
    btn.textContent = '←';
    btn.title = { vi: 'Quay lại', ja: '戻る' }[info.lang] || 'Back';
  }
})();
