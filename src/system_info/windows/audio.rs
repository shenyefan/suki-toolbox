use super::{
    HardwareDevice,
    pnp::{
        device_interface_instance_id, device_interface_name, device_node_guid, device_node_name,
    },
};
use std::{
    collections::HashSet,
    sync::atomic::{AtomicBool, Ordering},
};
use windows::{
    Win32::{
        Devices::{
            DeviceAndDriverInstallation::{
                CM_GET_DEVICE_INTERFACE_LIST_ALL_DEVICES, CM_Get_Device_Interface_List_SizeW,
                CM_Get_Device_Interface_ListW, CM_Get_Parent, CM_LOCATE_DEVNODE_NORMAL,
                CM_Locate_DevNodeW, CR_NO_SUCH_DEVNODE, CR_SUCCESS, GUID_BUS_TYPE_SW_DEVICE,
                GUID_DEVCLASS_MEDIA,
            },
            Properties::{DEVPKEY_Device_BusTypeGuid, DEVPKEY_Device_ClassGuid},
        },
        Media::KernelStreaming::KSCATEGORY_AUDIO,
    },
    core::{GUID, PCWSTR},
};

struct AudioDevice {
    node: u32,
    name: Option<String>,
    bus: Option<GUID>,
}

pub(super) fn read(cancelled: &AtomicBool, errors: &mut Vec<String>) -> Vec<HardwareDevice> {
    let interfaces = match audio_interfaces() {
        Ok(interfaces) => interfaces,
        Err(error) => {
            errors.push(format!("枚举声卡接口：{error}"));
            return Vec::new();
        }
    };
    let mut seen = HashSet::new();
    let mut chains = Vec::new();
    let mut unidentified = Vec::new();
    for interface in interfaces
        .split_inclusive(|&value| value == 0)
        .take_while(|value| value[0] != 0)
    {
        if cancelled.load(Ordering::Relaxed) {
            break;
        }
        let instance_id = match device_interface_instance_id(PCWSTR(interface.as_ptr())) {
            Ok(instance_id) => instance_id,
            Err(error) => {
                errors.push(format!("读取声卡接口身份：{error}"));
                let name = device_interface_name(PCWSTR(interface.as_ptr()), errors);
                unidentified.push(HardwareDevice::new(name.unwrap_or_default(), None));
                continue;
            }
        };
        // Several wave, capture and topology interfaces can belong to one devnode.
        if !seen.insert(instance_id.to_ascii_uppercase()) {
            continue;
        }
        let mut node = 0;
        let instance_id: Vec<_> = instance_id.encode_utf16().chain(Some(0)).collect();
        let status = unsafe {
            CM_Locate_DevNodeW(
                &mut node,
                PCWSTR(instance_id.as_ptr()),
                CM_LOCATE_DEVNODE_NORMAL,
            )
        };
        // ALL_DEVICES includes old registrations. Keep configured devices,
        // including disabled ones, but not detached/removed device instances.
        if status == CR_NO_SUCH_DEVNODE {
            continue;
        }
        if status != CR_SUCCESS {
            errors.push(format!("定位声卡设备：{}", status.0));
            let name = device_interface_name(PCWSTR(interface.as_ptr()), errors);
            unidentified.push(HardwareDevice::new(name.unwrap_or_default(), None));
            continue;
        }
        let media_device = match device_node_guid(node, &DEVPKEY_Device_ClassGuid) {
            Ok(class) => class == Some(GUID_DEVCLASS_MEDIA),
            Err(error) => {
                errors.push(format!("读取声卡设备类别：{error}"));
                false
            }
        };
        let mut chain = Vec::new();
        loop {
            if cancelled.load(Ordering::Relaxed) {
                break;
            }
            let name = device_node_name(node, errors).or_else(|| {
                if !chain.is_empty() {
                    return None;
                }
                device_interface_name(PCWSTR(interface.as_ptr()), errors)
            });
            let bus = match device_node_guid(node, &DEVPKEY_Device_BusTypeGuid) {
                Ok(bus) => bus,
                Err(error) => {
                    errors.push(format!("读取声卡总线属性：{error}"));
                    None
                }
            };
            chain.push(AudioDevice { node, name, bus });
            if !media_device {
                break;
            }
            let mut parent = 0;
            let status = unsafe { CM_Get_Parent(&mut parent, node, 0) };
            if status == CR_NO_SUCH_DEVNODE {
                break;
            }
            if status != CR_SUCCESS {
                errors.push(format!("读取声卡父设备：{}", status.0));
                break;
            }
            match device_node_guid(parent, &DEVPKEY_Device_ClassGuid) {
                // ACX can expose separate MEDIA function children. Only walk
                // a continuous audio-class chain, never their shared PCI/USB bus.
                Ok(Some(class)) if class == GUID_DEVCLASS_MEDIA => node = parent,
                Ok(_) => break,
                Err(error) => {
                    errors.push(format!("读取声卡父设备类别：{error}"));
                    break;
                }
            }
        }
        chains.push(chain);
    }
    let mut devices = merge_audio(chains);
    devices.extend(unidentified);
    devices
}

