// node --test app/js/  (npm test)
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const V = require('./voice-core.js');

test('giọng theo ngôn ngữ chữ', () => {
  assert.equal(V.forLanguage('vi'), 'vi-el');
  assert.equal(V.forLanguage('en'), 'en');
  assert.equal(V.forLanguage('ja'), 'ja');
  assert.equal(V.forLanguage('xx'), 'ja');
});

test('đã chọn giọng thì giữ nguyên, kể cả trên bản cài mới', () => {
  assert.deepEqual(V.initial({ saved: 'vi', fresh: true, textLang: 'en' }), { voice: 'vi', persist: false });
  assert.deepEqual(V.initial({ saved: 'en', fresh: false, textLang: 'vi' }), { voice: 'en', persist: false });
});

test('cài mới: giọng theo chữ và ghi lại ngay', () => {
  assert.deepEqual(V.initial({ saved: null, fresh: true, textLang: 'vi' }), { voice: 'vi-el', persist: true });
  assert.deepEqual(V.initial({ saved: null, fresh: true, textLang: 'ja' }), { voice: 'ja', persist: true });
});

test('người dùng cũ chưa chọn giọng: không tự đổi', () => {
  assert.deepEqual(V.initial({ saved: null, fresh: false, textLang: 'vi' }), { voice: 'ja', persist: false });
  // Giá trị lạ trong localStorage coi như chưa chọn.
  assert.deepEqual(V.initial({ saved: 'fr', fresh: false, textLang: 'vi' }), { voice: 'ja', persist: false });
});

test('mời đổi giọng: chỉ khi chữ Việt, giọng chưa Việt, chưa mời, không tắt tiếng', () => {
  const base = { textLang: 'vi', voice: 'ja', offered: false, muted: false };
  assert.equal(V.offerDue(base), true);
  assert.equal(V.offerDue({ ...base, voice: 'en' }), true);
  assert.equal(V.offerDue({ ...base, voice: 'vi' }), false);
  assert.equal(V.offerDue({ ...base, voice: 'vi-el' }), false);
  assert.equal(V.offerDue({ ...base, textLang: 'ja' }), false);
  assert.equal(V.offerDue({ ...base, offered: true }), false);
  assert.equal(V.offerDue({ ...base, muted: true }), false);
});

test('mỗi giọng có thư mục âm thanh, và giọng Việt có clip câu mẫu', () => {
  const audio = path.join(__dirname, '..', 'media', 'audio');
  for (const voice of V.VOICES) {
    assert.ok(fs.existsSync(path.join(audio, voice, 'headpat.mp3')), voice);
    if (V.isVietnamese(voice)) {
      assert.ok(fs.readdirSync(path.join(audio, voice, 'lines')).length > 100, voice + '/lines');
      assert.ok(fs.existsSync(path.join(audio, voice, 'bubble-muteOff.mp3')), voice + ' bubble');
    }
  }
});

test('trang pet nạp voice-core trước pet-config', () => {
  const html = fs.readFileSync(path.join(__dirname, '..', 'pet.html'), 'utf8');
  const core = html.indexOf('js/voice-core.js');
  assert.ok(core > 0 && core < html.indexOf('js/pet-config.js'));
});
