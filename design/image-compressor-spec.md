# Image Compressor (target-size) — Specification

## 1. Goal

A standalone **Windows and macOS** desktop app that compresses and resizes
images to hit a **target file size** (KB or MB cap), for a single image or a
whole batch. "Get this under 500 KB" / "get all of these under 2 MB" — fast,
offline, works on any folder of any common format. Built with Tauri; the
compression engine is Rust.

The app is cross-platform by design: the Rust engine, the web UI, and all file
access via Tauri plugins are OS-agnostic. Supporting both platforms is a
build/packaging concern, not a code concern — keep all logic platform-neutral
and never branch on OS in the engine.

## 2. The core feature: hit a size cap

This is what the app is *for*, so the algorithm is specified precisely.

`compress_to_target(image, cap_bytes, format, opts) -> Result`

1. Decode the source once.
2. If a `max_dimension` is set, downscale to it first (never upscale).
3. **Quality binary search** for the format's quality parameter in
   `[q_min, q_max]`:
   - encode to an in-memory buffer at quality `q`,
   - if `bytes <= cap`: record as the best-under-cap candidate and search
     higher (better quality / larger file, still under cap),
   - else: search lower,
   - ~7–8 iterations is enough to converge.
4. If a best-under-cap candidate exists, that's the result: the **largest file
   that still fits**, i.e. best quality within the cap.
5. If even `q_min` exceeds the cap, **downscale dimensions** by a factor
   (derived from the size overshoot, clamped; e.g. start ~0.85×) and repeat
   from step 3.
6. If dimensions fall below a floor (e.g. 16 px on the long edge) and the cap
   still can't be met, mark the file **unreachable** and continue the batch.

Rules:
- Search in memory; write to disk only once, at the end, for the winning
  encode. Decode each source only once.
- Strip metadata during the size-capping encode (smaller files, and good
  hygiene) — on by default, toggleable.
- "Already under cap" handling is configurable: skip/copy as-is (default) or
  re-encode anyway.
- Define "close enough": the result must be `<= cap`; if it lands far under
  cap only because `q_max` was hit, that's fine (image is just small).

## 3. Features by tier

### Tier 1 — MVP
- Input: a single image or a batch (drag-drop files/folder, or native dialog).
- Formats in: JPEG, PNG, WebP, TIFF.
- **Size cap** with KB/MB unit toggle.
- Optional **max dimension** (longest edge) cap, independent of size.
- Output format: keep-original / JPEG / WebP (default JPEG for compatibility).
- Output folder + filename (suffix e.g. `-compressed`, or a simple pattern),
  with collision handling (skip / overwrite / suffix).
- Parallel batch processing (`rayon`), capped to core count.
- Progress UI (overall + per-file), **cancel**, and **per-file isolation** — a
  corrupt or unreachable file is recorded and the batch continues.
- Result summary per file: original → final size, % saved, final quality and
  dimensions, and any failures / unreachable caps.

### Tier 2 — Enhanced
- AVIF output (best ratio; slower encode — warn on large batches).
- Common-cap **presets**: e.g. Email (< 10 MB total/each), Web (< 500 KB),
  Portal upload (< 2 MB) — one click.
- PNG / transparency handling: if target is JPEG and the source has alpha,
  warn and flatten onto a chosen background; offer an alpha-capable target
  (WebP/AVIF) instead.
- Toggles: "skip if already under cap", "don't re-encode losslessly-smaller".
- Per-image cap override in batch.
- Before/after preview with live size readout on one sample.

### Tier 3 — Advanced
- Lossless PNG path: `oxipng` optimization + optional color quantization for
  PNGs that must stay PNG.
- High-compression backend toggle (mozjpeg / libwebp) — see library risk.
- Total-folder budget: fit a set of images under one combined cap.
- Watch folder for auto-compression.
- Windows shell integration: right-click "Compress to size" on selected files.

## 4. Tech stack (decided)

