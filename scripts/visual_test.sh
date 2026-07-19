#!/usr/bin/env bash
# ENH-007 visual-regression harness — the GPU layer.
#
# For each row in tests/golden/manifest.txt:
#   1. `imgdiff gen-preset`  — build the row's preset (schema-correct, via the
#      app's own FractalParams) where `--preset` reads.
#   2. run `par-fractal`     — fixed window size, `--quality ultra` (caps LOD at
#      ultra; LOD stays active, so the settle delay + the golden-mismatch retry
#      below absorb the occasional frame caught mid-restore), screenshot after
#      a settle delay to a fixed path, then auto-exit.
#   3. `imgdiff` compare      — the screenshot vs the committed golden tile.
#
# Modes (env vars):
#   BLESS=1       — (re)write tests/golden/<id>.png from this run's screenshot.
#   CROSSCHECK=1  — also render the CPU f64 reference and compare by Pearson
#                   correlation over luma (qualitative; never gates the run).
#                   render-ref mirrors the GPU's sRGB surface encoding and
#                   effective max_iter; correlation (not per-pixel MAE) tolerates
#                   the f32/double-float/perturbation-vs-f64 boundary drift that
#                   makes ~half the pixels differ on a correct render. A correct
#                   render scores r~0.67-0.87; --min-corr 0.5 flags gross
#                   failures (black frame, wrong fractal, collapsed zoom).
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

# Render one manifest row into $out_png. Defined once here (not inside the
# loop) so the dimension-mismatch retry below can re-run it without
# duplicating the app's flag list. References the loop-set $id/$size/$out_png
# via bash's dynamic scoping.
render_row() {
  "$BIN" --preset "$id" --window-size "$size" --quality ultra \
         --screenshot-path "$out_png" --screenshot-delay "$SETTLE_S" --exit-delay "$EXIT_S"
}

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

  # 2. run the app (non-fatal: a crash is a row failure, not a script abort).
  #    Retry once if the screenshot comes back at the wrong dimensions: an
  #    intermittent capture hiccup where the window briefly reports a
  #    non-requested size before settling (seen ~once per many runs, on the
  #    first deep-zoom row). The render itself is correct — only the capture
  #    size hiccups — so a single re-render reliably fixes it.
  render_row || true

  if [ ! -f "$out_png" ]; then
    echo "  FAIL: no screenshot produced (app crash or no GPU?)"
    fail=1
    continue
  fi

  # Guard against the capture-dimension hiccup: re-render once if the
  # screenshot's pixel size doesn't match what the manifest requested.
  actual_size=$(file -b "$out_png" | grep -oE '[0-9]+ x [0-9]+' | head -1 | tr -d ' ')
  if [ "$actual_size" != "$size" ]; then
    echo "  WARN: capture dims ${actual_size:-unknown} != requested $size (capture hiccup); re-rendering once"
    render_row || true
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
      # Transient convergence flake: each row's render is deterministic in
      # isolation (rendered twice → identical), but ~1 run in 8 catches one
      # shallow row mid-flight. Root cause: `--quality` enables LOD (it sets
      # `lod_config.enabled = true`, not a hard pin), and the df shallow path
      # is LOD-sensitive while the perturbation (deep) path pins iterations
      # independently — so only shallow rows occasionally catch a frame before
      # LOD fully restores to ultra under sequential-launch timing jitter.
      # Re-render once and re-compare: a real regression fails twice; a
      # transient flake passes on retry. (The deeper fix is a `--no-lod`
      # determinism switch; deferred as more invasive than this harness guard.)
      echo "  WARN: golden mismatch — re-rendering once to rule out a transient flake"
      render_row || true
      if "$IMGDIFF" "$out_png" "$golden_png"; then
        echo "  PASS (golden, on retry)"
      else
        echo "  FAIL (golden mismatch)"
        fail=1
      fi
    fi
  fi

  # 3b. optional CPU-vs-GPU cross-check (qualitative; never gates the run).
  # render-ref mirrors the GPU's sRGB surface encoding and effective max_iter
  # (max_iterations + zoom_iteration_bonus), so it is structurally faithful.
  # The gate is Pearson correlation over luma (--min-corr), NOT per-pixel MAE:
  # the GPU iterates f32 / double-float / perturbation while the reference is
  # f64-exact, so fractal-boundary pixels diverge on ~half of all pixels even
  # for a correct render (bad_pixel_fraction 0.2-0.9), making MAE+frac gates
  # unpassable. Correlation captures structural agreement instead — a correct
  # render scores ~0.67-0.87 across the manifest; a black frame or wrong
  # fractal scores ~0. --min-corr 0.5 passes every correct row with margin.
  if [ "$CROSSCHECK" -eq 1 ]; then
    ref_png="$OUT_DIR/$id.ref.png"
    "$IMGDIFF" render-ref "$kind" "$cx" "$cy" "$zoom" "$iters" "$size" "$ref_png" || true
    if [ -f "$ref_png" ] && "$IMGDIFF" "$out_png" "$ref_png" --min-corr 0.5; then
      echo "  PASS (cross-check vs CPU reference)"
    else
      echo "  WARN: cross-check correlation < 0.5 (likely a real regression, not f32/f64 drift)"
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
