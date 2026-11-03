// Mọi khoá trong js/strings/*.js có đủ vi / ja / en, cùng biến {…}; mọi khoá
// HTML (data-i18n*) và JS (I18n.t('…')) dùng tới đều có trong bảng.
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const I18n = require('./i18n.js');

const APP = path.join(__dirname, '..');
const STRINGS = path.join(__dirname, 'strings');
const tables = fs
  .readdirSync(STRINGS)
  .filter((f) => f.endsWith('.js'))
  .map((f) => ({ file: f, table: require(path.join(STRINGS, f)) }));

const vars = (s) => [...String(s).matchAll(/\{(\w+)\}/g)].map((m) => m[1]).sort();

test('mỗi bảng chữ có đủ 3 ngôn ngữ, cùng khoá, cùng biến', () => {
  assert.ok(tables.length > 0);
  for (const { file, table } of tables) {
    const keys = Object.keys(table.vi || {});
    assert.ok(keys.length > 0, file);
    for (const l of I18n.LANGS) {
      assert.deepEqual(Object.keys(table[l] || {}).sort(), [...keys].sort(), `${file}: khoá ${l} lệch với vi`);
      for (const k of keys) {
        const s = table[l][k];
        assert.ok(typeof s === 'string' && s.trim() !== '', `${file}: ${l}.${k} rỗng`);
        assert.deepEqual(vars(s), vars(table.vi[k]), `${file}: ${l}.${k} khác biến với vi`);
      }
    }
  }
});

test('không khoá nào bị khai hai lần ở hai bảng', () => {
  const seen = new Map();
  for (const { file, table } of tables) {
    for (const k of Object.keys(table.vi)) {
      assert.ok(!seen.has(k), `${k} có ở cả ${seen.get(k)} và ${file}`);
      seen.set(k, file);
    }
  }
});

function walk(dir, out = []) {
  for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
    const p = path.join(dir, e.name);
    if (e.isDirectory()) {
      if (!['lib', 'live2d', 'audio', 'node_modules'].includes(e.name)) walk(p, out);
    } else if (/\.(html|js)$/.test(e.name) && !e.name.endsWith('.test.js') && e.name !== 'i18n.js') out.push(p);
  }
  return out;
}

test('mọi khoá giao diện dùng tới đều có trong bảng', () => {
  const known = new Set(tables.flatMap(({ table }) => Object.keys(table.vi)));
  const missing = [];
  for (const file of walk(APP)) {
    const src = fs.readFileSync(file, 'utf8');
    const re = /data-i18n(?:-title|-placeholder|-aria-label)?="([^"]+)"|I18n\.t\(\s*'([^']+)'/g;
    for (const m of src.matchAll(re)) {
      const key = m[1] || m[2];
      if (!known.has(key)) missing.push(`${path.relative(APP, file)}: ${key}`);
    }
  }
  assert.deepEqual(missing, []);
});

test('t(): ngôn ngữ hiện tại, rơi về tiếng Việt, điền biến', () => {
  I18n.add({ vi: { 'x.a': 'Còn {n} việc', 'x.b': 'Chỉ vi' }, ja: { 'x.a': '残り{n}件' }, en: { 'x.a': '{n} left' } });
  I18n.setLang('ja');
  assert.equal(I18n.t('x.a', { n: 2 }), '残り2件');
  assert.equal(I18n.t('x.b'), 'Chỉ vi');
  assert.equal(I18n.t('x.none'), 'x.none');
  I18n.setLang('xx');
  assert.equal(I18n.lang(), 'vi');
  assert.equal(I18n.t('x.a', {}), 'Còn {n} việc');
});
