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
  // Giọng đã chọn cho từng engine — đổi engine qua lại không mất lựa chọn.
  const ttsVoices = { windows: '', edge: 'vi-VN-HoaiMyNeural' };

  const TTS_NOTES = {
    off: '',
    windows: 'Chạy ngay trên máy, không gửi gì ra ngoài. Giọng hơi máy móc.',
    edge: 'Giọng tự nhiên, có tiếng Việt. Đây là dịch vụ không chính thức của Microsoft: câu em nói được gửi lên máy chủ Microsoft, và dịch vụ có thể ngừng bất cứ lúc nào.',
  };

  async function syncTtsBlock() {
    const engine = $('ttsEngine').value;
    $('ttsEngineNote').textContent = TTS_NOTES[engine] || '';
    $('ttsVoiceBlock').style.display = engine === 'off' ? 'none' : '';
    if (engine === 'off') return;
    const sel = $('ttsVoice');
    const st = $('ttsStatus');
    st.className = 'status';
    st.textContent = '';
    sel.innerHTML = '';
    let voices = [];
    try {
      voices = await invoke('tts_voices', { engine });
    } catch (e) {
      st.className = 'status err';
      st.textContent = String(e);
    }
    if (engine !== $('ttsEngine').value) return; // đã đổi engine trong lúc chờ
    if (engine === 'windows') {
      const def = document.createElement('option');
      def.value = ''; def.textContent = 'Mặc định của Windows';
      sel.appendChild(def);
    }
    for (const [id, name] of voices) {
      const opt = document.createElement('option');
      opt.value = id; opt.textContent = name;
      sel.appendChild(opt);
    }
    const wanted = ttsVoices[engine] || '';
    sel.value = [...sel.options].some((o) => o.value === wanted) ? wanted : (sel.options[0]?.value ?? '');
    ttsVoices[engine] = sel.value;
    if (engine === 'windows' && !voices.some(([, name]) => /vi-VN/i.test(name))) {
      st.textContent = 'Máy chưa có giọng tiếng Việt. Thêm ở: Cài đặt Windows → Thời gian & ngôn ngữ → Giọng nói → Thêm giọng nói.';
    }
  }

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
      span.style.cssText = 'flex:1; font-size:12.5px; background:var(--panel); color:var(--text); border:1px solid var(--border); border-radius:6px; padding:5px 8px; overflow:hidden; text-overflow:ellipsis; white-space:nowrap;';
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
      // Màu theo biến của ui.css để giao diện Xứ Tuyết (theme-snow.css) cũng đổi theo.
      span.style.cssText = 'flex:1; font-size:12.5px; background:var(--panel); color:var(--text); border:1px solid var(--border); border-radius:6px; padding:5px 8px;';
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

  // Xưng hô tiếng Việt chỉ hiện khi nói tiếng Việt; hậu tố Nhật chỉ khi nói
  // tiếng Nhật. Ô tự nhập chỉ hiện khi chọn "Tự nhập…".
  function syncCompanionFields() {
    const lang = $('messageLanguage').value;
    $('viPairBlock').style.display = lang === 'vi' ? '' : 'none';
    $('jaHonorificBlock').style.display = lang === 'ja' ? '' : 'none';
    $('viCustom').style.display = $('viPair').value === 'custom' ? '' : 'none';
  }
  $('messageLanguage').addEventListener('change', syncCompanionFields);
  $('viPair').addEventListener('change', syncCompanionFields);

  function viPronouns() {
    const v = $('viPair').value;
    if (v === 'custom') return [$('viSelf').value.trim(), $('viUser').value.trim()];
    return v.split('|');
  }

  // Chu kỳ nhắc sức khoẻ (phút, 0 = tắt). Giá trị lạ (sửa tay settings.json)
  // được thêm vào danh sách để không bị đổi ngầm khi Lưu.
  const REMIND_CHOICES = [0, 30, 45, 60, 90, 120];
  function fillRemind(id, value) {
    const sel = $(id);
    const values = REMIND_CHOICES.includes(value) ? REMIND_CHOICES : [...REMIND_CHOICES, value].sort((a, b) => a - b);
    sel.innerHTML = '';
    for (const v of values) {
      const opt = document.createElement('option');
      opt.value = String(v);
      opt.textContent = v === 0 ? 'Tắt' : `Sau ${v} phút ngồi máy`;
      sel.appendChild(opt);
    }
    sel.value = String(value);
  }

  // Ô chọn ngưỡng CPU / RAM (%). Giá trị lạ (sửa tay settings.json) được thêm
  // vào danh sách để không bị đổi ngầm khi Lưu.
  const WARN_CHOICES = [70, 80, 90, 95];
  function fillWarn(id, value) {
    const sel = $(id);
    const values = WARN_CHOICES.includes(value) ? WARN_CHOICES : [...WARN_CHOICES, value].sort((a, b) => a - b);
    sel.innerHTML = '';
    for (const v of values) {
      const opt = document.createElement('option');
      opt.value = String(v);
      opt.textContent = `Từ ${v}%`;
      sel.appendChild(opt);
    }
    sel.value = String(value);
  }

  // Ô chọn có sẵn danh sách: giá trị lạ thì thêm một dòng để vẫn hiện đúng.
  function selectValue(id, value, label) {
    const sel = $(id);
    const v = String(value);
    if (![...sel.options].some((o) => o.value === v)) {
      const opt = document.createElement('option');
      opt.value = v;
      opt.textContent = label(value);
      sel.appendChild(opt);
    }
    sel.value = v;
  }

  // Sinh nhật: hai ô ngày / tháng, "—" = không nhập. Lưu dạng "MM-DD".
  (function fillBirthday() {
    const day = $('bdayDay');
    const month = $('bdayMonth');
    day.add(new Option('Ngày —', ''));
    month.add(new Option('Tháng —', ''));
    for (let d = 1; d <= 31; d++) day.add(new Option(`Ngày ${d}`, String(d).padStart(2, '0')));
    for (let m = 1; m <= 12; m++) month.add(new Option(`Tháng ${m}`, String(m).padStart(2, '0')));
  })();
  function setBirthday(value) {
    const m = /^(\d{2})-(\d{2})$/.exec(value || '');
    $('bdayMonth').value = m ? m[1] : '';
    $('bdayDay').value = m ? m[2] : '';
  }
  /** "MM-DD", "" nếu không nhập, hoặc null nếu chỉ chọn một ô. */
  function getBirthday() {
    const m = $('bdayMonth').value;
    const d = $('bdayDay').value;
    if (!m && !d) return '';
    if (!m || !d) return null;
    return `${m}-${d}`;
  }

  // Ngưỡng CPU / RAM chỉ hiện khi bật cảnh báo.
  function syncSysmon() {
    $('sysmonBlock').style.display = $('sysmonEnabled').checked ? '' : 'none';
  }
  $('sysmonEnabled').addEventListener('change', syncSysmon);

  $('toastTest').addEventListener('click', async () => {
    const st = $('toastStatus');
    try {
      await invoke('notify_test');
      st.className = 'status ok';
      st.textContent = 'Đã gửi. Không thấy thông báo thì kiểm tra Cài đặt Windows → Hệ thống → Thông báo (và chế độ Không làm phiền / Focus).';
    } catch (e) {
      st.className = 'status err';
      st.textContent = 'Không gửi được thông báo: ' + e;
    }
  });

  // Nhóm Thời tiết: áp dụng ngay để xem trước, không chờ Lưu (giống Bunny Lounge).
  const WEATHER_HINT = $('weatherStatus').textContent;
  async function showWeatherStatus() {
    const st = $('weatherStatus');
    let msg = '';
    try { msg = await invoke('weather_fx_status'); } catch (_) { /* ignore */ }
    st.className = msg ? 'status err' : 'status';
    st.textContent = msg || WEATHER_HINT;
  }
  async function applyWeather() {
    try {
      syncWeatherFields();
      await invoke('weather_fx_set', {
        mode: $('weatherMode').value,
        kind: $('weatherKind').value,
        intensity: $('weatherIntensity').value,
        pauseBattery: $('weatherBattery').checked,
        talk: $('weatherTalk').checked,
        lightning: $('weatherLightning').checked,
      });
      // Lớp phủ dựng trong vòng nền (nửa giây một nhịp): chờ một chút rồi hỏi kết quả.
      setTimeout(showWeatherStatus, 1200);
    } catch (e) {
      const st = $('weatherStatus');
      st.className = 'status err';
      st.textContent = 'Không đổi được: ' + e;
    }
  }
  // "Theo mùa" tự chọn hiệu ứng → ô chọn hiệu ứng chỉ dùng cho chế độ chọn tay.
  const WEATHER_MODE_HINT = {
    off: '',
    manual: 'Hiệu ứng bạn chọn ở ô bên dưới.',
    season: 'Xuân hoa anh đào · hè nắng · thu lá bay · đông tuyết · đêm có sao. Không cần mạng.',
    live: 'Theo trời thật ở thành phố bạn chọn (cập nhật 30 phút/lần). Trời nhiều mây thì không vẽ gì.',
  };
  function syncWeatherFields() {
    const mode = $('weatherMode').value;
    $('weatherKind').disabled = mode !== 'manual';
    $('weatherIntensity').disabled = mode === 'live';
    $('weatherLightning').disabled = mode !== 'live';
    $('weatherModeHint').textContent = WEATHER_MODE_HINT[mode] || '';
  }
  $('weatherOpen').addEventListener('click', () => invoke('open_weather').catch((e) => log('open_weather: ' + e)));
  $('openCollection').addEventListener('click', () => invoke('open_collection').catch((e) => log('open_collection: ' + e)));

  // Nhóm 🧩 Widget: tick = ghim / gỡ ngay (không chờ Lưu). Trạng thái đọc lại
  // mỗi lần bảng được focus, vì widget có thể đã gỡ bằng ✕ trên chính nó.
  const WIDGETS = { wCalendar: 'calendar', wClock: 'clock', wWeather: 'weather' };
  async function syncWidgets() {
    try {
      const st = await invoke('widgets_status');
      for (const [box, id] of Object.entries(WIDGETS)) $(box).checked = !!st[id];
    } catch (e) {
      log('widgets_status: ' + e);
    }
  }
  function widgetMsg(text, ok) {
    const status = $('widgetStatus');
    status.className = ok ? 'status ok' : 'status err';
    status.textContent = text;
  }
  for (const [box, id] of Object.entries(WIDGETS)) {
    $(box).addEventListener('change', async () => {
      const want = $(box).checked;
      try {
        if (id === 'weather') await invoke('weather_pin_set', { pinned: want });
        else await invoke('widget_pin_set', { id, pinned: want });
        widgetMsg(want ? 'Đã ghim lên màn hình ✓' : 'Đã gỡ khỏi màn hình.', true);
      } catch (e) {
        // Thời tiết chưa chọn thành phố: mở thẻ dự báo để chọn (như menu tròn).
        if (id === 'weather') invoke('open_weather').catch(() => {});
        widgetMsg('Không ghim được: ' + e, false);
        log('widget ' + id + ': ' + e);
      }
      syncWidgets();
    });
  }
  $('widgetsTidy').addEventListener('click', async () => {
    try {
      await invoke('widgets_tidy');
      widgetMsg('Đã xếp gọn các widget đang ghim ✓', true);
    } catch (e) {
      widgetMsg('Không xếp được: ' + e, false);
    }
  });
  // Ô chọn đồng hồ thêm: danh sách thành phố của clock-core.js, lưu cùng nút Lưu.
  for (const id of ['clockZone1', 'clockZone2']) {
    const sel = $(id);
    sel.append(new Option(window.ClockCore.T.vi.none, ''));
    for (const [zone, vi] of window.ClockCore.ZONES) sel.append(new Option(vi, zone));
  }
  function setZone(sel, zone) {
    // Múi lưu từ trước nhưng không có trong danh sách: vẫn giữ, hiện tên theo đuôi.
    if (zone && ![...sel.options].some((o) => o.value === zone)) sel.append(new Option(window.ClockCore.zoneName(zone, 'vi'), zone));
    sel.value = zone || '';
  }
  window.addEventListener('focus', syncWidgets);
  syncWidgets();
  for (const id of ['weatherMode', 'weatherKind', 'weatherIntensity', 'weatherBattery', 'weatherTalk', 'weatherLightning']) {
    $(id).addEventListener('change', applyWeather);
  }

  // Bật / tắt nhân vật bám cạnh bảng: áp dụng ngay để xem trước, không chờ Lưu.
  $('sideGirls').addEventListener('change', async () => {
    const box = $('sideGirls');
    try {
      await invoke('set_side_girls', { enabled: box.checked });
    } catch (e) {
      box.checked = !box.checked;
      const st = $('status');
      st.className = 'status err';
      st.textContent = 'Không đổi được: ' + e;
    }
  });

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
      $('sideGirls').checked = s.side_girls === true;
      $('weatherMode').value = ['off', 'manual', 'season', 'live'].includes(s.weather_mode) ? s.weather_mode : 'off';
      $('weatherLightning').checked = s.weather_lightning === true;
      $('weatherKind').value = ['rain', 'snow', 'sakura', 'leaves', 'fog', 'sun', 'stars'].includes(s.weather_manual) ? s.weather_manual : 'rain';
      $('weatherTalk').checked = s.weather_talk !== false;
      $('calendarTalk').checked = s.calendar_talk !== false;
      setZone($('clockZone1'), (s.clock_zones || [])[0]);
      setZone($('clockZone2'), (s.clock_zones || [])[1]);
      syncWeatherFields();
      $('weatherIntensity').value = ['light', 'medium', 'heavy'].includes(s.weather_intensity) ? s.weather_intensity : 'medium';
      $('weatherBattery').checked = s.weather_pause_battery !== false;
      $('weatherShortcut').value = s.weather_shortcut ?? 'Ctrl+Alt+W';
      showWeatherStatus();
      $('proactive').value = String(s.proactive_minutes ?? 60);
      // Giá trị lạ (sửa tay settings.json) → vẫn hiện được thay vì ô trống.
      if ($('proactive').value === '') $('proactive').value = '60';
      $('ttsEngine').value = s.tts_engine || 'off';
      if (!$('ttsEngine').value) $('ttsEngine').value = 'off';
      ttsVoices.windows = s.tts_voice_windows || '';
      ttsVoices.edge = s.tts_voice_edge || 'vi-VN-HoaiMyNeural';
      // Bạn đồng hành
      $('petName').value = s.pet_name || '';
      $('userName').value = s.user_name || '';
      $('messageLanguage').value = ['vi', 'ja', 'en'].includes(s.message_language) ? s.message_language : 'vi';
      $('jaHonorific').value = s.ja_honorific ?? 'さん';
      const pair = `${s.vi_self || 'em'}|${s.vi_user || 'anh'}`;
      const known = [...$('viPair').options].some((o) => o.value === pair);
      $('viPair').value = known ? pair : 'custom';
      $('viSelf').value = s.vi_self || 'em';
      $('viUser').value = s.vi_user || 'anh';
      syncCompanionFields();
      // Nhắc nhở
      fillRemind('remindEyes', s.remind_eyes_min ?? 45);
      fillRemind('remindWater', s.remind_water_min ?? 60);
      fillRemind('remindStand', s.remind_stand_min ?? 60);
      $('quietGap').value = String(s.quiet_gap_min ?? 10);
      if ($('quietGap').value === '') $('quietGap').value = '10';
      $('dueToast').checked = s.due_toast !== false;
      // Tập trung
      selectValue('focusMin', s.focus_min ?? 25, (v) => `${v} phút`);
      selectValue('breakMin', s.break_min ?? 5, (v) => (v ? `${v} phút` : 'Không nghỉ'));
      // Nhận biết máy tính
      selectValue('sleepyIdle', s.sleepy_idle_min ?? 10, (v) => (v ? `Sau ${v} phút` : 'Tắt'));
      $('welcomeBack').checked = s.welcome_back !== false;
      $('quietFullscreen').checked = s.quiet_fullscreen !== false;
      $('sysmonEnabled').checked = s.sysmon_enabled === true;
      fillWarn('cpuWarn', s.cpu_warn_pct ?? 90);
      fillWarn('ramWarn', s.ram_warn_pct ?? 90);
      syncSysmon();
      setBirthday(s.birthday || '');
    } catch (e) { log('settings_get: ' + e); }
    try {
      $('autostart').checked = await invoke('autostart_status');
    } catch (e) { log('autostart_status: ' + e); }
    syncProviderBlocks();
    void syncTtsBlock();
  }

  $('ttsEngine').addEventListener('change', () => void syncTtsBlock());
  $('ttsVoice').addEventListener('change', () => {
    ttsVoices[$('ttsEngine').value] = $('ttsVoice').value;
  });

  // Nghe thử: tổng hợp một câu mẫu bằng engine + giọng đang chọn (chưa cần Lưu).
  let previewAudio = null;
  $('ttsPreview').addEventListener('click', async () => {
    const st = $('ttsStatus');
    const btn = $('ttsPreview');
    const engine = $('ttsEngine').value;
    const voice = $('ttsVoice').value;
    const lang = (voice.match(/[a-z]{2}-[A-Z]{2}/) || ['vi-VN'])[0].slice(0, 2);
    const samples = {
      vi: 'Chào anh, em là bạn nhỏ ở góc màn hình nè.',
      ja: 'こんにちは、今日も一緒に頑張ろうね。',
      zh: '你好，今天也一起加油吧。',
      ko: '안녕하세요, 오늘도 같이 힘내요.',
    };
    const text = samples[lang] || 'Hi there, I am right here in the corner of your screen.';
    btn.disabled = true;
    st.className = 'status'; st.textContent = 'Đang tạo giọng...';
    try {
      const res = await invoke('tts_preview', { engine, voice, text });
      const bin = atob(res.data);
      const bytes = new Uint8Array(bin.length);
      for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
      const url = URL.createObjectURL(new Blob([bytes], { type: res.mime }));
      if (previewAudio) previewAudio.pause();
      previewAudio = new Audio(url);
      previewAudio.addEventListener('ended', () => URL.revokeObjectURL(url));
      await previewAudio.play();
      st.className = 'status ok'; st.textContent = 'Đang phát ✓';
    } catch (e) {
      st.className = 'status err'; st.textContent = String(e);
    } finally {
      btn.disabled = false;
    }
  });

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
    const birthday = getBirthday();
    if (birthday === null) {
      const st = $('status');
      st.className = 'status err';
      st.textContent = 'Sinh nhật: chọn cả ngày và tháng, hoặc để trống cả hai.';
      return;
    }
    const settings = {
      birthday,
      provider: $('provider').value,
      api_key: $('apiKey').value.trim(),
      model: $('model').value,
      gemini_key: $('geminiKey').value.trim(),
      gemini_model: $('geminiModel').value,
      translate_target: $('target').value,
      contexts: contexts,
      default_context: $('defaultContext').value,
      auto_flower: $('autoFlower').checked,
      side_girls: $('sideGirls').checked,
      proactive_minutes: Number($('proactive').value) || 0,
      tts_engine: $('ttsEngine').value,
      tts_voice_windows: ttsVoices.windows,
      tts_voice_edge: ttsVoices.edge,
      hotkey: hotkeyValue,
      pet_model: petModel,
      custom_models: customModels,
      pet_name: $('petName').value.trim(),
      user_name: $('userName').value.trim(),
      vi_self: viPronouns()[0],
      vi_user: viPronouns()[1],
      ja_honorific: $('jaHonorific').value,
      message_language: $('messageLanguage').value,
      remind_eyes_min: Number($('remindEyes').value) || 0,
      remind_water_min: Number($('remindWater').value) || 0,
      remind_stand_min: Number($('remindStand').value) || 0,
      quiet_gap_min: Number($('quietGap').value) || 10,
      due_toast: $('dueToast').checked,
      focus_min: Number($('focusMin').value) || 25,
      break_min: Number($('breakMin').value) || 0,
      sleepy_idle_min: Number($('sleepyIdle').value) || 0,
      welcome_back: $('welcomeBack').checked,
      quiet_fullscreen: $('quietFullscreen').checked,
      sysmon_enabled: $('sysmonEnabled').checked,
      cpu_warn_pct: Number($('cpuWarn').value) || 90,
      ram_warn_pct: Number($('ramWarn').value) || 90,
      weather_mode: $('weatherMode').value,
      weather_talk: $('weatherTalk').checked,
      calendar_talk: $('calendarTalk').checked,
      // Rust làm sạch (bỏ trống / trùng, tối đa 2).
      clock_zones: [$('clockZone1').value, $('clockZone2').value],
      weather_lightning: $('weatherLightning').checked,
      weather_manual: $('weatherKind').value,
      weather_intensity: $('weatherIntensity').value,
      weather_pause_battery: $('weatherBattery').checked,
      weather_shortcut: $('weatherShortcut').value.trim(),
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
      showWeatherStatus();
    } catch (e) {
      status.className = 'status err';
      status.textContent = String(e);
      log('settings_set: ' + e);
    }
  });

  // ── Updates ─────────────────────────────────────────────────────────────
  $('checkUpdate').addEventListener('click', async () => {
    const st = $('updateStatus');
    st.className = 'status'; st.textContent = 'Đang kiểm tra...';
    $('installUpdate').style.display = 'none';
    try {
      const info = await invoke('check_update');
      if (info.available) {
        st.className = 'status ok';
        st.textContent = 'Có bản mới: v' + info.version + ' (đang dùng v' + info.current + ')';
        $('installUpdate').style.display = '';
      } else {
        st.className = 'status';
        st.textContent = 'Bạn đang dùng bản mới nhất (v' + info.current + ').';
      }
    } catch (e) {
      st.className = 'status err';
      st.textContent = String(e);
    }
  });

  // Việc tải/cài diễn ra trong hộp thoại cập nhật (có ghi chú phát hành +
  // thanh tiến trình), giống hệt hộp thoại tự bật lúc khởi động app.
  $('installUpdate').addEventListener('click', async () => {
    const st = $('updateStatus');
    try {
      await invoke('open_update_dialog');
    } catch (e) {
      st.className = 'status err';
      st.textContent = String(e);
    }
  });

  if (window.DC.listen) window.DC.listen('settings-refresh', load);

  // Giao diện theo bộ nhân vật: js/theme-apply.js. Bật / tắt Bunny Lounge ngay
  // trên trang này cũng đổi giao diện (Rust phát collection-changed; gọi lại cho chắc).
  $('sideGirls').addEventListener('change', () => window.ThemeApply && window.ThemeApply.refresh());

  load();
})();