fn audio_interfaces() -> Result<Vec<u16>, String> {
    let mut length = 0;
    let status = unsafe {
        CM_Get_Device_Interface_List_SizeW(
            &mut length,
            &KSCATEGORY_AUDIO,
            PCWSTR::null(),
            CM_GET_DEVICE_INTERFACE_LIST_ALL_DEVICES,
        )
    };
    if status != CR_SUCCESS {
        return Err(format!("CM_Get_Device_Interface_List_SizeW：{}", status.0));
    }
    let mut interfaces = vec![0u16; length as usize];
    let status = unsafe {
        CM_Get_Device_Interface_ListW(
            &KSCATEGORY_AUDIO,
            PCWSTR::null(),
            &mut interfaces,
            CM_GET_DEVICE_INTERFACE_LIST_ALL_DEVICES,
        )
    };
    if status != CR_SUCCESS {
        return Err(format!("CM_Get_Device_Interface_ListW：{}", status.0));
    }
    Ok(interfaces)
}

fn merge_audio(chains: Vec<Vec<AudioDevice>>) -> Vec<HardwareDevice> {
    let mut adapters: Vec<AudioDevice> = Vec::new();
    for mut chain in chains {
        if let Some(mut adapter) = chain.pop() {
            // Prefer the adapter name, then the nearest named audio function.
            adapter.name = adapter
                .name
                .or_else(|| chain.into_iter().rev().find_map(|device| device.name));
            if let Some(existing) = adapters.iter_mut().find(|item| item.node == adapter.node) {
                existing.name = existing.name.take().or(adapter.name);
                existing.bus = existing.bus.or(adapter.bus);
            } else {
                adapters.push(adapter);
            }
        }
    }
    adapters
        .into_iter()
        .map(|adapter| {
            // Software-created endpoints are not enumerated here. This is the
            // adapter's own bus; other/missing buses leave its type unconfirmed.
            let virtual_device = adapter
                .bus
                .filter(|&bus| bus == GUID_BUS_TYPE_SW_DEVICE)
                .map(|_| true);
            HardwareDevice::new(adapter.name.unwrap_or_default(), virtual_device)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn device(node: u32, name: Option<&str>) -> AudioDevice {
        AudioDevice {
            node,
            name: name.map(str::to_owned),
            bus: None,
        }
    }

    #[test]
    fn interfaces_and_audio_function_children_form_one_adapter() {
        let devices = merge_audio(vec![
            vec![device(1, Some("Senary"))],
            vec![device(1, Some("Senary"))],
            vec![
                device(3, Some("HDMI function")),
                device(2, Some("Display Audio")),
            ],
            vec![
                device(4, Some("DP function")),
                device(2, Some("Display Audio")),
            ],
            vec![device(2, Some("Display Audio"))],
        ]);
        assert_eq!(devices.len(), 2);
        assert_eq!(devices[0].text, "Senary");
        assert_eq!(devices[1].text, "Display Audio");
    }

    #[test]
    fn different_adapters_keep_identical_names_and_use_available_function_names() {
        let devices = merge_audio(vec![
            vec![device(1, Some("USB Audio"))],
            vec![device(2, Some("USB Audio"))],
            vec![device(4, Some("Named function")), device(3, None)],
            vec![device(5, None)],
        ]);
        assert_eq!(devices.len(), 4);
        assert_eq!(devices[0].text, devices[1].text);
        assert_eq!(devices[2].text, "Named function");
        assert_eq!(devices[3].text, "未知设备");
    }

    #[test]
    fn virtual_type_comes_from_the_adapter_and_not_its_function_name_or_bus() {
        let mut virtual_adapter = device(2, Some("Audio"));
        virtual_adapter.bus = Some(GUID_BUS_TYPE_SW_DEVICE);
        let mut function = device(3, Some("Function"));
        function.bus = Some(GUID_BUS_TYPE_SW_DEVICE);
        let devices = merge_audio(vec![
            vec![device(1, Some("Virtual Audio"))],
            vec![virtual_adapter],
            vec![function, device(4, Some("Physical adapter"))],
        ]);
        assert_eq!(devices[0].virtual_device, None);
        assert_eq!(devices[1].virtual_device, Some(true));
        assert_eq!(devices[2].virtual_device, None);
    }
}
