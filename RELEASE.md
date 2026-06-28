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
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | `companion` |

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

## Cutting a release

1. **Bump the version** in three files (keep them identical):
   - `package.json` → `"version"`
   - `src-tauri/Cargo.toml` → `version`
   - `src-tauri/tauri.conf.json` → `"version"`

2. **Commit and tag** (the tag must be `v` + the version):
   ```bash
   git add -A
   git commit -m "chore: release v0.2.0"
   git push
   git tag v0.2.0
   git push origin v0.2.0
   ```

3. **CI does the rest** (`.github/workflows/release.yml`):
   - builds the NSIS + MSI installers,
   - signs the update artifacts and generates `latest.json`,
   - creates a **draft** GitHub Release with everything attached.

4. **Publish**: go to **Releases**, open the draft, click **Publish release**.

That's it. Every running app on an older version will detect the new release on
its next launch and offer to update.

## How the updater finds releases

`src-tauri/tauri.conf.json` → `plugins.updater.endpoints` points at:

```
https://github.com/xShiroeNguyenx/desktop-companion/releases/latest/download/latest.json
```

`latest.json` is generated and attached automatically by the release workflow, so
the "latest" release always serves the right manifest.
