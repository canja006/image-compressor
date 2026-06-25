#!/usr/bin/env node
// Build the `imgc` CLI and stage it as a Tauri sidecar at `src-tauri/binaries/imgc-<target-triple>`
// so the installer ships the CLI alongside the GUI. Tauri's `externalBin` requires the exact
// host-target-triple suffix; this derives it from `rustc -vV` and copies the freshly built binary into
// place. Wired into tauri.conf.json's beforeDev/BuildCommand so local `tauri dev`/`tauri build` just work.
//
// Builds for the HOST triple — which is what a local build and the Windows CI build target. The macOS
// *universal* release build instead needs `imgc-universal-apple-darwin`, which CI produces explicitly
// (lipo of both arches) before the bundle step; see .github/workflows/release.yml.
//
// Flags: `--mozjpeg` builds with the trellis JPEG encoder (release; needs NASM). `--force` rebuilds
// even if the staged sidecar already exists (default: skip when present, for fast repeat `tauri dev`).
import { execFileSync } from 'node:child_process'
import { mkdirSync, copyFileSync, existsSync } from 'node:fs'
import { join, dirname } from 'node:path'
import { fileURLToPath } from 'node:url'

const args = process.argv.slice(2)
const withMozjpeg = args.includes('--mozjpeg')
const force = args.includes('--force')

const root = join(dirname(fileURLToPath(import.meta.url)), '..')
const srcTauri = join(root, 'src-tauri')

// Host target triple, e.g. `aarch64-apple-darwin` / `x86_64-pc-windows-msvc`.
const triple = execFileSync('rustc', ['-vV'], { encoding: 'utf8' })
  .match(/^host:\s*(.+)$/m)?.[1]
  ?.trim()
if (!triple) throw new Error('could not determine the host target triple from `rustc -vV`')

const ext = triple.includes('windows') ? '.exe' : ''
const dest = join(srcTauri, 'binaries', `imgc-${triple}${ext}`)

if (!force && existsSync(dest)) {
  console.log(`[sidecar] up to date: ${dest}`)
  process.exit(0)
}

const features = withMozjpeg ? ['--features', 'mozjpeg'] : []
console.log(`[sidecar] building imgc (${triple})${withMozjpeg ? ' +mozjpeg' : ''}…`)
execFileSync('cargo', ['build', '-p', 'imgc', '--release', ...features], {
  cwd: srcTauri,
  stdio: 'inherit',
})

mkdirSync(join(srcTauri, 'binaries'), { recursive: true })
copyFileSync(join(srcTauri, 'target', 'release', `imgc${ext}`), dest)
console.log(`[sidecar] staged ${dest}`)
