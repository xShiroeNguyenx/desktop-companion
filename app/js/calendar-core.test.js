// node --test app/js/  (npm test)
const test = require('node:test');
const assert = require('node:assert/strict');
const C = require('./calendar-core.js');

const lunar = (d, m, y) => {
  const l = C.toLunar(d, m, y);
  return `${l.day}/${l.month}${l.leap ? 'N' : ''}/${l.year}`;
};

test('mỗi ngôn ngữ có đủ chữ như tiếng Việt', () => {
  const keys = Object.keys(C.T.vi).sort();
  const hol = Object.keys(C.T.vi.h).sort();
  for (const l of ['ja', 'en']) {
    assert.deepEqual(Object.keys(C.T[l]).sort(), keys, l);
    assert.deepEqual(Object.keys(C.T[l].h).sort(), hol, l);
  }
  for (const l of ['vi', 'ja', 'en']) {
    assert.equal(C.T[l].weekdays.length, 7);
    assert.equal(C.T[l].weekdaysLong.length, 7);
  }
});

test('Tết Nguyên đán các năm đã biết', () => {
  assert.equal(lunar(22, 1, 2023), '1/1/2023');
  assert.equal(lunar(10, 2, 2024), '1/1/2024');
  assert.equal(lunar(29, 1, 2025), '1/1/2025');
  assert.equal(lunar(17, 2, 2026), '1/1/2026');
  assert.equal(lunar(25, 1, 2020), '1/1/2020');
  assert.equal(lunar(5, 2, 2000), '1/1/2000');
  // Ngày cuối năm âm thuộc năm âm trước, dù đã sang năm dương mới.
  assert.equal(lunar(24, 1, 2020), '30/12/2019');
  assert.equal(lunar(16, 2, 2026), '29/12/2025');
  assert.equal(lunar(1, 1, 2000), '25/11/1999');
});

test('Tết 1985 của Việt Nam (múi +7) sớm hơn Trung Quốc một tháng', () => {
  assert.equal(lunar(21, 1, 1985), '1/1/1985');
  assert.equal(lunar(20, 2, 1985), '1/2/1985');
});

test('tháng nhuận', () => {
  assert.equal(lunar(22, 3, 2023), '1/2N/2023');
  assert.equal(lunar(20, 2, 2023), '1/2/2023');
  assert.equal(lunar(25, 7, 2025), '1/6N/2025');
  assert.equal(lunar(25, 6, 2025), '1/6/2025');
});

test('Trung thu và ngày thường', () => {
  assert.equal(lunar(25, 9, 2026), '15/8/2026');
  assert.equal(lunar(16, 10, 2026), '7/9/2026');
});

test('can chi năm âm', () => {
  assert.equal(C.canChi(2026, 'vi'), 'Bính Ngọ');
  assert.equal(C.canChi(2023, 'vi'), 'Quý Mão');
  assert.equal(C.canChi(2024, 'ja'), '甲辰');
  assert.equal(C.canChi(2023, 'en'), 'Year of the Cat');
  assert.equal(C.canChi(1984, 'vi'), 'Giáp Tý');
});

test('lưới tháng bắt đầu thứ Hai, đủ hàng chứa trọn tháng', () => {
  const oct = C.monthGrid(2026, 10);
  assert.equal(oct.length, 35);
  // 1/10/2026 là thứ Năm: ba ô đầu là 28, 29, 30/9.
  assert.deepEqual(oct.slice(0, 4).map((c) => `${c.day}/${c.month}`), ['28/9', '29/9', '30/9', '1/10']);
  assert.equal(oct[3].weekday, 3);
  assert.equal(oct.filter((c) => c.inMonth).length, 31);
  // Tháng 2/2021 bắt đầu thứ Hai, 28 ngày: vừa 4 hàng.
  assert.equal(C.monthGrid(2021, 2).length, 28);
  // Tháng 8/2021 bắt đầu Chủ nhật, 31 ngày: cần 6 hàng.
  assert.equal(C.monthGrid(2021, 8).length, 42);
  assert.ok(C.monthGrid(2024, 2).some((c) => c.inMonth && c.day === 29));
});

