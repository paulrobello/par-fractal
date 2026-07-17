#!/usr/bin/env bash
# ENH-007 visual-regression harness — the GPU layer.
#
# For each row in tests/golden/manifest.txt:
#   1. `imgdiff gen-preset`  — build the row's preset (schema-correct, via the
#      app's own FractalParams) where `--preset` reads.
#   2. run `par-fractal`     — fixed window size, `--quality ultra` (pins LOD;
#      LOD is off by default), screenshot after a settle delay to a fixed path,
#      then auto-exit.
#   3. `imgdiff` compare      — the screenshot vs the committed golden tile.
#
# Modes (env vars):
#   BLESS=1       — (re)write tests/golden/<id>.png from this run's screenshot.
#   CROSSCHECK=1  — also render the CPU f64 reference and compare (looser
#                   tolerance; flags DF-vs-f64 drift beyond golden determinism).
#                   NOTE: the GPU screenshot is post-processed (composite color
#                   grading + FXAA) while the CPU reference is raw smooth-`t`, so
#                   this is a *qualitative* check — useful for spotting gross
#                   failures (black frames, wrong fractal), not last-bit deltas.
#   SETTLE_S=<n>  — seconds to wait before screenshotting (default 8; the
#                   double-float HP path is slower and needs the headroom to
#                   finish a complete frame — too low and deep rows go black).
#
# Degrades gracefully: on a headless box (no DISPLAY/WAYLAND_DISPLAY and not
# macOS) it prints a clear skip and exits 0 — the CPU math teeth still run via
# plain `cargo test` / `make test`, so CI never needs a GPU.
#
# No `set -e`: a single row failing (crash, golden mismatch) is reported and
# the run continues; the script exits non-zero only if any row failed.

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

MANIFEST="tests/golden/manifest.txt"
GOLDEN_DIR="tests/golden"
OUT_DIR="target/visual"
BIN="target/release/par-fractal"
IMGDIFF="target/release/imgdiff"

SETTLE_S="${SETTLE_S:-8}"   # seconds the app runs before screenshotting
EXIT_S="${EXIT_S:-12}"      # seconds before the app auto-exits
BLESS="${BLESS:-0}"
CROSSCHECK="${CROSSCHECK:-0}"

mkdir -p "$OUT_DIR"

# --- build release binaries if missing ---------------------------------------
need_build=0
[ -x "$BIN" ] || need_build=1
[ -x "$IMGDIFF" ] || need_build=1
if [ "$need_build" -eq 1 ]; then
  echo "visual-test: building release binaries (par-fractal, imgdiff)…"
  cargo build --release --bin par-fractal --bin imgdiff || { echo "visual-test: build failed"; exit 2; }
fi

if [ ! -f "$MANIFEST" ]; then
  echo "visual-test: manifest not found at $MANIFEST"
  exit 2
fi

# --- headless guard ----------------------------------------------------------
# macOS always has a window server; Linux needs DISPLAY or WAYLAND_DISPLAY.
if [ "$(uname)" != "Darwin" ] && [ -z "${DISPLAY:-}" ] && [ -z "${WAYLAND_DISPLAY:-}" ]; then
  echo "visual-test: no display detected (unset DISPLAY/WAYLAND_DISPLAY on non-macOS) — skipping GPU layer."
  echo "  CPU math teeth still run via 'cargo test' / 'make test'."
  exit 0
fi

fail=0
rows=0
while read -r id kind ftype cx cy zoom iters color_mode size; do
  # Skip comments and blank lines.
  case "$id" in
    ''|\#*) continue ;;
  esac
  rows=$((rows + 1))
  echo ":: $id ($ftype, zoom=$zoom, ${size})"

  out_png="$OUT_DIR/$id.png"
  golden_png="$GOLDEN_DIR/$id.png"

  # 1. preset
  "$IMGDIFF" gen-preset "$id" "$ftype" "$cx" "$cy" "$zoom" "$iters" "$color_mode" \
    || { echo "  FAIL: gen-preset"; fail=1; continue; }

  # 2. run the app (non-fatal: a crash is a row failure, not a script abort)
  "$BIN" --preset "$id" --window-size "$size" --quality ultra \
         --screenshot-path "$out_png" --screenshot-delay "$SETTLE_S" --exit-delay "$EXIT_S" \
    || true

  if [ ! -f "$out_png" ]; then
    echo "  FAIL: no screenshot produced (app crash or no GPU?)"
    fail=1
    continue
  fi

  # 3a. golden bless / compare
  if [ "$BLESS" -eq 1 ]; then
    cp "$out_png" "$golden_png"
    echo "  blessed → $golden_png"
  else
    if [ ! -f "$golden_png" ]; then
      echo "  FAIL: no golden at $golden_png (run 'make visual-bless' to create it)"
      fail=1
      continue
    fi
    if "$IMGDIFF" "$out_png" "$golden_png"; then
      echo "  PASS (golden)"
    else
      echo "  FAIL (golden mismatch)"
      fail=1
    fi
  fi

  # 3b. optional CPU-vs-GPU cross-check (looser tolerance; never gates the run)
  if [ "$CROSSCHECK" -eq 1 ]; then
    ref_png="$OUT_DIR/$id.ref.png"
    "$IMGDIFF" render-ref "$kind" "$cx" "$cy" "$zoom" "$iters" "$size" "$ref_png" || true
    if [ -f "$ref_png" ] && "$IMGDIFF" "$out_png" "$ref_png" --mae 4.0; then
      echo "  PASS (cross-check vs CPU reference)"
    else
      echo "  WARN: cross-check outside tolerance (may be legitimate DF/f64 drift)"
    fi
  fi
done < "$MANIFEST"

echo "----------------------------------------"
echo "visual-test: $rows row(s) processed"
if [ "$fail" -ne 0 ]; then
  echo "visual-test: FAILURES present — see above"
  exit 1
fi
echo "visual-test: all rows OK"
