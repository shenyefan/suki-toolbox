use super::utf16;
use windows::{
    Win32::{
        Devices::{
            DeviceAndDriverInstallation::{
                CM_Get_DevNode_PropertyW, CM_Get_Device_Interface_PropertyW,
                CM_LOCATE_DEVNODE_NORMAL, CM_Locate_DevNodeW, CONFIGRET, CR_BUFFER_SMALL,
                CR_NO_SUCH_VALUE, CR_SUCCESS, MAX_DEVICE_ID_LEN,
            },
            Properties::{
                DEVPKEY_Device_DeviceDesc, DEVPKEY_Device_FriendlyName, DEVPKEY_Device_InstanceId,
                DEVPKEY_DeviceInterface_FriendlyName, DEVPROP_TYPE_GUID, DEVPROP_TYPE_STRING,
                DEVPROPTYPE,
            },
        },
        Foundation::DEVPROPKEY,
    },
    core::{GUID, PCWSTR},
};

pub(super) fn device_interface_instance_id(path: PCWSTR) -> Result<String, String> {
    let mut buffer = [0u16; MAX_DEVICE_ID_LEN as usize];
    let mut bytes = size_of_val(&buffer) as u32;
    let mut kind = DEVPROPTYPE::default();
    let status = unsafe {
        CM_Get_Device_Interface_PropertyW(
            path,
            &DEVPKEY_Device_InstanceId,
            &mut kind,
            Some(buffer.as_mut_ptr().cast()),
            &mut bytes,
            0,
        )
    };
    if status != CR_SUCCESS || kind != DEVPROP_TYPE_STRING {
        return Err(format!("CM_Get_Device_Interface_PropertyW：{}", status.0));
    }
    Ok(utf16(&buffer))
}

fn device_node(instance_id: &str) -> Result<u32, String> {
    let instance_id: Vec<_> = instance_id.encode_utf16().chain(Some(0)).collect();
    let mut node = 0;
    let status = unsafe {
        CM_Locate_DevNodeW(
            &mut node,
            PCWSTR(instance_id.as_ptr()),
            CM_LOCATE_DEVNODE_NORMAL,
        )
    };
    if status != CR_SUCCESS {
        return Err(format!("CM_Locate_DevNodeW：{}", status.0));
    }
    Ok(node)
}

pub(super) fn device_node_guid(node: u32, key: &DEVPROPKEY) -> Result<Option<GUID>, String> {
    let mut value = GUID::default();
    let mut kind = DEVPROPTYPE::default();
    let mut bytes = size_of::<GUID>() as u32;
    let status = unsafe {
        CM_Get_DevNode_PropertyW(
            node,
            key,
            &mut kind,
            Some((&mut value as *mut GUID).cast()),
            &mut bytes,
            0,
        )
    };
    if status == CR_NO_SUCH_VALUE {
        return Ok(None);
    }
    if status != CR_SUCCESS || kind != DEVPROP_TYPE_GUID {
        return Err(format!("CM_Get_DevNode_PropertyW：{}", status.0));
    }
    Ok(Some(value))
}

pub(super) fn device_interface_name(interface: PCWSTR, errors: &mut Vec<String>) -> Option<String> {
    match property_text(|kind, buffer, bytes| unsafe {
        CM_Get_Device_Interface_PropertyW(
            interface,
            &DEVPKEY_DeviceInterface_FriendlyName,
            kind,
            buffer,
            bytes,
            0,
        )
    }) {
        Ok(name) => name,
        Err(error) => {
            errors.push(format!("读取设备接口名称：{error}"));
            None
        }
    }
}

pub(super) fn device_friendly_name(interface: PCWSTR, errors: &mut Vec<String>) -> Option<String> {
    match device_interface_instance_id(interface) {
        Ok(instance_id) => pnp_device_name(Some(&instance_id), errors)
            .or_else(|| device_interface_name(interface, errors)),
        Err(error) => {
            errors.push(format!("读取设备接口身份：{error}"));
            device_interface_name(interface, errors)
        }
    }
}

pub(super) fn pnp_device_name(
    instance_id: Option<&str>,
    errors: &mut Vec<String>,
) -> Option<String> {
    let instance_id = instance_id?;
    match device_node(instance_id) {
        Ok(node) => device_node_name(node, errors),
        Err(error) => {
            errors.push(format!("读取设备 PnP 名称：{error}"));
            None
        }
    }
}

pub(super) fn device_node_name(node: u32, errors: &mut Vec<String>) -> Option<String> {
    for key in [DEVPKEY_Device_FriendlyName, DEVPKEY_Device_DeviceDesc] {
        let value = property_text(|kind, buffer, bytes| unsafe {
            CM_Get_DevNode_PropertyW(node, &key, kind, buffer, bytes, 0)
        });
        match value {
            Ok(Some(name)) => return Some(name),
            Ok(None) => {}
            Err(error) => errors.push(format!("读取设备名称：{error}")),
        }
    }
    None
}

fn property_text(
    mut read: impl FnMut(&mut DEVPROPTYPE, Option<*mut u8>, &mut u32) -> CONFIGRET,
) -> Result<Option<String>, String> {
    let mut kind = DEVPROPTYPE::default();
    let mut bytes = 0;
    let status = read(&mut kind, None, &mut bytes);
    if status == CR_NO_SUCH_VALUE {
        return Ok(None);
    }
    if status != CR_BUFFER_SMALL || kind != DEVPROP_TYPE_STRING {
        return Err(format!(
            "读取文本属性容量：{}，属性类型 {}",
            status.0, kind.0
        ));
    }
    let mut value = vec![0u16; (bytes as usize).div_ceil(size_of::<u16>())];
    let status = read(&mut kind, Some(value.as_mut_ptr().cast()), &mut bytes);
    if status != CR_SUCCESS || kind != DEVPROP_TYPE_STRING {
        return Err(format!("读取文本属性：{}，属性类型 {}", status.0, kind.0));
    }
    let value = utf16(&value);
    Ok((!value.is_empty()).then_some(value))
}

pub(super) fn wmi_instance_matches(instance: &str, device: &str) -> bool {
    instance.eq_ignore_ascii_case(device)
        || instance.rsplit_once('_').is_some_and(|(base, suffix)| {
            !suffix.is_empty()
                && suffix.bytes().all(|value| value.is_ascii_digit())
                && base.eq_ignore_ascii_case(device)
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wmi_names_match_only_the_same_device_with_an_optional_numeric_suffix() {
        assert!(wmi_instance_matches(
            r"ACPI\BATTERY\ONE_0",
            r"acpi\battery\one"
        ));
        assert!(wmi_instance_matches(
            r"ACPI\BATTERY\ONE",
            r"ACPI\BATTERY\ONE"
        ));
        assert!(!wmi_instance_matches(
            r"ACPI\BATTERY\TWO_0",
            r"ACPI\BATTERY\ONE"
        ));
        assert!(!wmi_instance_matches(
            r"ACPI\BATTERY\ONE_extra",
            r"ACPI\BATTERY\ONE"
        ));
        assert!(!wmi_instance_matches(
            r"ACPI\BATTERY\ONE_",
            r"ACPI\BATTERY\ONE"
        ));
    }
}
