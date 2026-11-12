use super::super::model::format_bytes;
use super::pnp::{device_interface_instance_id, pnp_device_name};
use super::{HardwareDevice, Reader, utf16};
use windows::{
    Wdk::Graphics::Direct3D::{
        D3DKMT_ADAPTERTYPE, D3DKMT_CLOSEADAPTER, D3DKMT_OPENADAPTERFROMLUID,
        D3DKMT_QUERYADAPTERINFO, D3DKMTCloseAdapter, D3DKMTOpenAdapterFromLuid,
        D3DKMTQueryAdapterInfo, KMTQAITYPE_ADAPTERTYPE,
    },
    Win32::{Devices::Display::*, Foundation::LUID, Graphics::Dxgi::*},
    core::PCWSTR,
};

struct GraphicsDevice {
    name: Option<String>,
    instance_id: Option<String>,
    memory: Option<(u64, u64)>,
    virtual_device: Option<bool>,
}

pub(super) fn read(reader: &mut Reader<'_>) -> Vec<HardwareDevice> {
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
            // DXGI also exposes the Windows software rasterizer. It is not
            // a graphics device or an installed virtual display adapter.
            if desc.Flags & DXGI_ADAPTER_FLAG_SOFTWARE.0 as u32 != 0 {
                continue;
            }
            let description = utf16(&desc.Description);
            let instance_id = match graphics_instance_id(desc.AdapterLuid) {
                Ok(instance_id) => instance_id,
                Err(error) => {
                    reader
                        .errors
                        .push(format!("读取显卡设备身份（{description}）：{error}"));
                    None
                }
            };
            let virtual_device = match graphics_virtual_device(desc.AdapterLuid) {
                Ok(value) => Some(value),
                Err(error) => {
                    reader
                        .errors
                        .push(format!("读取显卡类型（{description}）：{error}"));
                    None
                }
            };
            adapters.push(GraphicsDevice {
                name: if description.is_empty() {
                    pnp_device_name(instance_id.as_deref(), &mut reader.errors)
                } else {
                    Some(description)
                },
                instance_id,
                memory: Some((
                    desc.DedicatedVideoMemory as u64,
                    desc.SharedSystemMemory as u64,
                )),
                virtual_device,
            });
        }
        Ok(())
    })();
    if let Err(error) = dxgi {
        reader.errors.push(format!("读取 DXGI 显卡：{error}"));
    }
    let devices = reader.query(
        "root/cimv2",
        "Win32_VideoController",
        &["Name", "Caption", "Description", "PNPDeviceID"],
        None,
    );
    let devices = devices
        .iter()
        .map(|item| {
            let instance_id = item.text("PNPDeviceID");
            let name = item
                .first_text(&["Name", "Caption", "Description"])
                .or_else(|| pnp_device_name(instance_id.as_deref(), &mut reader.errors));
            GraphicsDevice {
                name,
                instance_id,
                memory: None,
                // The matching DXGI kernel adapter supplies the device type.
                virtual_device: None,
            }
        })
        .collect();
    merge_graphics(devices, adapters)
}

/// DisplayConfig identifies the enumerated display device itself. A virtual
/// display's DXGI description and physical-adapter PnP key can name its backing GPU.
fn graphics_instance_id(luid: LUID) -> Result<Option<String>, String> {
    let mut name = DISPLAYCONFIG_ADAPTER_NAME {
        header: DISPLAYCONFIG_DEVICE_INFO_HEADER {
            r#type: DISPLAYCONFIG_DEVICE_INFO_GET_ADAPTER_NAME,
            size: size_of::<DISPLAYCONFIG_ADAPTER_NAME>() as u32,
            adapterId: luid,
            id: 0,
        },
        ..Default::default()
    };
    let status = unsafe { DisplayConfigGetDeviceInfo(&mut name.header) };
    if status != 0 {
        return Err(format!("DisplayConfigGetDeviceInfo：{status}"));
    }
    // Software renderers have no display interface.
    if name.adapterDevicePath[0] == 0 {
        return Ok(None);
    }
    device_interface_instance_id(PCWSTR(name.adapterDevicePath.as_ptr())).map(Some)
}

