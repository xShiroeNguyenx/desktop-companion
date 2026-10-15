// Popup logic: translate / quick-reply / save-task over the captured selection.
(function () {
  const { invoke, listen, log } = window.DC;
  const $ = (id) => document.getElementById(id);

  let selection = { text: '', source: '' };
  // Mở từ menu chuột phải của pet: không có đoạn bôi đen, người dùng tự gõ.
  let manual = false;

  /** Văn bản nguồn cho cả 3 tab: ô tự gõ (chế độ tự nhập) hoặc đoạn đã bôi đen. */
  function sourceText() {
    return manual ? $('selInput').value.trim() : selection.text;
  }

  // Bật / tắt chế độ tự nhập: đổi khung đoạn bôi đen thành ô gõ được.
  function setManual(on) {
    manual = on;
    $('selectedText').style.display = on ? 'none' : '';
    $('selInput').style.display = on ? '' : 'none';
    if (on) {
      $('selInput').value = '';
      setTimeout(() => $('selInput').focus(), 60);
    }
  }

  // Populate the context dropdown from the saved Settings list.
  async function loadContexts() {
    try {
      const s = await invoke('settings_get');
      const list = Array.isArray(s.contexts) ? s.contexts : [];
      const sel = $('trContext');
      sel.innerHTML = '<option value="">(Mặc định)</option>';
      for (const c of list) {
        const opt = document.createElement('option');
        opt.value = c; opt.textContent = c;
        sel.appendChild(opt);
      }
      if (s.default_context) sel.value = s.default_context;
    } catch (e) { /* ignore */ }
  }

  async function loadSelection() {
    try {
      const sel = await invoke('get_selection');
      if (sel && sel.text) {
        selection = sel;
        $('selectedText').textContent = sel.text;
      } else {
        $('selectedText').textContent = '(chưa có văn bản được chọn)';
      }
    } catch (e) {
      console.error('[popup] get_selection', e);
    }
  }

  // Tabs
  function switchTab(name) {
    document.querySelectorAll('.tab').forEach((x) => x.classList.toggle('active', x.dataset.tab === name));
    document.querySelectorAll('.panel').forEach((x) => x.classList.remove('active'));
    const panel = $('panel-' + name);
    if (panel) panel.classList.add('active');
    // Clarify what the selected text represents per tab.
    const lbl = $('selLabel');
    if (lbl) {
      const what =
        name === 'reply' ? 'Tin nhắn cần trả lời (ngữ cảnh)'
        : name === 'task' ? 'Nội dung lưu thành task'
        : 'Văn bản cần dịch';
      lbl.textContent = manual ? 'Nhập ' + what.charAt(0).toLowerCase() + what.slice(1) : what;
    }
  }
  document.querySelectorAll('.tab').forEach((t) => {
    t.addEventListener('click', () => switchTab(t.dataset.tab));
  });

  // Translate
  async function runTranslate() {
    if (!sourceText()) { setStatus('trStatus', manual ? 'Gõ văn bản cần dịch trước nha.' : 'Chưa có văn bản.', 'err'); return; }
    const btn = $('trGo'); btn.disabled = true;
    setStatus('trStatus', 'Đang dịch...');
    try {
      const out = await invoke('llm_translate', {
        text: sourceText(),
        context: $('trContext').value,
        target: $('trTarget').value,
      });
      $('trResult').textContent = out;
      $('trResult').classList.remove('empty');
      setStatus('trStatus', '');
    } catch (e) {
      setStatus('trStatus', String(e), 'err');
    }
    btn.disabled = false;
  }
  $('trGo').addEventListener('click', runTranslate);
  $('trCopy').addEventListener('click', () => copy($('trResult').textContent, 'trStatus'));

  // Reply
  $('rpGo').addEventListener('click', async () => {
    if (!sourceText()) { setStatus('rpStatus', manual ? 'Gõ tin nhắn cần trả lời trước nha.' : 'Chưa có văn bản gốc.', 'err'); return; }
    if (!$('rpInput').value.trim()) { setStatus('rpStatus', 'Nhập câu trả lời tiếng Việt.', 'err'); return; }
    const btn = $('rpGo'); btn.disabled = true;
    setStatus('rpStatus', 'Đang tạo câu trả lời...');
    try {
      const out = await invoke('llm_quick_reply', {
        source: sourceText(),
        reply: $('rpInput').value,
        target: $('rpTarget').value,
      });
      $('rpResult').textContent = out;
      $('rpResult').classList.remove('empty');
      setStatus('rpStatus', '');
    } catch (e) {
      setStatus('rpStatus', String(e), 'err');
    }
    btn.disabled = false;
  });
  $('rpCopy').addEventListener('click', () => copy($('rpResult').textContent, 'rpStatus'));

  // Save task
  $('tkGo').addEventListener('click', async () => {
    if (!sourceText()) { setStatus('tkStatus', manual ? 'Gõ nội dung công việc trước nha.' : 'Chưa có văn bản.', 'err'); return; }
    try {
      await invoke('tasks_add', { text: sourceText(), note: $('tkNote').value, source: manual ? 'manual' : selection.source || '' });
      setStatus('tkStatus', 'Đã lưu công việc! ✓', 'ok');
      $('tkNote').value = '';
    } catch (e) {
      setStatus('tkStatus', String(e), 'err');
    }
  });
  $('tkOpen').addEventListener('click', () => invoke('open_tasks'));

  // Copy the original captured selection (the auto-capture restores the user's
  // old clipboard, so the selected text would otherwise be unreachable).
  $('selCopy').addEventListener('click', async () => {
    if (!sourceText()) return;
    const btn = $('selCopy');
    try {
      await navigator.clipboard.writeText(sourceText());
      const old = btn.textContent;
      btn.textContent = 'Đã chép ✓';
      setTimeout(() => { btn.textContent = old; }, 1200);
    } catch (e) {
      log('selCopy: ' + e);
    }
  });

  function setStatus(id, msg, kind) {
    const el = $(id);
    el.className = 'status' + (kind ? ' ' + kind : '');
    el.textContent = msg;
  }

  async function copy(text, statusId) {
    if (!text) return;
    try {
      await navigator.clipboard.writeText(text);
      setStatus(statusId, 'Đã sao chép! ✓', 'ok');
    } catch (e) {
      setStatus(statusId, 'Không sao chép được.', 'err');
    }
  }

  function resetResults() {
    $('trResult').textContent = 'Bản dịch sẽ hiện ở đây.'; $('trResult').classList.add('empty');
    $('rpResult').textContent = 'Câu trả lời sẽ hiện ở đây.'; $('rpResult').classList.add('empty');
    $('rpInput').value = '';
    setStatus('trStatus', ''); setStatus('rpStatus', ''); setStatus('tkStatus', '');
  }

  // Re-fetch when the popup (full 3-tab menu) is reused for a fresh selection.
  // Payload là tab cần mở sẵn ("translate" | "reply" | "task") — ring gửi
  // "reply"/"task" nên phải tôn trọng nó, không mặc định về tab Dịch.
  // "manual" = mở từ menu chuột phải của pet: tab Dịch + ô tự gõ văn bản.
  listen('popup-refresh', async (event) => {
    const payload = (event && typeof event.payload === 'string' && event.payload) || 'translate';
    const isManual = payload === 'manual';
    resetResults();
    setManual(isManual);
    await Promise.all([isManual ? Promise.resolve() : loadSelection(), loadContexts()]);
    switchTab(isManual ? 'translate' : payload);
  });

  loadContexts();
  loadSelection();
})();
