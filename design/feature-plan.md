# Image Compressor — Feature Plan v0.5 (additions to the shipped v0.4.0 app)

This plan extends the existing repo (`canja006/image-compressor`). It assumes
the current architecture: a pure-Rust `engine` workspace crate
(`src-tauri/crates/engine`) with no Tauri dependency, a thin Tauri bridge, and
a React/TS frontend. `npm run verify` (typecheck, lint, vitest, cargo
fmt/clippy/test) already gates everything.

## Scope

In (agreed): **A1** mozjpeg-quality JPEG (optional build flag), **A2**
EXIF/metadata + ICC/sRGB, **A4** smarter quality search, **B1** watch folder,
**B2** presets/profiles, **B3** CLI mode, **B4** MLS/web delivery presets,
**B5** OS shell/context-menu integration, **B6** quality metrics readout, **B7**
bulk rename with patterns.

Out: **A3** lossy WebP — dropped as redundant with the AVIF output the app
already ships.

## Two cross-cutting principles

1. **Main screen stays simple; tuning lives in Advanced/Settings.**
   - *Main screen (unchanged feel):* drop files → pick a **preset** or a size
     cap → go. Presets are the mechanism that keeps it simple.
   - *Advanced/Settings panel (off by default for the common case):* metadata
     mode, sRGB conversion, encoder backend toggle, perceptual-quality floor,
     quality-metric display, watch-folder config, rename patterns.
   - CLI and shell integration live outside the main flow by nature.

2. **A1 never touches the user's machine.** The C compiler + NASM that mozjpeg
   needs are BUILD-time only; the C library is statically linked into the
   binary. Users download the same `.dmg`/`.msi` and double-click as today.
   A1 ships as an optional Cargo feature so the default build stays pure-Rust;
   release installers are built with the feature ON.

---

## Phased plan

Each phase ends green (`npm run verify`) and is committed before the next.
Difficulty and main-vs-advanced placement noted per item.

### Phase 1 — Encoder & color pipeline (A1, A2)
- **A1 mozjpeg as optional feature.** Add a Cargo feature `mozjpeg` to the
  engine. When enabled, JPEG encodes via the `mozjpeg`/`mozjpeg-sys` path
  (trellis quantization → smaller files at equal quality). When disabled,
  fall back to the existing pure-Rust `image` encoder. Default build = OFF
  (pure Rust). The quality-search code is encoder-agnostic. *Difficulty: Med.*
  *Advanced (a backend selector appears only when compiled in).* 
- **A2 metadata + color.** Engine options: metadata = strip-all / keep-all /
  keep-orientation+ICC / strip-GPS-only; and a "convert to sRGB" toggle.
  Pure-Rust path: read EXIF with `kamadak-exif`; ICC→sRGB transform with a
  pure-Rust CMS (`qcms` or `moxcms` — agent confirms viability); strip by not
  re-embedding. *Difficulty: Med.* *Advanced.* Default for delivery use:
  strip GPS + convert sRGB.
- **CI:** add a release build that installs NASM + C toolchain and builds with
  `--features mozjpeg`; keep the plain `verify` build pure-Rust.
- **Acceptance:** JPEG output is smaller with the feature on (a test compares
  byte sizes at equal quality); metadata/sRGB options behave per setting;
  default `verify` build still needs no C toolchain.

### Phase 2 — Smarter search + quality metrics (A4, B6)
These share one perceptual metric, so build them together.
- **Implement SSIM and PSNR directly in the engine** (do NOT pull `dssim` — it
  is AGPL/commercial-dual-licensed and a liability for a paid app; PSNR is
  trivial and SSIM is a well-defined, permissively-implementable algorithm).
  *Difficulty: Med.*
- **A4 smarter search:** (a) a perceptual floor so the quality search won't
  drop below a visual threshold even if the cap would allow it; (b) cache
  search bounds across similar images in a batch to speed large jobs.
  *Advanced (the floor is a setting; default sensible).*
- **B6 quality readout:** show SSIM/PSNR next to size in the before/after
  preview. *Advanced toggle; off by default to keep the preview clean.*
- **Acceptance:** the floor prevents sub-threshold output (tested); metrics
  match reference values on fixtures; batch search is measurably faster.

