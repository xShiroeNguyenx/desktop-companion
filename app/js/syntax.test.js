// node --test app/js/  (npm test)
// Các trang (settings.js, tasks.js…) không có test riêng: ít nhất phải dịch
// được. Lỗi cú pháp ở đây làm hỏng trắng cả trang mà không test nào khác bắt.
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const vm = require('node:vm');
const { spawnSync } = require('node:child_process');

// File có import / export là ES module (pet, menu tròn): dịch theo kiểu module
// bằng `node --check` trên bản sao .mjs; còn lại là script thường.
const isModule = (src) => /^\s*(import|export)\s/m.test(src);

test('mọi file JS của app dịch được', () => {
  const dirs = [__dirname, path.join(__dirname, '..', 'media', 'webview')];
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'dc-syntax-'));
  let n = 0;
  try {
    for (const dir of dirs) {
      for (const f of fs.readdirSync(dir)) {
        if (!f.endsWith('.js') || f.endsWith('.test.js')) continue;
        const file = path.join(dir, f);
        const src = fs.readFileSync(file, 'utf8');
        if (isModule(src)) {
          const copy = path.join(tmp, f.replace(/\.js$/, '.mjs'));
          fs.writeFileSync(copy, src);
          const r = spawnSync(process.execPath, ['--check', copy], { encoding: 'utf8' });
          assert.equal(r.status, 0, `${file}\n${r.stderr}`);
        } else {
          assert.doesNotThrow(() => new vm.Script(src, { filename: file }), file);
        }
        n++;
      }
    }
  } finally {
    fs.rmSync(tmp, { recursive: true, force: true });
  }
  assert.ok(n > 10, `chỉ thấy ${n} file`);
});

test('mọi biểu cảm Miu được chọn động đều có file ảnh', () => {
  const W = require('./weather-core.js');
  const K = require('./clock-core.js');
  const faces = new Set(['cheer', 'surprised', 'fight', 'wink']); // calendar-pin.js
  for (const code of [0, 1, 3, 45, 63, 73, 95]) {
    for (const day of [true, false]) for (const bad of [true, false]) faces.add(W.face(code, day, bad));
  }
  for (const p of ['idle', 'focus', 'break']) for (let h = 0; h < 24; h++) faces.add(K.face(p, h));
  for (const f of faces) {
    const file = path.join(__dirname, '..', 'media', 'game', `rps-${f}.png`);
    assert.ok(fs.existsSync(file), file);
  }
});
