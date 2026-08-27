//! Redirect gamescope at a streaming client's display when one is active.
//!
//! Steam Remote Play captures a whole output, so a host with an ultrawide
//! sends a client mostly letterbox. The host side of that problem is solved by
//! giving Steam a virtual output at the client's own resolution and moving the
//! game onto it.
//!
//! That is not enough on its own. gamescope fixes its render resolution from
//! `-W`/`-H` when it starts, so a game launched with the desktop's geometry
//! still renders at that shape and is merely scaled into the smaller output —
//! a 2540x1440 game inside a 1280x800 output is letterboxed exactly as before.
//! The resolution has to be right at launch, and this shim is the only point
//! in Steam's launch chain that sees the arguments.
//!
//! The active target is published by the host-side watcher as a small JSON
//! file, present only while a client is streaming. Its absence is the normal
//! case and means "leave the arguments alone", so desktop play is untouched.

use std::env;
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};

/// Where a streaming client wants the game rendered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamTarget {
    /// niri output name, passed to gamescope as `--prefer-output`.
    pub output: String,
    pub width: u32,
    pub height: u32,
    /// Refresh rate of the target output, if it advertises one.
    pub refresh: Option<u32>,
}

/// Path of the file the host-side watcher publishes, if one is configured.
///
/// The runtime directory is the default because the target is meaningful only
/// for the current login session and should not outlive it.
fn target_path() -> Option<PathBuf> {
    if let Ok(explicit) = env::var("STEAM_COMMAND_RUNNER_STREAM_TARGET") {
        if !explicit.is_empty() {
            return Some(PathBuf::from(explicit));
        }
    }

    let runtime_dir = env::var("XDG_RUNTIME_DIR").ok()?;
    if runtime_dir.is_empty() {
        return None;
    }
    Some(PathBuf::from(runtime_dir).join("stream-mode/target.json"))
}

impl StreamTarget {
    /// Read the published target, or `None` when nothing is streaming.
    ///
    /// A missing file is the ordinary case, not an error. A malformed one is
    /// also treated as "not streaming": refusing to launch the game because a
    /// streaming hint could not be parsed would be a far worse failure than
    /// rendering at the desktop's resolution.
    pub fn detect() -> Option<Self> {
        let path = target_path()?;
        let contents = std::fs::read_to_string(&path).ok()?;
        Self::parse(&contents)
    }

    pub fn parse(contents: &str) -> Option<Self> {
        let value: serde_json::Value = serde_json::from_str(contents).ok()?;
        let output = value.get("output")?.as_str()?.to_string();
        let width = u32::try_from(value.get("width")?.as_u64()?).ok()?;
        let height = u32::try_from(value.get("height")?.as_u64()?).ok()?;
        let refresh = value
            .get("refresh")
            .and_then(|r| r.as_u64())
            .and_then(|r| u32::try_from(r).ok());

        if output.is_empty() || width == 0 || height == 0 {
            return None;
        }

        Some(Self {
            output,
            width,
            height,
            refresh,
        })
    }
}

/// How long to wait for the target output to exist before launching anyway.
///
/// The host-side watcher creates it when a client connects, which is normally
/// well before a game is launched, so this only covers the case where a launch
/// races it. Launching anyway after the wait is deliberate: a game that starts
/// on the wrong display is a poor outcome, but a game that never starts is a
/// worse one.
fn wait_timeout() -> Duration {
    let secs = env::var("STEAM_COMMAND_RUNNER_STREAM_WAIT")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(20);
    Duration::from_secs(secs)
}

