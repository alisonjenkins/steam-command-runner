//! Raw USB HID control of an Ultimate Hacking Keyboard (UHK), for switching
//! its active keymap from outside UHK Agent -- e.g. a Steam launch hook.

mod device;
mod protocol;

pub use device::{switch_keymap, switch_keymap_default};
pub use protocol::build_switch_keymap_packet;

/// UHK 80's USB vendor id, confirmed live against a UHK 80 Right via its own
/// device-detection log (`vendorId":"0x37A8"`).
pub const UHK_VENDOR_ID: u16 = 0x37A8;

/// UHK 80's USB product id, confirmed the same way (`productId":"0x9"`).
pub const UHK_PRODUCT_ID: u16 = 0x0009;

/// Vendor-defined usage page of the UHK's generic-HID command interface. The
/// UHK exposes other HID interfaces (keyboard, mouse) at the same
/// vendor/product id, so this is what picks out the one that accepts
/// firmware commands -- the same interface UHK Agent itself talks to.
const UHK_GENERIC_HID_USAGE_PAGE: u16 = 0xFF00;

/// Firmware USB command id for switching the active keymap
/// (`right/src/usb_protocol_handler.h`, `UsbCommandId_SwitchKeymap`).
const CMD_SWITCH_KEYMAP: u8 = 0x11;

/// Firmware's `KEYMAP_ABBREVIATION_LENGTH` (`right/src/keymap.h`) -- the
/// longest abbreviation the SwitchKeymap command will accept.
pub const MAX_KEYMAP_ABBREVIATION_LEN: usize = 3;