| Concern        | Choice                                                   |
|----------------|----------------------------------------------------------|
| App shell      | Tauri 2 (Rust core; WebView2 on Windows, WKWebView on macOS) |
| Frontend       | React 18 + TypeScript (strict) + Vite                    |
| Styling        | Tailwind CSS                                             |
| State          | Zustand                                                  |
| File dialogs   | `@tauri-apps/plugin-dialog`                              |
| File IO        | `@tauri-apps/plugin-fs`                                  |
| Settings/presets | `@tauri-apps/plugin-store`                             |
| Decode/encode  | Rust `image`; resize via `fast_image_resize`            |
| Parallelism    | Rust `rayon`                                            |
| Rust errors    | `thiserror` (lib) + `anyhow` (commands)                 |
| Testing        | Vitest + RTL (frontend); `cargo test` (Rust)            |
| Lint/format    | ESLint + Prettier; clippy + rustfmt                     |

### Library risk (decide early)
The JPEG/WebP encoder choice is a real fork on Windows:
- **Pure-Rust encoders** (`image` built-ins / `jpeg-encoder`, `ravif` for AVIF):
  easy build on BOTH Windows and macOS, no external toolchain. Files slightly
  larger at a given quality.
- **mozjpeg / libwebp** (`mozjpeg`, `webp` crates): meaningfully smaller files
  at the same quality — directly helps the size goal — but the native deps
  need a C toolchain / NASM at build time on Windows AND Xcode tools on macOS,
  i.e. build friction on both platforms.

Default to pure-Rust for a clean build everywhere; expose mozjpeg/libwebp as an
optional Tier 3 backend. If a chosen crate won't build on either OS, the agent
stops and flags it rather than forcing it.

## 5. Architecture

```
src/                          (frontend)
  components/
    Intake.tsx                single/batch input
    CapControls.tsx           size cap + unit, max dimension, format
    PresetBar.tsx             common caps (web/email/portal)
    ProgressView.tsx          overall + per-file, cancel
    SummaryView.tsx           original->final, % saved, unreachable flags
    SamplePreview.tsx         before/after + live size (Tier 2)
    Settings.tsx              output folder, naming, defaults
    ErrorFallback.tsx
  lib/
    tauri.ts                  invoke + event wrappers
    types.ts                  Job/Options/Result shared types
  store/useStore.ts
  App.tsx / main.tsx

src-tauri/                    (Rust core)
  src/
    lib.rs                    command registration
    commands.rs               compress_batch, preview_sample, cancel_job
    engine/
      mod.rs                  batch orchestration + rayon fan-out
      decode.rs               load + detect format
      target.rs               compress_to_target search (section 2)
      resize.rs               fast_image_resize wrapper
      encode.rs               per-format encode to memory/disk
      naming.rs               output filename
    model.rs                  Options/Result (serde)
    error.rs                  thiserror types
    progress.rs               event payloads
```

### Data flow
Frontend builds a `Job` (files + `Options`) → `compress_batch` fans out with
`rayon` → each file runs `compress_to_target`, writes the winner, emits a
`progress` event → Rust returns a summary. Cancellation via an atomic flag
checked between files.

## 6. Non-functional requirements
- A corrupt/unsupported/unreachable file is logged and skipped — never aborts
  the batch or panics.
- All size search happens in memory; one decode and one disk write per file.
- Cancel stops promptly and reports partial results.
- No network calls anywhere. Output never overwrites unless the policy says so.
- Minimal scoped Tauri capabilities (fs user-selected + app-data, dialog,
  store).
- Accessible, keyboard-operable UI; AA contrast.

## 7. Acceptance (definition of done)
- Tier 1 works and is tested, including: a batch where one file is corrupt and
  one cap is unreachable still completes and reports both; outputs are always
  `<= cap` when reachable; the quality search returns the best quality under
  the cap (a test asserts the result is under cap and that one quality step
  higher would exceed it).
- `npm run verify` passes (typecheck, lint, vitest, `cargo fmt --check`,
  `cargo clippy -D warnings`, `cargo test`).
- `npm run tauri build` produces a working installer on each target OS: a
  Windows `.msi`/`.exe` and a macOS `.dmg`/`.app`, built via CI on their
  respective runners.
- No `any` / `@ts-ignore` / `eslint-disable`; no Rust `unwrap`/`expect`/`panic!`
  outside tests, except where documented with a reason. No OS-specific branching
  in the engine.
- README covers setup (Rust toolchain + per-OS prerequisites), the size
  algorithm, the encoder-backend choice, and the dual-platform CI build.