### Phase 3 — Presets, delivery presets, bulk rename (B2, B4, B7)
- **B2 presets/profiles.** A named recipe captures the full config: size cap,
  resize mode + dimensions, format, metadata/sRGB, encoder, rename pattern.
  Save/load/delete via the existing `store` plugin. The main screen gains a
  preset picker. *Difficulty: Low–Med.* *Main screen + management in Settings.*
- **B4 delivery presets (built on B2).** Ship starter presets: "MLS JPEG
  ≤ 2 MB, 2048px long edge, sRGB, strip GPS"; "Web hero AVIF ≤ 200 KB";
  "Client gallery — fit whole shoot under a combined budget" (reuses the
  existing total-folder budget). *Difficulty: Low.* *Main screen.*
- **B7 bulk rename.** Token output naming: `{name}`, `{seq:000}`, `{date}`,
  `{w}x{h}`, plus a free prefix (e.g. `{address}-{room}-{seq}`), with a live
  preview and collision handling. *Difficulty: Low.* *Advanced / per-preset.*
- **Acceptance:** a preset round-trips and reproduces identical output; the
  three delivery presets produce compliant files; rename tokens expand
  correctly and never collide silently.

### Phase 4 — Watch folder (B1)
- Watch a chosen folder with the pure-Rust `notify` crate +
  `notify-debouncer-full`. On a new image: **wait for the file to settle**
  (size stable for a beat) before processing; **exclude the output folder and
  non-image files** so the app never re-processes its own output (infinite
  loop); apply a chosen **preset**. *Difficulty: Med.* *Advanced panel:
  enable, pick folder + preset; clear start/stop; status events to the UI.*
- Run on a background thread/task; survive the watched folder being
  deleted/unreadable; per-file isolation still applies.
- **Acceptance:** dropping a file auto-compresses it with the selected preset;
  output is not re-ingested; start/stop is clean; a vanished folder doesn't
  crash the app.

### Phase 5 — CLI mode (B3)
- A second binary target (or `--cli` flag) that depends on the **engine crate
  only, not the Tauri app**, exposing cap, resize mode, format, metadata,
  preset, and rename via flags; reads files/folders; exits non-zero on failure.
  *Difficulty: Med.* *Outside the GUI; for scripting/CI/power users.*
- **Acceptance:** the CLI compresses a folder to a cap headlessly and is
  covered by at least one integration test; it reuses the same engine code
  paths as the GUI (no duplicated logic).

### Phase 6 — OS shell / context-menu integration (B5)
- Windows: an Explorer right-click "Compress with Image Compressor" entry.
  macOS: a Finder Quick Action / Services entry. Both hand selected files to
  the app (or CLI). *Difficulty: Med, and genuinely per-OS — treat as two
  separate sub-tasks.* *Outside the main flow.*
- **Acceptance:** right-clicking images on each OS launches a compression with
  a chosen/default preset.

---

## Library / licensing notes (decide as they arise)
- **mozjpeg** (`mozjpeg-sys`) needs C compiler + NASM at build time only;
  statically linked; optional feature. If it won't build on a runner, the
  agent flags it and the pure-Rust default still ships.
- **ICC/sRGB**: prefer a pure-Rust CMS (`qcms`/`moxcms`). If neither is
  viable, the agent proposes an alternative rather than pulling a C lib.
- **Quality metrics**: implement SSIM/PSNR in-house; do NOT depend on `dssim`
  (AGPL) given possible commercial release.
- **Lossy PNG quantization** (if ever added): `imagequant` is GPLv3 — avoid for
  a paid product.

---

# VS Code Agent Prompt — paste this whole block

Paste into Claude Code (or your orchestrating agent) in the existing
`image-compressor` repo. Keep this file in the repo (e.g. `design/`) so it can
be read. Assumes a local-model delegation tool (e.g. the `@houtini/lm` MCP).

---

You are the ORCHESTRATOR extending the existing Image Compressor app (a shipped
Tauri 2 + Rust desktop app; pure-Rust `engine` workspace crate at
`src-tauri/crates/engine` with no Tauri deps). Read this file's "Phased plan"
above and the repo's `design/` brief and `README.md`; treat them as the source
of truth. Implement the phases in order.

