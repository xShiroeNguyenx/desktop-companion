// Settings window: load/save app settings (provider, keys, model, target,
// auto-flower) + autostart toggle.
(function () {
  const { invoke, log } = window.DC;
  const $ = (id) => document.getElementById(id);

  function syncProviderBlocks() {
    const p = $('provider').value;
    $('anthropicBlock').style.display = p === 'anthropic' ? '' : 'none';
    $('geminiBlock').style.display = p === 'gemini' ? '' : 'none';
  }

  let petModel = 'hiyori';
  let contexts = [];
  let customModels = [];

  function renderCustomModels() {
    const list = $('customModelList');
    list.innerHTML = '';
    if (!customModels.length) {
      const hint = document.createElement('div');
      hint.className = 'status';
      hint.textContent = 'Chưa có model tự thêm.';
      list.appendChild(hint);
      return;
    }
    customModels.forEach((m) => {
      const row = document.createElement('div');
      row.className = 'row tight';
      row.style.alignItems = 'center';
      const span = document.createElement('span');
      span.textContent = m.name;
      span.title = m.dir;
      span.style.cssText = 'flex:1; font-size:12.5px; background:#272935; border:1px solid #3a3d4d; border-radius:6px; padding:5px 8px; overflow:hidden; text-overflow:ellipsis; white-space:nowrap;';
      const del = document.createElement('button');
      del.className = 'task-del'; del.textContent = '🗑'; del.title = 'Xoá'; del.type = 'button';
      del.style.flex = '0 0 auto';
      del.addEventListener('click', async () => {
        await invoke('remove_custom_model', { id: m.id });
        customModels = customModels.filter((x) => x.id !== m.id);
        renderCustomModels();
      });
      row.append(span, del);
      list.appendChild(row);
    });
  }

  // ── Hotkey capture ──────────────────────────────────────────────────────
  let hotkeyValue = 'Ctrl+Shift+Space';

  // Map a KeyboardEvent.code to the key token Tauri's global-shortcut expects.
  function codeToToken(e) {
    const c = e.code;
    if (c.startsWith('Key')) return c.slice(3);        // KeyD -> D
    if (c.startsWith('Digit')) return c.slice(5);       // Digit1 -> 1
    if (c.startsWith('Numpad')) return 'Numpad' + c.slice(6);
    const map = {
      Space: 'Space', Enter: 'Enter', Tab: 'Tab', Backquote: 'Backquote',
      Minus: 'Minus', Equal: 'Equal', BracketLeft: 'BracketLeft', BracketRight: 'BracketRight',
      Semicolon: 'Semicolon', Quote: 'Quote', Comma: 'Comma', Period: 'Period', Slash: 'Slash',
      Backslash: 'Backslash', ArrowUp: 'Up', ArrowDown: 'Down', ArrowLeft: 'Left', ArrowRight: 'Right',
      Home: 'Home', End: 'End', PageUp: 'PageUp', PageDown: 'PageDown', Insert: 'Insert', Delete: 'Delete',
    };
    if (map[c]) return map[c];
    if (/^F\d{1,2}$/.test(c)) return c;                 // F1..F12
    return null;
  }

  function setHotkeyDisplay(v) {
    hotkeyValue = v;
    $('hotkey').value = v.replace(/\+/g, ' + ');
  }

  $('hotkey').addEventListener('keydown', (e) => {
    e.preventDefault();
    const mods = [];
    if (e.ctrlKey) mods.push('Ctrl');
    if (e.shiftKey) mods.push('Shift');
    if (e.altKey) mods.push('Alt');
    if (e.metaKey) mods.push('Super');
    const key = codeToToken(e);
    const st = $('hotkeyStatus');
    if (!key) { st.className = 'status'; st.textContent = 'Nhấn thêm phím chính (chữ/số/F1...).'; return; }
    if (!mods.length && !/^F\d/.test(key)) {
      st.className = 'status err';
      st.textContent = 'Nên có ít nhất 1 phím bổ trợ (Ctrl/Shift/Alt) để tránh trùng.';
    } else {
      st.className = 'status'; st.textContent = 'Đã ghi nhận. Bấm Lưu để áp dụng.';
    }
    setHotkeyDisplay([...mods, key].join('+'));
  });

  $('hotkeyReset').addEventListener('click', () => {
    setHotkeyDisplay('Ctrl+Shift+Space');
    $('hotkeyStatus').className = 'status';
    $('hotkeyStatus').textContent = 'Đã đặt về mặc định. Bấm Lưu để áp dụng.';
  });

  $('addModelFolder').addEventListener('click', async () => {
    const st = $('modelFolderStatus');
    st.className = 'status'; st.textContent = 'Đang mở hộp thoại chọn thư mục...';
    try {
      const dialog = window.__TAURI__.dialog;
      const dir = await dialog.open({ directory: true, multiple: false, title: 'Chọn thư mục model Live2D' });
      if (!dir) { st.textContent = ''; return; }
      const model = await invoke('add_model_folder', { dir });
      if (!customModels.some((m) => m.id === model.id)) customModels.push(model);
      renderCustomModels();
      st.className = 'status ok';
      st.textContent = 'Đã thêm: ' + model.name + ' ✓ (mở menu chuột phải → Diện mạo để chọn)';
    } catch (e) {
      st.className = 'status err';
      st.textContent = String(e);
    }
  });

  // Render the editable context list + refresh the default-context dropdown.
  function renderContexts() {
    const list = $('contextList');
    list.innerHTML = '';
    contexts.forEach((c, i) => {
      const row = document.createElement('div');
      row.className = 'row tight';
      row.style.alignItems = 'center';
      const span = document.createElement('span');
      span.textContent = c;
      span.style.cssText = 'flex:1; font-size:12.5px; background:#272935; border:1px solid #3a3d4d; border-radius:6px; padding:5px 8px;';
      const del = document.createElement('button');
      del.className = 'task-del'; del.textContent = '🗑'; del.title = 'Xoá'; del.type = 'button';
      del.style.flex = '0 0 auto';
      del.addEventListener('click', () => { contexts.splice(i, 1); renderContexts(); });
      row.append(span, del);
      list.appendChild(row);
    });

    const dc = $('defaultContext');
    const prev = dc.value;
    dc.innerHTML = '<option value="">(Không dùng ngữ cảnh)</option>';
    for (const c of contexts) {
      const opt = document.createElement('option');
      opt.value = c; opt.textContent = c;
      dc.appendChild(opt);
    }
    if ([...dc.options].some((o) => o.value === prev)) dc.value = prev;
  }

  $('addContext').addEventListener('click', () => {
    const v = $('newContext').value.trim();
    if (!v) return;
    contexts.push(v);
    $('newContext').value = '';
    renderContexts();
  });
  $('newContext').addEventListener('keydown', (e) => { if (e.key === 'Enter') $('addContext').click(); });

  async function load() {
    try {
      const s = await invoke('settings_get');
      petModel = s.pet_model || 'hiyori';
      $('provider').value = s.provider || 'anthropic';
      $('apiKey').value = s.api_key || '';
      // Saved Claude model may come from a dynamic fetch — add it if missing.
      const cm = s.model || 'claude-haiku-4-5-20251001';
      const mSel = $('model');
      if (![...mSel.options].some((o) => o.value === cm)) {
        const opt = document.createElement('option');
        opt.value = cm; opt.textContent = cm;
        mSel.insertBefore(opt, mSel.firstChild);
      }
      mSel.value = cm;
      $('geminiKey').value = s.gemini_key || '';
      // Saved model may not be one of the static <option>s (it could come from a
      // dynamic fetch) — add it so the selection sticks.
      const gm = s.gemini_model || 'gemini-2.5-flash';
      const sel = $('geminiModel');
      if (![...sel.options].some((o) => o.value === gm)) {
        const opt = document.createElement('option');
        opt.value = gm; opt.textContent = gm;
        sel.insertBefore(opt, sel.firstChild);
      }
      sel.value = gm;
      $('target').value = s.translate_target || 'vi';
      contexts = Array.isArray(s.contexts) ? s.contexts.slice() : [];
      renderContexts();
      $('defaultContext').value = s.default_context || '';
      customModels = Array.isArray(s.custom_models) ? s.custom_models.slice() : [];
      renderCustomModels();
      setHotkeyDisplay(s.hotkey || 'Ctrl+Shift+Space');
      $('autoFlower').checked = s.auto_flower !== false;
    } catch (e) { log('settings_get: ' + e); }
    try {
      $('autostart').checked = await invoke('autostart_status');
    } catch (e) { log('autostart_status: ' + e); }
    syncProviderBlocks();
  }

  $('provider').addEventListener('change', syncProviderBlocks);

  // Fetch the live model list from the Gemini API using the entered key.
  $('loadGeminiModels').addEventListener('click', async () => {
    const key = $('geminiKey').value.trim();
    const st = $('geminiModelStatus');
    if (!key) { st.className = 'status err'; st.textContent = 'Nhập Gemini API key trước.'; return; }
    st.className = 'status'; st.textContent = 'Đang tải danh sách model...';
    try {
      const models = await invoke('gemini_models', { apiKey: key });
      if (!models || !models.length) { st.className = 'status err'; st.textContent = 'Không tìm thấy model nào.'; return; }
      const cur = $('geminiModel').value;
      const sel = $('geminiModel');
      sel.innerHTML = '';
      for (const [id, name] of models) {
        const opt = document.createElement('option');
        opt.value = id;
        opt.textContent = name + ' (' + id + ')';
        sel.appendChild(opt);
      }
      // keep previous choice if still present
      if ([...sel.options].some((o) => o.value === cur)) sel.value = cur;
      st.className = 'status ok';
      st.textContent = 'Đã tải ' + models.length + ' model. ✓';
    } catch (e) {
      st.className = 'status err';
      st.textContent = String(e);
    }
  });

  // Fetch the live model list from the Anthropic API using the entered key.
  $('loadClaudeModels').addEventListener('click', async () => {
    const key = $('apiKey').value.trim();
    const st = $('claudeModelStatus');
    if (!key) { st.className = 'status err'; st.textContent = 'Nhập Anthropic API key trước.'; return; }
    st.className = 'status'; st.textContent = 'Đang tải danh sách model...';
    try {
      const models = await invoke('anthropic_models', { apiKey: key });
      if (!models || !models.length) { st.className = 'status err'; st.textContent = 'Không tìm thấy model nào.'; return; }
      const cur = $('model').value;
      const sel = $('model');
      sel.innerHTML = '';
      for (const [id, name] of models) {
        const opt = document.createElement('option');
        opt.value = id;
        opt.textContent = name + ' (' + id + ')';
        sel.appendChild(opt);
      }
      if ([...sel.options].some((o) => o.value === cur)) sel.value = cur;
      st.className = 'status ok';
      st.textContent = 'Đã tải ' + models.length + ' model. ✓';
    } catch (e) {
      st.className = 'status err';
      st.textContent = String(e);
    }
  });

  $('saveBtn').addEventListener('click', async () => {
    const settings = {
      provider: $('provider').value,
      api_key: $('apiKey').value.trim(),
      model: $('model').value,
      gemini_key: $('geminiKey').value.trim(),
      gemini_model: $('geminiModel').value,
      translate_target: $('target').value,
      contexts: contexts,
      default_context: $('defaultContext').value,
      auto_flower: $('autoFlower').checked,
      hotkey: hotkeyValue,
      pet_model: petModel,
      custom_models: customModels,
    };

    const status = $('status');
    try {
      await invoke('settings_set', { settings });
      await invoke('autostart_set', { enabled: $('autostart').checked });
      // Apply the hotkey (registers it live + persists hotkey field).
      try {
        await invoke('set_hotkey', { hotkey: hotkeyValue });
      } catch (e) {
        status.className = 'status err';
        status.textContent = 'Đã lưu, nhưng phím tắt lỗi: ' + e;
        return;
      }
      status.className = 'status ok';
      status.textContent = 'Đã lưu! ✓';
    } catch (e) {
      status.className = 'status err';
      status.textContent = String(e);
      log('settings_set: ' + e);
    }
  });

  if (window.DC.listen) window.DC.listen('settings-refresh', load);
  load();
})();
