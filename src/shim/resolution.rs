//! Render a streamed game at the client's resolution.
//!
//! A game launched directly, without gamescope, renders at whatever
//! resolution it saved last, not at the size of the display it is streamed
//! from. HD2 kept a hand-set 1440x900 and stretched it into a 1728x1080
//! window, and a fixed setting cannot suit a 1280x800 Deck and a 1728x1080
//! Mac at once.
//!
//! Most engines take a resolution on the command line that overrides the
//! saved one, so the engine is recognised from the game's files and the
//! matching arguments appended. That covers any game on those engines,
//! installed or not. Some engines save the size they ran at on exit, so what
//! they had saved is recorded first and put back afterwards. Games on other
//! engines get a rule that rewrites the setting in their own file for the
//! length of the run.

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Engine {
    Unity,
    Unreal,
    Godot,
    Source,
}

/// Recognise the engine from the files around the game's binary.
pub fn detect_engine(game: &Path) -> Option<Engine> {
    let dir = game.parent()?;
    let stem = game.file_stem()?.to_str()?;

    // Unity: <name>_Data beside <name>.exe.
    if dir.join(format!("{stem}_Data")).is_dir() {
        return Some(Engine::Unity);
    }
    // Unreal: the real binary lives in <Project>/Binaries/Win64, and the
    // launcher stub at the root sits beside an Engine directory.
    let path = game.to_string_lossy();
    if path.contains("/Binaries/Win64/") || dir.join("Engine").is_dir() {
        return Some(Engine::Unreal);
    }
    // Godot: an exported game ships a .pck beside the executable.
    if dir.join(format!("{stem}.pck")).is_file() {
        return Some(Engine::Godot);
    }
    // Source: gameinfo.txt in a mod directory beside the launcher.
    if fs::read_dir(dir)
        .ok()?
        .flatten()
        .any(|e| e.path().join("gameinfo.txt").is_file())
    {
        return Some(Engine::Source);
    }
    None
}

/// Command-line arguments that make the engine render at width x height.
pub fn engine_args(engine: Engine, width: u32, height: u32) -> Vec<String> {
    let (w, h) = (width.to_string(), height.to_string());
    match engine {
        Engine::Unity => vec!["-screen-width".into(), w, "-screen-height".into(), h],
        Engine::Unreal => vec![format!("-ResX={w}"), format!("-ResY={h}")],
        Engine::Godot => vec!["--resolution".into(), format!("{w}x{h}")],
        Engine::Source => vec!["-w".into(), w, "-h".into(), h],
    }
}

/// A game's own settings file, rewritten for the length of a streamed run.
///
/// `pattern` is a regular expression and `replacement` its replacement, in
/// which `{width}` and `{height}` are the client's size and `$1`-style
/// references name the pattern's groups. `file` may use `{prefix}` (the
/// Proton prefix) and `{game_dir}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolutionRule {
    pub file: String,
    pub pattern: String,
    pub replacement: String,
}

/// What a rule replaced, so the file can be put back after the game exits.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Applied {
    pub file: PathBuf,
    pub pattern: String,
    pub originals: Vec<String>,
    /// Restore at the next launch, not at exit: wineserver writes the
    /// registry after the game has gone and would overwrite an early restore.
    #[serde(default)]
    pub deferred: bool,
}

/// Where an engine saves the resolution it ran at. The arguments override
/// the saved size for the run, but Unity saves the running size on exit,
/// and Unreal and Source may too, which would carry the client's size into
/// the next run at the desk.
pub struct Saved {
    pub file: PathBuf,
    pub pattern: &'static str,
    pub deferred: bool,
}

const UNITY_SAVED: &str =
    r#"(?m)^"Screenmanager Resolution (?:Width|Height)(?: Default)?_h\d+"=dword:[0-9a-fA-F]+"#;
