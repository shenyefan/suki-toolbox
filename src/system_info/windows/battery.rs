use super::pnp::{pnp_device_name, wmi_instance_matches};
use super::{HardwareDevice, Reader, positive};

pub(super) fn read(reader: &mut Reader<'_>) -> Vec<HardwareDevice> {
    let batteries = reader.query(
        "root/cimv2",
        "Win32_Battery",
        &["Name", "Caption", "Description", "PNPDeviceID"],
        None,
    );
    if batteries.is_empty() {
        return Vec::new();
    }
    let full = reader.query(
        "root/WMI",
        "BatteryFullChargedCapacity",
        &["InstanceName", "FullChargedCapacity"],
        None,
    );
    let cycles = reader.query(
        "root/WMI",
        "BatteryCycleCount",
        &["InstanceName", "CycleCount"],
        None,
    );
    let static_data = reader.query(
        "root/WMI",
        "BatteryStaticData",
        &[
            "InstanceName",
            "DeviceName",
            "DesignedCapacity",
            "Capabilities",
        ],
        None,
    );
    let values: Vec<String> = if static_data.is_empty() {
        batteries
            .iter()
            .map(|item| {
                item.first_text(&["Name", "Caption", "Description"])
                    .or_else(|| {
                        pnp_device_name(item.text("PNPDeviceID").as_deref(), &mut reader.errors)
                    })
                    .unwrap_or_default()
            })
            .collect()
    } else {
        static_data
            .iter()
            .map(|item| {
                let identity = item.text("InstanceName");
                let name = item
                    .text("DeviceName")
                    .or_else(|| {
                        let instance = identity.as_deref()?;
                        let basic = batteries.iter().find(|battery| {
                            battery
                                .text("PNPDeviceID")
                                .is_some_and(|pnp_id| wmi_instance_matches(instance, &pnp_id))
                        })?;
                        basic
                            .first_text(&["Name", "Caption", "Description"])
                            .or_else(|| {
                                pnp_device_name(
                                    basic.text("PNPDeviceID").as_deref(),
                                    &mut reader.errors,
                                )
                            })
                    })
                    .unwrap_or_else(|| "未知设备".into());
                let mut parts = vec![name];
                let design = battery_capacity(item.number("DesignedCapacity"));
                let charged = full
                    .iter()
                    .find(|row| identity.is_some() && row.text("InstanceName") == identity)
                    .and_then(|row| battery_capacity(row.number("FullChargedCapacity")));
                // BATTERY_CAPACITY_RELATIVE means the capacities use relative units.
                let unit = item.number("Capabilities").map(|flags| {
                    if flags & 0x4000_0000 == 0 {
                        "Wh"
                    } else {
                        "相对单位"
                    }
                });
                for (label, value) in [("设计容量", design), ("满充容量", charged)] {
                    if let Some(value) = value {
                        parts.push(match unit {
                            Some("Wh") => format!("{label} {:.1} Wh", value as f64 / 1000.),
                            Some(unit) => format!("{label} {value} {unit}"),
                            None => format!("{label} {value}（提供程序单位）"),
                        });
                    }
                }
                if let Some((design, full)) = design.zip(charged) {
                    parts.push(format!(
                        "健康度 {:.1}%",
                        (full as f64 / design as f64 * 100.).min(100.)
                    ));
                }
                if let Some(count) = cycles
                    .iter()
                    .find(|row| identity.is_some() && row.text("InstanceName") == identity)
                    .and_then(|row| row.number("CycleCount"))
                    .filter(|&value| value != u32::MAX as u64)
                {
                    parts.push(format!("循环 {count} 次"));
                }
                parts.join(" · ")
            })
            .collect()
    };
    values
        .into_iter()
        .map(|text| HardwareDevice::new(text, None))
        .collect()
}

fn battery_capacity(value: Option<u64>) -> Option<u64> {
    positive(value).filter(|&value| value != u32::MAX as u64)
}
