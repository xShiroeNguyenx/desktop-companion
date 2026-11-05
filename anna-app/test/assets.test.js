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