const UNITY_NATIVE_SAVED: &str =
    r#"<pref name="Screenmanager Resolution (?:Width|Height)(?: Default)?" type="int">\d+</pref>"#;
const UNREAL_SAVED: &str =
    r"(?m)^(?:LastUserConfirmed)?(?:ResolutionSize[XY]|DesiredScreen(?:Width|Height))=-?\d+";
const SOURCE_SAVED: &str = r#""setting\.defaultres(?:height)?"\s+"\d+""#;

pub fn engine_saves(engine: Engine, game: &Path, prefix: Option<&str>) -> Vec<Saved> {
    let saved = |file: PathBuf, pattern, deferred| Saved {
        file,
        pattern,
        deferred,
    };
    match engine {
        Engine::Unity if is_windows_binary(game) => prefix
            .map(|p| saved(Path::new(p).join("user.reg"), UNITY_SAVED, true))
            .into_iter()
            .collect(),
        Engine::Unity => unity_native_prefs(game)
            .map(|file| saved(file, UNITY_NATIVE_SAVED, false))
            .into_iter()
            .collect(),
        Engine::Unreal => {
            let (Some(prefix), Some(project)) = (prefix, unreal_project(game)) else {
                return Vec::new();
            };
            let config = Path::new(prefix)
                .join("drive_c/users/steamuser/AppData/Local")
                .join(project)
                .join("Saved/Config");
            ["Windows", "WindowsNoEditor"]
                .into_iter()
                .map(|platform| {
                    saved(
                        config.join(platform).join("GameUserSettings.ini"),
                        UNREAL_SAVED,
                        false,
                    )
                })
                .collect()
        }
        Engine::Godot => Vec::new(),
        Engine::Source => game
            .parent()
            .and_then(|dir| fs::read_dir(dir).ok())
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .filter(|mod_dir| mod_dir.join("gameinfo.txt").is_file())
            .map(|mod_dir| saved(mod_dir.join("cfg/video.txt"), SOURCE_SAVED, false))
            .collect(),
    }
}

fn is_windows_binary(game: &Path) -> bool {
    game.extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("exe"))
}

/// A native Unity build keeps its prefs under `~/.config/unity3d`, named by
/// the company and product on the first two lines of `<name>_Data/app.info`.
fn unity_native_prefs(game: &Path) -> Option<PathBuf> {
    let stem = game.file_stem()?.to_str()?;
    let info = fs::read_to_string(game.parent()?.join(format!("{stem}_Data/app.info"))).ok()?;
    let mut lines = info.lines();
    let (company, product) = (lines.next()?, lines.next()?);
    Some(
        dirs::config_dir()?
            .join("unity3d")
            .join(company)
            .join(product)
            .join("prefs"),
    )
}

/// The project an Unreal binary belongs to: the directory above
/// `Binaries`. Steam often launches the stub at the root instead, whose name
/// need not match (Palworld.exe beside Pal/), so for a stub it is the
/// sibling directory holding `Binaries/Win64`, other than `Engine`.
fn unreal_project(game: &Path) -> Option<String> {
    if let Some(binaries) = game
        .ancestors()
        .find(|a| a.file_name() == Some("Binaries".as_ref()))
    {
        return Some(binaries.parent()?.file_name()?.to_str()?.to_string());
    }
    let sibling = fs::read_dir(game.parent()?)
        .ok()
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .find(|p| p.file_name() != Some("Engine".as_ref()) && p.join("Binaries/Win64").is_dir());
    match sibling {
        Some(dir) => Some(dir.file_name()?.to_str()?.to_string()),
        None => Some(game.file_stem()?.to_str()?.to_string()),
    }
}

