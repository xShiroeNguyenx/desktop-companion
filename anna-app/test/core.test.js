import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
  applyTaskTags,
  chatSystemPrompt,
  parseReply,
  petMenuItems,
  replyPrompt,
  streamingText,
  translatePrompt,
  trimHistory,
  uiLang,
  unwrap,
} from '../bundle/core.js';
import { STRINGS } from '../bundle/i18n.js';
import {
  award,
  bondLevel,
  fmtClock,
  focusLeft,
  focusStart,
  isUnlocked,
  modelCost,
  newProgress,
  normalizePayload,
  pageQuery,
  pendingTab,
  restoreSize,
  withLanguageHint,
  pickLine,
  unlockModel,
} from '../bundle/core.js';

test('award pays events and respects the daily cap', () => {
  const day1 = new Date(2026, 9, 29, 10);
  let p = newProgress();
  for (let i = 0; i < 25; i++) p = award(p, 'chat', day1).progress;
  assert.equal(p.candy, 20);
  assert.equal(p.counts.chat, 20);
  const next = award(p, 'chat', new Date(2026, 9, 30, 9));
  assert.deepEqual(next.gained, { candy: 1, bond: 1 });
  assert.equal(next.progress.counts.chat, 1);
  assert.deepEqual(award(p, 'nope', day1).gained, { candy: 0, bond: 0 });
});

test('login streak counts consecutive days', () => {
  let p = award(newProgress(), 'login', new Date(2026, 9, 28)).progress;
  p = award(p, 'login', new Date(2026, 9, 29)).progress;
  assert.equal(p.streak, 2);
  p = award(p, 'login', new Date(2026, 9, 31)).progress;
  assert.equal(p.streak, 1);
});

test('bondLevel walks the thresholds', () => {
  assert.deepEqual(bondLevel(0), { level: 1, into: 0, need: 25 });
  assert.deepEqual(bondLevel(30), { level: 2, into: 5, need: 55 });
  assert.equal(bondLevel(9999).level, 5);
  assert.equal(bondLevel(9999).need, 0);
});

test('models unlock with candy, free ones are owned', () => {
  assert.equal(modelCost('gothic-pastel'), 0);
  assert.ok(isUnlocked(newProgress(), 'snow-dancer'));
  assert.equal(unlockModel({ ...newProgress(), candy: 10 }, 'pink-angel', 30).error, 'candy');
  const ok = unlockModel({ ...newProgress(), candy: 40 }, 'pink-angel', 30).progress;
  assert.equal(ok.candy, 10);
  assert.ok(isUnlocked(ok, 'pink-angel'));
  assert.equal(unlockModel(ok, 'pink-angel', 30).error, 'owned');
});

test('focus timer counts down and formats', () => {
  const s = focusStart('work', 1000);
  assert.equal(focusLeft(s, 1000), 25 * 60_000);
  assert.equal(focusLeft(s, 1000 + 26 * 60_000), 0);
  assert.equal(focusLeft({ ...s, running: false }, 1000), 0);
  assert.equal(fmtClock(61_000), '01:01');
  assert.equal(fmtClock(500), '00:01');
});

test('normalizePayload accepts loose shapes from the assistant', () => {
  assert.equal(normalizePayload(null), null);
  assert.deepEqual(normalizePayload('hello'), { mode: 'chat', text: 'hello', target: '', intent: '', tone: '', autorun: true });
  const tr = normalizePayload({ mode: 'Translate', text: ' こんにちは ', target: 'EN' });
  assert.equal(tr.mode, 'translate');
  assert.equal(tr.text, 'こんにちは');
  assert.equal(tr.target, 'en');
  assert.equal(normalizePayload({ mode: 'games' }).mode, 'games');
  assert.equal(normalizePayload({ mode: 'hack' }), null);
  assert.equal(normalizePayload({ target: 'xx', text: 'hi' }).target, '');
});

test('pickLine avoids repeating the last line', () => {
  assert.equal(pickLine(['a', 'b'], () => 0, 'a'), 'b');
  assert.equal(pickLine([], () => 0), '');
});

