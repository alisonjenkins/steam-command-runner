//! Raw USB HID control of an Ultimate Hacking Keyboard (UHK), for switching
//! its active keymap from outside UHK Agent -- e.g. a Steam launch hook.

mod device;
mod protocol;

pub use device::switch_keymap;
pub use protocol::build_switch_keymap_packet;

/// Every UHK model's (vendor id, product id, HID report id) in keyboard mode
/// (not bootloader/firmware-update mode -- those don't accept runtime
/// commands), from UHK Agent's own device table
/// (`packages/uhk-common/src/models/uhk-products.ts`). Both the current
/// vendor id (0x37A8) and the legacy one used by early UHK 60 firmware
/// (0x1D50, shared with other open hardware projects) are covered, since a
/// UHK's vendor/product id depends on which firmware it's running, not just
/// which model it is.
///
/// The report id matters as much as the vendor/product id: the UHK 60's
/// generic-HID interface declares report id 0 (the USB HID spec's "no
/// numbered reports" sentinel -- no id byte goes on the wire), while the
/// UHK 80 and Dongle declare report id 4 (a real numbered report -- every
/// write and read is prefixed with that byte). Writing to a UHK 80 without
/// the 0x04 prefix isn't just wrong framing: the firmware's `set_report`
/// handler checks the first byte against its expected report id and, on a
/// mismatch, returns immediately without responding *or re-arming its OUT
/// endpoint* -- so the write silently no-ops, and the next write fails with
/// a broken pipe instead of an error from the device.
const KNOWN_UHK_KEYBOARDS: &[(u16, u16, u8)] = &[
    (0x1D50, 0x6122, 0), // UHK 60 v1, legacy VID
    (0x37A8, 0x0001, 0), // UHK 60 v1
    (0x1D50, 0x6124, 0), // UHK 60 v2, legacy VID
    (0x37A8, 0x0003, 0), // UHK 60 v2
    (0x37A8, 0x0007, 4), // UHK 80 left
    (0x37A8, 0x0009, 4), // UHK 80 right
    (0x37A8, 0x0005, 4), // UHK Dongle
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

/// Firmware status byte for a successful command
/// (`right/src/usb_protocol_handler.h`, `UsbStatusCode_Success`).
const STATUS_SUCCESS: u8 = 0;

/// Firmware status byte: the abbreviation was longer than
/// `MAX_KEYMAP_ABBREVIATION_LEN` (`UsbStatusCode_SwitchKeymap_InvalidAbbreviationLength`).
const STATUS_INVALID_ABBREVIATION_LENGTH: u8 = 2;

/// Firmware status byte: no keymap with that abbreviation exists on the
/// device (`UsbStatusCode_SwitchKeymap_InvalidAbbreviation`).
const STATUS_UNKNOWN_ABBREVIATION: u8 = 3;

/// Describes a firmware response status byte for a SwitchKeymap command.
/// `Ok(())` for success; `Err(reason)` with a human-readable reason otherwise
/// -- including status bytes not in the two documented error codes, since
/// the firmware could return others we don't have names for.
pub(crate) fn describe_switch_keymap_status(status: u8) -> Result<(), String> {
    match status {
        STATUS_SUCCESS => Ok(()),
        STATUS_INVALID_ABBREVIATION_LENGTH => Err(format!(
            "abbreviation longer than {MAX_KEYMAP_ABBREVIATION_LEN} characters"
        )),
        STATUS_UNKNOWN_ABBREVIATION => {
            Err("no keymap with that abbreviation exists on the device".to_string())
        }
        other => Err(format!("device returned status {other}")),
    }
}

/// If a HID interface identified by `(vendor_id, product_id)` and
/// `(usage_page, usage)` is a UHK's vendor-defined command interface, returns
/// the HID report id to frame commands with. `None` otherwise -- either it's
/// not a UHK, or it's a UHK's keyboard-HID/mouse-HID sub-interface rather
/// than its command interface. Pulled out of `device::switch_keymap` so the
/// matching logic is testable without a real device attached --
/// `hidapi::DeviceInfo` has no public constructor.
pub(crate) fn uhk_command_interface_report_id(
    vendor_id: u16,
    product_id: u16,
    usage_page: u16,
    usage: u16,
) -> Option<u8> {
    if !UHK_COMMAND_USAGE.contains(&(usage_page, usage)) {
        return None;
    }
    KNOWN_UHK_KEYBOARDS
        .iter()
        .find(|(v, p, _)| *v == vendor_id && *p == product_id)
        .map(|(_, _, report_id)| *report_id)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    #[test]
    fn matches_uhk_80_right_on_current_firmware_with_its_report_id() {
        assert_eq!(
            uhk_command_interface_report_id(0x37A8, 0x0009, 0xFF00, 0x0001),
            Some(4)
        );
    }

    #[test]
    fn matches_uhk_60_v1_on_legacy_vendor_id_and_old_usage_with_its_report_id() {
        assert_eq!(
            uhk_command_interface_report_id(0x1D50, 0x6122, 0x0080, 0x0081),
            Some(0)
        );
    }

    #[test]
    fn rejects_known_vid_pid_with_wrong_usage() {
        // Same (vendor id, product id) as the UHK 80 right, but the usage of
        // its keyboard-HID sub-interface, not its command interface.
        assert_eq!(
            uhk_command_interface_report_id(0x37A8, 0x0009, 0x0001, 0x0006),
            None
        );
    }

    #[test]
    fn rejects_unrelated_vendor_id() {
        assert_eq!(
            uhk_command_interface_report_id(0x046D, 0xC52B, 0xFF00, 0x0001),
            None
        );
    }

    #[test]
    fn rejects_uhk_vendor_id_with_bootloader_product_id() {
        // Bootloader mode doesn't accept runtime commands like SwitchKeymap.
        assert_eq!(
            uhk_command_interface_report_id(0x37A8, 0x0008, 0xFF00, 0x0001),
            None
        );
    }

    #[test]
    fn describes_success_status() {
        assert_eq!(describe_switch_keymap_status(0), Ok(()));
    }

    #[test]
    fn describes_invalid_abbreviation_length_status() {
        assert!(describe_switch_keymap_status(2).is_err());
    }

    #[test]
    fn describes_unknown_abbreviation_status() {
        assert!(describe_switch_keymap_status(3).is_err());
    }

    #[test]
    fn describes_unrecognised_status_generically() {
        let err = describe_switch_keymap_status(99).unwrap_err();
        assert!(err.contains("99"));
    }
}
