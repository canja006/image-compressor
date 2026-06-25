# Bug Test Brief
This document outlines a comprehensive test plan for the Image Compressor
application. It is intended for manual testers and developers to verify that the
application behaves correctly across a wide range of scenarios, including edge
cases and adversarial inputs.

---
## A. INTAKE & LIST
- Drag‑drop files; drag‑drop folders (recursive scan); native picker.
- Add the same file twice; mix of supported + unsupported extensions.
- Unsupported/garbage files should be ignored, not crash.
- Thumbnails render; per‑row ESTIMATED size (~size) fills in; "cap too small"
shows for impossible caps. Change cap/format rapidly — estimates must
update and stay responsive (this was a perf fix; confirm it's fast and
correct).
- Remove rows / Clear during idle.

## B. TARGET SIZE & CAP MODES
- Per‑file cap in KB and MB; very small and very large caps.
- Per‑file cap OVERRIDE on individual rows (the inline "cap" control).
- Total‑folder BUDGET: verify combined output ≤ budget. Try a budget with one
huge file + many tiny ones (a tiny share may be unreachable — fine, but must be
reported, not crash).
- Verify oracle #1 and #2 by checking actual output file sizes on disk.

## C. RESIZE MODES
- Fit: with/without max‑dimension; never upscales.
- Exact (crop‑to‑fill): exact W×H output, anchors start/center/end, allow‑
upscale on/off. Verify oracle #3 (exact dims; unreachable not shrunk).

## D. OUTPUT FORMATS
- JPEG, PNG, AVIF, "keep". For images WITH alpha: PNG/AVIF keep transparency;
JPEG flattens onto the configured background color (try several colors).
- AVIF is the slow encoder — confirm it still works and the UI stays usable.

## E. METADATA / COLOR
- Metadata modes: strip all / keep ICC / keep all / no‑GPS. Confirm GPS is gone
when expected; ICC is preserved when expected.
- sRGB conversion on a wide‑gamut/ICC image.
- Orientation: feed images with EXIF orientation 1–8; outputs upright (#6).

## F. NAMING / COLLISIONS
- Suffix; rename pattern tokens {name} {seq:000} {date} {w} {h}; live preview.
- Filenames with spaces, Unicode, emoji, very long names, leading dots, path‑
unsafe characters — outputs must be sanitized, not corrupt paths.
- Collision policy: Number it / Overwrite / Skip. Output folder == input folder:
confirm originals are NOT clobbered unexpectedly (#10).
- "Skip if already under cap": such files are copied as‑is.

## G. PRESETS
- Apply each built‑in delivery preset (MLS, Web hero AVIF, Client gallery).
- Save current settings as a named preset; reload app; preset persists; delete.

## H. QUALITY READOUT
- Toggle "Show metrics"; SSIM/PSNR appear in the preview.
- Perceptual quality floor: with it on, output stays above the SSIM floor (it
should trade resolution rather than drop quality below the floor).

## I. RUN / CANCEL / ISOLATION
- Run a large mixed batch including: a corrupt file (e.g. a .png that's actually
text), a 0‑byte file, an impossible‑cap file. Verify each is reported and the
batch completes; NO crash (#4).
- Cancel: immediately, mid‑run, and right at the end — partial results correct,
no crash, can re‑run.

## J. WATCH FOLDER (advanced panel)
- Pick a watched folder and a DIFFERENT output folder; Start. Drop/copy images
in — they compress automatically with the current recipe; the live log shows
results. Verify the output is NOT picked up again (#8 — no loop).
- Try output folder == watched folder (must be rejected).
- Drop a LARGE file that takes time to copy (it must wait until fully written,
not compress a half‑written file).
- Drop many files at once.
- While watching: rename/delete the watched folder; rename the output folder.
The app must not crash. Stop cleanly.
- Reload the webview while watching — does state restore sanely?

## K. CLI (imgc)
- `imgc compress <folder> --cap 500k --format jpeg --out <dir>`; check outputs
are ≤ cap and exit code 0.
- Impossible cap (`--cap 10 --force-recompress`) → exit code non‑zero (#9).
- A folder with no images → non‑zero. Bad flags → clean error.
- Compare a few CLI outputs to GUI outputs for the same settings (#9).
- `imgc shell print --target windows` and `--target macos` (inspect output);
on macOS try `imgc shell install --services-dir <tmp>` then uninstall.

## L. UI / UX
- Light/dark theme toggle (incl. during a run). Window resize / very small
window. Keyboard navigation & focus states. Help dialog. Empty states.

---
## Adversarial / Fuzz Inputs (throw these at intake, run, watch, and CLI)
- 0‑byte file; truncated/corrupt JPEG/PNG; a text file renamed to .jpg.
- Wrong extension (real PNG named .jpg and vice‑versa).
- Huge image (50–100+ megapixels) — memory/decode/perf, no crash.
- 1×1 px image; extreme aspect ratio (e.g. 10000×1).
- CMYK JPEG; 16‑bit PNG; grayscale; progressive JPEG; multi‑page/odd TIFF;
animated WebP (only first frame expected; must not crash).
- Image already smaller than the cap (skip‑as‑copy path).
- Output folder that is read‑only / nonexistent / deleted mid‑run → graceful
per‑file failure, not a crash.
- Symlinked folders / a folder that links back into itself (must terminate).
- Duplicate paths; the same folder added twice.
- Disk‑near‑full (if you can simulate) → graceful failure.

---
## Code‑Level Checks (skip if you're a black‑box/manual tester)
- Run `npm run verify` — it must be green. Try to break a test legitimately.
- Grep the Rust for banned patterns OUTSIDE `#[cfg(test)]`: `unwrap(`, `expect(`,
`panic!`, and any networking (reqwest/ureq/TcpStream/http). There should be
none in processing paths.
- Confirm crates/cli depends on the engine crate only (NOT on Tauri).
- Look for: integer overflow in size math, race conditions in the watch worker
and the estimate cache, panics on lock poisoning, and any path where a cap
can be exceeded or an original file overwritten.

---
## Do Not
- Don't test on irreplaceable photos — use copies/fixtures.
- Don't modify the app to "make a test pass"; report instead.
- Don't make network calls part of your test harness for the app itself.

---
## Report Format (one entry per bug, most severe first)
- **Title:** short, specific
- **Severity:** Crash / Data‑loss / Correctness / Perf / Cosmetic
- **Oracle broken:** which invariant # (if any)
- **Environment:** OS, GUI or CLI, settings/preset used
- **Repro steps:** numbered, exact (inputs, clicks, flags)
- **Expected:** what should happen
- **Actual:** what happened (include the output file size/dimensions, the
on‑screen status, exit code, or stack/log if any)
- **Evidence:** file path / screenshot / console or terminal output

Start with the ORACLE checks (A–K) on real outputs (open the files, measure
their sizes/dimensions), then the ADVERSARIAL inputs. Summarize at the end: what
you covered, what you couldn't, and your top 5 risks.

---
*Generated by the test‑plan assistant.*