test('every UI language has the same keys', () => {
  const keys = Object.keys(STRINGS.en).sort();
  for (const lang of ['vi', 'ja']) assert.deepEqual(Object.keys(STRINGS[lang]).sort(), keys, lang);
});

test('uiLang maps Anna locales to supported UI languages', () => {
  assert.equal(uiLang('vi-VN'), 'vi');
  assert.equal(uiLang('ja'), 'ja');
  assert.equal(uiLang('zh-CN'), 'en');
  assert.equal(uiLang(undefined), 'en');
});

test('unwrap handles both response shapes', () => {
  assert.equal(unwrap({ ok: true, result: 5 }), 5);
  assert.deepEqual(unwrap({ value: 1 }), { value: 1 });
  assert.throws(() => unwrap({ ok: false, error: { code: 'permission_denied', message: 'no' } }), (e) => e.code === 'permission_denied');
});

test('parseReply strips emotion tags and collects tasks / done ids', () => {
  const r = parseReply('[happy] Got it! [task: send mockups | due: 2026-10-30] [shy] [done: #4821] Bye');
  assert.equal(r.text, 'Got it! Bye');
  assert.deepEqual(r.emotions, ['happy', 'shy']);
  assert.deepEqual(r.tasks, ['send mockups']);
  assert.deepEqual(r.done, ['4821']);
});

test('parseReply keeps unknown bracket text', () => {
  assert.equal(parseReply('see [link] here').text, 'see [link] here');
});

test('streamingText hides a half-written tag', () => {
  assert.equal(streamingText('[happy] Hello [ta'), 'Hello');
  assert.equal(streamingText('[hap'), '');
});

test('trimHistory keeps the last N turns', () => {
  const h = Array.from({ length: 30 }, (_, i) => ({ role: i % 2 ? 'assistant' : 'user', content: String(i) }));
  const t = trimHistory(h, 10);
  assert.equal(t.length, 20);
  assert.equal(t[0].content, '10');
});

test('applyTaskTags adds, completes and de-duplicates', () => {
  const start = [{ id: '1111', text: 'Pay rent', done: false }];
  let seq = 0;
  const rand = () => [0.5, 0.6][seq++ % 2];
  const added = applyTaskTags(start, { tasks: ['pay rent', 'Buy milk'], done: [] }, rand);
  assert.equal(added.length, 2);
  assert.equal(added[1].text, 'Buy milk');
  assert.notEqual(added[1].id, '1111');
  const finished = applyTaskTags(added, { tasks: [], done: ['1111'] }, rand);
  assert.equal(finished[0].done, true);
  assert.equal(start[0].done, false);
});