/// Record what a file holds now, to put back later without changing it.
/// `None` when the file or the values are not there yet.
pub fn snapshot(saved: &Saved) -> Option<Applied> {
    let pattern = Regex::new(saved.pattern).ok()?;
    let text = fs::read_to_string(&saved.file).ok()?;
    let originals: Vec<String> = pattern
        .find_iter(&text)
        .map(|m| m.as_str().to_string())
        .collect();
    (!originals.is_empty()).then(|| Applied {
        file: saved.file.clone(),
        pattern: saved.pattern.to_string(),
        originals,
        deferred: saved.deferred,
    })
}

pub fn expand_path(
    template: &str,
    prefix: Option<&str>,
    game_dir: Option<&str>,
) -> Option<PathBuf> {
    let mut path = template.to_string();
    if path.contains("{prefix}") {
        path = path.replace("{prefix}", prefix?);
    }
    if path.contains("{game_dir}") {
        path = path.replace("{game_dir}", game_dir?);
    }
    if let Some(rest) = path.strip_prefix("~/") {
        path = format!("{}/{rest}", std::env::var("HOME").ok()?);
    }
    Some(PathBuf::from(path))
}

/// Expand one match's replacement. Group references expand piece by piece
/// around the size: substituted first, `$1{width}` would read as `$11280`.
fn expand(caps: &regex::Captures, replacement: &str, width: u32, height: u32) -> String {
    let mut out = String::new();
    let mut rest = replacement;
    loop {
        let next = [("{width}", width), ("{height}", height)]
            .into_iter()
            .filter_map(|(token, value)| rest.find(token).map(|at| (at, token, value)))
            .min_by_key(|&(at, _, _)| at);
        let Some((at, token, value)) = next else {
            caps.expand(rest, &mut out);
            return out;
        };
        let (before, after) = rest.split_at(at);
        caps.expand(before, &mut out);
        out.push_str(&value.to_string());
        rest = after.strip_prefix(token).unwrap_or(after);
    }
}

/// Replace every match of `pattern` in `text`, returning the new text and
/// what each match was.
pub fn rewrite(
    text: &str,
    pattern: &Regex,
    replacement: &str,
    width: u32,
    height: u32,
) -> (String, Vec<String>) {
    let originals = pattern
        .find_iter(text)
        .map(|m| m.as_str().to_string())
        .collect();
    let new_text = pattern
        .replace_all(text, |caps: &regex::Captures| {
            expand(caps, replacement, width, height)
        })
        .into_owned();
    (new_text, originals)
}

/// Put back what `rewrite` replaced, match by match, in the file as it is
/// now. Only the rewritten values revert: anything else the player changed
/// during the run is kept.
pub fn restore(text: &str, pattern: &Regex, originals: &[String]) -> String {
    let mut originals = originals.iter();
    pattern
        .replace_all(text, |caps: &regex::Captures| {
            originals
                .next()
                .cloned()
                .unwrap_or_else(|| caps[0].to_string())
        })
        .into_owned()
}

/// Apply a rule to its file. `Err` carries a message for the shim log.
pub fn apply_rule(
    rule: &ResolutionRule,
    file: &Path,
    width: u32,
    height: u32,
) -> Result<Applied, String> {
    let pattern = Regex::new(&rule.pattern).map_err(|e| format!("bad pattern: {e}"))?;
    let text = fs::read_to_string(file).map_err(|e| format!("{}: {e}", file.display()))?;
    let (new_text, originals) = rewrite(&text, &pattern, &rule.replacement, width, height);
    if originals.is_empty() {
        return Err(format!("{}: pattern matched nothing", file.display()));
    }
    // undo finds the values again by the same pattern; if the rewrite hides
    // them, the originals could never go back.
    if pattern.find_iter(&new_text).count() != originals.len() {
        return Err(format!(
            "{}: replacement {:?} leaves text the pattern cannot find again, not applied",
            file.display(),
            rule.replacement
        ));
    }
    fs::write(file, new_text).map_err(|e| format!("{}: {e}", file.display()))?;
    Ok(Applied {
        file: file.to_path_buf(),
        pattern: rule.pattern.clone(),
        originals,
        deferred: false,
    })
}