test('chuyển tháng qua năm', () => {
  assert.deepEqual(C.addMonths(2026, 1, -1), { year: 2025, month: 12 });
  assert.deepEqual(C.addMonths(2026, 12, 1), { year: 2027, month: 1 });
  assert.deepEqual(C.addMonths(2026, 10, -22), { year: 2024, month: 12 });
});

test('ngày lễ dương, âm, Giao thừa; tháng nhuận không có lễ', () => {
  const ids = (y, m, d) => C.holidays(y, m, d).map((h) => h.id);
  assert.deepEqual(ids(2026, 9, 2), ['national']);
  assert.deepEqual(ids(2026, 9, 25), ['midAutumn']);
  assert.deepEqual(ids(2026, 2, 17), ['tet1']);
  // Năm 2026 tháng Chạp thiếu: Giao thừa là 29 Chạp.
  assert.deepEqual(ids(2026, 2, 16), ['newYearEve']);
  // Năm 2020 tháng Chạp đủ: 29 Chạp chưa phải Giao thừa, 30 mới là.
  assert.deepEqual(ids(2020, 1, 23), []);
  assert.deepEqual(ids(2020, 1, 24), ['newYearEve']);
  // Rằm tháng 6 nhuận 2025 không phải lễ gì.
  assert.deepEqual(ids(2025, 8, 8), []);
  assert.equal(C.holidays(2026, 4, 30)[0].off, true);
  assert.equal(C.holidays(2026, 2, 14)[0].off, false);
});

test('task có hạn gom theo ngày địa phương, bỏ việc xong / không hạn', () => {
  const at = (y, m, d, h) => String(new Date(y, m - 1, d, h, 0).getTime());
  const now = new Date(2026, 9, 16, 12, 0).getTime();
  const tasks = [
    { id: '1', text: 'b', done: false, due_at: at(2026, 10, 16, 17) },
    { id: '2', text: 'a', done: false, due_at: at(2026, 10, 16, 9) },
    { id: '3', text: 'xong', done: true, due_at: at(2026, 10, 16, 10) },
    { id: '4', text: 'không hạn', done: false, due_at: '' },
    { id: '5', text: 'sau', done: false, due_at: at(2026, 10, 20, 8) },
    { id: '6', text: 'hỏng', done: false, due_at: 'abc' },
  ];
  const by = C.tasksByDay(tasks, now);
  assert.deepEqual(Object.keys(by).sort(), ['2026-10-16', '2026-10-20']);
  assert.deepEqual(by['2026-10-16'].map((t) => t.id), ['2', '1']);
  assert.equal(by['2026-10-16'][0].over, true);
  assert.equal(by['2026-10-16'][1].over, false);
  assert.deepEqual(C.tasksByDay(null, now), {});
});

test('hạn chót từ ngày + giờ', () => {
  assert.equal(C.dueAt('2026-10-16', '09:30'), new Date(2026, 9, 16, 9, 30).getTime());
  assert.equal(C.dueAt('2026-10-16', '24:00'), null);
  assert.equal(C.dueAt('16/10/2026', '09:30'), null);
  assert.equal(C.dueAt('2026-10-16', ''), null);
});

test('tiêu đề tháng và chữ âm lịch trong ô', () => {
  assert.equal(C.monthTitle(2026, 10, 'vi'), 'Tháng 10, 2026');
  assert.equal(C.monthTitle(2026, 10, 'ja'), '2026年10月');
  assert.equal(C.monthTitle(2026, 10, 'en'), 'October 2026');
  assert.equal(C.lunarLabel({ day: 1, month: 9, leap: false }, 'vi'), '1/9');
  assert.equal(C.lunarLabel({ day: 1, month: 6, leap: true }, 'vi'), '1/6*');
  assert.equal(C.lunarLabel({ day: 15, month: 8, leap: false }, 'vi'), 'Rằm');
  assert.equal(C.lunarLabel({ day: 7, month: 9, leap: false }, 'en'), '7');
});
