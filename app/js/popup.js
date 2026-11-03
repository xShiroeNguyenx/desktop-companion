// Popup logic: translate / quick-reply / save-task over the captured selection.
(function () {
  const { invoke, listen, log } = window.DC;
  const $ = (id) => document.getElementById(id);
  const T = (key, vars) => window.I18n.t(key, vars);

  let selection = { text: '', source: '' };
  // Mở từ menu chuột phải của pet: không có đoạn bôi đen, người dùng tự gõ.
  let manual = false;
  // Tab đang mở (null = chưa mở lần nào: nhãn chung "Văn bản đã chọn").
  let currentTab = null;

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
      sel.innerHTML = '';
      const def = document.createElement('option');
      def.value = '';
      def.dataset.i18n = 'popup.ctxDefault';
      def.textContent = T('popup.ctxDefault');
      sel.appendChild(def);
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
        selection = { text: '', source: '' };
        $('selectedText').textContent = T('popup.noSelection');
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
    currentTab = name;
    paintSelLabel();
  }

  // Clarify what the selected text represents per tab.
  function paintSelLabel() {
    const lbl = $('selLabel');
    if (!lbl) return;
    if (!currentTab) { lbl.textContent = T('popup.selected'); return; }
    const what = currentTab === 'reply' || currentTab === 'task' ? currentTab : 'translate';
    lbl.textContent = T((manual ? 'popup.in.' : 'popup.sel.') + what);
  }
  document.querySelectorAll('.tab').forEach((t) => {
    t.addEventListener('click', () => switchTab(t.dataset.tab));
  });

  // Translate
  async function runTranslate() {
    if (!sourceText()) { setStatus('trStatus', manual ? T('popup.trNeedManual') : T('popup.needText'), 'err'); return; }
    const btn = $('trGo'); btn.disabled = true;
    setStatus('trStatus', T('popup.translating'));
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
    if (!sourceText()) { setStatus('rpStatus', manual ? T('popup.rpNeedManual') : T('popup.rpNeedSource'), 'err'); return; }
    if (!$('rpInput').value.trim()) { setStatus('rpStatus', T('popup.rpNeedReply'), 'err'); return; }
    const btn = $('rpGo'); btn.disabled = true;
    setStatus('rpStatus', T('popup.replying'));
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
    if (!sourceText()) { setStatus('tkStatus', manual ? T('popup.tkNeedManual') : T('popup.needText'), 'err'); return; }
    try {
      await invoke('tasks_add', { text: sourceText(), note: $('tkNote').value, source: manual ? 'manual' : selection.source || '' });
      setStatus('tkStatus', T('popup.tkSaved'), 'ok');
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
      btn.textContent = T('popup.copied');
      setTimeout(() => { btn.textContent = T('popup.copy'); }, 1200);
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
      setStatus(statusId, T('popup.copyOk'), 'ok');
    } catch (e) {
      setStatus(statusId, T('popup.copyFail'), 'err');
    }
  }

  function resetResults() {
    $('trResult').textContent = T('popup.trEmpty'); $('trResult').classList.add('empty');
    $('rpResult').textContent = T('popup.rpEmpty'); $('rpResult').classList.add('empty');
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

  // Đổi ngôn ngữ: chữ tĩnh do I18n.apply() lo; ở đây vẽ lại chữ dựng bằng JS
  // (kết quả đã có thì giữ nguyên, chỉ đổi chữ "sẽ hiện ở đây").
  window.I18n.onChange(() => {
    paintSelLabel();
    if (!selection.text) $('selectedText').textContent = T('popup.noSelection');
    if ($('trResult').classList.contains('empty')) $('trResult').textContent = T('popup.trEmpty');
    if ($('rpResult').classList.contains('empty')) $('rpResult').textContent = T('popup.rpEmpty');
  });

  loadContexts();
  loadSelection();
})();
