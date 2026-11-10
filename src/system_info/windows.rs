use super::{
    mi::{Client, Record},
    model::{HardwareKind, Snapshot, format_bytes, memory_type},
};
use std::sync::atomic::{AtomicBool, Ordering};
use windows::{
    Win32::{
        Graphics::{Dxgi::*, Gdi::*},
        System::{Registry::*, SystemInformation::GetTickCount64},
    },
    core::{PCWSTR, w},
};

pub(super) fn uptime() -> u64 {
    unsafe { GetTickCount64() / 1000 }
}

struct Reader<'a> {
    client: Client,
    cancelled: &'a AtomicBool,
    errors: Vec<String>,
}

impl Reader<'_> {
    fn query(
        &mut self,
        namespace: &str,
        class: &str,
        fields: &'static [&'static str],
        filter: Option<&str>,
    ) -> Vec<Record> {
        match self
            .client
            .query(namespace, class, fields, filter, 5, self.cancelled)
        {
            Ok(records) => records,
            Err(error) => {
                if !self.cancelled.load(Ordering::Relaxed) {
                    self.errors.push(error);
                }
                Vec::new()
            }
        }
    }
}

pub(super) fn read(cancelled: &AtomicBool) -> Snapshot {
    let mut snapshot = Snapshot::default();
    let client = match Client::new() {
        Ok(client) => client,
        Err(error) => {
            snapshot.errors.push(error);
            return snapshot;
        }
    };
    let mut reader = Reader {
        client,
        cancelled,
        errors: Vec::new(),
    };
    for item in reader.query(
        "root/cimv2",
        "Win32_ComputerSystem",
        &["Manufacturer", "Model"],
        None,
    ) {
        snapshot.model = joined(&item, &["Manufacturer", "Model"]);
    }
    for item in reader.query(
        "root/cimv2",
        "Win32_OperatingSystem",
        &["Caption", "Version", "BuildNumber"],
        None,
    ) {
        let key = w!("SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion");
        let version = registry_string(key, w!("DisplayVersion"), &mut reader.errors);
        let ubr = registry_number(key, w!("UBR"), &mut reader.errors);
        let mut parts = vec![item.text("Caption").unwrap_or_else(|| "Windows".into())];
        if let Some(version) = version {
            parts.push(format!("({version})"));
        }
        if let Some(build) = item.text("BuildNumber") {
            parts.push(match ubr {
                Some(ubr) => format!("| {build}.{ubr}"),
                None => format!("| {build}"),
            });
        }
        snapshot.os = parts.join(" ");
    }
    let boards = reader.query(
        "root/cimv2",
        "Win32_BaseBoard",
        &["Manufacturer", "Product"],
        None,
    );
    snapshot.add(
        HardwareKind::Mainboard,
        boards
            .iter()
            .map(|item| joined(item, &["Manufacturer", "Product"]))
            .collect(),
    );

    let processors = reader.query(
        "root/cimv2",
        "Win32_Processor",
        &[
            "Name",
            "NumberOfCores",
            "NumberOfLogicalProcessors",
            "L2CacheSize",
            "L3CacheSize",
        ],
        None,
    );
    snapshot.add(
        HardwareKind::Processor,
        processors
            .iter()
            .map(|item| {
                let mut parts = vec![item.text("Name").unwrap_or_else(|| "未知处理器".into())];
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
                parts.join(" · ")
            })
            .collect(),
    );

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
        ],
        None,
    );
    let mut memory_lines = memory
        .iter()
        .map(|item| {
            let mut parts = Vec::new();
            if let Some(manufacturer) = item.text("Manufacturer") {
                parts.push(manufacturer);
            }
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
    if slots > 0 && !memory.is_empty() {
        let occupancy = format!("已用内存位 {} / {}", memory.len(), slots);
        if let Some(last) = memory_lines.last_mut() {
            last.push_str(&format!("（{occupancy}）"));
        }
    }
    snapshot.add(HardwareKind::Memory, memory_lines);
    read_graphics(&mut reader, &mut snapshot);
    read_monitors(&mut reader, &mut snapshot);
    let disks = reader.query("root/cimv2", "Win32_DiskDrive", &["Model", "Size"], None);
    snapshot.add(
        HardwareKind::Disk,
        disks
            .iter()
            .map(|item| {
                let name = item.text("Model").unwrap_or_else(|| "未知硬盘".into());
                match positive(item.number("Size")) {
                    Some(size) => format!("{name}（{}）", format_bytes(size)),
                    None => name,
                }
            })
            .collect(),
    );
    read_batteries(&mut reader, &mut snapshot);
    let audio = reader.query("root/cimv2", "Win32_SoundDevice", &["Name"], None);
    snapshot.add(
        HardwareKind::Audio,
        audio
            .iter()
            .filter_map(|item| item.text("Name"))
            .filter(|name| !virtual_device(name))
            .collect(),
    );
    let network = reader.query(
        "root/cimv2",
        "Win32_NetworkAdapter",
        &["Name"],
        Some("PhysicalAdapter = TRUE"),
    );
    snapshot.add(
        HardwareKind::Network,
        network
            .iter()
            .filter_map(|item| item.text("Name"))
            .filter(|name| {
                !virtual_device(name) && !name.to_ascii_lowercase().contains("bluetooth")
            })
            .collect(),
    );
    snapshot.errors = reader.errors;
    snapshot
}

fn read_graphics(reader: &mut Reader<'_>, snapshot: &mut Snapshot) {
    struct Adapter {
        name: String,
        dedicated: u64,
        shared: u64,
    }
    let mut adapters = Vec::new();
    let dxgi = (|| -> Result<(), String> {
        let factory =
            unsafe { CreateDXGIFactory1::<IDXGIFactory1>() }.map_err(|error| error.to_string())?;
        let mut index = 0;
        loop {
            let adapter = match unsafe { factory.EnumAdapters1(index) } {
                Ok(adapter) => adapter,
                Err(error) if error.code() == DXGI_ERROR_NOT_FOUND => break,
                Err(error) => return Err(error.to_string()),
            };
            index += 1;
            let desc = unsafe { adapter.GetDesc1() }.map_err(|error| error.to_string())?;
            if desc.Flags & DXGI_ADAPTER_FLAG_SOFTWARE.0 as u32 != 0 {
                continue;
            }
            let name = utf16(&desc.Description);
            if name.is_empty() || virtual_device(&name) {
                continue;
            }
            adapters.push(Adapter {
                name,
                dedicated: desc.DedicatedVideoMemory as u64,
                shared: desc.SharedSystemMemory as u64,
            });
        }
        Ok(())
    })();
    if let Err(error) = dxgi {
        reader.errors.push(format!("读取 DXGI 显卡：{error}"));
    }
    let devices = reader.query("root/cimv2", "Win32_VideoController", &["Name"], None);
    let mut values = Vec::new();
    for name in devices
        .iter()
        .filter_map(|item| item.text("Name"))
        .filter(|name| !virtual_device(name))
    {
        if let Some(ix) = adapters
            .iter()
            .position(|adapter| adapter.name.eq_ignore_ascii_case(&name))
        {
            let adapter = adapters.remove(ix);
            values.push(format!(
                "{name}（专用显存 {}，共享上限 {}）",
                format_bytes(adapter.dedicated),
                format_bytes(adapter.shared)
            ));
        } else {
            values.push(name);
        }
    }
    values.extend(adapters.into_iter().map(|adapter| {
        format!(
            "{}（专用显存 {}，共享上限 {}）",
            adapter.name,
            format_bytes(adapter.dedicated),
            format_bytes(adapter.shared)
        )
    }));
    snapshot.add(HardwareKind::Graphics, values);
}

fn read_batteries(reader: &mut Reader<'_>, snapshot: &mut Snapshot) {
    let batteries = reader.query("root/cimv2", "Win32_Battery", &["Name"], None);
    if batteries.is_empty() {
        return;
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
    let values = if static_data.is_empty() {
        batteries
            .iter()
            .filter_map(|item| item.text("Name"))
            .collect()
    } else {
        static_data
            .iter()
            .map(|item| {
                let identity = item.text("InstanceName");
                let mut parts = vec![item.text("DeviceName").unwrap_or_else(|| "电池".into())];
                let design = battery_capacity(item.number("DesignedCapacity"));
                let charged = full
                    .iter()
                    .find(|row| row.text("InstanceName") == identity)
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
                    .find(|row| row.text("InstanceName") == identity)
                    .and_then(|row| row.number("CycleCount"))
                    .filter(|&value| value != u32::MAX as u64)
                {
                    parts.push(format!("循环 {count} 次"));
                }
                parts.join(" · ")
            })
            .collect()
    };
    snapshot.add(HardwareKind::Battery, values);
}

fn read_monitors(reader: &mut Reader<'_>, snapshot: &mut Snapshot) {
    let ids = reader.query(
        "root/WMI",
        "WmiMonitorID",
        &["InstanceName", "UserFriendlyName"],
        Some("Active = TRUE"),
    );
    let sizes = reader.query(
        "root/WMI",
        "WmiMonitorBasicDisplayParams",
        &[
            "InstanceName",
            "MaxHorizontalImageSize",
            "MaxVerticalImageSize",
        ],
        Some("Active = TRUE"),
    );
    let mut values = Vec::new();
    let mut display_index = 0;
    loop {
        let mut display = DISPLAY_DEVICEW {
            cb: size_of::<DISPLAY_DEVICEW>() as u32,
            ..Default::default()
        };
        if !unsafe { EnumDisplayDevicesW(None, display_index, &mut display, 0) }.as_bool() {
            break;
        }
        display_index += 1;
        if !display
            .StateFlags
            .contains(DISPLAY_DEVICE_ATTACHED_TO_DESKTOP)
        {
            continue;
        }
        let mut mode = DEVMODEW {
            dmSize: size_of::<DEVMODEW>() as u16,
            ..Default::default()
        };
        let has_mode = unsafe {
            EnumDisplaySettingsW(
                PCWSTR(display.DeviceName.as_ptr()),
                ENUM_CURRENT_SETTINGS,
                &mut mode,
            )
        }
        .as_bool();
        let mut monitor_index = 0;
        loop {
            let mut monitor = DISPLAY_DEVICEW {
                cb: size_of::<DISPLAY_DEVICEW>() as u32,
                ..Default::default()
            };
            if !unsafe {
                EnumDisplayDevicesW(
                    PCWSTR(display.DeviceName.as_ptr()),
                    monitor_index,
                    &mut monitor,
                    // EDD_GET_DEVICE_INTERFACE_NAME from wingdi.h.
                    1,
                )
            }
            .as_bool()
            {
                break;
            }
            monitor_index += 1;
            if !monitor.StateFlags.contains(DISPLAY_DEVICE_ACTIVE) {
                continue;
            }
            let device_id = utf16(&monitor.DeviceID);
            let id = ids.iter().find(|row| {
                row.text("InstanceName")
                    .is_some_and(|instance| monitor_matches(&device_id, &instance))
            });
            let name = id
                .map(|row| decode_monitor_text(row.numbers("UserFriendlyName")))
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| utf16(&monitor.DeviceString));
            let mut details = Vec::new();
            if let Some(size) = id.and_then(|id| {
                sizes
                    .iter()
                    .find(|row| row.text("InstanceName") == id.text("InstanceName"))
            }) && let Some((width, height)) = positive(size.number("MaxHorizontalImageSize"))
                .zip(positive(size.number("MaxVerticalImageSize")))
            {
                details.push(format!(
                    "{:.1}″",
                    (width as f64).hypot(height as f64) / 2.54
                ));
            }
            if has_mode {
                details.push(format!("{} × {}", mode.dmPelsWidth, mode.dmPelsHeight));
                if mode.dmDisplayFrequency > 1 {
                    details.push(format!("{} Hz", mode.dmDisplayFrequency));
                }
            }
            let primary = if display.StateFlags.contains(DISPLAY_DEVICE_PRIMARY_DEVICE) {
                "主屏 "
            } else {
                ""
            };
            values.push(if details.is_empty() {
                format!("{primary}{name}")
            } else {
                format!("{primary}{name} [{}]", details.join(" · "))
            });
        }
    }
    snapshot.add(HardwareKind::Monitor, values);
}

fn monitor_matches(device: &str, instance: &str) -> bool {
    let device = device
        .trim_start_matches(r"\\?\")
        .split('#')
        .take(3)
        .collect::<Vec<_>>()
        .join("\\");
    let instance = instance.rsplit_once('_').map_or(instance, |(base, _)| base);
    device.eq_ignore_ascii_case(instance)
}

fn decode_monitor_text(value: &[u64]) -> String {
    let text = value
        .iter()
        .take_while(|&&c| c != 0)
        .map(|&c| c as u16)
        .collect::<Vec<_>>();
    String::from_utf16_lossy(&text).trim().to_owned()
}

fn utf16(value: &[u16]) -> String {
    String::from_utf16_lossy(value.split(|&c| c == 0).next().unwrap_or_default())
        .trim()
        .to_owned()
}

fn virtual_device(value: &str) -> bool {
    let value = value.to_ascii_lowercase();
    [
        "virtual",
        "basic render",
        "microsoft remote",
        "wan miniport",
        "vb-audio",
        "voicemeeter",
        "nahimic",
        "sonar",
        "steam streaming",
    ]
    .iter()
    .any(|keyword| value.contains(keyword))
}

fn joined(item: &Record, fields: &[&str]) -> String {
    fields
        .iter()
        .filter_map(|field| item.text(field))
        .collect::<Vec<_>>()
        .join(" ")
}

fn positive(value: Option<u64>) -> Option<u64> {
    value.filter(|&value| value > 0)
}

fn battery_capacity(value: Option<u64>) -> Option<u64> {
    positive(value).filter(|&value| value != u32::MAX as u64)
}

fn registry_string(path: PCWSTR, name: PCWSTR, errors: &mut Vec<String>) -> Option<String> {
    unsafe {
        let mut bytes = 0;
        let status = RegGetValueW(
            HKEY_LOCAL_MACHINE,
            path,
            name,
            RRF_RT_REG_SZ,
            None,
            None,
            Some(&mut bytes),
        );
        if status == windows::Win32::Foundation::ERROR_FILE_NOT_FOUND {
            return None;
        }
        if status.is_err() {
            errors.push(format!(
                "读取注册表 {}：{}",
                name.to_string().unwrap_or_default(),
                status.to_hresult()
            ));
            return None;
        }
        let mut buffer = vec![0u16; bytes as usize / 2];
        let status = RegGetValueW(
            HKEY_LOCAL_MACHINE,
            path,
            name,
            RRF_RT_REG_SZ,
            None,
            Some(buffer.as_mut_ptr().cast()),
            Some(&mut bytes),
        );
        if status.is_err() {
            errors.push(format!(
                "读取注册表 {}：{}",
                name.to_string().unwrap_or_default(),
                status.to_hresult()
            ));
            return None;
        }
        Some(utf16(&buffer)).filter(|value| !value.is_empty())
    }
}

fn registry_number(path: PCWSTR, name: PCWSTR, errors: &mut Vec<String>) -> Option<u32> {
    unsafe {
        let mut value = 0u32;
        let mut bytes = size_of::<u32>() as u32;
        let status = RegGetValueW(
            HKEY_LOCAL_MACHINE,
            path,
            name,
            RRF_RT_REG_DWORD,
            None,
            Some((&mut value as *mut u32).cast()),
            Some(&mut bytes),
        );
        if status == windows::Win32::Foundation::ERROR_FILE_NOT_FOUND {
            return None;
        }
        if status.is_err() {
            errors.push(format!(
                "读取注册表 {}：{}",
                name.to_string().unwrap_or_default(),
                status.to_hresult()
            ));
            return None;
        }
        Some(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn monitor_identity_keeps_the_device_instance() {
        assert!(monitor_matches(
            r"\\?\DISPLAY#DEL1234#5&ABC&0&UID4352#{guid}",
            r"DISPLAY\DEL1234\5&abc&0&UID4352_0"
        ));
        assert!(!monitor_matches(
            r"\\?\DISPLAY#DEL1234#5&ABC&0&UID4352#{guid}",
            r"DISPLAY\DEL1234\5&ABC&0&UID4353_0"
        ));
    }

    #[test]
    #[ignore = "queries this Windows machine through MI and DXGI"]
    fn local_system_information() {
        let snapshot = read(&AtomicBool::new(false));
        for error in &snapshot.errors {
            eprintln!("{error}");
        }
        eprintln!(
            "hardware rows={}, devices={}",
            snapshot.rows.len(),
            snapshot
                .rows
                .iter()
                .map(|row| row.devices.len())
                .sum::<usize>()
        );
        assert!(!snapshot.model.is_empty(), "local MI computer query failed");
        assert!(
            !snapshot.os.is_empty(),
            "local MI operating system query failed"
        );
        assert!(snapshot.rows.len() <= 9);
        for kind in [
            HardwareKind::Mainboard,
            HardwareKind::Processor,
            HardwareKind::Memory,
        ] {
            assert!(
                snapshot
                    .rows
                    .iter()
                    .any(|row| row.kind == kind && !row.devices.is_empty())
            );
        }
        assert!(snapshot.errors.is_empty(), "hardware summary query failed");
    }
}
