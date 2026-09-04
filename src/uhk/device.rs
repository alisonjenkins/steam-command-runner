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

use super::protocol::build_switch_keymap_packet;
use super::{describe_switch_keymap_status, uhk_command_interface_report_id};
use crate::error::AppError;
use hidapi::HidApi;
use tracing::{debug, warn};

/// Firmware's response report is a fixed 64-byte buffer
/// (`USB_COMMAND_BUFFER_LENGTH`, `right/src/usb_protocol_handler.h`); allow
/// one more byte for a leading report-id byte on models that use one.
const RESPONSE_BUFFER_LEN: usize = 65;

/// How long to wait for the firmware's response before giving up. The
/// command is a same-transaction, synchronous turnaround on the device side
/// (see `command_app.cpp`'s `set_report`/`send_report`), so a slow response
/// means something is actually wrong, not just busy.
const RESPONSE_TIMEOUT_MS: i32 = 1000;

/// Sends the UHK firmware's SwitchKeymap command over the keyboard's
/// generic-HID interface, auto-discovering the connected UHK rather than
/// requiring its vendor/product id up front, and reads back the firmware's
/// response to confirm the switch actually happened. Not unit tested -- it
/// talks to real hardware; the packet it sends is `build_switch_keymap_packet`,
/// the device it looks for is matched by `uhk_command_interface_report_id`,
/// and the response is interpreted by `describe_switch_keymap_status` --
/// all three are.
pub fn switch_keymap(abbreviation: &str) -> Result<(), AppError> {
    let packet = build_switch_keymap_packet(abbreviation)?;

    let api = HidApi::new().map_err(|e| AppError::UhkHidOpen(e.to_string()))?;

    let mut candidates = api.device_list().filter_map(|d| {
        uhk_command_interface_report_id(d.vendor_id(), d.product_id(), d.usage_page(), d.usage())
            .map(|report_id| (d, report_id))
    });
    let (device_info, report_id) = candidates.next().ok_or(AppError::UhkDeviceNotFound)?;

    debug!(
        vendor_id = format!("{:#06x}", device_info.vendor_id()),
        product_id = format!("{:#06x}", device_info.product_id()),
        usage_page = format!("{:#06x}", device_info.usage_page()),
        usage = device_info.usage(),
        report_id,
        manufacturer = device_info.manufacturer_string().unwrap_or("<unknown>"),
        product = device_info.product_string().unwrap_or("<unknown>"),
        serial_number = device_info.serial_number().unwrap_or("<unknown>"),
        path = ?device_info.path(),
        "found UHK command interface",
    );

    // A single physical UHK should expose exactly one command interface; more
    // than one (e.g. a UHK 60 and a UHK 80 both plugged in) is ambiguous, so
    // send to the first and say so rather than silently guessing which the
    // caller meant.
    if candidates.next().is_some() {
        warn!("multiple UHK command interfaces found, using the first one enumerated");
    }

    let device = device_info
        .open_device(&api)
        .map_err(|e| AppError::UhkHidOpen(e.to_string()))?;

    // hidapi always addresses the first byte of a write to the report id,
    // even for devices whose sole report is id 0 (the USB HID spec's
    // "unnumbered report" sentinel) -- hidapi omits it from the actual wire
    // bytes in that case, but still expects the caller to put it there.
    let mut framed = Vec::with_capacity(packet.len().saturating_add(1));
    framed.push(report_id);
    framed.extend_from_slice(&packet);
    debug!(bytes = ?framed, "writing SwitchKeymap command");

    let written = device
        .write(&framed)
        .map_err(|e| AppError::UhkHidWrite(e.to_string()))?;
    if written != framed.len() {
        return Err(AppError::UhkHidWrite(format!(
            "wrote {written} of {} bytes",
            framed.len()
        )));
    }

    // The firmware's command handler is a synchronous, single-buffered
    // request/response turnaround (see command_app.cpp's set_report): it
    // doesn't re-arm its OUT endpoint for the next command until it has sent
    // this response. Skipping this read is what left the endpoint stalled
    // and made the *next* switch_keymap call fail with a broken pipe.
    let mut response = [0u8; RESPONSE_BUFFER_LEN];
    let read = device
        .read_timeout(&mut response, RESPONSE_TIMEOUT_MS)
        .map_err(|e| AppError::UhkHidWrite(e.to_string()))?;
    debug!(bytes = ?response.get(..read), "read SwitchKeymap response");

    // Devices using a nonzero report id prefix reads with that byte too;
    // report id 0 doesn't (see the framing comment above).
    let status_offset = usize::from(report_id != 0);
    let status = *response.get(status_offset).ok_or_else(|| {
        AppError::UhkHidWrite("response too short to contain a status byte".to_string())
    })?;

    describe_switch_keymap_status(status).map_err(|reason| AppError::UhkKeymapSwitchRejected {
        abbreviation: abbreviation.to_string(),
        reason,
    })
}
