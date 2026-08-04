# Building Payjar for Windows & macOS

Payjar needs Rust **edition 2024** (Rust 1.85+) to compile at all — several
of its dependencies (egui's clipboard backend, etc.) require it too. There's
no way to get that toolchain onto a locked-down sandbox, and there's no way
to legitimately cross-compile a macOS binary from Linux without Apple's SDK.
The correct, actually-working fix is to let GitHub's own Windows and macOS
runners build the binaries natively — which is exactly what `.github/workflows/build.yml`
in this bundle does.

## One-time setup

1. Copy `.github/workflows/build.yml` into your Payjar repo (same relative
   path), and merge `.gitignore` into your existing one — yours currently
   has leftover `<<<<<<< HEAD` / `=======` / `>>>>>>>` conflict markers in it
   from an old merge, so `git status` will look confusing until that's
   cleaned up.

2. **Stop tracking the pre-built binary.** Your upload includes a compiled
   `Payjar` executable (Linux ELF, ~11 MB) committed straight into the repo.
   Build artifacts shouldn't live in git — they bloat the repo and go stale.
   ```bash
   git rm --cached Payjar
   ```
   (the new `.gitignore` already excludes `/target/` and `dist/` so future
   local builds won't get committed either.)

3. Push to GitHub:
   ```bash
   git add -A
   git commit -m "Add CI build workflow for Windows/macOS"
   git remote add origin https://github.com/<you>/Payjar.git   # if not already
   git push -u origin main
   ```

## Getting binaries

- **Every push to `main`** triggers a build on both Windows and macOS
  runners (quick sanity check — did it compile?). You can watch it under
  the repo's **Actions** tab, and download the zipped binaries from the
  "Artifacts" section of that run even without a tag.

- **Tagged releases** (what you actually want to hand to users) — cut one
  like this:
  ```bash
  git tag v1.4.0
  git push origin v1.4.0
  ```
  That triggers the same builds, then bundles both zips into a proper
  **GitHub Release** with download links: `Payjar-windows-x86_64.zip` and
  `Payjar-macos-universal.zip` (the Mac build is a universal binary — one
  file that runs natively on both Apple Silicon and Intel Macs).

## What "working" means here, honestly

- **Windows**: builds with the stock MSVC toolchain GitHub provides — no
  gotchas expected. `rusqlite`'s bundled SQLite and `eframe`'s GUI windows
  both compile fine there.
- **macOS**: same — the workflow ad-hoc-signs the binary so Gatekeeper
  doesn't flat-out refuse to run it, but since it isn't signed with a paid
  Apple Developer ID, first-time users will still get one "unidentified
  developer" warning in System Settings → they right-click → Open once.
  Real notarization removes that, but requires an Apple Developer
  account ($99/yr) — say the word if you want that added later.
- Both jobs use `--locked`, so they build against the exact dependency
  versions in your `Cargo.lock`. Worth knowing: `Cargo.toml` pins
  `rustls = "0.24.0-dev.0"`, which is a pre-release/dev version of rustls,
  not a stable release — it'll build, but it's worth swapping to a stable
  `0.23.x` before you ship this to real users, since dev versions can
  change or vanish without the usual semver guarantees.

## Android

Skipped for now per your call — flagging for later: `eframe`'s GUI layer
(used for `gui.alert/confirm/prompt`) defaults to a desktop windowing
backend, so an Android build isn't a simple recompile — it needs the
`android-activity` backend, an `AndroidManifest.xml`, and packaging via
`cargo apk` or `cargo-ndk`. Happy to scope that out whenever you're ready.
