// Quick-translate window: fetch the selection, translate with the default
// context, show the result + a Copy button. Nothing else.
(function () {
  const { invoke, listen, log } = window.DC;
  const $ = (id) => document.getElementById(id);
  let lastResult = '';

  function setStatus(msg, kind) {
    const el = $('status');
    el.className = 'status' + (kind ? ' ' + kind : '');
    el.textContent = msg || '';
  }

  async function run() {
    lastResult = '';
    $('source').textContent = '...';
    $('result').textContent = 'Đang dịch...';
    $('result').classList.add('empty');
    setStatus('');
    let sel;
    try {
      sel = await invoke('get_selection');
    } catch (e) {
      setStatus(String(e), 'err');
      return;
    }
    if (!sel || !sel.text) {
      $('source').textContent = '(chưa có văn bản)';
      $('result').textContent = '(chưa có văn bản được chọn)';
      return;
    }
    $('source').textContent = sel.text;
    try {
      // Empty context → Rust falls back to the default context from Settings.
      const out = await invoke('llm_translate', { text: sel.text, context: '', target: '' });
      lastResult = out;
      $('result').textContent = out;
      $('result').classList.remove('empty');
    } catch (e) {
      $('result').textContent = '';
      setStatus(String(e), 'err');
    }
  }

  $('copyBtn').addEventListener('click', async () => {
    if (!lastResult) return;
    try {
      await navigator.clipboard.writeText(lastResult);
      setStatus('Đã sao chép! ✓', 'ok');
    } catch (e) {
      setStatus('Không sao chép được.', 'err');
    }
  });

  listen('quick-refresh', run);
  run();
  log('quick.js ready');
})();
