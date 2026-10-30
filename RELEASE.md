# Release guide

Desktop Companion ships **signed auto-updates**. On launch the app checks GitHub
Releases for a newer signed version and nudges the user; Settings has a manual
"Kiểm tra cập nhật" button too.

## One-time setup (do this once)

The release CI signs each update with a private key. You must add the key to the
repo's secrets.

### 1. Add GitHub secrets

Repo → **Settings** → **Secrets and variables** → **Actions** → **New repository secret**:

| Secret name | Value |
|-------------|-------|
| `TAURI_SIGNING_PRIVATE_KEY` | the full contents of `updater-private.key` (the file in the project root, NOT committed) |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | the key's password — take it from your private password store (never write it in the repo) |

> Get the private key contents on Windows:
> ```powershell
> Get-Content .\updater-private.key -Raw | Set-Clipboard
> ```
> Then paste into the `TAURI_SIGNING_PRIVATE_KEY` secret.

> ⚠️ **Never commit `updater-private.key`.** It's already in `.gitignore`. If you
> lose it, existing installs can no longer be auto-updated (you'd have to ship a
> new key + a manual reinstall).

### 2. Enable GitHub Pages (for the landing page)

Repo → **Settings** → **Pages** → **Source** → **GitHub Actions**.

The site is served at **https://desktop.techdecoded.net/** (custom domain):

- **Settings** → **Pages** → **Custom domain** = `desktop.techdecoded.net`, with **Enforce HTTPS** on.
- DNS: `CNAME desktop → xshiroenguyenx.github.io`.
- Because Pages deploys via Actions, a `CNAME` file in the repo is ignored — the domain lives only in repo settings.
- `docs/landing/robots.txt`, `sitemap.xml` and the canonical / Open Graph tags in `index.html` all point at this domain; update them together if it ever changes.

## Cutting a release

1. **Bump the version** in these files (keep them identical):
   - `package.json` → `"version"`
   - `package-lock.json` → top-level `"version"` and `packages[""].version`
   - `src-tauri/Cargo.toml` → `version`
   - `src-tauri/tauri.conf.json` → `"version"`

   Then run `cargo test` (in `src-tauri/`) so `Cargo.lock` picks up the new
   version — CI builds with `--locked` and fails if the lock file is stale.
   Run `npm test` too; CI runs both.

2. **Commit and tag** (the tag must be `v` + the version):
   ```bash
   git add -A
   git commit -m "chore: release v0.2.0"
   git push
   git tag v0.2.0
   git push origin v0.2.0
   ```

3. **CI does the rest** (`.github/workflows/release.yml`):
   - builds the NSIS + MSI installers (Windows) and a universal `.dmg` (macOS),
   - signs the update artifacts and generates `latest.json`,
   - creates a **draft** GitHub Release with everything attached.

4. **Check the draft** before publishing. Both matrix jobs (`windows-latest`,
   `macos-latest`) must be green; with `fail-fast: false` one can fail while the
   other still attaches its files. GitHub turns the space in file names into a
   dot, so the draft should have:
   - Windows: `Desktop.Companion_x.y.z_x64-setup.exe` and `…_x64_en-US.msi`, each with a `.sig`;
   - macOS: `Desktop.Companion_x.y.z_universal.dmg` and an updater `….app.tar.gz` with its `.sig`;
   - `latest.json` listing the `windows-x86_64*` platforms **and** macOS ones
     (`darwin-aarch64` / `darwin-x86_64`, or `darwin-universal`). If the macOS
     entries are missing, Macs will not see the update: re-run the macOS job
     before publishing.

5. **Publish**: go to **Releases**, open the draft, click **Publish release**.

That's it. Every running app on an older version will detect the new release on
its next launch and offer to update.

## macOS build notes

- The `.dmg` is **ad-hoc signed** (`bundle.macOS.signingIdentity: "-"`) and **not
  notarized**, so no Apple secrets are needed in CI. Users clear the quarantine
  flag once (`xattr -dr com.apple.quarantine …`, see Install in the README).
- Because the signature is ad-hoc, macOS may treat each update as a new app and
  drop its **Accessibility** permission (text capture). The README tells users to
  remove and re-add the app in that list.
- Transparent windows need Tauri's `macos-private-api` feature and
  `app.macOSPrivateApi: true`; both are already set. Apps using the private API
  cannot go to the Mac App Store, which is fine for GitHub releases.
- CI's `macos-check` job only runs `cargo check`. The first real macOS build of
  each release is the release workflow itself, so test the `.dmg` on a Mac
  before publishing the draft.
- To notarize later: add an Apple Developer ID certificate and the `APPLE_*`
  secrets that `tauri-action` reads, and replace `signingIdentity: "-"`.

## Releasing a Live2D model (separate download)

Collection models (e.g. Gothic Aristocrat) are **not** in the installer. The app
downloads one after the user unlocks it, from a fixed GitHub Release tagged
`models`, and only trusts it if `manifest.json` is signed with the updater key
and the zip matches the SHA-256 listed there. Installed models also update
themselves at launch when the manifest has a higher version. So a model update
does **not** need an app release.

> ⚠️ The `models` release must be a **prerelease and not "latest"**. The app
> updater reads `releases/latest/download/latest.json`; if `models` became the
> latest release, every running app would stop finding updates.

**First time only:** create the release (after packing and signing, steps 1–2):
```powershell
gh release create models --prerelease --latest=false --title "Live2D models" --notes "Downloadable Live2D models for the Collection. Do not mark as latest." dist-models/gothic-aristocrat-1.zip models/manifest.json models/manifest.json.sig
```

**Every model release (new model or update):**

1. **Pack** (from `src-tauri/`). The version must be higher than the one in
   `models/manifest.json`; a new model starts at 1:
   ```powershell
   cargo run --example pack_model -- <model folder> <id> <version>
   ```
   This checks the model (required motions and groups, every referenced file,
   ≤ 6 MB), writes `dist-models/<id>-<version>.zip` (git-ignored) with only the
   files `model3.json` references, and updates `models/manifest.json`. Name the
   model files however you like: inside the package the main `*.model3.json` is
   always stored as `model.model3.json`, which is the name the app opens.
2. **Sign the manifest** (from the repo root). It prompts for the key password;
   don't pass it with `-p`, since command lines are visible to other processes:
   ```powershell
   npx tauri signer sign -f updater-private.key models/manifest.json
   ```
3. **Upload** the zip, manifest and signature (replacing the old manifest):
   ```powershell
   gh release upload models dist-models/<id>-<version>.zip models/manifest.json models/manifest.json.sig --clobber
   ```
4. **Commit** `models/manifest.json` and `models/manifest.json.sig` so the repo
   records which version is live.

A **new** model also needs an app release, because the app has to know it: a
`BUILTIN` row in `src-tauri/src/models.rs`, a `model.<id>` item in
`collection.rs`, a drop rate in `gacha::LEGENDARY_PCT` and a preview image in
`app/media/model-previews/`. Publish the model (steps 1–3) **before** that app
release goes out, or buyers get "not published yet" until you do.

## How the updater finds releases

`src-tauri/tauri.conf.json` → `plugins.updater.endpoints` points at:

```
https://github.com/xShiroeNguyenx/desktop-companion/releases/latest/download/latest.json
```

`latest.json` is generated and attached automatically by the release workflow, so
the "latest" release always serves the right manifest.