fn graphics_virtual_device(luid: LUID) -> Result<bool, String> {
    let mut adapter = D3DKMT_OPENADAPTERFROMLUID {
        AdapterLuid: luid,
        ..Default::default()
    };
    let status = unsafe { D3DKMTOpenAdapterFromLuid(&mut adapter) };
    if status.0 < 0 {
        return Err(format!("D3DKMTOpenAdapterFromLuid：{:#x}", status.0));
    }
    let mut kind = D3DKMT_ADAPTERTYPE::default();
    let mut query = D3DKMT_QUERYADAPTERINFO {
        hAdapter: adapter.hAdapter,
        Type: KMTQAITYPE_ADAPTERTYPE,
        pPrivateDriverData: (&mut kind as *mut D3DKMT_ADAPTERTYPE).cast(),
        PrivateDriverDataSize: size_of::<D3DKMT_ADAPTERTYPE>() as u32,
    };
    let status = unsafe { D3DKMTQueryAdapterInfo(&mut query) };
    let close = unsafe {
        D3DKMTCloseAdapter(&D3DKMT_CLOSEADAPTER {
            hAdapter: adapter.hAdapter,
        })
    };
    if status.0 < 0 {
        return Err(format!("D3DKMTQueryAdapterInfo：{:#x}", status.0));
    }
    if close.0 < 0 {
        return Err(format!("D3DKMTCloseAdapter：{:#x}", close.0));
    }
    Ok(graphics_type_is_virtual(unsafe { kind.Anonymous.Value }))
}

fn graphics_type_is_virtual(flags: u32) -> bool {
    // D3DKMT_ADAPTERTYPE fields from the Windows SDK d3dkmthk.h.
    const INDIRECT_DISPLAY_DEVICE: u32 = 1 << 6;
    const PARAVIRTUALIZED: u32 = 1 << 7;
    flags & (INDIRECT_DISPLAY_DEVICE | PARAVIRTUALIZED) != 0
}

fn merge_graphics(
    devices: Vec<GraphicsDevice>,
    adapters: Vec<GraphicsDevice>,
) -> Vec<HardwareDevice> {
    let mut merged: Vec<GraphicsDevice> = Vec::new();
    // WMI owns device names; DXGI supplies memory for the same device instance.
    for device in devices.into_iter().chain(adapters) {
        if let Some(existing) = merged.iter_mut().find(|existing| {
            existing
                .instance_id
                .as_ref()
                .zip(device.instance_id.as_ref())
                .is_some_and(|(a, b)| a.eq_ignore_ascii_case(b))
        }) {
            existing.name = existing.name.take().or(device.name);
            existing.memory = existing.memory.or(device.memory);
            existing.virtual_device = existing.virtual_device.max(device.virtual_device);
        } else {
            merged.push(device);
        }
    }
    merged.into_iter().map(graphics_summary).collect()
}

