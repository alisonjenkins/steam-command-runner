//! Raw USB HID control of an Ultimate Hacking Keyboard (UHK), for switching
//! its active keymap from outside UHK Agent -- e.g. a Steam launch hook.

mod device;
mod protocol;

pub use device::switch_keymap;
pub use protocol::build_switch_keymap_packet;

/// A UHK's USB (vendor id, product id) pair in normal keyboard mode (not
/// bootloader/firmware-update mode -- those don't accept runtime commands).
type VidPid = (u16, u16);

/// Every UHK model's (vendor id, product id) in keyboard mode, from UHK
/// Agent's own device table (`packages/uhk-common/src/models/uhk-products.ts`).
/// Both the current vendor id (0x37A8) and the legacy one used by early
/// UHK 60 firmware (0x1D50, shared with other open hardware projects) are
/// covered, since a UHK's vendor/product id depends on which firmware it's
/// running, not just which model it is.
const KNOWN_UHK_KEYBOARDS: &[VidPid] = &[
    (0x1D50, 0x6122), // UHK 60 v1, legacy VID
    (0x37A8, 0x0001), // UHK 60 v1
    (0x1D50, 0x6124), // UHK 60 v2, legacy VID
    (0x37A8, 0x0003), // UHK 60 v2
    (0x37A8, 0x0007), // UHK 80 left
    (0x37A8, 0x0009), // UHK 80 right
    (0x37A8, 0x0005), // UHK Dongle
];

/// (usage_page, usage) pairs that identify a UHK's vendor-defined generic-HID
/// command interface, as opposed to its keyboard-HID or mouse-HID
/// sub-interfaces at the same vendor/product id. Two pairs because older UHK
/// 60 firmware used a different usage page than current firmware.
const UHK_COMMAND_USAGE: &[(u16, u16)] = &[
    (0xFF00, 0x0001), // current firmware
    (0x0080, 0x0081), // old UHK 60 firmware
];

/// Firmware USB command id for switching the active keymap
/// (`right/src/usb_protocol_handler.h`, `UsbCommandId_SwitchKeymap`).
const CMD_SWITCH_KEYMAP: u8 = 0x11;

/// Firmware's `KEYMAP_ABBREVIATION_LENGTH` (`right/src/keymap.h`) -- the
/// longest abbreviation the SwitchKeymap command will accept.
pub const MAX_KEYMAP_ABBREVIATION_LEN: usize = 3;

/// True if a HID interface identified by `(vendor_id, product_id)` and
/// `(usage_page, usage)` is a UHK's vendor-defined command interface, as
/// opposed to the same physical keyboard's keyboard-HID or mouse-HID
/// sub-interface (which share the same vendor/product id but a different
/// usage). Pulled out of `device::switch_keymap` so the matching logic is
/// testable without a real device attached -- `hidapi::DeviceInfo` has no
/// public constructor.
pub(crate) fn matches_uhk_command_interface(
    vendor_id: u16,
    product_id: u16,
    usage_page: u16,
    usage: u16,
) -> bool {
    KNOWN_UHK_KEYBOARDS.contains(&(vendor_id, product_id))
        && UHK_COMMAND_USAGE.contains(&(usage_page, usage))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    #[test]
    fn matches_uhk_80_right_on_current_firmware() {
        assert!(matches_uhk_command_interface(
            0x37A8, 0x0009, 0xFF00, 0x0001
        ));
    }

    #[test]
    fn matches_uhk_60_v1_on_legacy_vendor_id_and_old_usage() {
        assert!(matches_uhk_command_interface(
            0x1D50, 0x6122, 0x0080, 0x0081
        ));
    }

    #[test]
    fn rejects_known_vid_pid_with_wrong_usage() {
        // Same (vendor id, product id) as the UHK 80 right, but the usage of
        // its keyboard-HID sub-interface, not its command interface.
        assert!(!matches_uhk_command_interface(
            0x37A8, 0x0009, 0x0001, 0x0006
        ));
    }

    #[test]
    fn rejects_unrelated_vendor_id() {
        assert!(!matches_uhk_command_interface(
            0x046D, 0xC52B, 0xFF00, 0x0001
        ));
    }

    #[test]
    fn rejects_uhk_vendor_id_with_bootloader_product_id() {
        // Bootloader mode doesn't accept runtime commands like SwitchKeymap.
        assert!(!matches_uhk_command_interface(
            0x37A8, 0x0008, 0xFF00, 0x0001
        ));
    }
}
