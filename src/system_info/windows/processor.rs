use super::super::model::format_bytes;
use super::{HardwareDevice, Reader, positive};

pub(super) fn read(reader: &mut Reader<'_>) -> Vec<HardwareDevice> {
    let processors = reader.query(
        "root/cimv2",
        "Win32_Processor",
        &[
            "Name",
            "Caption",
            "Description",
            "NumberOfCores",
            "NumberOfLogicalProcessors",
            "L2CacheSize",
            "L3CacheSize",
        ],
        None,
    );
    processors
        .iter()
        .map(|item| {
            let mut parts = vec![
                item.first_text(&["Name", "Caption", "Description"])
                    .unwrap_or_else(|| "未知设备".into()),
            ];
            for (field, suffix) in [
                ("NumberOfCores", "核"),
                ("NumberOfLogicalProcessors", "线程"),
            ] {
                if let Some(value) = positive(item.number(field)) {
                    parts.push(format!("{value} {suffix}"));
                }
            }
            for (field, label) in [("L2CacheSize", "L2"), ("L3CacheSize", "L3")] {
                if let Some(value) = positive(item.number(field)) {
                    parts.push(format!("{label} 缓存 {}", format_bytes(value * 1024)));
                }
            }
            HardwareDevice::new(parts.join(" · "), None)
        })
        .collect()
}
