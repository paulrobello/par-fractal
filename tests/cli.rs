//! ENH-003: the clap CLI surface — validation and semantics pinned without
//! launching the app.

use clap::Parser;
use par_fractal::cli::Cli;

#[test]
fn all_flags_parse() {
    let cli = Cli::try_parse_from([
        "par-fractal",
        "--preset",
        "Mandelbulb Classic",
        "--quality",
        "high",
        "--screenshot-delay",
        "8",
        "--exit-delay",
        "12",
        "--screenshot-path",
        "/tmp/out.png",
        "--profile-dump",
        "/tmp/profile.yaml",
        "--window-size",
        "256x256",
        "--resize-after",
        "2",
        "512x512",
        "--switch-after",
        "Hopalong2D",
        "5",
        "--camera-pos",
        "1.5,-2,9",
    ])
    .expect("all flags valid");

    assert_eq!(cli.preset.as_deref(), Some("Mandelbulb Classic"));
    assert_eq!(cli.quality, Some(1));
    assert_eq!(cli.screenshot_delay, Some(8.0));
    assert_eq!(cli.exit_delay, Some(12.0));
    assert_eq!(cli.window_size, Some((256, 256)));
    assert_eq!(
        cli.resize_after.as_deref(),
        Some(&["2".to_string(), "512x512".to_string()][..])
    );
    assert_eq!(
        cli.switch_after.as_deref(),
        Some(&["Hopalong2D".to_string(), "5".to_string()][..])
    );
    assert_eq!(cli.camera_pos, Some([1.5, -2.0, 9.0]));
    assert!(!cli.clear_settings);
    assert!(!cli.list_presets);
}

#[test]
fn quality_rejects_invalid_level() {
    let err = Cli::try_parse_from(["par-fractal", "--quality", "nonsense"]).unwrap_err();
    assert!(err.to_string().contains("invalid quality level"), "{err}");
}

#[test]
fn negative_delay_is_rejected() {
    for flag in ["--screenshot-delay", "--exit-delay"] {
        // `-1` bare would lex as a flag; the `=-1` form reaches the value
        // parser and gets the clear validation message.
        let err = Cli::try_parse_from(["par-fractal", &format!("{flag}=-1")]).unwrap_err();
        assert!(
            err.to_string().contains("delay must be >= 0"),
            "{flag}: {err}"
        );
    }
}

#[test]
fn non_finite_delay_is_rejected() {
    for value in ["inf", "nan"] {
        let err = Cli::try_parse_from(["par-fractal", "--exit-delay", value]).unwrap_err();
        assert!(
            err.to_string().contains("delay must be finite"),
            "{value}: {err}"
        );
    }
}

#[test]
fn zero_delay_is_accepted() {
    let cli = Cli::try_parse_from(["par-fractal", "--exit-delay", "0"]).expect("0 is valid");
    assert_eq!(cli.exit_delay, Some(0.0));
}

#[test]
fn window_size_rejects_bad_shapes() {
    for value in ["256", "axb", "0x0", "256x"] {
        let err = Cli::try_parse_from(["par-fractal", "--window-size", value]).unwrap_err();
        assert!(err.to_string().contains("size"), "{value}: {err}");
    }
}

#[test]
fn missing_value_is_an_error() {
    assert!(Cli::try_parse_from(["par-fractal", "--preset"]).is_err());
    assert!(Cli::try_parse_from(["par-fractal", "--screenshot-delay"]).is_err());
    assert!(Cli::try_parse_from(["par-fractal", "--window-size"]).is_err());
}

#[test]
fn completions_flag_parses_even_though_hidden() {
    let cli = Cli::try_parse_from(["par-fractal", "--generate-completions", "bash"])
        .expect("hidden flag parses");
    assert_eq!(cli.generate_completions, Some(clap_complete::Shell::Bash));
}

#[test]
fn help_lists_all_fourteen_options() {
    let err = Cli::try_parse_from(["par-fractal", "--help"]).unwrap_err();
    use clap::error::ErrorKind;
    assert_eq!(err.kind(), ErrorKind::DisplayHelp);
    let help = err.to_string();
    // --generate-completions is intentionally hidden (agent helper) and is
    // covered by completions_flag_parses_even_though_hidden.
    for flag in [
        "--clear-settings",
        "--preset",
        "--quality",
        "--list-presets",
        "--screenshot-delay",
        "--screenshot-path",
        "--exit-delay",
        "--profile-dump",
        "--window-size",
        "--resize-after",
        "--switch-after",
        "--camera-pos",
        "--help",
    ] {
        assert!(help.contains(flag), "help missing {flag}:\n{help}");
    }
}
