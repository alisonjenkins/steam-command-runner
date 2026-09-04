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

use super::matches_uhk_command_interface;
use super::protocol::build_switch_keymap_packet;
use crate::error::AppError;
use hidapi::HidApi;
use tracing::warn;

/// Sends the UHK firmware's SwitchKeymap command over the keyboard's
/// generic-HID interface, auto-discovering the connected UHK rather than
/// requiring its vendor/product id up front. Not unit tested -- it talks to
/// real hardware; the packet it sends is `build_switch_keymap_packet` and the
/// device it looks for is matched by `matches_uhk_command_interface`, both of
/// which are.
pub fn switch_keymap(abbreviation: &str) -> Result<(), AppError> {
    let packet = build_switch_keymap_packet(abbreviation)?;

    let api = HidApi::new().map_err(|e| AppError::UhkHidOpen(e.to_string()))?;

    let mut candidates = api.device_list().filter(|d| {
        matches_uhk_command_interface(d.vendor_id(), d.product_id(), d.usage_page(), d.usage())
    });
    let device_info = candidates.next().ok_or(AppError::UhkDeviceNotFound)?;

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

    let written = device
        .write(&packet)
        .map_err(|e| AppError::UhkHidWrite(e.to_string()))?;

    if written != packet.len() {
        return Err(AppError::UhkHidWrite(format!(
            "wrote {written} of {} bytes",
            packet.len()
        )));
    }

    Ok(())
}
