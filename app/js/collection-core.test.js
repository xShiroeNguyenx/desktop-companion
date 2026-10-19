// node --test app/js/  (npm test)
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const C = require('./collection-core.js');

const item = (over) => ({ id: 'skin.x', cat: 'skin', rarity: 'common', price: 60, owned: false, selected: false, on: false, asset: 'x', ...over });

test('mỗi ngôn ngữ có đủ chữ như tiếng Việt', () => {
  const flat = (o, p = '') => Object.entries(o).flatMap(([k, v]) => (typeof v === 'object' ? flat(v, p + k + '.') : [p + k]));
  const keys = flat(C.T.vi).sort();
  for (const l of ['ja', 'en']) assert.deepEqual(flat(C.T[l]).sort(), keys, l);
  for (const cat of C.CATS) for (const l of ['vi', 'ja', 'en']) assert.ok(C.T[l].tabs[cat], `${l}.tabs.${cat}`);
});

test('gom theo nhóm theo thứ tự tab, bỏ nhóm trống', () => {
  const items = [item({ id: 'fx.a', cat: 'effect' }), item({ id: 'skin.a' }), item({ id: 'miu.a', cat: 'miu_set' })];
  assert.deepEqual(C.groupByCategory(items).map(([c, l]) => [c, l.length]), [['skin', 1], ['effect', 1], ['miu_set', 1]]);
  assert.deepEqual(C.groupByCategory([]), []);
});

test('sắp xếp: đang dùng → đã có → rẻ trước', () => {
  const items = [
    item({ id: 'skin.c', price: 150 }),
    item({ id: 'skin.b', price: 60, owned: true }),
    item({ id: 'skin.a', price: 60 }),
    item({ id: 'skin.d', price: 0, owned: true, selected: true }),
  ];
  assert.deepEqual(C.sortItems(items).map((i) => i.id), ['skin.d', 'skin.b', 'skin.a', 'skin.c']);
  assert.equal(items[0].id, 'skin.c', 'không đổi mảng vào');
});

test('trạng thái nút', () => {
  assert.deepEqual(C.buttonState(item({ price: 60 }), 20, 'vi'), { kind: 'short', label: '🍬 Thiếu 40', disabled: true });
  assert.deepEqual(C.buttonState(item({ price: 60 }), 60, 'en'), { kind: 'buy', label: 'Trade 60 🍬', disabled: false });
  assert.equal(C.buttonState(item({ owned: true }), 0, 'vi').kind, 'use');
  assert.equal(C.buttonState(item({ owned: true, selected: true }), 0, 'vi').kind, 'using');
  assert.equal(C.buttonState(item({ cat: 'effect', owned: true }), 0, 'vi').kind, 'off');
  assert.equal(C.buttonState(item({ cat: 'effect', owned: true, on: true }), 0, 'ja').label, 'オン');
  assert.equal(C.buttonState(item({ cat: 'line_pack', owned: true, price: 0 }), 0, 'vi').label, 'Có sẵn');
});

test('mã lỗi Rust → câu đọc được', () => {
  assert.equal(C.errorText('not_enough:12', 'vi'), 'Còn thiếu 12 🍬 nữa.');
  assert.equal(C.errorText('not_owned', 'en'), "You don't have this item yet.");
  assert.equal(C.errorText('already_owned', 'ja'), 'もう持ってるよ～');
  assert.equal(C.errorText('boom', 'vi'), 'Không làm được: boom');
});

test('ô xem trước: mọi skin trong skins.css có swatch, ảnh đại diện đúng chỗ', () => {
  const css = fs.readFileSync(path.join(__dirname, '..', 'skins.css'), 'utf8');
  const skins = [...css.matchAll(/data-skin="([a-z]+)"/g)].map((m) => m[1]);
  assert.ok(skins.length >= 6);
  // Skin nào ngoài Oải hương cũng phải có màu xem trước riêng (không rơi về mặc định).
  for (const s of skins.filter((x) => x !== 'lavender')) assert.notEqual(C.swatch(s), C.swatch('lavender'), s);
  for (const s of skins) assert.ok(C.swatch(s).startsWith('linear-gradient('), s);
  assert.equal(C.swatchInk('sakura'), '#5b2f47');
  assert.equal(C.swatchInk('night'), '#fff');
  for (const sk of skins) assert.ok(/^#[0-9a-f]{6}$/.test(C.swatchAccent(sk)), sk);
  assert.equal(C.swatchAccent('mint'), '#2fb58a');
  assert.equal(C.thumb(item({ cat: 'side_set', asset: 'bunny' })), 'media/side/settings-girl.png');
  assert.equal(C.thumb(item({ cat: 'side_set', asset: 'snow', thumb: 'media/side/snow/ice.webp' })), 'media/side/snow/ice.webp');
  assert.equal(C.thumb(item({ cat: 'side_set', asset: 'neko' })), '');
  assert.equal(C.thumb(item({ cat: 'miu_set', asset: 'media/game' })), 'media/game/rps-cheer.png');
  assert.equal(C.thumb(item({ cat: 'skin' })), '');
  assert.equal(C.effectIcon('fireworks'), '🎆');
  assert.equal(C.rarityClass('epic'), 'r-epic');
  assert.equal(C.rarityClass('zzz'), 'r-common');
});

test('nút quay: đủ kẹo / thiếu / máy khoá', () => {
  assert.deepEqual(C.pullState(100, 250, true, 'vi'), { ok: true, label: '' });
  assert.deepEqual(C.pullState(1000, 250, true, 'vi'), { ok: false, label: 'Thiếu 750 🍬' });
  assert.equal(C.pullState(100, 9999, false, 'en').ok, false);
});

test('tóm tắt kết quả quay', () => {
  const res = [
    { rarity: 'common', dup: true, refund: 60 },
    { rarity: 'epic', dup: false, refund: 0 },
    { rarity: 'rare', dup: true, refund: 150 },
  ];
  assert.deepEqual(C.summarize(res), { fresh: 1, refund: 210, best: 'epic' });
  assert.deepEqual(C.summarize([]), { fresh: 0, refund: 0, best: 'common' });
});

test('món nổi bật: chưa có trước, hiếm trước, có ảnh trước, bỏ món có sẵn', () => {
  const items = [
    item({ id: 'skin.a', price: 600 }),
    item({ id: 'side.x', cat: 'side_set', rarity: 'epic', price: 3000, owned: true }),
    item({ id: 'fx.a', cat: 'effect', rarity: 'rare', price: 1500 }),
    item({ id: 'miu.a', cat: 'miu_set', rarity: 'rare', price: 1500 }),
    item({ id: 'skin.free', price: 0 }),
  ];
  assert.deepEqual(C.featured(items, 3).map((i) => i.id), ['miu.a', 'fx.a', 'skin.a']);
  assert.equal(C.featured(items).length, 4);
});

test('giờ trong lịch sử theo ngôn ngữ', () => {
  const t = new Date(2026, 9, 19, 8, 5).getTime();
  assert.equal(C.formatTime(t, 'vi'), '08:05 19/10');
  assert.equal(C.formatTime(t, 'ja'), '10/19 08:05');
});
