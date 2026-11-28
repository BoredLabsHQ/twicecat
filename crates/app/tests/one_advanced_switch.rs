//! One switch reveals every setting (UI review fixes, Task 12, D8): the
//! **Show advanced settings** switch is read by the Settings pages and by
//! `open_audio_controls`, and by no other card, so there is one switch and
//! one place. The old button to a separate audio card is gone.

use std::fs;
use std::path::Path;

fn sources(dir: &Path, out: &mut Vec<(String, String)>) {
    for entry in fs::read_dir(dir).expect("a readable source dir") {
        let path = entry.expect("a dir entry").path();
        if path.is_dir() {
            sources(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            let text = fs::read_to_string(&path).expect("a readable source file");
            out.push((path.display().to_string(), text));
        }
    }
}

/// The code of a file without its comment lines and without its unit tests
/// (the `#[cfg(test)]` module closes every file of this crate).
fn code_of(text: &str) -> String {
    let code = text.split("#[cfg(test)]").next().unwrap_or(text);
    code.lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn app_sources() -> Vec<(String, String)> {
    let mut files = Vec::new();
    sources(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
        &mut files,
    );
    files
}

/// The files that may name the switch: the Settings pages (`settings.rs`,
/// the Video page) and the audio opener.
const MAY_READ: [&str; 4] = [
    "src/settings.rs",
    "src/screens/settings.rs",
    "src/screens/video_settings.rs",
    "src/audio_settings.rs",
];

#[test]
fn only_settings_code_and_the_audio_opener_read_the_advanced_switch() {
    let mut offenders = Vec::new();
    for (name, text) in app_sources() {
        if MAY_READ.iter().any(|ok| name.ends_with(ok)) {
            continue;
        }
        if code_of(&text).contains("advanced_settings") {
            offenders.push(name);
        }
    }
    assert!(offenders.is_empty(), "another card reads it: {offenders:?}");
}

#[test]
fn in_the_audio_module_only_open_audio_controls_reads_the_switch() {
    let (_, text) = app_sources()
        .into_iter()
        .find(|(name, _)| name.ends_with("src/audio_settings.rs"))
        .expect("the audio module");
    let code = code_of(&text);
    let hits: Vec<usize> = code
        .match_indices("advanced_settings")
        .map(|(at, _)| at)
        .collect();
    assert!(!hits.is_empty(), "the opener reads the switch");
    for at in hits {
        let owner = code[..at].rsplit("fn ").next().unwrap_or("");
        assert!(
            owner.starts_with("open_audio_controls"),
            "read in `{}`",
            owner.lines().next().unwrap_or("")
        );
    }
}

#[test]
fn there_is_no_more_audio_options_button() {
    for (name, text) in app_sources() {
        assert!(
            !text.contains("More audio options"),
            "{name} still has the button to the separate audio card"
        );
    }
}
