# ENH-003 — clap-based CLI: tested parser, validation, shell completions

**Priority**: low · **Estimate**: M · **Card**: filed as `[ENH-003]` with the `enhancement` tag
**Subsumes**: ARC-038 (table-driven parser, delay validation). **Feeds**: QA-035 (removes `main`'s
cx-46 loop), DOC-004 (the CLI table can be generated).

## Goal

Replace the hand-rolled argument parser in `src/main.rs:157-416` with `clap` derive, so parsing is
tested, delays are validated, `--help` is generated from one definition, and users get shell
completions.

## Current state

- `main.rs` parses args in a hand-written loop (cx 46): every flag repeats the same
  "read next arg, validate, else eprintln + help + exit" pattern; delays accept negative and
  non-finite values; the help text is maintained by hand next to the parsing code; the options
  appear in three partially-stale doc copies (DOC-004).
- CLI surface: 14 options (see `print_help`, `main.rs:29-53`) — this is the definition source.

## Implementation

1. Add `clap = { version = "4", features = ["derive", "string"] }` (pin the minor in one dep
   bump; check no conflict with existing deps).
2. Define `#[derive(Parser)] struct Cli` with all 14 options, keeping current long names and
   short forms (`-q`, `-h`); defaults and `value_parser`s mirroring existing clamps
   (`--quality` enum low|medium|high|ultra; delays `f64` with a validator finite ≥ 0).
3. Keep `--clear-settings`, `--preset`, `--list-presets` semantics identical (early-exit paths).
4. Replace `main.rs:157-416` with `Cli::parse()`; keep `print_help` deleted (clap generates it).
   Preserve the current help wording where possible via per-arg `help` strings — copy them from
   `print_help` so user-visible text doesn't change materially.
5. Extract nothing else — the CLI struct becomes `AppOptions`' source (ARC-032) via `From<Cli>`.
6. Shell completions: `clap_complete` behind a hidden `--generate-completions <shell>` flag
   (bash/zsh/fish). Optional; include if trivial.
7. Tests: `Cli::try_parse_from` cases for every flag (valid, missing value, invalid enum,
   negative/non-finite delay rejected), plus an integration check that `--help` exits 0 and lists
   all 14 options.
8. DOC-004 consumes this: generate the FEATURES.md table from the struct (or hand-write it once
   from the final `--help`).

## Files touched

`src/main.rs` (parser replaced, ~260 lines removed), `Cargo.toml`, new `tests/cli.rs`,
`docs/FEATURES.md` (via DOC-004), `docs/RELEASING.md` mention of completions (DOC-017).

## Verify

- `cargo test --test cli` green.
- `par-fractal --help` lists all 14 options with the same meanings.
- `par-fractal --screenshot-delay -1` errors with a clear validation message.
- `--preset`, `--list-presets`, `--clear-settings` behave exactly as before (manual check).
- Web build unaffected (clap is native-only; gate the dep under the native target table).

## Rollback

Revert restores the hand parser; no persisted state involved.
