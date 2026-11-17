import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
  applyTaskTags,
  chatSystemPrompt,
  easeToRest,
  FACE_REST,
  parseReply,
  shouldBlink,
  startSize,
  tabDrag,
  voiceDenied,
  focusFinish,
  spokenText,
  pickLang,
  UI_LANGS,
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

  // Language ring: "same as Anna" plus each language under its own name; the pick is marked.
  const langs = (items) => items.find((i) => i.id === 'lang').items.map((i) => [i.action, i.label, !!i.active]);
  assert.deepEqual(langs(main), [
    ['lang:auto', STRINGS.vi.langAuto, true],
    ['lang:en', 'English', false],
    ['lang:vi', 'Tiếng Việt', false],
    ['lang:ja', '日本語', false],
  ]);
  assert.deepEqual(
    langs(petMenuItems({ view: 'main', models, lang: 'ja', t })).filter((l) => l[2]).map((l) => l[0]),
    ['lang:ja'],
  );

  const pet = petMenuItems({ view: 'pet', models, current: 'a', t });
  assert.ok(pet.some((i) => i.id === 'lang'), 'the floating pet can change language too');
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

test('easeToRest brings a face parameter back to neutral at any frame rate', () => {
  const settle = (start, rest, fps, seconds = 1) => {
    let v = start;
    for (let i = 0; i < fps * seconds; i++) v = easeToRest(v, rest, 1000 / fps);
    return v;
  };
  // The leftovers measured after a "happy" reaction: smile 0.38, eyes 0.68 open.
  for (const fps of [15, 30, 60, 144]) {
    assert.equal(settle(0.38, 0, fps), 0, `smile at ${fps} fps`);
    assert.equal(settle(0.68, 1, fps), 1, `eyes at ${fps} fps`);
    assert.equal(settle(-0.13, 0, fps), 0, `frown at ${fps} fps`);
  }
  // Same distance covered in the same time, whatever the frame rate.
  assert.ok(Math.abs(settle(1, 0, 30, 0.2) - settle(1, 0, 120, 0.2)) < 1e-9);
  // Eases, never jumps or overshoots.
  const step = easeToRest(1, 0, 1000 / 60);
  assert.ok(step > 0.8 && step < 1);
  assert.equal(easeToRest(0.5, 0, 0), 0.5);
  assert.equal(easeToRest(NaN, 1, 16), 1);
});

test('FACE_REST is the neutral face: eyes open, everything else at zero', () => {
  for (const [id, rest] of Object.entries(FACE_REST)) {
    assert.equal(rest, /Eye[LR]Open$/.test(id) ? 1 : 0, id);
  }
  // The mouth opening belongs to lip-sync, the eyeballs to the idle motion.
  assert.ok(!('ParamMouthOpenY' in FACE_REST) && !('ParamEyeBallX' in FACE_REST));
});

test('shouldBlink: always at rest, and a reaction only lets a started blink finish', () => {
  for (const state of [0, 1, 2, 3, 4]) assert.equal(shouldBlink(false, state), true, `rest, state ${state}`);
  assert.equal(shouldBlink(true, 0), false);
  assert.equal(shouldBlink(true, 1), false);
  for (const state of [2, 3, 4]) assert.equal(shouldBlink(true, state), true, `reacting, state ${state}`);
  assert.equal(shouldBlink(true, undefined), false);
});

test('startSize shrinks a panel Anna opened across the workspace, once', () => {
  const declared = { w: 440, h: 720 };
  // Measured on Anna: the 440×720 panel opens with a 1414×699 page.
  assert.deepEqual(startSize({ w: 1414, h: 699 }, declared, false), { w: 440, h: 720 });
  // Already at (or near) the declared size: leave it.
  assert.equal(startSize({ w: 438, h: 682 }, declared, false), null);
  assert.equal(startSize({ w: 600, h: 800 }, declared, false), null);
  // A page switch inside the same window must not undo the user's own resize.
  assert.equal(startSize({ w: 1414, h: 699 }, declared, true), null);
  // Hosts that give no declared size.
  assert.equal(startSize({ w: 1414, h: 699 }, null, false), null);
  assert.equal(startSize({ w: 1414, h: 699 }, { w: 0, h: 0 }, false), null);
});

test('tabDrag scrolls the strip against the mouse and tells a drag from a click', () => {
  const start = { x: 200, left: 40 };
  // Mouse goes left: the strip shows what is further right.
  assert.deepEqual(tabDrag(start, 120), { scrollLeft: 120, moved: true });
  // Mouse goes right: back toward the first tab, never past it.
  assert.deepEqual(tabDrag(start, 230), { scrollLeft: 10, moved: true });
  assert.deepEqual(tabDrag(start, 400), { scrollLeft: 0, moved: true });
  // A shaky click is still a click.
  assert.equal(tabDrag(start, 203).moved, false);
  assert.equal(tabDrag(start, 195).moved, false);
});

