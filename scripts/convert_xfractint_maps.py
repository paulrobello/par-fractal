#!/usr/bin/env python3
"""Convert xfractint .map files to Rust ColorPalette constants."""

import re
from pathlib import Path


def parse_map_file(path: Path) -> list[tuple[int, int, int]]:
    """Parse a xfractint .map file and return list of RGB tuples (0-255)."""
    colors = []
    with open(path, encoding="utf-8", errors="replace") as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            # Split by whitespace and take first 3 values
            parts = line.split()
            if len(parts) >= 3:
                try:
                    r = int(parts[0])
                    g = int(parts[1])
                    b = int(parts[2])
                    # Clamp to valid range
                    r = max(0, min(255, r))
                    g = max(0, min(255, g))
                    b = max(0, min(255, b))
                    colors.append((r, g, b))
                except ValueError:
                    continue
    return colors


def sample_colors(colors: list[tuple[int, int, int]], n: int = 5) -> list[tuple[float, float, float]]:
    """Sample n colors evenly from the palette, converting to 0.0-1.0 range."""
    if not colors:
        return [(0.0, 0.0, 0.0)] * n
    if len(colors) == 1:
        c = colors[0]
        return [(c[0] / 255.0, c[1] / 255.0, c[2] / 255.0)] * n

    result = []
    for i in range(n):
        t = i / (n - 1)
        idx_f = t * (len(colors) - 1)
        idx = int(idx_f)
        frac = idx_f - idx

        if idx + 1 < len(colors):
            # Linear interpolation
            c1, c2 = colors[idx], colors[idx + 1]
            r = (c1[0] + (c2[0] - c1[0]) * frac) / 255.0
            g = (c1[1] + (c2[1] - c1[1]) * frac) / 255.0
            b = (c1[2] + (c2[2] - c1[2]) * frac) / 255.0
        else:
            c = colors[idx]
            r, g, b = c[0] / 255.0, c[1] / 255.0, c[2] / 255.0

        result.append((round(r, 3), round(g, 3), round(b, 3)))
    return result


def name_to_rust_const(filename: str) -> str:
    """Convert filename to Rust constant name."""
    name = Path(filename).stem.upper()
    # Replace non-alphanumeric with underscore
    name = re.sub(r"[^A-Z0-9]", "_", name)
    # Prefix with XF_ for xfractint
    return f"XF_{name}"


def name_to_display_name(filename: str) -> str:
    """Convert filename to display name."""
    name = Path(filename).stem
    # Title case, replace underscores with spaces
    name = name.replace("_", " ").replace("-", " ")
    # Handle some special cases
    special = {
        "altern": "Alternating Grey",
        "chroma": "Chromatic",
        "defaultw": "Default White",
        "firestrm": "Fire Storm",
        "froth3": "Froth 3",
        "froth6": "Froth 6",
        "froth316": "Froth 3-16",
        "froth616": "Froth 6-16",
        "gamma1": "Gamma 1",
        "gamma2": "Gamma 2",
        "glasses1": "3D Glasses 1",
        "glasses2": "3D Glasses 2",
        "goodega": "Good EGA",
        "headach2": "Headache 2",
        "landscap": "Landscape",
        "paintjet": "PaintJet",
    }
    stem = Path(filename).stem.lower()
    if stem in special:
        return special[stem]
    return name.title()


def generate_rust_const(name: str, display_name: str, colors: list[tuple[float, float, float]]) -> str:
    """Generate Rust const definition for a palette."""
    lines = [f'    pub const {name}: ColorPalette = ColorPalette {{']
    lines.append(f'        name: "{display_name}",')
    lines.append("        colors: [")
    for i, (r, g, b) in enumerate(colors):
        lines.append(f"            Vec3::new({r}, {g}, {b}),")
    lines.append("        ],")
    lines.append("    };")
    return "\n".join(lines)


def main():
    maps_dir = Path("../xfractint-20.04p16/maps")
    if not maps_dir.exists():
        print(f"Maps directory not found: {maps_dir}")
        return

    map_files = sorted(maps_dir.glob("*.map"))
    print(f"Found {len(map_files)} map files")

    consts = []
    const_names = []

    for map_file in map_files:
        colors = parse_map_file(map_file)
        if len(colors) < 2:
            print(f"Skipping {map_file.name}: not enough colors ({len(colors)})")
            continue

        sampled = sample_colors(colors, 5)
        const_name = name_to_rust_const(map_file.name)
        display_name = name_to_display_name(map_file.name)

        rust_const = generate_rust_const(const_name, display_name, sampled)
        consts.append(rust_const)
        const_names.append(const_name)

        print(f"  {map_file.name} -> {const_name} ({display_name})")

    # Output all constants
    print("\n// === Xfractint Palettes ===")
    print("    // Imported from xfractint-20.04p16/maps/")
    for const in consts:
        print()
        print(const)

    # Output ALL array additions
    print("\n    // Add to ALL array:")
    for name in const_names:
        print(f"        Self::{name},")


if __name__ == "__main__":
    main()