/// Is the named output present in niri right now?
///
/// `None` means the question could not be answered — niri missing, IPC not
/// responding — which is treated as "do not wait", since blocking a game
/// launch on an unanswerable question would be worse than launching.
fn output_present(name: &str) -> Option<bool> {
    let output = Command::new("niri")
        .args(["msg", "--json", "outputs"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let parsed: serde_json::Value = serde_json::from_slice(&output.stdout).ok()?;
    Some(parsed.get(name).is_some())
}

/// Block until the target output exists, or the wait runs out.
///
/// Returns whether it is there. gamescope resolves `--prefer-output` when it
/// starts, so launching before the output exists puts the game on the desktop
/// and no later change moves it — the whole point of waiting.
pub fn wait_for_output(name: &str) -> bool {
    if output_present(name) != Some(false) {
        // Present, or unanswerable. Either way there is nothing to wait for.
        return true;
    }

    let deadline = Instant::now() + wait_timeout();
    while Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(250));
        match output_present(name) {
            Some(true) => return true,
            // Stop waiting on an unanswerable question rather than burn the
            // whole timeout on it.
            None => return false,
            Some(false) => {}
        }
    }
    false
}

/// gamescope flags that take a value and that a stream target replaces.
const SIZE_FLAGS: [&str; 4] = ["-w", "-h", "-W", "-H"];
const OUTPUT_FLAGS: [&str; 2] = ["-O", "--prefer-output"];
const REFRESH_FLAGS: [&str; 2] = ["-r", "--nested-refresh"];