test('pickLang: the language picked in the app wins, otherwise Anna decides', () => {
  assert.equal(pickLang('vi', 'en-US'), 'vi');
  assert.equal(pickLang('ja', 'vi-VN'), 'ja');
  assert.equal(pickLang('en', 'ja'), 'en');
  // Nothing picked (or a value we do not know): follow Anna, as before.
  assert.equal(pickLang('', 'vi-VN'), 'vi');
  assert.equal(pickLang(undefined, 'ja-JP'), 'ja');
  assert.equal(pickLang('auto', 'zh-CN'), 'en');
  assert.equal(pickLang('fr', 'vi'), 'vi');
  assert.equal(pickLang('toString', 'vi'), 'vi');
  // Every language offered has its own strings.
  for (const id of Object.keys(UI_LANGS)) assert.ok(STRINGS[id] && STRINGS[id].menuLang && STRINGS[id].langAuto, id);
});

test('voiceDenied tells "voice not granted" from a passing error', () => {
  assert.equal(voiceDenied({ code: 'CAPABILITY_NOT_GRANTED', message: 'audio.speak' }), true);
  assert.equal(voiceDenied({ message: 'Permission denied for audio' }), true);
  assert.equal(voiceDenied({ code: 'TIMEOUT', message: 'request timed out' }), false);
  assert.equal(voiceDenied(new Error('no audio returned')), false);
  assert.equal(voiceDenied(null), false);
});

test('spokenText keeps the words of a bubble line and drops what a voice cannot say', () => {
  assert.equal(spokenText('Yay! Bạn đoán đúng rồi! 🎉'), 'Yay! Bạn đoán đúng rồi!');
  assert.equal(spokenText('Tới lượt bạn! Chọn đi~'), 'Tới lượt bạn! Chọn đi');
  assert.equal(spokenText('Em thắng ván này~ 💕'), 'Em thắng ván này');
  assert.equal(spokenText('Hãy đoán số nào? Em sẽ đưa gợi ý nhé! Cố lên! ♡'), 'Hãy đoán số nào? Em sẽ đưa gợi ý nhé! Cố lên!');
  assert.equal(spokenText('やったー！ ⭐ 正解だよ～'), 'やったー！ 正解だよ');
  assert.equal(spokenText('Số 10 thấp quá rồi, từ 1 đến 20!'), 'Số 10 thấp quá rồi, từ 1 đến 20!');
  assert.equal(spokenText('💕 ♡ ~'), '');
  assert.equal(spokenText(null), '');
});

test('focusFinish ends a timer once, in whichever window is open, and counts sessions per day', () => {
  const now = new Date(2026, 10, 17, 10, 0, 0).getTime();
  const today = '2026-11-17';
  const running = { phase: 'work', running: true, endsAt: now + 60_000, day: today, sessions: 2 };
  // Still counting down, stopped, or nothing saved: nothing to do.
  assert.equal(focusFinish(running, now), null);
  assert.equal(focusFinish({ ...running, running: false }, now + 120_000), null);
  assert.equal(focusFinish(null, now), null);
  // Reached the end: stopped, one more session today.
  const done = focusFinish(running, now + 60_000);
  assert.deepEqual(done, { state: { ...running, running: false, sessions: 3 }, finishedWork: true });
  // The stopped state does not finish a second time (no double candy).
  assert.equal(focusFinish(done.state, now + 61_000), null);
  // A break ending is not a session.
  const rest = focusFinish({ ...running, phase: 'break' }, now + 60_000);
  assert.equal(rest.finishedWork, false);
  assert.equal(rest.state.sessions, 2);
  // The first session of a new day starts the count again.
  const next = focusFinish({ ...running, day: '2026-11-16', sessions: 5 }, now + 60_000);
  assert.deepEqual([next.state.day, next.state.sessions], [today, 1]);
});

test('pendingTab takes a fresh pet request only', () => {
  const now = 1_000_000;
  assert.equal(pendingTab({ tab: 'tasks', at: now - 2000 }, now), 'tasks');
  assert.equal(pendingTab({ tab: 'tasks', at: now - 60_000 }, now), null);
  assert.equal(pendingTab('notes', now), 'notes');
  assert.equal(pendingTab(null, now), null);
  assert.equal(pendingTab({ at: now }, now), null);
});
