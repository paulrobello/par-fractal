# Contributing to Par Fractal

Contributions are welcome. This guide covers setup, verification, and the expectations for pull requests so fixes and features land cleanly.

## Prerequisites

- **Rust 1.85+** (Edition 2024). Check with `rustc --version`.
- A modern GPU with drivers for Vulkan, Metal, or DirectX 12.
- **Linux only:** install Vulkan dev packages first with `make install-deps`.

## Setup

```bash
git clone https://github.com/paulrobello/par-fractal.git
cd par-fractal
make build
```

For day-to-day work, run in release mode — debug builds are too slow for meaningful rendering feedback:

```bash
make r
```

## Verification Gate

`make checkall` MUST pass before every pull request. It runs formatting, clippy with auto-fix, and the full test suite. This is required, not optional.

```bash
make checkall
```

If a check fails, fix the underlying issue rather than silencing it. A failing lint or test usually points to a real problem in the change.

## Testing

Add tests for numeric and layout-sensitive code. Two established patterns to follow:

- `src/fractal/tests.rs` — parameter conversion and palette tests
- `src/renderer/uniforms.rs` — uniform buffer layout tests (the Rust struct must stay in lockstep with the WGSL `Uniforms` struct)

Run the full suite with `make test`, or target a single test:

```bash
cargo test <test_name>
```

Use `--nocapture` to see `println!` output from a failing test:

```bash
cargo test <test_name> -- --nocapture
```

### Visual regression (deep-zoom harness, ENH-007)

The CI-safe **CPU teeth** live in `tests/reference_math.rs` (+ `src/reference.rs`): an f64
reference renderer and a Rust mirror of the shader's double-float math. These run under
`make test` with no GPU. When changing the shader's DF primitives, smooth-coloring formula, or
escape semantics, the `render_*_df` / `render` mirrors in `src/reference.rs` must be updated to
match — the DF-vs-f64 tests will tell you if the mirror drifted.

The **GPU golden-image layer** is local-only (CI has no GPU): `make visual-test` renders each
row of `tests/golden/manifest.txt` through the real binary and compares against the committed
`tests/golden/*.png` tiles. When a rendering change legitimately moves the goldens, re-bless
and include the before/after in the PR:

```bash
make visual-bless   # rewrites tests/golden/*.png from the current binary
```


## Code Style

- Run `make fmt` (cargo fmt) before committing.
- `make clippy` must be warning-free.
- Match the style of surrounding code; do not reformat unrelated lines in the same commit.

## Commits and Branches

- Keep commits atomic — one logical change each.
- Follow the existing git log prefixes: `fix:`, `feat:`, `chore:`, `docs:`, `refactor:`, `test:`.
- Branch from `main` and rebase onto the latest `main` before opening a pull request.

## Pull Requests

- One logical change per PR.
- Describe what changed and why. Call out anything that touches the uniform buffer or shader layout — those changes ripple into `shaders/fractal.wgsl` and must be tested across fractal types.
- Run `make checkall` locally before pushing.

## Adding a New Fractal

Follow the "Adding a New Fractal" checklist in [`CLAUDE.md`](CLAUDE.md). It covers the `FractalType` enum, the WGSL distance estimator, UI controls, presets, and the test steps.

## Documentation Updates

For user-visible changes, keep these in sync so documentation does not drift:

- `README.md`
- `docs/FEATURES.md`
- The in-app About page (the "What's New" section must match the changelog)

Add a `CHANGELOG.md` entry under `[Unreleased]` when the change affects users.

## License

Contributions are licensed under the project's [MIT License](LICENSE). By submitting a pull request, you agree your contributions are licensed the same way.

## Related Documentation

- [docs/README.md](docs/README.md) — documentation index
- [CLAUDE.md](CLAUDE.md) — architecture, uniform-buffer rules, and fractal checklist
- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) — detailed architecture and data flow
