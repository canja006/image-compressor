# Image Compressor (target-size) — Roadmap

> **Status: implemented through v0.3.3.** Tier 1 (Phases 0–3) and Tier 2 (Phase 4) are complete,
> and Phase 6 (dual-OS GitHub Actions CI + release) is in place. Tier 3 (Phase 5) is partially
> done — `oxipng` lossless-PNG optimization and the total-folder budget shipped; mozjpeg/libwebp,
> watch folder, and Windows shell integration remain optional. See the README for the current
> feature set and the published GitHub Release for installers. This document is the original brief.

Build in phases. Each phase ends green (`npm run verify`, both TS and Rust) and
is committed before the next. Target platform is Windows.

---

## Phase 0 — Scaffold and guardrails

Target platforms are Windows and macOS. Develop on either; build both via CI.

**Prerequisite:** Rust toolchain (`rustup`/`cargo`) plus per-OS Tauri
prerequisites — on Windows: Microsoft C++ Build Tools / MSVC + WebView2 runtime
(usually present on Win 10/11); on macOS: Xcode command-line tools. The agent
checks `cargo --version` and the platform toolchain for the OS it's running on,
and stops with setup steps if missing.

**Tasks**
1. `npm create tauri-app@latest` — React + TypeScript + Vite.
2. Add plugins `dialog`, `fs`, `store`; minimal scoped capabilities.
3. Add Tailwind, ESLint, Prettier, Vitest, RTL; TS strict; clippy + rustfmt.
4. **Decide the encoder backend** (spec section 4 library risk): default to
   pure-Rust encoders for a clean build on both Windows and macOS. Record the
   decision in README.
5. Scripts in `package.json`:
   ```json
   {
     "scripts": {
       "dev": "tauri dev",
       "build": "tsc -b && vite build",
       "tauri": "tauri",
       "typecheck": "tsc --noEmit",
       "lint": "eslint . --max-warnings 0",
       "format": "prettier --write .",
       "test": "vitest run",
       "rust:check": "cd src-tauri && cargo fmt --check && cargo clippy -- -D warnings && cargo test",
       "verify": "npm run typecheck && npm run lint && npm run test && npm run rust:check"
     }
   }
   ```
6. React error boundary + one smoke test rendering `<App />`.

**Acceptance:** `npm run verify` green; `npm run tauri dev` opens a window on
the development OS. (Cross-platform builds are verified in Phase 6.)

---

## Phase 1 — The target-size engine (the core)

Single file, no UI beyond a minimal harness. Get the algorithm right first.

**Tasks**
1. `decode.rs`: load JPEG/PNG/WebP/TIFF; corrupt/unsupported → typed error.
2. `encode.rs`: encode to an in-memory buffer at a given quality, per format.
3. `resize.rs`: `fast_image_resize` downscale; never upscale.
4. `target.rs`: implement `compress_to_target` exactly as spec section 2 —
   quality binary search, then dimension downscale fallback, then unreachable.
5. `naming.rs`: output filename (suffix / pattern).
6. Rust tests (mandatory):
   - output is always `<= cap` when reachable,
   - the search returns the best quality under cap (one step higher exceeds it),
   - a tiny cap on a large image triggers downscale and still meets cap,
   - an impossible cap is reported `unreachable`, not a panic.

**Acceptance:** a command compresses one file to a target size correctly.
`verify` green.

---

## Phase 2 — Batch, parallelism, progress, cancel

**Tasks**
1. `engine/mod.rs`: fan out over files with `rayon`, capped to core count.
2. Per-file isolation: corrupt / unreachable recorded, batch continues.
3. Emit `progress` events; frontend `ProgressView` shows overall + per-file.
4. Cancellation via shared atomic flag between files.
5. End-of-run summary (original→final, % saved, failures, unreachable).
6. Test: mixed batch with one corrupt and one unreachable file finishes and
   reports both, processing the rest.

**Acceptance:** drop a folder, compress all to a cap in parallel, watch
progress, cancel mid-run, see an accurate summary. `verify` green.

---

## Phase 3 — Frontend: caps, formats, output, presets

**Tasks**
1. `Intake.tsx` (single + batch) + native dialog.
2. `CapControls.tsx`: size cap + KB/MB toggle, optional max dimension, output
   format (keep/JPEG/WebP).
3. `Settings.tsx`: output folder, naming, collision policy, "skip if already
   under cap", strip-metadata default.
4. `PresetBar.tsx`: common caps (Web < 500 KB, Portal < 2 MB, Email < 10 MB).
5. `SummaryView.tsx`.

**Acceptance:** full Tier 1 flow from the UI, tested. `verify` green. Tag
`v0.1.0`.

---

## Phase 4 — Enhanced

**Tasks**
1. AVIF output (warn on big batches).
2. PNG/alpha handling: flatten-to-background for JPEG target, or steer to
   WebP/AVIF; clear warning.
3. `SamplePreview.tsx`: before/after with live size readout.
4. Per-image cap override in batch.

**Acceptance:** Tier 2 complete and tested. `verify` green. Tag `v0.2.0`.

---

## Phase 5 — Advanced (pick per appetite)

Lossless PNG path (`oxipng` + quantization), mozjpeg/libwebp high-compression
backend toggle, total-folder budget, watch folder, Windows right-click "Compress
to size" shell integration.

**Acceptance:** unstable items behind flags; `verify` green per feature.

---

## Phase 6 — Packaging and distribution (Windows + macOS)

You cannot build both installers on one machine — each OS's bundle must be
built on that OS. Use CI to build both from one tag.

**Tasks**
1. Icons, product name, version, identifier in `tauri.conf.json` (shared
   across both platforms).
2. Confirm local `npm run tauri build` works on your dev OS first.
3. Add a **GitHub Actions release workflow** using `tauri-action` with a matrix
   of `macos-latest` and `windows-latest`. On a version tag it builds both and
   attaches the artifacts to a release:
   - Windows → `.msi` (WiX) and/or `.exe` (NSIS),
   - macOS → `.dmg` / `.app` (build a universal binary for Apple Silicon +
     Intel).
4. Optional, per platform and independent:
   - Windows: Authenticode code signing (avoids SmartScreen warnings),
   - macOS: Apple Developer ID signing + notarization (avoids Gatekeeper
     warnings).

**Acceptance:** pushing a tag yields working installers for both Windows and
macOS; a clean machine of each OS can install and run the app.

---

## Cross-phase rules
- Commit format `phase(N): description`; commit only on green.
- The Phase 1 size-algorithm tests are permanent regression tests.
- No skipped / `.only` tests; no phase ships red. Update README each phase.
- Blocked task → note the blocker in the commit body, continue with the rest.
