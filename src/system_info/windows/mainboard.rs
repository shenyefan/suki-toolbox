use super::{HardwareDevice, Reader, joined};

pub(super) fn read(reader: &mut Reader<'_>) -> Vec<HardwareDevice> {
    let boards = reader.query(
        "root/cimv2",
        "Win32_BaseBoard",
        &["Manufacturer", "Product", "Name", "Caption", "Description"],
        None,
    );
    boards
        .iter()
        .map(|item| {
            let name = joined(item, &["Manufacturer", "Product"]);
            HardwareDevice::new(
                if name.is_empty() {
                    item.first_text(&["Name", "Caption", "Description"])
                        .unwrap_or_default()
                } else {
                    name
                },
                None,
            )
        })
        .collect()
}
