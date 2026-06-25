# Command line (`imgc`) and OS right-click integration

The compression engine ships as a small command-line tool, **`imgc`**, in addition to the desktop
app. Both are backed by the *same* `engine` crate (`src-tauri/crates/engine`) — there is no separate
compression implementation, so the CLI produces byte-for-byte the same results as the GUI for the same
settings.

`imgc` has two subcommands:

- `imgc compress` — compress files/folders headlessly (scripting, CI, batch jobs).
- `imgc shell` — install/remove the OS right-click **"Compress with Image Compressor"** menu, which
  itself just runs `imgc compress`.

## Building `imgc`

It is a member of the Cargo workspace under `src-tauri/`:

```bash
cd src-tauri
cargo build --release -p imgc        # binary at src-tauri/target/release/imgc
```

## `imgc compress`

```
imgc compress [OPTIONS] --cap <SIZE> <INPUTS>...
```

`<INPUTS>` are files or folders (folders are walked recursively, same as the GUI). Each image is
compressed to the **per-file** byte cap. The process exits non-zero if any file failed or was
unreachable, so it is safe to use in CI / `make` pipelines.

| Flag | Meaning | Default |
| --- | --- | --- |
| `--cap <SIZE>` | Target size per file. `500k`, `2MB`, `1.5mb`, or raw bytes (base 1024). **Required.** | — |
| `--format <FMT>` | `keep` \| `jpeg` \| `png` \| `avif` | `keep` |
| `--max-dimension <PX>` | Bound the longest edge (fit mode). | none |
| `--exact <WxH>` | Crop-to-fill to exactly `W×H` (exact mode). Conflicts with `--max-dimension`. | — |
| `--anchor <A>` | Crop anchor for `--exact`: `start` \| `center` \| `end`. | `center` |
| `--allow-upscale` | Allow upscaling to reach the `--exact` size. | off |
| `--out <DIR>` | Write outputs here. | next to each source |
| `--suffix <S>` | Appended to each output's file stem. | `-compressed` |
| `--metadata <M>` | `strip-all` \| `keep-all` \| `keep-orientation-icc` \| `strip-gps`. | `strip-all` |
| `--srgb` | Convert pixels to sRGB before encoding. | off |
| `--rename <PATTERN>` | Output-name pattern: `{name} {seq} {seq:000} {date} {w} {h}`. | off |
| `--collision <C>` | `suffix` \| `overwrite` \| `skip`. | `suffix` |
| `--quality-min` / `--quality-max` | JPEG quality search bounds (1–100). | `10` / `95` |
| `--floor <PCT>` | Perceptual quality floor (SSIM %, 50–100). | off |
| `--force-recompress` | Re-encode files even if already under the cap. | off (copies as-is) |
| `--quiet` | Suppress per-file progress lines. | off |

Example — fit a whole folder under 800 KB JPEGs, max 2048 px, strip GPS:

```bash
imgc compress ~/Photos/listing --cap 800k --format jpeg --max-dimension 2048 --metadata strip-gps
```

Orientation is always baked into the pixels, per-file failures are isolated (one bad file never aborts
the run), and **no processing path makes a network call** — identical guarantees to the GUI.

## `imgc shell` — right-click integration (B5)

```
imgc shell <install|uninstall|print> [--target auto|macos|windows]
                                      [--cap 500k] [--format keep] [--out DIR]
```

The recipe baked into the menu command is controlled by `--cap`, `--format`, and `--out` (everything
else uses the `compress` defaults). `print` writes the integration assets to stdout without installing
anything — useful to inspect, redirect to a file, or install by hand.

### macOS (Finder Quick Action)

```bash
imgc shell install            # writes ~/Library/Services/Compress with Image Compressor.workflow
imgc shell uninstall          # removes it
imgc shell print              # prints the one-line command + Info.plist + document.wflow
```

This installs an Automator **Service** (Quick Action). After installing, right-click one or more
images in Finder → **Quick Actions** (or the **Services** submenu) → **Compress with Image
Compressor**. If it doesn't appear, enable it under **System Settings → Keyboard → Keyboard Shortcuts
→ Services → Files and Folders**.

> **Verification note.** The generated `Info.plist` and `document.wflow` are validated with
> `plutil -lint` in the test suite, and install/uninstall is round-trip tested. The actual Finder
> right-click could not be exercised in the headless build environment. If the auto-generated Quick
> Action does not run on your macOS version, run `imgc shell print --target macos`: it prints the exact
> one-line shell command, which you can paste into **Automator → New → Quick Action → Run Shell Script**
> (set *Pass input: as arguments*, *Workflow receives: image files*) to recreate it manually.

### Windows (Explorer context menu)

On Windows, `imgc shell install` writes the registry entries directly (per-user, `HKCU`). From any
other OS you can still generate them:

```bash
imgc shell print --target windows > install.reg     # then on Windows: double-click / reg import
```

This registers a **"Compress with Image Compressor"** verb under
`HKCU\Software\Classes\SystemFileAssociations\.<ext>\shell` for every supported image extension
(`.jpg .jpeg .png .webp .tif .tiff`), whose command runs `imgc compress … "%1"` on the clicked file.
`imgc shell uninstall` (or the matching `-` keys via `print` + import) removes them.

The registry paths are per-extension and per-user, so installation needs no administrator rights and
touches nothing global.