/// Where a launch records what it rewrote until the game exits.
pub fn journal_path(app_id: u32) -> Option<PathBuf> {
    Some(
        dirs::state_dir()?
            .join("steam-command-runner")
            .join(format!("resolution-{app_id}.json")),
    )
}

/// Puts every rewritten file back when dropped, so an early return (a game
/// that fails to spawn) cannot leave the stream size saved as the player's.
///
/// The journal covers what Drop cannot: a shim killed with the game. Left
/// behind, it is replayed by `recover` at the game's next launch; otherwise
/// that launch would save the stream size as the original and lose the
/// player's own for good.
pub struct Restore {
    applied: Vec<Applied>,
    journal: Option<PathBuf>,
    /// Recovering at a later launch: the game and its wineserver are gone,
    /// so deferred entries are restored too.
    recovering: bool,
    log: fn(&str),
}

impl Restore {
    pub fn new(applied: Vec<Applied>, journal: Option<PathBuf>, log: fn(&str)) -> Self {
        let journal = journal.filter(|_| !applied.is_empty());
        if let Some(path) = &journal {
            if let Err(e) = write_journal(path, &applied) {
                log(&format!(
                    "could not record the rewrite in {}, a killed shim will not restore it: {e}",
                    path.display()
                ));
            }
        }
        Self {
            applied,
            journal,
            recovering: false,
            log,
        }
    }

    /// Restore what a launch that never finished left rewritten.
    pub fn recover(journal: PathBuf, log: fn(&str)) {
        let text = match fs::read_to_string(&journal) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return,
            Err(e) => return log(&format!("{}: {e}", journal.display())),
        };
        match serde_json::from_str::<Vec<Applied>>(&text) {
            Ok(applied) => {
                log(&format!(
                    "restoring the resolution a previous launch left in {} file(s)",
                    applied.len()
                ));
                drop(Self {
                    applied,
                    journal: Some(journal),
                    recovering: true,
                    log,
                });
            }
            Err(e) => {
                log(&format!("{}: unreadable, removing: {e}", journal.display()));
                if let Err(e) = fs::remove_file(&journal) {
                    log(&format!("{}: {e}", journal.display()));
                }
            }
        }
    }
}

fn write_journal(path: &Path, applied: &[Applied]) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string(applied).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| e.to_string())
}

impl Drop for Restore {
    fn drop(&mut self) {
        let (mut later, now): (Vec<Applied>, Vec<Applied>) = std::mem::take(&mut self.applied)
            .into_iter()
            .partition(|a| a.deferred && !self.recovering);
        // Restoring now would race wineserver's own write, and without a
        // journal there is no next launch to do it at.
        if self.journal.is_none() && !later.is_empty() {
            for skipped in later.drain(..) {
                (self.log)(&format!(
                    "{}: not restored, no app id to defer it under",
                    skipped.file.display()
                ));
            }
        }
        for applied in &now {
            if let Err(e) = undo(applied) {
                (self.log)(&format!("could not restore the resolution: {e}"));
            }
        }
        let Some(path) = &self.journal else { return };
        // Removed even after a failed undo, so a settings file that is gone
        // does not fail every later launch.
        let result = if later.is_empty() {
            fs::remove_file(path).map_err(|e| e.to_string())
        } else {
            write_journal(path, &later)
        };
        if let Err(e) = result {
            (self.log)(&format!("{}: {e}", path.display()));
        }
    }
}

