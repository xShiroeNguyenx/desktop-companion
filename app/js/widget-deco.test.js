// node --test app/js/  (npm test)
// Trang trí widget chạy mãi trên 5 cửa sổ WebView2 trong suốt cùng lúc, ở tần số
// quét màn hình (120 Hz trên laptop): đo 2026-11-05, kiểu cũ (đổi `top` mỗi
// khung) làm mỗi widget tốn ~1% CPU liên tục. Test giữ hai quy tắc ở đầu
// widget-deco.css.
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');

const css = fs.readFileSync(path.join(__dirname, '..', 'widget-deco.css'), 'utf8').replace(/\/\*[\s\S]*?\*\//g, '');

function keyframes() {
  const out = {};
  const re = /@keyframes\s+([\w-]+)\s*\{/g;
  let m;
  while ((m = re.exec(css))) {
    let depth = 1;
    let i = re.lastIndex;
    while (depth && i < css.length) {
      if (css[i] === '{') depth++;
      else if (css[i] === '}') depth--;
      i++;
    }
    out[m[1]] = css.slice(re.lastIndex, i - 1);
  }
  return out;
}

test('hoạt cảnh trang trí không đổi thuộc tính bố cục (top / left / width…)', () => {
  const kf = keyframes();
  assert.ok(Object.keys(kf).length >= 3, 'tìm thấy @keyframes');
  for (const [name, body] of Object.entries(kf)) {
    const props = [...body.matchAll(/([\w-]+)\s*:/g)].map((x) => x[1]);
    for (const p of props) {
      assert.ok(
        !['top', 'left', 'right', 'bottom', 'width', 'height', 'margin', 'padding', 'font-size'].includes(p),
        `@keyframes ${name} đổi "${p}": phải tính lại bố cục mỗi khung, dùng transform`,
      );
    }
  }
});

test('mọi hoạt cảnh chạy mãi đều hạ nhịp bằng steps()', () => {
  const kf = keyframes();
  const decls = [...css.matchAll(/animation\s*:\s*([^;]+);/g)].map((x) => x[1]).filter((v) => !/none/.test(v));
  assert.ok(decls.length > 0);
  for (const decl of decls) {
    for (const part of decl.split(',')) {
      if (!/infinite/.test(part)) continue;
      const name = part.trim().split(/\s+/).find((w) => kf[w]);
      assert.ok(name, `không thấy @keyframes cho "${part.trim()}"`);
      // steps() trên phần tử, hoặc trong từng keyframe (đoạn dài ngắn khác nhau).
      const stepped = /steps\(/.test(part) || /animation-timing-function\s*:\s*steps\(/.test(kf[name]);
      assert.ok(stepped, `"${part.trim()}" chạy mãi mà không có steps(): vẽ theo tần số quét màn hình`);
    }
  }
});
