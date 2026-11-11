// node --test app/js/  (npm test)
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const F = require('./flower-apps.js');

test('so khớp tên như Rust: hoa thường, có hoặc không ".exe"', () => {
  assert.equal(F.stem('InfraCompanion.exe'), 'infracompanion');
  assert.equal(F.stem('  MyTerm.EXE '), 'myterm');
  assert.equal(F.stem('Game Client'), 'game client');
  assert.equal(F.stem(''), '');
});

test('tên app của một dòng', () => {
  assert.equal(F.lineApp('MyTerm.exe shift'), 'MyTerm.exe');
  assert.equal(F.lineApp('  Game Client  skip '), 'Game Client');
  assert.equal(F.lineApp('# ghi chú shift'), null);
  assert.equal(F.lineApp(''), null);
  assert.equal(F.lineApp('myterm.exe'), null);
});

test('thêm app mới vào cuối, không để dòng trống thừa', () => {
  assert.equal(F.addRule('', 'MyTerm.exe', 'shift'), 'MyTerm.exe shift');
  assert.equal(F.addRule('a.exe skip', 'MyTerm.exe', 'shift'), 'a.exe skip\nMyTerm.exe shift');
  assert.equal(F.addRule('a.exe skip\n\n', 'MyTerm.exe', 'skip'), 'a.exe skip\nMyTerm.exe skip');
});

test('app đã có dòng thì đổi cách chép tại chỗ, giữ ghi chú và dòng khác', () => {
  const text = '# terminal công ty\nmyterm shift\nb.exe ctrl';
  assert.equal(F.addRule(text, 'MyTerm.exe', 'skip'), '# terminal công ty\nMyTerm.exe skip\nb.exe ctrl');
  // Dòng ghi chú có tên app không bị coi là luật.
  assert.equal(F.addRule('# myterm.exe shift', 'myterm.exe', 'skip'), '# myterm.exe shift\nmyterm.exe skip');
});

test('cách chép lạ hoặc tên trống thì không đổi gì', () => {
  assert.equal(F.addRule('a.exe skip', 'b.exe', 'copy'), 'a.exe skip');
  assert.equal(F.addRule('a.exe skip', '  ', 'shift'), 'a.exe skip');
});

test('cách chép trùng với bộ Rust chấp nhận', () => {
  const rs = fs.readFileSync(path.join(__dirname, '../../src-tauri/src/selection.rs'), 'utf8');
  for (const m of F.MODES) assert.match(rs, new RegExp(`"${m}" =>`), m);
});

test('Cài đặt có ô, nút, nạp script và gửi trường flower_apps', () => {
  const html = fs.readFileSync(path.join(__dirname, '../settings.html'), 'utf8');
  for (const id of ['flowerApps', 'flowerLast', 'flowerAddShift', 'flowerAddSkip']) {
    assert.match(html, new RegExp(`id="${id}"`), id);
  }
  assert.ok(html.indexOf('js/flower-apps.js') < html.indexOf('js/settings.js'), 'flower-apps.js nạp trước settings.js');
  const js = fs.readFileSync(path.join(__dirname, 'settings.js'), 'utf8');
  assert.match(js, /flower_apps: \$\('flowerApps'\)\.value/);
  assert.match(js, /invoke\('flower_last_app'\)/);
  const main = fs.readFileSync(path.join(__dirname, '../../src-tauri/src/main.rs'), 'utf8');
  assert.match(main, /commands::flower_last_app,/);
});
