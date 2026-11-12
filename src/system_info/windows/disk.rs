use super::super::model::format_bytes;
use super::{HardwareDevice, Reader, positive};

pub(super) fn read(reader: &mut Reader<'_>) -> Vec<HardwareDevice> {
    let disks = reader.query(
        "root/Microsoft/Windows/Storage",
        "MSFT_Disk",
        &["FriendlyName", "Model", "Manufacturer", "Size", "BusType"],
        None,
    );
    disks
        .iter()
        .map(|item| {
            let name = item
                .first_text(&["FriendlyName", "Model", "Manufacturer"])
                .unwrap_or_else(|| "未知设备".into());
            let virtual_device = item
                .number("BusType")
                .filter(|&bus| bus != 0)
                .map(|bus| matches!(bus, 14..=16));
            let text = match positive(item.number("Size")) {
                Some(size) => format!("{name}（{}）", format_bytes(size)),
                None => name,
            };
            HardwareDevice::new(text, virtual_device)
        })
        .collect()
}
