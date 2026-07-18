#!/usr/bin/env bash
# QA-027: winit 0.30 resize-lifecycle smoke test (local; needs a display).
#
# Launches the app at 256x256, resizes it to 512x512 mid-run via the
# `--resize-after` agent-operability hook, screenshots AFTER the resize, and
# asserts the screenshot is the new size — proving the Resized → surface-
# reconfigure → redraw path survives without driving a human input.
#
# This automates the ONE lifecycle case that doesn't need a human. The rest
# (minimize/restore, multi-monitor move, DPI change, mobile/bfcache, web
# load/orientation) is a manual pre-release sweep — see
# `docs/release-checklist.md`.
#
# Skips cleanly on a headless box (no display) like visual_test.sh.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

BIN="target/release/par-fractal"
OUT="target/visual/resize-smoke.png"

# macOS always has a window server; Linux needs DISPLAY/WAYLAND_DISPLAY.
if [ "$(uname)" != "Darwin" ] && [ -z "${DISPLAY:-}" ] && [ -z "${WAYLAND_DISPLAY:-}" ]; then
  echo "resize-smoke: no display detected — skipping (set DISPLAY/WAYLAND_DISPLAY on Linux)."
  exit 0
fi

[ -x "$BIN" ] || { echo "resize-smoke: building release binary…"; cargo build --release --bin par-fractal; }

mkdir -p "$(dirname "$OUT")"
rm -f "$OUT"

"$BIN" --window-size 256x256 --resize-after 2 512x512 \
       --screenshot-delay 4 --screenshot-path "$OUT" --exit-delay 6

[ -f "$OUT" ] || { echo "FAIL: no screenshot produced"; exit 1; }

# `file` prints "PNG image data, W x H, …" portably (no PIL/ImageMagick dep).
dims=$(file "$OUT" | grep -oE '[0-9]+ x [0-9]+' | head -1)
W=$(echo "$dims" | awk '{print $1}')
H=$(echo "$dims" | awk '{print $3}')

if [ "${W:-0}" = "512" ] && [ "${H:-0}" = "512" ]; then
  echo "PASS: resize lifecycle rendered at ${W}x${H} (started 256x256, resized at +2s, shot at +4s)"
  exit 0
else
  echo "FAIL: expected 512x512 after resize, got ${W:-?}x${H:-?}"
  exit 1
fi
