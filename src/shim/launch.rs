//! Decide how the shim launches a game, and prepare a launch without gamescope.

use crate::config::OverlayPolicy;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arch {
    X86_64,
    I386,
}

impl Arch {
    fn overlay_suffix(self) -> &'static str {
        match self {
            Arch::X86_64 => "ubuntu12_64/gameoverlayrenderer.so",
            Arch::I386 => "ubuntu12_32/gameoverlayrenderer.so",
        }
    }

    fn other(self) -> Arch {
        match self {
            Arch::X86_64 => Arch::I386,
            Arch::I386 => Arch::X86_64,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaunchMode {
    Gamescope,
    Direct,
}

pub fn launch_mode(
    gamescope_enabled: bool,
    bypass_when_streaming: bool,
    streaming: bool,
) -> LaunchMode {
    if !gamescope_enabled || (streaming && bypass_when_streaming) {
        LaunchMode::Direct
    } else {
        LaunchMode::Gamescope
    }
}

const PE_MACHINE_AMD64: u16 = 0x8664;
const PE_MACHINE_I386: u16 = 0x014C;
const ELF_CLASS_32: u8 = 1;
const ELF_CLASS_64: u8 = 2;
/// Offset of the PE header pointer (`e_lfanew`) in the DOS header.
const PE_HEADER_POINTER_OFFSET: u64 = 0x3C;

/// Architecture of a PE or ELF binary, or `None` for anything else.
pub fn binary_arch(path: &Path) -> Option<Arch> {
    let mut file = File::open(path).ok()?;
    let mut magic = [0u8; 5];
    file.read_exact(&mut magic).ok()?;

    match magic {
        [0x7F, b'E', b'L', b'F', ELF_CLASS_64] => Some(Arch::X86_64),
        [0x7F, b'E', b'L', b'F', ELF_CLASS_32] => Some(Arch::I386),
        [b'M', b'Z', ..] => pe_arch(&mut file),
        _ => None,
    }
}

fn pe_arch(file: &mut File) -> Option<Arch> {
    let mut pointer = [0u8; 4];
    file.seek(SeekFrom::Start(PE_HEADER_POINTER_OFFSET)).ok()?;
    file.read_exact(&mut pointer).ok()?;

    let mut header = [0u8; 6];
    file.seek(SeekFrom::Start(u64::from(u32::from_le_bytes(pointer))))
        .ok()?;
    file.read_exact(&mut header).ok()?;

    match header {
        [b'P', b'E', 0, 0, lo, hi] => match u16::from_le_bytes([lo, hi]) {
            PE_MACHINE_AMD64 => Some(Arch::X86_64),
            PE_MACHINE_I386 => Some(Arch::I386),
            _ => None,
        },
        _ => None,
    }
}

/// The game binary in Steam's `%command%` chain.
///
/// Proton chains end in `proton waitforexitandrun <game>.exe`; native chains
/// run the game right after the runtime's final `--`.
pub fn find_game_binary(command: &[String]) -> Option<PathBuf> {
    let exe = command.iter().rev().map(Path::new).find(|arg| {
        arg.extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("exe"))
            && arg.is_file()
    });
    if let Some(exe) = exe {
        return Some(exe.to_path_buf());
    }

    let last_separator = command.iter().rposition(|arg| arg == "--")?;
    let candidate = Path::new(command.get(last_separator.checked_add(1)?)?);
    binary_arch(candidate).map(|_| candidate.to_path_buf())
}

/// Which overlay build to keep; `None` keeps both.
pub fn overlay_to_keep(policy: OverlayPolicy, game_arch: Option<Arch>) -> Option<Arch> {
    match policy {
        OverlayPolicy::Auto => game_arch,
        OverlayPolicy::Both => None,
        OverlayPolicy::X86_64 => Some(Arch::X86_64),
        OverlayPolicy::I386 => Some(Arch::I386),
    }
}

/// Drop the other architecture's Steam overlay from an `LD_PRELOAD` value.
pub fn filter_overlay(ld_preload: &str, keep: Option<Arch>) -> String {
    let Some(keep) = keep else {
        return ld_preload.to_string();
    };
    let drop_suffix = keep.other().overlay_suffix();
    ld_preload
        .split(':')
        .filter(|entry| !entry.ends_with(drop_suffix))
        .collect::<Vec<_>>()
        .join(":")
}

/// Command line for running the game without gamescope.
pub fn direct_command(
    pre_command: Option<&str>,
    command: &[String],
    game_args: Option<&str>,
) -> Vec<String> {
    let mut result: Vec<String> = pre_command.and_then(shlex::split).unwrap_or_default();
    result.extend_from_slice(command);
    result.extend(game_args.and_then(shlex::split).unwrap_or_default());
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn pe_file(machine: u16) -> tempfile::NamedTempFile {
        let mut bytes = vec![0u8; 0x40];
        bytes[0] = b'M';
        bytes[1] = b'Z';
        bytes[0x3C..0x40].copy_from_slice(&0x40u32.to_le_bytes());
        bytes.extend_from_slice(b"PE\0\0");
        bytes.extend_from_slice(&machine.to_le_bytes());
        bytes.extend_from_slice(&[0u8; 32]);
        let mut file = tempfile::Builder::new().suffix(".exe").tempfile().unwrap();
        file.write_all(&bytes).unwrap();
        file
    }

    fn elf_file(class: u8) -> tempfile::NamedTempFile {
        let mut bytes = vec![0x7F, b'E', b'L', b'F', class];
        bytes.extend_from_slice(&[0u8; 59]);
        let mut file = tempfile::NamedTempFile::new().unwrap();
        file.write_all(&bytes).unwrap();
        file
    }

    fn path_of(file: &tempfile::NamedTempFile) -> String {
        file.path().to_string_lossy().into_owned()
    }

    fn strings(raw: &[&str]) -> Vec<String> {
        raw.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn gamescope_when_not_streaming() {
        assert_eq!(launch_mode(true, true, false), LaunchMode::Gamescope);
    }

    #[test]
    fn direct_when_streaming() {
        assert_eq!(launch_mode(true, true, true), LaunchMode::Direct);
    }

    #[test]
    fn gamescope_when_streaming_but_bypass_disabled() {
        assert_eq!(launch_mode(true, false, true), LaunchMode::Gamescope);
    }

    #[test]
    fn direct_when_gamescope_disabled() {
        assert_eq!(launch_mode(false, true, false), LaunchMode::Direct);
        assert_eq!(launch_mode(false, false, true), LaunchMode::Direct);
    }

    #[test]
    fn reads_pe_machine() {
        assert_eq!(binary_arch(pe_file(0x8664).path()), Some(Arch::X86_64));
        assert_eq!(binary_arch(pe_file(0x014C).path()), Some(Arch::I386));
        assert_eq!(binary_arch(pe_file(0xAA64).path()), None);
    }

    #[test]
    fn reads_elf_class() {
        assert_eq!(binary_arch(elf_file(2).path()), Some(Arch::X86_64));
        assert_eq!(binary_arch(elf_file(1).path()), Some(Arch::I386));
    }

    #[test]
    fn unknown_or_missing_files_have_no_arch() {
        let mut script = tempfile::NamedTempFile::new().unwrap();
        script.write_all(b"#!/usr/bin/env python3\n").unwrap();
        assert_eq!(binary_arch(script.path()), None);
        assert_eq!(binary_arch(Path::new("/nonexistent/game.exe")), None);
    }

    #[test]
    fn finds_the_exe_in_a_proton_chain() {
        let exe = pe_file(0x8664);
        let command = strings(&[
            "/steam/ubuntu12_32/steam-launch-wrapper",
            "--",
            "/steam/ubuntu12_32/reaper",
            "SteamLaunch",
            "AppId=553850",
            "--",
            "/steam/SteamLinuxRuntime_4/_v2-entry-point",
            "--verb=waitforexitandrun",
            "--",
            "/steam/compatibilitytools.d/Proton/proton",
            "waitforexitandrun",
            &path_of(&exe),
            "--bundle-dir",
            "data",
        ]);

        assert_eq!(find_game_binary(&command), Some(exe.path().to_path_buf()));
    }

    #[test]
    fn finds_the_binary_after_the_last_separator_in_a_native_chain() {
        let game = elf_file(2);
        let command = strings(&[
            "/steam/ubuntu12_32/reaper",
            "SteamLaunch",
            "--",
            "/steam/SteamLinuxRuntime_4/_v2-entry-point",
            "--verb=waitforexitandrun",
            "--",
            &path_of(&game),
            "-windowed",
        ]);

        assert_eq!(find_game_binary(&command), Some(game.path().to_path_buf()));
    }

    #[test]
    fn no_binary_in_an_unrecognisable_chain() {
        let command = strings(&["/nonexistent/launcher", "--", "/nonexistent/game.exe"]);
        assert_eq!(find_game_binary(&command), None);
    }

    #[test]
    fn auto_keeps_the_game_arch() {
        assert_eq!(
            overlay_to_keep(OverlayPolicy::Auto, Some(Arch::X86_64)),
            Some(Arch::X86_64)
        );
        assert_eq!(
            overlay_to_keep(OverlayPolicy::Auto, Some(Arch::I386)),
            Some(Arch::I386)
        );
        assert_eq!(overlay_to_keep(OverlayPolicy::Auto, None), None);
    }

    #[test]
    fn explicit_policies_ignore_the_game_arch() {
        assert_eq!(
            overlay_to_keep(OverlayPolicy::Both, Some(Arch::X86_64)),
            None
        );
        assert_eq!(
            overlay_to_keep(OverlayPolicy::I386, Some(Arch::X86_64)),
            Some(Arch::I386)
        );
        assert_eq!(
            overlay_to_keep(OverlayPolicy::X86_64, None),
            Some(Arch::X86_64)
        );
    }

    const STEAM_PRELOAD: &str = "/nix/store/x-filter/lib/libfilter.so:\
/steam/ubuntu12_32/gameoverlayrenderer.so:\
/steam/ubuntu12_64/gameoverlayrenderer.so";

    #[test]
    fn keeping_x86_64_drops_the_32_bit_overlay() {
        assert_eq!(
            filter_overlay(STEAM_PRELOAD, Some(Arch::X86_64)),
            "/nix/store/x-filter/lib/libfilter.so:/steam/ubuntu12_64/gameoverlayrenderer.so"
        );
    }

    #[test]
    fn keeping_i386_drops_the_64_bit_overlay() {
        assert_eq!(
            filter_overlay(STEAM_PRELOAD, Some(Arch::I386)),
            "/nix/store/x-filter/lib/libfilter.so:/steam/ubuntu12_32/gameoverlayrenderer.so"
        );
    }

    #[test]
    fn keeping_both_changes_nothing() {
        assert_eq!(filter_overlay(STEAM_PRELOAD, None), STEAM_PRELOAD);
    }

    #[test]
    fn direct_command_orders_pre_command_game_and_args() {
        let command = strings(&["/steam/reaper", "--", "/game.exe"]);
        assert_eq!(
            direct_command(
                Some("obs-gamecapture gamemoderun"),
                &command,
                Some("--skip-intro")
            ),
            strings(&[
                "obs-gamecapture",
                "gamemoderun",
                "/steam/reaper",
                "--",
                "/game.exe",
                "--skip-intro"
            ])
        );
    }

    #[test]
    fn direct_command_without_extras_is_the_game_command() {
        let command = strings(&["/steam/reaper", "--", "/game.exe"]);
        assert_eq!(direct_command(None, &command, None), command);
    }
}
