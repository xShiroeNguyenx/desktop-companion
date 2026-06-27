// Shared, defensive access to the Tauri API for the aux windows (flower /
// popup / tasks / settings). withGlobalTauri exposes window.__TAURI__, but the
// exact shape (.core.invoke vs .invoke) and injection timing can vary, so we
// resolve it LAZILY on every call (never capture a stale/empty reference) and
// surface any failure both to the Rust log and the browser console.
(function () {
  function rawInvoke() {
    const T = window.__TAURI__ || {};
    return (T.core && T.core.invoke) || T.invoke || null;
  }

  function invoke(cmd, args) {
    const fn = rawInvoke();
    if (!fn) return Promise.reject(new Error('Tauri invoke chưa sẵn sàng (' + cmd + ')'));
    return fn(cmd, args);
  }

  function listen(event, cb) {
    const T = window.__TAURI__ || {};
    const fn = T.event && T.event.listen;
    if (!fn) return Promise.resolve(() => {});
    return fn(event, cb);
  }

  function log(msg) {
    const fn = rawInvoke();
    if (fn) { try { fn('dc_log', { msg: String(msg) }); } catch (_) { /* ignore */ } }
    console.log(msg);
  }

  window.DC = { invoke, listen, log };

  // Make uncaught errors visible instead of silently killing the window script.
  window.addEventListener('error', (e) => {
    log('JS error: ' + (e.message || e.error) + ' @ ' + (e.filename || '') + ':' + (e.lineno || ''));
  });
  window.addEventListener('unhandledrejection', (e) => {
    log('Promise rejected: ' + (e.reason && e.reason.message ? e.reason.message : e.reason));
  });
})();
