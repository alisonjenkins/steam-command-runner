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
    fn empty_arguments_still_get_a_target() {
        let out = apply(Vec::new(), &target());
        assert_eq!(value_after(&out, "-W").as_deref(), Some("1280"));
    }
}
