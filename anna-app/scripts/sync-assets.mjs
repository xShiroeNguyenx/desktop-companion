// Copy the Live2D runtime libs and the bundled models into anna-app/bundle/.
// Models come from ../dist-models/<id>.zip, or the GitHub "models" release when
// the zip is not on disk. Output folders are gitignored; run `npm run assets`.
import { copyFileSync, existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
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
export const MODELS = JSON.parse(readFileSync(join(repoRoot, 'models', 'manifest.json'), 'utf8')).models.map((m) => ({
  id: m.id,
  zip: m.file,
  name: titleCase(m.id),
}));
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
  console.log(`✓ libs + ${index.length} models → bundle/`);
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main().catch((e) => {
    console.error(e.message);
    process.exit(1);
  });
}
