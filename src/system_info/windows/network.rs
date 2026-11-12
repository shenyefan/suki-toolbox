use super::{HardwareDevice, Reader};

pub(super) fn read(reader: &mut Reader<'_>) -> Vec<HardwareDevice> {
    let network = reader.query(
        "root/StandardCimv2",
        "MSFT_NetAdapter",
        &["InterfaceDescription", "Name", "Virtual"],
        Some("Hidden = FALSE"),
    );
    network
        .iter()
        .map(|item| {
            let name = item
                .first_text(&["InterfaceDescription", "Name"])
                .unwrap_or_default();
            HardwareDevice::new(name, item.boolean("Virtual"))
        })
        .collect()
}
