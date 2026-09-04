#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::unwrap_in_result,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::unreachable,
    clippy::todo,
    clippy::unimplemented,
    clippy::get_unwrap
)]

use super::{CMD_SWITCH_KEYMAP, MAX_KEYMAP_ABBREVIATION_LEN};
use crate::error::AppError;

/// Builds the firmware's SwitchKeymap command packet: command byte,
/// abbreviation length, then the abbreviation's raw ASCII bytes -- no
/// padding, no report-id prefix. Matches UHK Agent's own `switch-keymap.ts`
/// byte-for-byte, which is the reference implementation confirmed to work
/// against real hardware.
pub fn build_switch_keymap_packet(abbreviation: &str) -> Result<Vec<u8>, AppError> {
    let invalid = || AppError::UhkAbbreviationInvalid {
        abbreviation: abbreviation.to_string(),
        len: abbreviation.len(),
        max: MAX_KEYMAP_ABBREVIATION_LEN,
    };

    if abbreviation.is_empty()
        || abbreviation.len() > MAX_KEYMAP_ABBREVIATION_LEN
        || !abbreviation.is_ascii()
    {
        return Err(invalid());
    }

    let mut packet = Vec::with_capacity(2usize.saturating_add(abbreviation.len()));
    packet.push(CMD_SWITCH_KEYMAP);
    packet.push(u8::try_from(abbreviation.len()).map_err(|_| invalid())?);
    packet.extend_from_slice(abbreviation.as_bytes());
    Ok(packet)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    #[test]
    fn builds_packet_for_typical_abbreviation() {
        let packet = build_switch_keymap_packet("HD2").unwrap();
        assert_eq!(packet, vec![0x11, 3, b'H', b'D', b'2']);
    }

    #[test]
    fn builds_packet_for_single_char_abbreviation() {
        let packet = build_switch_keymap_packet("Q").unwrap();
        assert_eq!(packet, vec![0x11, 1, b'Q']);
    }

    #[test]
    fn accepts_abbreviation_at_exact_limit() {
        let packet = build_switch_keymap_packet("ABC").unwrap();
        assert_eq!(packet, vec![0x11, 3, b'A', b'B', b'C']);
    }

    #[test]
    fn rejects_empty_abbreviation() {
        let err = build_switch_keymap_packet("").unwrap_err();
        assert!(matches!(err, AppError::UhkAbbreviationInvalid { .. }));
    }

    #[test]
    fn rejects_abbreviation_longer_than_firmware_limit() {
        let err = build_switch_keymap_packet("TOOLONG").unwrap_err();
        assert!(matches!(err, AppError::UhkAbbreviationInvalid { .. }));
    }

    #[test]
    fn rejects_non_ascii_abbreviation() {
        let err = build_switch_keymap_packet("HÐ2").unwrap_err();
        assert!(matches!(err, AppError::UhkAbbreviationInvalid { .. }));
    }
}