## Division of labour (governs everything)
You (Claude) do NOT write the bulk of the implementation. You: (1) break each
phase into a large self-contained work package, (2) delegate it to the local
model via the local-model tool, (3) review what returns by running
`npm run verify` and reading the diff, (4) integrate and commit on green, (5)
fix bugs directly (small) or send a corrective brief (larger). The local model
does the feature implementation in large autonomous chunks; you architect,
review, integrate, debug.

## How to delegate (big tasks up front)
A work package is normally a whole phase. Each brief must contain: the goal and
which plan section it implements; the files/modules to touch (prefer the
`engine` crate for pure logic, the Tauri bridge for OS/event glue, the frontend
for UI); boundary types/signatures; the phase's acceptance criteria; the
self-check command `npm run verify`; and the hard rules below. Tell the local
model to work the whole package, run `npm run verify` itself, fix its own
failures, and only hand back when green or genuinely stuck. Review only after it
returns; don't micromanage mid-package.

## The self-healing loop (both levels)
On any failure: read the full error (Rust: include help/note lines); state the
root-cause hypothesis in one sentence; apply the smallest root-cause fix;
re-run `npm run verify`; up to 3 attempts per distinct error. If 3 fail, the
local model hands back with what it tried; you diagnose; if you also can't
resolve in 3 attempts, stop and give me two options. Never thrash or guess.

`npm run verify` covers TypeScript (typecheck, lint, vitest) and Rust
(`cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`). The default
`verify` build MUST stay pure-Rust (no C toolchain required).

## Hard rules (paste into every brief; enforce on review)
- No `any`/`@ts-ignore`/`eslint-disable`; no Rust `unwrap`/`expect`/`panic!`
  outside tests; no `#[allow]` to quiet clippy. Document any unavoidable
  suppression with the reason and risk.
- Never delete or weaken a test to make it pass. Fix the code, or fix a
  provably-wrong test and say so.
- Keep pure logic in the `engine` crate; the CLI depends on `engine` only, not
  Tauri — no duplicated logic between GUI and CLI.
- Per-file isolation holds everywhere: a corrupt/unreachable/failed file is
  recorded and the batch (or watch run) continues; never panic the process.
- No network calls in any processing path.

## Project-specific guarantees (verify on review)
- **A1 is an optional Cargo feature `mozjpeg`, default OFF.** Pure-Rust JPEG is
  the fallback. The C deps are build-time only and statically linked; nothing
  is required on the user's machine. Add a CI release build that installs
  NASM + C toolchain and builds `--features mozjpeg`, separate from the
  pure-Rust `verify` build.
- **Quality metrics are implemented in-house (SSIM/PSNR).** Do NOT add `dssim`
  (AGPL). Do NOT add `imagequant` (GPL). For ICC/sRGB prefer pure-Rust
  `qcms`/`moxcms`; if neither is viable, STOP and propose an alternative
  rather than pulling a C lib.
- **Watch folder must not re-ingest its own output** and must wait for files
  to settle before processing. A vanished/unreadable watched folder must not
  crash the app.
- **UI split:** main screen stays "drop → preset or cap → go". New tuning
  (metadata, sRGB, encoder backend, perceptual floor, metric display, watch
  config, rename patterns) goes in Advanced/Settings, off by default.

## Start now
1. Confirm you've read the plan and repo: summarise the architecture, the A1
   feature-flag handling, the in-house-metrics / licensing constraints, and how
   you'll split the six phases into work packages (list them).
2. Confirm the local-model tool is reachable (one-line test call).
3. Do Phase 1, then STOP and show me `npm run verify` output (pure-Rust build)
   and the result of a `--features mozjpeg` build before Phase 2.

---

## Tips
- Give the agent terminal permission so the verify loop runs; ensure the
  local-model MCP is connected.
- Phases 1–2 are mostly engine work and fully testable — make the local model
  prove the size-reduction, floor, and metric tests before you accept them.
- Build release installers (with `--features mozjpeg`) via the existing
  GitHub Actions matrix; add NASM to both runners. Don't cross-compile.
- If a package returns wrong twice, tighten the brief, not the chunk.