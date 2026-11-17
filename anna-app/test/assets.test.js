import { test } from 'node:test';
import assert from 'node:assert/strict';
import { seriesOrder } from '../scripts/sync-assets.mjs';

test('models sort by line, then purple → pink → snow, like the desktop Collection', () => {
  const ids = ['snow-dancer', 'pink-dream', 'pink-angel', 'snow-pastel', 'gothic-lolita', 'pink-kawaii', 'gothic-aristocrat', 'gothic-kawaii', 'custom-x'];
  assert.deepEqual(ids.sort(seriesOrder), [
    'gothic-kawaii', 'pink-kawaii', 'snow-pastel', 'pink-dream', 'gothic-aristocrat', 'pink-angel', 'gothic-lolita', 'snow-dancer', 'custom-x',
  ]);
});

test('desktop game pages load the Anna shim, with no inline script or style (Anna CSP)', async () => {
  const { readFileSync } = await import('node:fs');
  const { deskPage } = await import('../scripts/sync-assets.mjs');
  for (const name of ['game', 'guess']) {
    const { html, css } = deskPage(readFileSync(new URL(`../../app/${name}.html`, import.meta.url), 'utf8'), name);
    assert.ok(html.includes('<script src="../desk-shim.js"></script>'), name);
    // desk-fit.js decides the layout before the shim applies it; the portrait rules come after the page's own.
    assert.ok(html.indexOf('<script src="../desk-fit.js"></script>') > 0 && html.indexOf('../desk-fit.js') < html.indexOf('../desk-shim.js'), name);
    assert.ok(html.indexOf('../desk-narrow.css') > html.indexOf(`href="${name}.css"`), name);
    assert.ok(html.includes('<script type="module" src="../desk-host.js"></script>'), name);
    assert.ok(html.includes(`<link rel="stylesheet" href="${name}.css" />`), name);
    assert.ok(!/src="js\/(tauri-api|theme-apply)\.js"|href="theme-pages\.css"|<style/.test(html), name);
    assert.ok(css.includes('zoom') && css.length > 1000, name);
  }
  assert.throws(() => deskPage('<html></html>', 'x'), /tauri-api/);
});

test('the bundle links to no outside software or site (Anna review)', async () => {
  const { readFileSync, readdirSync } = await import('node:fs');
  const dir = new URL('../bundle/', import.meta.url);
  for (const name of readdirSync(dir).filter((f) => /\.(html|js)$/.test(f))) {
    const src = readFileSync(new URL(name, dir), 'utf8');
    assert.ok(!/href=["']https?:|window\.open\(|target=["']_blank/.test(src), name);
    assert.ok(!/techdecoded|github\.com|for Windows/i.test(src), name);
  }
});

test('[hidden] beats the flex layouts, so a closed note editor cannot be typed into', async () => {
  const { readFileSync } = await import('node:fs');
  const css = readFileSync(new URL('../bundle/app.css', import.meta.url), 'utf8');
  assert.match(css, /^\[hidden\] \{ display: none !important; \}$/m);
});

test('every view opens at its own size: no declared size trips Anna\'s open-fitted rule', async () => {
  // Anna's window manager (anna-app-window-manager.js, read 2026-11-17) opens a window
  // filling the whole work area when its declared size is >= 80% of the work area on
  // either axis. Work areas under 1280x720 are fitted whatever we declare, so 1280x720
  // is the smallest one a manifest can still be right for.
  const { readFileSync } = await import('node:fs');
  const manifest = JSON.parse(readFileSync(new URL('../manifest.json', import.meta.url), 'utf8'));
  for (const view of manifest.ui.views) {
    assert.ok(view.default_size.w < 1280 * 0.8, `${view.name} width ${view.default_size.w}`);
    assert.ok(view.default_size.h < 720 * 0.8, `${view.name} height ${view.default_size.h}`);
    assert.ok(view.default_size.h >= view.min_size.h && view.default_size.w >= view.min_size.w, `${view.name} below its minimum`);
  }
});

test('deskFit: the portrait canvas in the chat panel, the desktop canvas in a wide window', async () => {
  await import('../bundle/desk-fit.js');
  const { deskFit } = globalThis;
  // The chat panel on Anna: a 438×532 page.
  const panel = deskFit(438, 532);
  assert.equal(panel.narrow, true);
  assert.ok(Math.abs(panel.zoom - 0.95) < 0.001, String(panel.zoom));
  // Nearly full size, where the landscape canvas would be shrunk to half.
  assert.ok(panel.zoom > 0.9 && 438 / 900 < 0.5);
  // A wide window keeps the desktop layout.
  assert.deepEqual(deskFit(900, 560), { narrow: false, zoom: 1 });
  assert.equal(deskFit(718, 542).narrow, false);
  // Whatever the window, the canvas fits both ways and never collapses.
  for (const [w, h] of [[438, 532], [300, 700], [1414, 699], [100, 100]]) {
    const f = deskFit(w, h);
    const canvas = f.narrow ? { w: 450, h: 560 } : { w: 900, h: 560 };
    assert.ok(f.zoom >= 0.3, `${w}x${h}`);
    if (f.zoom > 0.3) assert.ok(canvas.w * f.zoom <= w + 1e-9 && canvas.h * f.zoom <= h + 1e-9, `${w}x${h}`);
  }
});