/// Rewrite gamescope arguments to render at the streaming client's display.
///
/// Existing size, output and refresh flags are dropped rather than edited in
/// place, because they can be repeated and the last occurrence is the one
/// gamescope honours; appending our own is what makes the result predictable.
/// Every other flag is preserved untouched — HDR, FSR, scaling and the rest
/// are the user's per-game choices and none of our business.
pub fn apply(args: Vec<String>, target: &StreamTarget) -> Vec<String> {
    let mut result: Vec<String> = Vec::with_capacity(args.len() + 8);
    let mut iter = args.into_iter().peekable();

    while let Some(arg) = iter.next() {
        let takes_value = SIZE_FLAGS.contains(&arg.as_str())
            || OUTPUT_FLAGS.contains(&arg.as_str())
            || REFRESH_FLAGS.contains(&arg.as_str());

        if takes_value {
            // Drop the flag and the value that follows it.
            let _ = iter.next();
            continue;
        }

        // Long forms may carry the value inline.
        let inline_dropped = ["--output-width=", "--output-height=", "--nested-width=",
                              "--nested-height=", "--prefer-output=", "--nested-refresh="]
            .iter()
            .any(|prefix| arg.starts_with(prefix));
        if inline_dropped {
            continue;
        }

        result.push(arg);
    }

    result.push("-W".to_string());
    result.push(target.width.to_string());
    result.push("-H".to_string());
    result.push(target.height.to_string());
    // Nested size matches the output size: gamescope is filling a window that
    // is exactly the client's display, so any other value reintroduces the
    // scaling this exists to remove.
    result.push("-w".to_string());
    result.push(target.width.to_string());
    result.push("-h".to_string());
    result.push(target.height.to_string());
    result.push("--prefer-output".to_string());
    result.push(target.output.clone());

    if let Some(refresh) = target.refresh {
        result.push("-r".to_string());
        result.push(refresh.to_string());
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target() -> StreamTarget {
        StreamTarget {
            output: "steam".to_string(),
            width: 1280,
            height: 800,
            refresh: Some(60),
        }
    }

    fn args(raw: &str) -> Vec<String> {
        raw.split_whitespace().map(str::to_string).collect()
    }

    fn value_after(args: &[String], flag: &str) -> Option<String> {
        args.iter()
            .position(|a| a == flag)
            .and_then(|i| args.get(i + 1))
            .cloned()
    }

    #[test]
    fn parses_a_published_target() {
        let parsed = StreamTarget::parse(
            r#"{"output":"steam","width":1280,"height":800,"refresh":60}"#,
        );
        assert_eq!(parsed, Some(target()));
    }

    #[test]
    fn refresh_is_optional() {
        let parsed = StreamTarget::parse(r#"{"output":"steam","width":1280,"height":800}"#)
            .expect("should parse without refresh");
        assert_eq!(parsed.refresh, None);
    }

    #[test]
    fn malformed_target_is_not_streaming() {
        // Refusing to launch because a streaming hint is unreadable would be a
        // worse failure than rendering at the desktop's resolution.
        assert_eq!(StreamTarget::parse("not json"), None);
        assert_eq!(StreamTarget::parse(r#"{"output":"steam"}"#), None);
        assert_eq!(
            StreamTarget::parse(r#"{"output":"","width":1280,"height":800}"#),
            None
        );
        assert_eq!(
            StreamTarget::parse(r#"{"output":"steam","width":0,"height":800}"#),
            None
        );
    }

    #[test]
    fn replaces_the_real_world_desktop_arguments() {
        // Verbatim from a game launched on the ultrawide while streaming.
        let out = apply(
            args("-w 2540 -h 1440 -W 2540 -H 1440 -b --rt --hdr-enabled -F fsr -r 120"),
            &target(),
        );

        assert_eq!(value_after(&out, "-W").as_deref(), Some("1280"));
        assert_eq!(value_after(&out, "-H").as_deref(), Some("800"));
        assert_eq!(value_after(&out, "-w").as_deref(), Some("1280"));
        assert_eq!(value_after(&out, "-h").as_deref(), Some("800"));
        assert_eq!(value_after(&out, "-r").as_deref(), Some("60"));
        assert_eq!(value_after(&out, "--prefer-output").as_deref(), Some("steam"));
        assert!(!out.iter().any(|a| a == "2540"));
        assert!(!out.iter().any(|a| a == "1440"));
    }

    #[test]
    fn preserves_unrelated_flags() {
        let out = apply(
            args("-w 2540 -h 1440 -b --rt --hdr-enabled --hdr-debug-force-support -F fsr"),
            &target(),
        );
        for flag in ["-b", "--rt", "--hdr-enabled", "--hdr-debug-force-support", "-F", "fsr"] {
            assert!(out.iter().any(|a| a == flag), "lost {flag}");
        }
    }

    #[test]
    fn replaces_an_existing_output_preference() {
        // -O DP-2 would send the game back to the physical display.
        let out = apply(args("-O DP-2 -W 2560 -H 1440"), &target());
        assert!(!out.iter().any(|a| a == "DP-2"));
        assert_eq!(value_after(&out, "--prefer-output").as_deref(), Some("steam"));
        assert_eq!(out.iter().filter(|a| *a == "--prefer-output").count(), 1);
    }

    #[test]
    fn replaces_inline_long_forms() {
        let out = apply(
            args("--output-width=2560 --output-height=1440 --prefer-output=DP-2 -b"),
            &target(),
        );
        assert!(!out.iter().any(|a| a.contains("2560")));
        assert!(!out.iter().any(|a| a.contains("DP-2")));
        assert!(out.iter().any(|a| a == "-b"));
    }

    #[test]
    fn adds_sizing_when_none_was_given() {
        let out = apply(args("-b --rt"), &target());
        assert_eq!(value_after(&out, "-W").as_deref(), Some("1280"));
        assert_eq!(value_after(&out, "--prefer-output").as_deref(), Some("steam"));
    }

    #[test]
    fn omits_refresh_when_the_target_has_none() {
        let mut t = target();
        t.refresh = None;
        let out = apply(args("-r 120 -b"), &t);
        assert!(!out.iter().any(|a| a == "-r"));
        assert!(!out.iter().any(|a| a == "120"));
    }

    #[test]
    fn waiting_is_skipped_when_niri_cannot_answer() {
        // Blocking a game launch on an unanswerable question would be worse
        // than launching. With no niri on PATH the probe cannot answer, so
        // this must return promptly rather than burn the timeout.
        let start = std::time::Instant::now();
        let present = wait_for_output("definitely-not-an-output");
        assert!(start.elapsed() < Duration::from_secs(5));
        // Either it answered "not present" and gave up fast, or it could not
        // answer and said so; both are non-blocking.
        let _ = present;
    }

    #[test]
    fn empty_arguments_still_get_a_target() {
        let out = apply(Vec::new(), &target());
        assert_eq!(value_after(&out, "-W").as_deref(), Some("1280"));
    }
}
