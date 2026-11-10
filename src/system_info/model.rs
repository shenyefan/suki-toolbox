#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum HardwareKind {
    Mainboard,
    Processor,
    Memory,
    Graphics,
    Monitor,
    Disk,
    Battery,
    Audio,
    Network,
}

impl HardwareKind {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Mainboard => "主板",
            Self::Processor => "处理器",
            Self::Memory => "内存",
            Self::Graphics => "显卡",
            Self::Monitor => "显示器",
            Self::Disk => "硬盘",
            Self::Battery => "电池",
            Self::Audio => "声卡",
            Self::Network => "网卡",
        }
    }
}

pub(super) struct HardwareRow {
    pub(super) kind: HardwareKind,
    pub(super) devices: Vec<String>,
}

#[derive(Default)]
pub(super) struct Snapshot {
    pub(super) model: String,
    pub(super) os: String,
    pub(super) rows: Vec<HardwareRow>,
    pub(super) errors: Vec<String>,
}

impl Snapshot {
    pub(super) fn copy_text(&self, uptime: u64) -> String {
        let mut lines = vec![
            format!(
                "型号信息：{}",
                if self.model.is_empty() {
                    "未知"
                } else {
                    &self.model
                }
            ),
            format!(
                "系统信息：{}",
                if self.os.is_empty() {
                    "未知"
                } else {
                    &self.os
                }
            ),
            format!("运行时间：{}", format_uptime(uptime)),
            String::new(),
            "详细信息".into(),
        ];
        for row in &self.rows {
            lines.push(format!("{}：", row.kind.label()));
            if row.devices.is_empty() {
                lines.push("  未知".into());
            } else {
                lines.extend(row.devices.iter().map(|device| format!("  {device}")));
            }
        }
        lines.join("\n")
    }

    pub(super) fn add(&mut self, kind: HardwareKind, devices: Vec<String>) {
        self.rows.push(HardwareRow {
            kind,
            devices: devices
                .into_iter()
                .filter(|value| !value.is_empty())
                .collect(),
        });
    }
}

pub(super) fn format_bytes(value: u64) -> String {
    let units = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut size = value as f64;
    let mut unit = 0;
    while size >= 1024. && unit < units.len() - 1 {
        size /= 1024.;
        unit += 1;
    }
    format!("{size:.1} {}", units[unit])
}

pub(super) fn memory_type(value: u64) -> Option<&'static str> {
    match value {
        18 => Some("DDR"),
        19 => Some("DDR2"),
        20 => Some("DDR2 FB-DIMM"),
        24 => Some("DDR3"),
        26 => Some("DDR4"),
        27 => Some("LPDDR"),
        28 => Some("LPDDR2"),
        29 => Some("LPDDR3"),
        30 => Some("LPDDR4"),
        34 => Some("DDR5"),
        35 => Some("LPDDR5"),
        _ => None,
    }
}

pub(super) fn format_uptime(seconds: u64) -> String {
    format!(
        "{}天{}小时{}分钟{}秒",
        seconds / 86_400,
        (seconds / 3_600) % 24,
        (seconds / 60) % 60,
        seconds % 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smbios_types_use_decimal_codes() {
        assert_eq!(memory_type(20), Some("DDR2 FB-DIMM"));
        assert_eq!(memory_type(26), Some("DDR4"));
        assert_eq!(memory_type(34), Some("DDR5"));
        assert_eq!(memory_type(35), Some("LPDDR5"));
        assert_eq!(memory_type(0), None);
    }

    #[test]
    fn capacity_units_match_binary_divisors() {
        assert_eq!(format_bytes(32 * 1024 * 1024 * 1024), "32.0 GiB");
        assert_eq!(format_bytes(32 * 1024 * 1024), "32.0 MiB");
    }
}