fn graphics_summary(device: GraphicsDevice) -> HardwareDevice {
    let name = device.name.unwrap_or_else(|| "未知设备".into());
    let text = if device.virtual_device != Some(true) {
        if let Some((dedicated, shared)) = device.memory {
            format!(
                "{name}（专用显存 {}，共享上限 {}）",
                format_bytes(dedicated),
                format_bytes(shared)
            )
        } else {
            name
        }
    } else {
        name
    };
    HardwareDevice::new(text, device.virtual_device)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn graphics_device(name: &str, instance_id: Option<&str>, memory: bool) -> GraphicsDevice {
        GraphicsDevice {
            name: (!name.is_empty()).then(|| name.into()),
            instance_id: instance_id.map(str::to_owned),
            memory: memory.then_some((128 * 1024 * 1024, 8 * 1024 * 1024 * 1024)),
            virtual_device: memory.then_some(false),
        }
    }

    #[test]
    fn virtual_display_uses_its_own_identity_instead_of_its_backing_gpu_name() {
        let mut virtual_adapter =
            graphics_device("Intel(R) Graphics", Some(r"ROOT\DISPLAY\0000"), true);
        virtual_adapter.virtual_device = Some(true);
        let lines = merge_graphics(
            vec![
                graphics_device(
                    "GameViewer Virtual Display Adapter",
                    Some(r"ROOT\DISPLAY\0000"),
                    false,
                ),
                graphics_device("Intel(R) Graphics", Some(r"PCI\VEN_8086\ONE"), false),
            ],
            vec![
                graphics_device("Intel(R) Graphics", Some(r"pci\ven_8086\one"), true),
                virtual_adapter,
                graphics_device("Intel alias", Some(r"PCI\VEN_8086\ONE"), true),
            ],
        );
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].text, "GameViewer Virtual Display Adapter");
        assert_eq!(lines[0].virtual_device, Some(true));
        assert_eq!(
            lines[1].text,
            "Intel(R) Graphics（专用显存 128.0 MiB，共享上限 8.0 GiB）"
        );
    }

    #[test]
    fn identical_gpu_models_with_different_device_instances_remain_separate() {
        let lines = merge_graphics(
            vec![
                graphics_device("Same GPU", Some(r"PCI\GPU\ONE"), false),
                graphics_device("Same GPU", Some(r"PCI\GPU\TWO"), false),
            ],
            vec![
                graphics_device("Same GPU", Some(r"PCI\GPU\TWO"), true),
                graphics_device("Same GPU", Some(r"PCI\GPU\ONE"), true),
            ],
        );
        assert_eq!(lines.len(), 2);
        assert!(lines.iter().all(|line| line.text.contains("专用显存")));
    }

    #[test]
    fn unidentified_adapters_are_preserved_without_name_based_merging() {
        let lines = merge_graphics(
            vec![graphics_device("GPU", Some(r"PCI\GPU\ONE"), false)],
            vec![graphics_device("GPU", None, true)],
        );
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].text, "GPU");
        assert_eq!(lines[0].virtual_device, None);
        assert!(lines[1].text.contains("专用显存"));
        assert_eq!(
            merge_graphics(
                vec![],
                vec![graphics_device("GPU", Some(r"PCI\GPU\ONE"), true)]
            )
            .len(),
            1
        );
    }

    #[test]
    fn unnamed_devices_use_the_other_source_or_remain_unknown() {
        let devices = merge_graphics(
            vec![
                graphics_device("", Some(r"PCI\GPU\ONE"), false),
                graphics_device("", Some(r"PCI\GPU\TWO"), false),
                graphics_device("", None, false),
            ],
            vec![graphics_device("DXGI GPU", Some(r"pci\gpu\one"), true)],
        );
        assert_eq!(devices.len(), 3);
        assert!(devices[0].text.starts_with("DXGI GPU（专用显存"));
        assert_eq!(devices[1].text, "未知设备");
        assert_eq!(devices[2].text, "未知设备");
    }

    #[test]
    fn virtual_classification_uses_reported_properties_instead_of_device_names() {
        assert!(graphics_type_is_virtual(1 << 6));
        assert!(graphics_type_is_virtual(1 << 7));
        assert!(graphics_type_is_virtual(0x342));
        assert!(!graphics_type_is_virtual(0x232b));
        assert!(!graphics_type_is_virtual(0));
        let device = graphics_device("Virtual GPU", Some(r"SWD\Example\ONE"), false);
        assert_eq!(device.virtual_device, None);
        let device = graphics_summary(device);
        assert_eq!(device.text, "Virtual GPU");
        assert_eq!(device.virtual_device, None);
    }
}
