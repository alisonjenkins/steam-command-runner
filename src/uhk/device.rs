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
use super::{UHK_GENERIC_HID_USAGE_PAGE, UHK_PRODUCT_ID, UHK_VENDOR_ID};
use crate::error::AppError;
use hidapi::HidApi;

/// Sends the UHK firmware's SwitchKeymap command over the keyboard's
/// generic-HID interface. Not unit tested -- it talks to real hardware; the
/// packet it sends is `build_switch_keymap_packet`, which is.
pub fn switch_keymap(abbreviation: &str, vendor_id: u16, product_id: u16) -> Result<(), AppError> {
    let packet = build_switch_keymap_packet(abbreviation)?;

    let api = HidApi::new().map_err(|e| AppError::UhkHidOpen(e.to_string()))?;

    // The UHK exposes several HID interfaces (keyboard, mouse, generic
    // command channel) at the same vendor/product id -- usage_page picks
    // out the one that accepts firmware commands.
    let device_info = api
        .device_list()
        .find(|d| {
            d.vendor_id() == vendor_id
                && d.product_id() == product_id
                && d.usage_page() == UHK_GENERIC_HID_USAGE_PAGE
        })
        .ok_or(AppError::UhkDeviceNotFound {
            vendor_id,
            product_id,
        })?;

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

/// [`switch_keymap`] against the UHK 80's default vendor/product id.
pub fn switch_keymap_default(abbreviation: &str) -> Result<(), AppError> {
    switch_keymap(abbreviation, UHK_VENDOR_ID, UHK_PRODUCT_ID)
}