test('prompts carry the right instructions', () => {
  const sys = chatSystemPrompt({ petName: 'Miu', tasks: [{ id: '42', text: 'Call mom', done: false }] });
  assert.match(sys, /"Miu"/);
  assert.match(sys, /\[#42\] Call mom/);
  assert.match(translatePrompt('こんにちは', 'en').systemPrompt, /into English/);
  const rp = replyPrompt({ message: 'Hi!', intent: 'say thanks', tone: 'formal' });
  assert.match(rp.systemPrompt, /formal and professional/);
  assert.match(rp.messages[0].content, /say thanks/);
  // The reply language rule closes the message, after the (other-language) note.
  assert.match(rp.messages[0].content, /say thanks[\s\S]*language of the original message\.$/);
});

test('petMenuItems: same ring as the desktop pet, per window', () => {
  const t = (k) => STRINGS.vi[k];
  const models = [
    { id: 'a', name: 'A', preview: 'models/a/preview.png' },
    { id: 'b', name: 'B', preview: 'models/b/preview.png' },
  ];
  const main = petMenuItems({ view: 'main', voice: true, models, current: 'b', t });
  const actions = (items) => items.flatMap((i) => (i.items ? actions(i.items) : [i.action]));
  assert.deepEqual(actions(main).filter((a) => a.startsWith('tab:')).sort(), [
    'tab:calendar', 'tab:chat', 'tab:collection', 'tab:focus', 'tab:games', 'tab:notes', 'tab:reply', 'tab:tasks', 'tab:translate',
  ]);
  assert.ok(actions(main).includes('voice'));
  assert.equal(main.find((i) => i.action === 'voice').icon, '🔊');
  assert.ok(!actions(main).includes('open'));
  const model = main.find((i) => i.id === 'model');
  assert.deepEqual(model.items.map((i) => [i.action, !!i.active]), [['model:a', false], ['model:b', true]]);
  assert.equal(model.items[0].img, 'models/a/preview.png');
  // Labels drop the tab emoji and every entry has a label in this language.
  assert.equal(main[0].label, bareLabel(STRINGS.vi.tabChat));
  for (const i of [...main, ...main.flatMap((x) => x.items || [])]) assert.ok(i.label, JSON.stringify(i));

  const pet = petMenuItems({ view: 'pet', models, current: 'a', t });
  assert.equal(pet[0].action, 'open');
  assert.ok(!actions(pet).includes('voice'), 'the floating pet has no voice');
  assert.ok(pet.length <= 8, 'the ring fits a small window');
});

function bareLabel(s) {
  return s.replace(/^\S+\s+/, '');
}

test('pageQuery keeps Anna\'s window keys across game pages, with the latest token', () => {
  const anna = '?wid=w1&t=old&theme=light&themeMode=light&lang=en-US';
  const toGame = new URLSearchParams(pageQuery(anna + '&view=main', { glang: 'vi', game: 'rps', back: '440x720' }, 'fresh'));
  assert.equal(toGame.get('wid'), 'w1');
  assert.equal(toGame.get('t'), 'fresh');
  assert.equal(toGame.get('lang'), 'en-US');
  assert.equal(toGame.get('theme'), 'light');
  assert.equal(toGame.get('glang'), 'vi');
  assert.equal(toGame.get('view'), null);
  const back = new URLSearchParams(pageQuery('?' + toGame + '&tab=games&wins=2', { view: 'main' }));
  assert.deepEqual([...back.keys()].sort(), ['lang', 't', 'theme', 'themeMode', 'view', 'wid']);
  assert.equal(back.get('t'), 'fresh');
  assert.equal(pageQuery(anna, {}, ''), 'wid=w1&t=old&theme=light&themeMode=light&lang=en-US');
});

test('restoreSize gives back the outer size, adding the title bar the game window measured', () => {
  const WIDE = { w: 720, h: 580 };
  // Anna: 720×580 outer showed a 718×542 page → 2×38 of chrome.
  assert.deepEqual(restoreSize('438x682', WIDE, { w: 718, h: 542 }), { w: 440, h: 720 });
  assert.equal(restoreSize('', WIDE, { w: 718, h: 542 }), null);
  assert.equal(restoreSize('junk', WIDE, { w: 718, h: 542 }), null);
  // A game window the user made bigger never shrinks the result.
  assert.deepEqual(restoreSize('438x682', WIDE, { w: 900, h: 700 }), { w: 438, h: 682 });
});

test('withLanguageHint adds the reminder to a copy of the last user turn only', () => {
  const history = [
    { role: 'user', content: 'Chào' },
    { role: 'assistant', content: 'Chào bạn!' },
    { role: 'user', content: 'How are you?' },
  ];
  const sent = withLanguageHint(history);
  assert.equal(sent[2].content, 'How are you?\n\n(Reply in the language of this message.)');
  assert.equal(sent[0].content, 'Chào');
  assert.equal(history[2].content, 'How are you?');
  assert.deepEqual(withLanguageHint([{ role: 'assistant', content: 'x' }]), [{ role: 'assistant', content: 'x' }]);
  assert.deepEqual(withLanguageHint(null), []);
});

test('pendingTab takes a fresh pet request only', () => {
  const now = 1_000_000;
  assert.equal(pendingTab({ tab: 'tasks', at: now - 2000 }, now), 'tasks');
  assert.equal(pendingTab({ tab: 'tasks', at: now - 60_000 }, now), null);
  assert.equal(pendingTab('notes', now), 'notes');
  assert.equal(pendingTab(null, now), null);
  assert.equal(pendingTab({ at: now }, now), null);
});
