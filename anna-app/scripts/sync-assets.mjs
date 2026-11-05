// Copy the Live2D runtime libs and the bundled models into anna-app/bundle/.
// Models come from ../dist-models/<id>.zip, or the GitHub "models" release when
// the zip is not on disk. Output folders are gitignored; run `npm run assets`.
import { copyFileSync, existsSync, mkdirSync, readdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { inflateRawSync } from 'node:zlib';

const here = dirname(fileURLToPath(import.meta.url));
const appRoot = resolve(here, '..');
const repoRoot = resolve(appRoot, '..');
const bundle = join(appRoot, 'bundle');

const LIBS = ['live2dcubismcore.min.js', 'pixi.min.js', 'cubism4.min.js'];
const SHARED_CORES = ['games-core.js', 'mines-core.js', 'calendar-core.js', 'notes-core.js'];
// Same list the desktop app downloads: models/manifest.json (latest version of each model).
const titleCase = (id) => id.split('-').map((w) => w[0].toUpperCase() + w.slice(1)).join(' ');
// Same order as the desktop Collection: by line, then purple (gothic) → pink → snow.
// Ids are "<colour>-<line>"; unknown lines / colours go last, by id.
const LINES = ['kawaii', 'pastel', 'dream', 'aristocrat', 'angel', 'lolita', 'dancer'];
const COLOURS = ['gothic', 'pink', 'snow'];
const rank = (list, v) => (list.includes(v) ? list.indexOf(v) : list.length);
export function seriesOrder(a, b) {
  const [ca, la] = a.split('-');
  const [cb, lb] = b.split('-');
  return rank(LINES, la) - rank(LINES, lb) || rank(COLOURS, ca) - rank(COLOURS, cb) || a.localeCompare(b);
}
export const MODELS = JSON.parse(readFileSync(join(repoRoot, 'models', 'manifest.json'), 'utf8'))
  .models.map((m) => ({ id: m.id, zip: m.file, name: titleCase(m.id) }))
  .sort((a, b) => seriesOrder(a.id, b.id));
const RELEASE = 'https://github.com/xShiroeNguyenx/desktop-companion/releases/download/models/';

/** Minimal zip reader (stored + deflate) via the central directory. */
export function unzip(buf) {
  let eocd = buf.length - 22;
  while (eocd >= 0 && buf.readUInt32LE(eocd) !== 0x06054b50) eocd--;
  if (eocd < 0) throw new Error('not a zip file');
  const count = buf.readUInt16LE(eocd + 10);
  let p = buf.readUInt32LE(eocd + 16);
  const files = [];
  for (let i = 0; i < count; i++) {
    if (buf.readUInt32LE(p) !== 0x02014b50) throw new Error('bad central directory');
    const method = buf.readUInt16LE(p + 10);
    const size = buf.readUInt32LE(p + 20);
    const nameLen = buf.readUInt16LE(p + 28);
    const extraLen = buf.readUInt16LE(p + 30);
    const commentLen = buf.readUInt16LE(p + 32);
    const local = buf.readUInt32LE(p + 42);
    const name = buf.toString('utf8', p + 46, p + 46 + nameLen);
    const dataStart = local + 30 + buf.readUInt16LE(local + 26) + buf.readUInt16LE(local + 28);
    const raw = buf.subarray(dataStart, dataStart + size);
    if (!name.endsWith('/')) {
      if (method !== 0 && method !== 8) throw new Error(`unsupported zip method ${method} for ${name}`);
      files.push({ name, data: method === 8 ? inflateRawSync(raw) : Buffer.from(raw) });
    }
    p += 46 + nameLen + extraLen + commentLen;
  }
  return files;
}

async function loadZip(zip) {
  const local = join(repoRoot, 'dist-models', zip);
  if (existsSync(local)) return readFileSync(local);
  const res = await fetch(RELEASE + zip);
  if (!res.ok) throw new Error(`download ${zip}: HTTP ${res.status}`);
  return Buffer.from(await res.arrayBuffer());
}

async function main() {
  const libDir = join(bundle, 'lib');
  mkdirSync(libDir, { recursive: true });
  for (const lib of LIBS) copyFileSync(join(repoRoot, 'app', 'media', 'lib', lib), join(libDir, lib));
  // PixiJS 6 compiles shaders with `new Function`; Anna's CSP forbids eval, so
  // ship the matching @pixi/unsafe-eval build (auto-installs on globalThis.PIXI).
  copyFileSync(join(appRoot, 'vendor', 'pixi-unsafe-eval.min.js'), join(libDir, 'pixi-unsafe-eval.min.js'));
  // Pure game / calendar / notes rules shared with the desktop app (UMD → window.*Core).
  for (const core of SHARED_CORES) copyFileSync(join(repoRoot, 'app', 'js', core), join(libDir, core));

  const modelsDir = join(bundle, 'models');
  rmSync(modelsDir, { recursive: true, force: true });
  const index = [];
  for (const m of MODELS) {
    const files = unzip(await loadZip(m.zip));
    for (const f of files) {
      if (f.name.includes('..') || f.name.startsWith('/')) throw new Error(`unsafe path in ${m.zip}: ${f.name}`);
      const out = join(modelsDir, m.id, f.name);
      mkdirSync(dirname(out), { recursive: true });
      writeFileSync(out, f.data);
    }
    const entry = files.find((f) => f.name.endsWith('.model3.json'));
    if (!entry) throw new Error(`${m.zip} has no .model3.json`);
    const preview = join(repoRoot, 'app', 'media', 'model-previews', `${m.id}.png`);
    if (existsSync(preview)) copyFileSync(preview, join(modelsDir, m.id, 'preview.png'));
    index.push({
      id: m.id,
      name: m.name,
      url: `models/${m.id}/${entry.name}`,
      preview: existsSync(preview) ? `models/${m.id}/preview.png` : '',
    });
    console.log(`  ✓ ${m.id}: ${files.length} files`);
  }
  writeFileSync(join(modelsDir, 'index.json'), JSON.stringify(index, null, 2) + '\n');
  syncDeskGames();
  console.log(`✓ libs + ${index.length} models + desktop games → bundle/`);
}

/**
 * The desktop Rock paper scissors / Guess the number pages, as they are (Miu's
 * faces, the big character, sounds), into bundle/desk/. bundle/desk-shim.js
 * replaces the Tauri bridge and bundle/desk-host.js connects the page to Anna;
 * the collection theme is desktop-only. Anna's CSP has no 'unsafe-inline', so
 * the page's <style> block moves to `<name>.css`. Returns { html, css }.
 */
export function deskPage(html, name) {
  let css = '';
  const out = html
    .replace(/<style>([\s\S]*?)<\/style>/g, (_, block) => {
      css += block;
      return css.length === block.length ? `<link rel="stylesheet" href="${name}.css" />` : '';
    })
    .replace(
      '<script src="js/tauri-api.js"></script>',
      '<script src="../desk-shim.js"></script>\n  <script type="module" src="../desk-host.js"></script>'
    )
    .replace(/\s*<script src="js\/theme-apply\.js"><\/script>/, '')
    .replace(/\s*<link rel="stylesheet" href="theme-pages\.css" \/>/, '');
  if (!out.includes('../desk-shim.js')) throw new Error('desktop game page: tauri-api.js script tag not found');
  if (/<script>|<style|\son[a-z]+="|\sstyle="/.test(out)) throw new Error('desktop game page: inline script / style left');
  return { html: out, css };
}

function syncDeskGames() {
  const app = join(repoRoot, 'app');
  const desk = join(bundle, 'desk');
  rmSync(desk, { recursive: true, force: true });
  const copy = (rel) => {
    mkdirSync(dirname(join(desk, rel)), { recursive: true });
    copyFileSync(join(app, rel), join(desk, rel));
  };
  mkdirSync(desk, { recursive: true });
  for (const name of ['game', 'guess']) {
    const { html, css } = deskPage(readFileSync(join(app, `${name}.html`), 'utf8'), name);
    writeFileSync(join(desk, `${name}.html`), html);
    writeFileSync(join(desk, `${name}.css`), css);
  }
  for (const js of ['games-core.js', 'game-rps.js', 'game-guess.js', 'miu-skin.js']) copy(`js/${js}`);
  for (const f of readdirSync(join(app, 'media', 'game'))) if (/^rps-.*\.png$/.test(f)) copy(`media/game/${f}`);
  for (const f of ['popup-girl.png', 'settings-girl.png', 'tasks-girl.png']) copy(`media/side/${f}`);
  for (const voice of ['en', 'ja', 'vi']) for (const f of ['headpat.mp3', 'poke.mp3']) copy(`media/audio/${voice}/${f}`);
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main().catch((e) => {
    console.error(e.message);
    process.exit(1);
  });
}
