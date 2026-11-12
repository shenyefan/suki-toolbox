use super::super::model::{format_bytes, memory_type};
use super::{HardwareDevice, Reader, positive};

pub(super) fn read(reader: &mut Reader<'_>) -> Vec<HardwareDevice> {
    let arrays = reader.query(
        "root/cimv2",
        "Win32_PhysicalMemoryArray",
        &["MemoryDevices"],
        None,
    );
    let slots: u64 = arrays
        .iter()
        .filter_map(|item| item.number("MemoryDevices"))
        .sum();
    let memory = reader.query(
        "root/cimv2",
        "Win32_PhysicalMemory",
        &[
            "Capacity",
            "ConfiguredClockSpeed",
            "SMBIOSMemoryType",
            "Manufacturer",
            "PartNumber",
            "Name",
            "Caption",
            "Description",
        ],
        None,
    );
    let mut memory_lines = memory
        .iter()
        .map(|item| {
            let mut parts = vec![
                item.first_text(&[
                    "Manufacturer",
                    "PartNumber",
                    "Name",
                    "Caption",
                    "Description",
                ])
                .unwrap_or_else(|| "未知设备".into()),
            ];
            if let Some(capacity) = positive(item.number("Capacity")) {
                parts.push(format_bytes(capacity));
            }
            if let Some(kind) = item.number("SMBIOSMemoryType").and_then(memory_type) {
                parts.push(kind.into());
            }
            if let Some(speed) = positive(item.number("ConfiguredClockSpeed")) {
                parts.push(format!("{speed} MT/s"));
            }
            parts.join(" · ")
        })
        .collect::<Vec<_>>();
    if slots > 0
        && let Some(last) = memory_lines.last_mut()
    {
        last.push_str(&format!(
            "（系统报告已使用插槽 {} / {slots}）",
            memory.len()
        ));
    }
    memory_lines
        .into_iter()
        .map(|text| HardwareDevice::new(text, None))
        .collect()
}