pub fn undo(applied: &Applied) -> Result<(), String> {
    let pattern = Regex::new(&applied.pattern).map_err(|e| format!("bad pattern: {e}"))?;
    let text = fs::read_to_string(&applied.file)
        .map_err(|e| format!("{}: {e}", applied.file.display()))?;
    // Values go back by position; a different count means they would land
    // on the wrong settings.
    let found = pattern.find_iter(&text).count();
    if found != applied.originals.len() {
        return Err(format!(
            "{}: {found} values now, {} recorded, left as is",
            applied.file.display(),
            applied.originals.len()
        ));
    }
    fs::write(&applied.file, restore(&text, &pattern, &applied.originals))
        .map_err(|e| format!("{}: {e}", applied.file.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game_in(dir: &Path, name: &str) -> PathBuf {
        let exe = dir.join(name);
        fs::write(&exe, b"MZ").unwrap();
        exe
    }

    #[test]
    fn recognises_unity() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("Game_Data")).unwrap();
        assert_eq!(
            detect_engine(&game_in(dir.path(), "Game.exe")),
            Some(Engine::Unity)
        );
    }

    #[test]
    fn recognises_unreal_by_binaries_path_and_by_the_root_stub() {
        let dir = tempfile::tempdir().unwrap();
        let bin = dir.path().join("Proj/Binaries/Win64");
        fs::create_dir_all(&bin).unwrap();
        assert_eq!(
            detect_engine(&game_in(&bin, "Proj-Win64-Shipping.exe")),
            Some(Engine::Unreal)
        );
        fs::create_dir(dir.path().join("Engine")).unwrap();
        assert_eq!(
            detect_engine(&game_in(dir.path(), "Proj.exe")),
            Some(Engine::Unreal)
        );
    }

    #[test]
    fn recognises_godot_and_source() {
        let godot = tempfile::tempdir().unwrap();
        fs::write(godot.path().join("Game.pck"), b"").unwrap();
        assert_eq!(
            detect_engine(&game_in(godot.path(), "Game.exe")),
            Some(Engine::Godot)
        );

        let source = tempfile::tempdir().unwrap();
        fs::create_dir(source.path().join("mod")).unwrap();
        fs::write(source.path().join("mod/gameinfo.txt"), b"").unwrap();
        assert_eq!(
            detect_engine(&game_in(source.path(), "hl2.exe")),
            Some(Engine::Source)
        );
    }

    #[test]
    fn an_unknown_layout_is_not_guessed() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(detect_engine(&game_in(dir.path(), "helldivers2.exe")), None);
    }

    #[test]
    fn engine_arguments() {
        assert_eq!(
            engine_args(Engine::Unity, 1280, 800),
            ["-screen-width", "1280", "-screen-height", "800"]
        );
        assert_eq!(
            engine_args(Engine::Unreal, 1280, 800),
            ["-ResX=1280", "-ResY=800"]
        );
        assert_eq!(
            engine_args(Engine::Godot, 1280, 800),
            ["--resolution", "1280x800"]
        );
        assert_eq!(
            engine_args(Engine::Source, 1280, 800),
            ["-w", "1280", "-h", "800"]
        );
    }

    const HD2: &str = "fullscreen = false\n\
        \trender_resolution = [\n\t\t1728\n\t\t1080\n\t]\n\
        screen_resolution = [\n\t1728\n\t1080\n]\nvsync = false\n";
    const HD2_PATTERN: &str = r"(?m)^(\s*(?:screen|render)_resolution = \[\s*)\d+(\s+)\d+";
    const HD2_REPLACEMENT: &str = "${1}{width}${2}{height}";

    #[test]
    fn a_rule_rewrites_and_restores_only_its_values() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("user_settings.config");
        fs::write(&file, HD2).unwrap();
        let rule = ResolutionRule {
            file: String::new(),
            pattern: HD2_PATTERN.into(),
            replacement: HD2_REPLACEMENT.into(),
        };

        let applied = apply_rule(&rule, &file, 1280, 800).unwrap();
        let during = fs::read_to_string(&file).unwrap();
        assert!(during.contains("render_resolution = [\n\t\t1280\n\t\t800"));
        assert!(during.contains("screen_resolution = [\n\t1280\n\t800"));
        assert!(!during.contains("1728"));

        // The player changes something else during the run; it must survive.
        fs::write(&file, during.replace("vsync = false", "vsync = true")).unwrap();
        undo(&applied).unwrap();
        let after = fs::read_to_string(&file).unwrap();
        assert_eq!(after, HD2.replace("vsync = false", "vsync = true"));
    }

    #[test]
    fn bare_group_references_before_the_size_still_expand() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("user_settings.config");
        fs::write(&file, HD2).unwrap();
        let rule = ResolutionRule {
            file: String::new(),
            pattern: HD2_PATTERN.into(),
            replacement: "$1{width}$2{height}".into(),
        };

        let applied = apply_rule(&rule, &file, 1280, 800).unwrap();
        assert!(fs::read_to_string(&file)
            .unwrap()
            .contains("screen_resolution = [\n\t1280\n\t800"));
        undo(&applied).unwrap();
        assert_eq!(fs::read_to_string(&file).unwrap(), HD2);
    }

    #[test]
    fn a_replacement_the_pattern_cannot_find_again_is_not_written() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("user_settings.config");
        fs::write(&file, HD2).unwrap();
        let rule = ResolutionRule {
            file: String::new(),
            pattern: HD2_PATTERN.into(),
            replacement: "{width}x{height}".into(),
        };
        assert!(apply_rule(&rule, &file, 1280, 800).is_err());
        assert_eq!(fs::read_to_string(&file).unwrap(), HD2);
    }

    #[test]
    fn dropping_the_guard_restores_the_file() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("user_settings.config");
        fs::write(&file, HD2).unwrap();
        let rule = ResolutionRule {
            file: String::new(),
            pattern: HD2_PATTERN.into(),
            replacement: HD2_REPLACEMENT.into(),
        };

        let journal = dir.path().join("state/resolution-553850.json");
        let guard = Restore::new(
            vec![apply_rule(&rule, &file, 1280, 800).unwrap()],
            Some(journal.clone()),
            |_| {},
        );
        assert_ne!(fs::read_to_string(&file).unwrap(), HD2);
        assert!(journal.is_file());
        drop(guard);
        assert_eq!(fs::read_to_string(&file).unwrap(), HD2);
        assert!(!journal.exists());
    }

    #[test]
    fn a_killed_launch_is_restored_at_the_next_one() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("user_settings.config");
        fs::write(&file, HD2).unwrap();
        let rule = ResolutionRule {
            file: String::new(),
            pattern: HD2_PATTERN.into(),
            replacement: HD2_REPLACEMENT.into(),
        };
        let journal = dir.path().join("resolution-553850.json");

        // A killed shim never runs Drop.
        std::mem::forget(Restore::new(
            vec![apply_rule(&rule, &file, 1280, 800).unwrap()],
            Some(journal.clone()),
            |_| {},
        ));
        assert_ne!(fs::read_to_string(&file).unwrap(), HD2);

        Restore::recover(journal.clone(), |_| {});
        assert_eq!(fs::read_to_string(&file).unwrap(), HD2);
        assert!(!journal.exists());
        Restore::recover(journal, |_| {});
    }

    #[test]
    fn engines_save_where_their_settings_live() {
        let unreal = engine_saves(
            Engine::Unreal,
            Path::new("/g/Proj/Binaries/Win64/Proj-Win64-Shipping.exe"),
            Some("/pfx"),
        );
        assert_eq!(
            unreal[0].file,
            PathBuf::from(
                "/pfx/drive_c/users/steamuser/AppData/Local/Proj/Saved/Config/Windows/GameUserSettings.ini"
            )
        );
        let stub = engine_saves(Engine::Unreal, Path::new("/g/Proj.exe"), Some("/pfx"));
        assert!(stub[0]
            .file
            .to_string_lossy()
            .contains("/Local/Proj/Saved/"));

        let unity = engine_saves(Engine::Unity, Path::new("/g/Game.exe"), Some("/pfx"));
        assert_eq!(unity[0].file, PathBuf::from("/pfx/user.reg"));
        assert!(unity[0].deferred);
        assert!(engine_saves(Engine::Unity, Path::new("/g/Game.exe"), None).is_empty());
    }

    const UNITY_REG: &str = "[Software\\\\Studio\\\\Game] 1737239046\n\
        \"Screenmanager Fullscreen mode_h3630240806\"=dword:00000001\n\
        \"Screenmanager Resolution Height_h2627697771\"=dword:00000438\n\
        \"Screenmanager Resolution Width_h182942802\"=dword:00000780\n";

    #[test]
    fn unitys_saved_size_goes_back_at_the_next_launch_not_at_exit() {
        let dir = tempfile::tempdir().unwrap();
        let reg = dir.path().join("user.reg");
        fs::write(&reg, UNITY_REG).unwrap();
        let journal = dir.path().join("resolution-1.json");
        let saves = engine_saves(Engine::Unity, Path::new("/g/Game.exe"), dir.path().to_str());
        let taken: Vec<Applied> = saves.iter().filter_map(snapshot).collect();
        assert_eq!(taken[0].originals.len(), 2);

        let guard = Restore::new(taken, Some(journal.clone()), |_| {});
        // Unity saves the streamed 1280x800 on exit.
        let streamed = UNITY_REG
            .replace("00000438", "00000320")
            .replace("00000780", "00000500");
        fs::write(&reg, &streamed).unwrap();
        drop(guard);
        assert_eq!(
            fs::read_to_string(&reg).unwrap(),
            streamed,
            "wineserver may still write it"
        );
        assert!(journal.is_file());

        Restore::recover(journal.clone(), |_| {});
        assert_eq!(fs::read_to_string(&reg).unwrap(), UNITY_REG);
        assert!(!journal.exists());
    }

    #[test]
    fn an_unreal_save_goes_back_at_exit() {
        let dir = tempfile::tempdir().unwrap();
        let ini = dir.path().join("GameUserSettings.ini");
        let original = "[/Script/Engine.GameUserSettings]\nResolutionSizeX=1728\n\
            ResolutionSizeY=1080\nLastUserConfirmedResolutionSizeX=1728\nbUseVSync=False\n";
        fs::write(&ini, original).unwrap();
        let saved = Saved {
            file: ini.clone(),
            pattern: UNREAL_SAVED,
            deferred: false,
        };
        let guard = Restore::new(
            vec![snapshot(&saved).unwrap()],
            Some(dir.path().join("j.json")),
            |_| {},
        );
        fs::write(
            &ini,
            original.replace("1728", "1280").replace("False", "True"),
        )
        .unwrap();
        drop(guard);
        assert_eq!(
            fs::read_to_string(&ini).unwrap(),
            original.replace("False", "True")
        );
    }

    #[test]
    fn an_unreal_stub_finds_its_project_beside_it() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("Engine/Binaries/Win64")).unwrap();
        fs::create_dir_all(dir.path().join("Pal/Binaries/Win64")).unwrap();
        let saves = engine_saves(
            Engine::Unreal,
            &game_in(dir.path(), "Palworld.exe"),
            Some("/pfx"),
        );
        assert!(saves[0]
            .file
            .to_string_lossy()
            .contains("/Local/Pal/Saved/"));
    }

    #[test]
    fn a_native_unity_game_saves_under_its_company_and_product() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("Cactus_Data")).unwrap();
        fs::write(
            dir.path().join("Cactus_Data/app.info"),
            "Witch Beam\nAssault Android Cactus",
        )
        .unwrap();
        let saves = engine_saves(Engine::Unity, &game_in(dir.path(), "Cactus.x86_64"), None);
        assert!(saves[0]
            .file
            .ends_with("unity3d/Witch Beam/Assault Android Cactus/prefs"));
        assert!(!saves[0].deferred);

        let prefs = "\t<pref name=\"Screenmanager Resolution Height\" type=\"int\">1440</pref>\n\
            \t<pref name=\"Screenmanager Resolution Width\" type=\"int\">2560</pref>\n";
        let found = Regex::new(saves[0].pattern)
            .unwrap()
            .find_iter(prefs)
            .count();
        assert_eq!(found, 2);
    }

    #[test]
    fn unreal_confirmed_desired_size_is_recorded_too() {
        let ini = "DesiredScreenWidth=1728\nLastUserConfirmedDesiredScreenWidth=1728\n\
            LastUserConfirmedDesiredScreenHeight=1080\n";
        assert_eq!(Regex::new(UNREAL_SAVED).unwrap().find_iter(ini).count(), 3);
    }

    #[test]
    fn a_deferred_restore_without_a_journal_is_skipped_not_raced() {
        let dir = tempfile::tempdir().unwrap();
        let reg = dir.path().join("user.reg");
        fs::write(&reg, UNITY_REG).unwrap();
        let saves = engine_saves(Engine::Unity, Path::new("/g/Game.exe"), dir.path().to_str());
        let guard = Restore::new(vec![snapshot(&saves[0]).unwrap()], None, |_| {});
        let streamed = UNITY_REG.replace("00000438", "00000320");
        fs::write(&reg, &streamed).unwrap();
        drop(guard);
        assert_eq!(fs::read_to_string(&reg).unwrap(), streamed);
    }

    #[test]
    fn a_changed_number_of_values_is_not_restored_onto_the_wrong_ones() {
        let dir = tempfile::tempdir().unwrap();
        let reg = dir.path().join("user.reg");
        fs::write(&reg, UNITY_REG).unwrap();
        let saves = engine_saves(Engine::Unity, Path::new("/g/Game.exe"), dir.path().to_str());
        let taken = snapshot(&saves[0]).unwrap();
        let grown = format!(
            "\"Screenmanager Resolution Height Default_h1380706816\"=dword:00000320\n{UNITY_REG}"
        );
        fs::write(&reg, &grown).unwrap();
        assert!(undo(&taken).is_err());
        assert_eq!(fs::read_to_string(&reg).unwrap(), grown);
    }

    #[test]
    fn no_saved_settings_yet_is_nothing_to_snapshot() {
        let dir = tempfile::tempdir().unwrap();
        let saved = Saved {
            file: dir.path().join("missing.ini"),
            pattern: UNREAL_SAVED,
            deferred: false,
        };
        assert_eq!(snapshot(&saved), None);
    }

    #[test]
    fn nothing_rewritten_writes_no_journal() {
        let dir = tempfile::tempdir().unwrap();
        let journal = dir.path().join("resolution-553850.json");
        drop(Restore::new(Vec::new(), Some(journal.clone()), |_| {}));
        assert!(!journal.exists());
    }

    #[test]
    fn a_rule_that_matches_nothing_leaves_the_file_alone() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("settings.ini");
        fs::write(&file, "nothing here\n").unwrap();
        let rule = ResolutionRule {
            file: String::new(),
            pattern: HD2_PATTERN.into(),
            replacement: HD2_REPLACEMENT.into(),
        };
        assert!(apply_rule(&rule, &file, 1280, 800).is_err());
        assert_eq!(fs::read_to_string(&file).unwrap(), "nothing here\n");
    }

    #[test]
    fn paths_expand_the_prefix_and_game_dir() {
        assert_eq!(
            expand_path("{prefix}/drive_c/x.ini", Some("/pfx"), None),
            Some(PathBuf::from("/pfx/drive_c/x.ini"))
        );
        assert_eq!(expand_path("{prefix}/x", None, None), None);
        assert_eq!(
            expand_path("{game_dir}/cfg.ini", None, Some("/g")),
            Some(PathBuf::from("/g/cfg.ini"))
        );
    }
}
