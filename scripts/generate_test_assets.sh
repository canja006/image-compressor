#!/usr/bin/env bash

# This script generates a collection of test assets for the Image Compressor bug‑test suite.
# It creates a temporary directory `test-assets` with subfolders for each hunt (H1‑H15).
# The script uses ImageMagick (`magick`) and exiftool where needed. Adjust paths as required.

set -euo pipefail

BASE_DIR="$(pwd)/test-assets"
mkdir -p "$BASE_DIR"

echo "Generating assets in $BASE_DIR"

# Helper to create a solid color image of given size and format
create_image() {
  local width=$1
  local height=$2
  local size_kb=$3   # approximate target size in KB (used for quality adjustments)
  local format=$4
  local out=$5
  # Use a gradient to get some data; adjust quality to approach size_kb
  if [[ "$format" == "jpeg" || "$format" == "jpg" ]]; then
    magick -size "${width}x${height}" gradient:blue-red -quality 85 "$out"
  else
    magick -size "${width}x${height}" gradient:blue-red "$out"
  fi
}

# H1 – Various formats and sizes
H1_DIR="$BASE_DIR/H1"
mkdir -p "$H1_DIR"
declare -a formats=(jpeg png webp tiff avif)
declare -a sizes=(1024 20480 51200 200000) # approx sizes in KB
for fmt in "${formats[@]}"; do
  for sz in "${sizes[@]}"; do
    # Create a 800x600 image; ImageMagick will produce different file sizes based on format
    out="$H1_DIR/img_${fmt}_${sz}k.$fmt"
    create_image 800 600 "$sz" "$fmt" "$out"
  done
done

# H2 – Large and many tiny files
H2_DIR="$BASE_DIR/H2"
mkdir -p "$H2_DIR"
# Large 50MP image (approx 8000x6250)
magick -size 8000x6250 gradient:gray "$H2_DIR/large_50mp.jpg"
# 200 tiny images (~5KB each)
for i in {1..200}; do
  magick -size 100x100 xc:white "$H2_DIR/tiny_$i.png"
done

# H4 – Metadata variants (EXIF orientation, GPS, ICC)
H4_DIR="$BASE_DIR/H4"
mkdir -p "$H4_DIR"
# Base image
magick -size 800x600 gradient:purple "$H4_DIR/base.jpg"
# Add EXIF tags using exiftool
exiftool -Orientation=6 -GPSLatitude=60 -GPSLongitude=24 -ICC_Profile=none "$H4_DIR/base.jpg" -overwrite_original
cp "$H4_DIR/base.jpg" "$H4_DIR/orient6.jpg"
# Strip all metadata
exiftool -all= "$H4_DIR/base.jpg" -overwrite_original -out "$H4_DIR/strip_all.jpg"
# Keep ICC only (assuming a dummy profile file exists; skip if not)
cp "$H4_DIR/base.jpg" "$H4_DIR/keep_icc.jpg"

# H5 – Wide‑gamut image for sRGB conversion test
H5_DIR="$BASE_DIR/H5"
mkdir -p "$H5_DIR"
magick -size 800x600 xc:"#ff00ff" "$H5_DIR/wide_gamut.png"

# H6 – Truncated/corrupt images
H6_DIR="$BASE_DIR/H6"
mkdir -p "$H6_DIR"
magick -size 800x600 gradient:orange "$H6_DIR/valid.jpg"
# Truncate to 60% of original size
truncate -s 60% "$H6_DIR/valid.jpg" "$H6_DIR/truncated.jpg"

# H8 – Filename edge cases
H8_DIR="$BASE_DIR/H8"
mkdir -p "$H8_DIR"
touch "$H8_DIR/normal.jpg"
touch "$H8_DIR/space\ file.jpg"
touch "$H8_DIR/emoji_😀.png"
touch "$H8_DIR/very_long_$(printf 'a%.0s' {1..260}).jpg"
touch "$H8_DIR/.hiddenfile"
touch "$H8_DIR/CON"
touch "$H8_DIR/..\..\escape.jpg"

echo "Asset generation complete."
