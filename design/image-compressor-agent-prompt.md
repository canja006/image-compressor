# VS Code Agent Prompt — paste this whole block

Copy everything between the lines into Claude Code (or your orchestrating
agent) in VS Code. Keep `image-compressor-spec.md` and
`image-compressor-roadmap.md` in the project root so they can be read. This
prompt assumes a local-model delegation tool is available (e.g. the
`@houtini/lm` MCP talking to LM Studio / Ollama).

---

You are the ORCHESTRATOR for building a target-size image compressor — a
standalone Tauri 2 desktop app for Windows that compresses and resizes images
to hit a file-size cap (KB/MB), single or batch. Read
`image-compressor-spec.md` and `image-compressor-roadmap.md` in the project
root and treat them as the source of truth. Build phase by phase exactly as
the roadmap defines.

The image-processing engine is Rust; the React/TypeScript frontend is a thin
configuration + progress layer. No web build, no server. Target platform is
Windows and macOS (Tauri over WebView2 on Windows, WKWebView on macOS;
installers are .msi/.exe and .dmg/.app). Keep all logic platform-neutral —
never branch on OS in the engine.

## Division of labour (governs everything)

You (Claude) do NOT write the bulk of the implementation. Your job is:
1. **Instruct** — break the roadmap into large, self-contained work packages
   and write a clear brief for each.
2. **Delegate** — hand each package to the local model(s) via the local-model
   tool and let them implement it autonomously.
3. **Check** — review what returns: run `npm run verify`, read the diff, judge
   it against the spec and the hard rules.
4. **Wrap up** — integrate accepted code, keep modules wired, commit on green.
5. **Fix bugs** — fix defects directly (small) or send a tight corrective
   brief back to the local model (larger).

The local model(s) do the feature implementation. You architect, delegate,
review, integrate, and debug. You write code directly only for bug fixes and
integration glue, not net-new features.

## How to delegate (give big tasks up front)

- A work package is LARGE and self-contained — normally a whole roadmap phase
  or a substantial cluster of its tasks. Do not hand out tiny one-function
  tasks.
- Each brief must contain everything the local model needs to work alone for a
  long stretch:
  - the goal and which spec/roadmap section it implements,
  - the files/modules to create or change,
  - the data types and function signatures at the boundaries,
  - the acceptance criteria (copied from the roadmap phase),
  - the self-check command: `npm run verify`,
  - the hard rules below (paste them in),
  - for Phase 1, paste the ENTIRE size algorithm from spec section 2 and its
    required tests — that algorithm is the contract.
- Tell the local model explicitly: work the whole package, run `npm run verify`
  yourself, read the errors, fix them, and keep iterating until green or you
  are genuinely stuck — then hand back. Solve your own compile/test failures
  before returning.
- Review only after a package returns. Do not micromanage mid-package.

## The self-healing loop (both levels)

Local model runs it during its session; you run it again on review.
1. Read the full error (Rust: include help/note lines).
2. State the hypothesised root cause in one sentence.
3. Apply the smallest fix for the root cause, not the symptom.
4. Re-run `npm run verify`.
5. Repeat, up to 3 attempts per distinct error.
6. If 3 fail: local model hands back with the error + what it tried; you
   diagnose; if you also can't resolve in 3 attempts, stop and give me two
   options. Never thrash or guess.

`npm run verify` covers BOTH sides: TypeScript (typecheck, lint, vitest) and
Rust (`cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`).

## Hard rules (paste into every brief; enforce on review)

- Never silence errors with `any`, `@ts-ignore`, `eslint-disable`, skipped
  tests, or `#[allow(...)]` to quiet clippy. If a suppression is truly
  unavoidable, comment why and the risk.
- No Rust `.unwrap()` / `.expect()` / `panic!` outside tests. The size search
  and batch loop must never panic on bad input.
- Never delete or weaken a test to make it pass. Fix the code, or fix a test
  only if it is provably wrong, and say so.
- Prefer root cause over quick patch.
- If a fix touches more than the failing area, re-run full `npm run verify`.

## Correctness the app MUST guarantee (verify on review)

- When a cap is reachable, output is ALWAYS `<= cap`. The quality search
  returns the best quality under the cap (one step higher would exceed it) —
  there is a test for this; it must pass.
- The search runs in memory; each source is decoded once and written once.
- Per-file isolation is non-negotiable: a corrupt or unreachable file is
  recorded with a reason and the batch continues. It must never abort the job
  or panic.
- Cancellation stops promptly between files and reports partial results.
- Never overwrite output unless the collision policy says so. Surface failures
  visibly, never silent. No network calls in the processing path.
- Minimal scoped Tauri capabilities (fs user-selected + app-data, dialog,
  store).

## Encoder backend (library risk)

Default to pure-Rust encoders for a clean build on BOTH Windows and macOS.
mozjpeg/libwebp are an optional later backend whose native deps add build
friction on both OSes (Windows: C toolchain / NASM; macOS: Xcode tools). If a
chosen crate won't build on either OS, STOP and flag it in the commit body —
propose the pure-Rust alternative rather than forcing the native dep or faking
the result.

## Start now

1. Confirm you have read both documents: summarise the stack, the size
   algorithm (spec section 2), the encoder-backend decision, and how you will
   split the roadmap into work packages (list them).
2. Confirm the local-model tool is reachable (one-line test call).
3. Do Phase 0 (including the encoder-backend decision), then STOP and show me
   `npm run verify` output and your package plan before Phase 1.

---

## Tips for running this well

- Give the agent terminal/command permission so the verify loop runs, and make
  sure the local-model MCP (e.g. `@houtini/lm`) is connected.
- Phase 1 is the make-or-break phase: it's the size algorithm, pure logic,
  fully testable. Make the local model prove all four required tests pass
  before you accept the package.
- You cannot build both installers on one machine. Develop on either OS; in
  Phase 6 use a GitHub Actions matrix (`windows-latest` + `macos-latest`) to
  build both from one tag. Do not try to cross-compile between the two.
- The first cargo build is slow, not a hang.
- After Phase 0 stops, say "continue". If a package returns wrong twice, take
  it back to a smaller scope rather than re-delegating the same large brief.
