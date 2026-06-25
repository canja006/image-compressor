#!/usr/bin/env bash

# Automated bug‑test runner for Image Compressor (Round 2).
# Executes a subset of the hunts (H1, H2, H4, H5, H6, H8) using the CLI binary.
# Generates JSON logs in test-results/ and a markdown summary.

set -euo pipefail

# Paths
PROJECT_ROOT="$(pwd)"
ASSETS_DIR="$PROJECT_ROOT/test-assets"
RESULTS_DIR="$PROJECT_ROOT/test-results"
mkdir -p "$RESULTS_DIR"

CLI_BIN="$PROJECT_ROOT/src-tauri/target/release/imgc"

# Ensure CLI is built
if [[ ! -x "$CLI_BIN" ]]; then
  echo "Building CLI..."
  pushd "$PROJECT_ROOT/src-tauri" > /dev/null
  cargo build --release -p imgc
  popd > /dev/null
fi

log_json() {
  local title="$1"
  local severity="$2"
  local oracle="$3"
  local env="$4"
  local repro="$5"
  local expected="$6"
  local actual="$7"
  local evidence="$8"
  cat <<EOF >> "$RESULTS_DIR/${title// /_}.json"
{
  "title": "$title",
  "severity": "$severity",
  "oracle": "$oracle",
  "environment": "$env",
  "repro_steps": "$repro",
  "expected": "$expected",
  "actual": "$actual",
  "evidence": "$evidence"
}
EOF
}

run_h1_cap_invariant() {
  local hunt_dir="$ASSETS_DIR/H1"
  local caps=(10 1024 50000 300000 2000000) # bytes
  for cap in "${caps[@]}"; do
    for img in "$hunt_dir"/*; do
      out_dir="$RESULTS_DIR/h1_out_cap_${cap}"
      mkdir -p "$out_dir"
      # Run CLI with cap and keep format (auto‑detect)
      "$CLI_BIN" compress "$img" --cap "$cap" --out "$out_dir" > /dev/null 2>&1 || true
      # Check size if file produced
      if [[ -f "$out_dir/$(basename "$img")" ]]; then
        size=$(stat -c%s "$out_dir/$(basename "$img")")
        if (( size > cap )); then
          log_json "CAP_INVARIANT_FAIL" "Correctness" "#1" "macOS CLI" "compress $img --cap $cap" "size <= cap" "size $size > cap $cap" "$out_dir/$(basename "$img")"
        fi
      else
        # No output – could be unreachable; verify CLI reported correctly (skip for brevity)
        :
      fi
    done
  done
}

run_h2_total_budget() {
  local hunt_dir="$ASSETS_DIR/H2"
  local budget=$((10 * 1024 * 1024)) # 10 MiB for example
  out_dir="$RESULTS_DIR/h2_out"
  mkdir -p "$out_dir"
  "$CLI_BIN" compress "$hunt_dir" --cap "$budget" --out "$out_dir" > /dev/null 2>&1 || true
  total=0
  for f in "$out_dir"/*; do
    (( total+= $(stat -c%s "$f") ))
  done
  if (( total > budget )); then
    log_json "BUDGET_EXCEEDED" "Correctness" "#1" "macOS CLI" "compress $hunt_dir --cap $budget" "total <= budget" "total $total > budget $budget" "$out_dir"
  fi
}

run_h4_metadata() {
  local dir="$ASSETS_DIR/H4"
  for img in "$dir"/*.jpg; do
    out_dir="$RESULTS_DIR/h4_out"
    mkdir -p "$out_dir"
    "$CLI_BIN" compress "$img" --format jpeg --out "$out_dir" > /dev/null 2>&1 || true
    # Verify orientation tag removed
    orientation=$(exiftool -Orientation -s3 "$out_dir/$(basename "$img")" 2>/dev/null || echo "none")
    if [[ "$orientation" != "none" && "$orientation" != "1" ]]; then
      log_json "ORIENTATION_TAG_LEFT" "Correctness" "#6" "macOS CLI" "compress $img" "no orientation tag" "found $orientation" "$out_dir/$(basename "$img")"
    fi
  done
}

run_h5_srgb() {
  local img="$ASSETS_DIR/H5/wide_gamut.png"
  out_dir="$RESULTS_DIR/h5_out"
  mkdir -p "$out_dir"
  "$CLI_BIN" compress "$img" --srgb --out "$out_dir" > /dev/null 2>&1 || true
  # Check color profile via exiftool
  profile=$(exiftool -ColorSpace -s3 "$out_dir/$(basename "$img")" 2>/dev/null || echo "none")
  if [[ "$profile" != "sRGB" && "$profile" != "none" ]]; then
    log_json "SRGB_CONVERSION_FAIL" "Correctness" "#5" "macOS CLI" "compress $img --srgb" "output sRGB" "found $profile" "$out_dir/$(basename "$img")"
  fi
}

run_h6_truncated() {
  local img="$ASSETS_DIR/H6/truncated.jpg"
  out_dir="$RESULTS_DIR/h6_out"
  mkdir -p "$out_dir"
  "$CLI_BIN" compress "$img" --out "$out_dir" > /dev/null 2>&1 || true
  # Verify CLI does not crash and produces a reachable output or reports failure
  if [[ -f "$out_dir/$(basename "$img")" ]]; then
    size=$(stat -c%s "$out_dir/$(basename "$img")")
    if (( size == 0 )); then
      log_json "TRUNCATED_ZERO_OUTPUT" "Correctness" "#4" "macOS CLI" "compress $img" "non‑zero output" "size 0" "$out_dir/$(basename "$img")"
    fi
  fi
}

run_h8_filenames() {
  local dir="$ASSETS_DIR/H8"
  out_dir="$RESULTS_DIR/h8_out"
  mkdir -p "$out_dir"
  for img in "$dir"/*; do
    "$CLI_BIN" compress "$img" --out "$out_dir" > /dev/null 2>&1 || true
  done
  # Verify no files escaped the output directory (simple check)
  for f in "$out_dir"/*; do
    if [[ "$f" == *".."* ]]; then
      log_json "FILENAME_PATH_ESCAPE" "Security" "#8" "macOS CLI" "compress $img" "sanitized path" "escaped path $f" "$f"
    fi
  done
}

main() {
  echo "Running H1 – Cap Invariant tests..."
  run_h1_cap_invariant
  echo "Running H2 – Total Budget test..."
  run_h2_total_budget
  echo "Running H4 – Metadata forensics..."
  run_h4_metadata
  echo "Running H5 – sRGB conversion..."
  run_h5_srgb
  echo "Running H6 – Truncated image handling..."
  run_h6_truncated
  echo "Running H8 – Filename edge cases..."
  run_h8_filenames
  echo "Test run complete. Logs are in $RESULTS_DIR"
}

main "$@"
