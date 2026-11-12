use super::{Reader, Snapshot, joined, utf16};
use windows::{
    Win32::System::{Registry::*, SystemInformation::GetTickCount64},
    core::{PCWSTR, w},
};

pub(in crate::system_info) fn uptime() -> u64 {
    unsafe { GetTickCount64() / 1000 }
}

pub(super) fn read(reader: &mut Reader<'_>, snapshot: &mut Snapshot) {
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
        &["Caption", "BuildNumber"],
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
