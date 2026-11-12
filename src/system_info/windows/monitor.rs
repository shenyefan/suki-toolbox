use super::pnp::{device_friendly_name, wmi_instance_matches};
use super::{HardwareDevice, Reader, positive, utf16};
use windows::{Win32::Devices::Display::*, core::PCWSTR};

pub(super) fn read(reader: &mut Reader<'_>) -> Vec<HardwareDevice> {
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
    let (paths, modes) = match active_display_paths() {
        Ok(topology) => topology,
        Err(error) => {
            reader.errors.push(error);
            return Vec::new();
        }
    };
    let mut values = Vec::new();
    for path in paths {
        let mut target = DISPLAYCONFIG_TARGET_DEVICE_NAME {
            header: DISPLAYCONFIG_DEVICE_INFO_HEADER {
                r#type: DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME,
                size: size_of::<DISPLAYCONFIG_TARGET_DEVICE_NAME>() as u32,
                adapterId: path.targetInfo.adapterId,
                id: path.targetInfo.id,
            },
            ..Default::default()
        };
        let status = unsafe { DisplayConfigGetDeviceInfo(&mut target.header) };
        if status != 0 {
            reader.errors.push(format!("读取显示器设备名称：{status}"));
        }
        let device_id = utf16(&target.monitorDevicePath);
        let id = ids.iter().find(|row| {
            row.text("InstanceName")
                .is_some_and(|instance| monitor_matches(&device_id, &instance))
        });
        let mut name = id
            .map(|row| decode_monitor_text(row.numbers("UserFriendlyName")))
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| utf16(&target.monitorFriendlyDeviceName));
        if name.is_empty() && target.monitorDevicePath[0] != 0 {
            name = device_friendly_name(
                PCWSTR(target.monitorDevicePath.as_ptr()),
                &mut reader.errors,
            )
            .unwrap_or_default();
        }
        let name = if name.is_empty() {
            "未知设备".into()
        } else {
            name
        };
        let virtual_device =
            (path.targetInfo.outputTechnology != DISPLAYCONFIG_OUTPUT_TECHNOLOGY_OTHER).then_some(
                path.targetInfo.outputTechnology
                    == DISPLAYCONFIG_OUTPUT_TECHNOLOGY_INDIRECT_VIRTUAL,
            );
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
        // QDC_ONLY_ACTIVE_PATHS without VIRTUAL_MODE_AWARE returns modeInfoIdx,
        // not the packed virtual-mode indexes.
        let source = modes
            .get(unsafe { path.sourceInfo.Anonymous.modeInfoIdx } as usize)
            .filter(|mode| mode.infoType == DISPLAYCONFIG_MODE_INFO_TYPE_SOURCE)
            .map(|mode| unsafe { mode.Anonymous.sourceMode });
        if let Some(source) = source {
            details.push(format!("{} × {}", source.width, source.height));
        }
        let rate = path.targetInfo.refreshRate;
        if rate.Numerator > 0 && rate.Denominator > 0 {
            let hz = format!("{:.2}", rate.Numerator as f64 / rate.Denominator as f64);
            details.push(format!(
                "{} Hz",
                hz.trim_end_matches('0').trim_end_matches('.')
            ));
        }
        let primary = source.is_some_and(|source| source.position.x == 0 && source.position.y == 0);
        let text = if details.is_empty() {
            name
        } else {
            format!("{name} [{}]", details.join(" · "))
        };
        values.push(HardwareDevice {
            primary,
            ..HardwareDevice::new(text, virtual_device)
        });
    }
    values
}

fn active_display_paths()
-> Result<(Vec<DISPLAYCONFIG_PATH_INFO>, Vec<DISPLAYCONFIG_MODE_INFO>), String> {
    let mut path_count = 0;
    let mut mode_count = 0;
    let status = unsafe {
        GetDisplayConfigBufferSizes(QDC_ONLY_ACTIVE_PATHS, &mut path_count, &mut mode_count)
    };
    if status.0 != 0 {
        return Err(format!("读取显示拓扑容量：{}", status.0));
    }
    let mut paths = vec![DISPLAYCONFIG_PATH_INFO::default(); path_count as usize];
    let mut modes = vec![DISPLAYCONFIG_MODE_INFO::default(); mode_count as usize];
    let status = unsafe {
        QueryDisplayConfig(
            QDC_ONLY_ACTIVE_PATHS,
            &mut path_count,
            paths.as_mut_ptr(),
            &mut mode_count,
            modes.as_mut_ptr(),
            None,
        )
    };
    if status.0 != 0 {
        return Err(format!("读取显示拓扑：{}", status.0));
    }
    paths.truncate(path_count as usize);
    modes.truncate(mode_count as usize);
    Ok((paths, modes))
}

fn monitor_matches(device: &str, instance: &str) -> bool {
    let device = device
        .trim_start_matches(r"\\?\")
        .split('#')
        .take(3)
        .collect::<Vec<_>>()
        .join("\\");
    wmi_instance_matches(instance, &device)
}

fn decode_monitor_text(value: &[u64]) -> String {
    let text = value
        .iter()
        .take_while(|&&c| c != 0)
        .map(|&c| c as u16)
        .collect::<Vec<_>>();
    String::from_utf16_lossy(&text).trim().to_owned()
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
}
