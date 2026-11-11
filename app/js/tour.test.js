// node --test app/js/  (npm test)
// Thẻ ❓ Hướng dẫn nhanh (app/media/webview/tour.js) là ES module dùng core.js của
// trang pet: chép sang thư mục tạm (package "module" + core.js giả) rồi import.
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { pathToFileURL } = require('node:url');

async function loadTour() {
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'dc-tour-'));
  fs.writeFileSync(path.join(tmp, 'package.json'), '{"type":"module"}');
  fs.writeFileSync(path.join(tmp, 'core.js'), 'export const vscode = { postMessage() {} };\n');
  fs.copyFileSync(path.join(__dirname, '..', 'media', 'webview', 'tour.js'), path.join(tmp, 'tour.js'));
  try {
    return await import(pathToFileURL(path.join(tmp, 'tour.js')).href);
  } finally {
    fs.rmSync(tmp, { recursive: true, force: true });
  }
}

test('mọi ngôn ngữ có đủ chữ và cùng các bước', async () => {
  const { LABELS } = await loadTour();
  const keys = Object.keys(LABELS.vi).sort();
  for (const lang of ['vi', 'ja', 'en']) {
    const L = LABELS[lang];
    assert.deepEqual(Object.keys(L).sort(), keys, lang);
    assert.equal(L.steps.length, LABELS.vi.steps.length, lang);
    L.steps.forEach(([title, body, withSettings], i) => {
      assert.ok(title.trim() && body.trim(), `${lang} bước ${i + 1}`);
      // Bước nào có nút "Mở Cài đặt" thì mọi ngôn ngữ đều có.
      assert.equal(!!withSettings, !!LABELS.vi.steps[i][2], `${lang} bước ${i + 1}`);
      for (const m of `${title} ${body}`.matchAll(/\{(\w+)\}/g)) {
        assert.ok(['pet', 'hotkey'].includes(m[1]), `${lang} bước ${i + 1}: biến lạ {${m[1]}}`);
      }
    });
  }
});

test('điền tên pet và phím tắt', async () => {
  const { fillStep } = await loadTour();
  assert.equal(fillStep('Hỏi {pet}, nhấn {hotkey}', { pet: 'Miu', hotkey: 'Ctrl+Shift+Space' }), 'Hỏi Miu, nhấn Ctrl+Shift+Space');
  assert.equal(fillStep('{pet}{pet}', { pet: 'A' }), 'AA');
  assert.equal(fillStep('{lạ}', { pet: 'A' }), '{lạ}', 'biến khác giữ nguyên');
});
