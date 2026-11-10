//! Windows shell boundary; launching never claims a running-process state.
use super::Tool;
use std::os::windows::ffi::OsStrExt;
use std::{io, path::Path};
use windows::{
    Win32::{
        System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx, CoUninitialize},
        UI::{
            Shell::{SEE_MASK_FLAG_NO_UI, SEE_MASK_NOASYNC, SHELLEXECUTEINFOW, ShellExecuteExW},
            WindowsAndMessaging::SW_SHOWNORMAL,
        },
    },
    core::PCWSTR,
};

#[derive(Clone, Copy)]
pub(super) enum ToolAction {
    Open,
    Administrator,
    Reveal,
    Link,
}

pub(super) fn execute(tool: &Tool, action: ToolAction) -> Result<(), String> {
    let action = if matches!(action, ToolAction::Open) && tool.exe.is_none() {
        ToolAction::Link
    } else {
        action
    };
    let directory = tool
        .exe
        .as_ref()
        .and_then(|path| path.parent())
        .unwrap_or(Path::new(""));
    let (file, parameters, verb) = match action {
        ToolAction::Link => {
            let url = tool.link.trim();
            if !(url.starts_with("https://") || url.starts_with("http://")) {
                return Err("链接必须使用 http:// 或 https://".into());
            }
            (std::path::PathBuf::from(url), String::new(), "open")
        }
        ToolAction::Open | ToolAction::Administrator => {
            let path = tool.exe.as_ref().ok_or("工具路径为空")?;
            let (file, parameters) = launch_target(path, &tool.arguments)?;
            (
                file,
                parameters,
                if matches!(action, ToolAction::Administrator) {
                    "runas"
                } else {
                    "open"
                },
            )
        }
        ToolAction::Reveal => {
            let path = tool.exe.as_ref().ok_or("工具路径为空")?;
            let explorer = std::env::var_os("WINDIR").ok_or("无法定位 Windows 目录")?;
            (
                Path::new(&explorer).join("explorer.exe"),
                format!("/select,\"{}\"", path.display()),
                "open",
            )
        }
    };
    let file = if matches!(action, ToolAction::Link) {
        file
    } else {
        file.components().collect()
    };
    let directory: std::path::PathBuf = directory.components().collect();
    let wide = |value: &std::ffi::OsStr| -> Result<Vec<u16>, String> {
        let mut value: Vec<u16> = value.encode_wide().collect();
        if value.contains(&0) {
            return Err("路径或参数包含空字符".into());
        }
        value.push(0);
        Ok(value)
    };
    let file = wide(file.as_os_str())?;
    let parameters = wide(parameters.as_ref())?;
    let verb = wide(verb.as_ref())?;
    let directory = wide(directory.as_os_str())?;
    // All pointers refer to live NUL-terminated buffers for this synchronous call.
    unsafe {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED)
            .ok()
            .map_err(|error| error.to_string())?;
        let mut info = SHELLEXECUTEINFOW {
            cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
            fMask: SEE_MASK_NOASYNC | SEE_MASK_FLAG_NO_UI,
            lpVerb: PCWSTR(verb.as_ptr()),
            lpFile: PCWSTR(file.as_ptr()),
            lpParameters: PCWSTR(parameters.as_ptr()),
            lpDirectory: PCWSTR(directory.as_ptr()),
            nShow: SW_SHOWNORMAL.0,
            ..Default::default()
        };
        let result = ShellExecuteExW(&mut info);
        CoUninitialize();
        result.map_err(|error| io::Error::from_raw_os_error(error.code().0 & 0xffff).to_string())
    }
}

/// Script hosts are explicit so file associations cannot open an editor instead.
fn launch_target(path: &Path, arguments: &str) -> Result<(std::path::PathBuf, String), String> {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("");
    let host = match extension.to_ascii_lowercase().as_str() {
        "bat" | "cmd" => Some((
            "cmd.exe",
            format!("/d /s /c \"\"{}\" {}\"", path.display(), arguments),
        )),
        "ps1" => Some((
            "WindowsPowerShell/v1.0/powershell.exe",
            format!(
                "-NoLogo -NoProfile -File \"{}\" {}",
                path.display(),
                arguments
            ),
        )),
        "vbs" | "js" => Some((
            "cscript.exe",
            format!("//nologo \"{}\" {}", path.display(), arguments),
        )),
        _ => None,
    };
    match host {
        Some((host, parameters)) => {
            let windows = std::env::var_os("WINDIR").ok_or("无法定位 Windows 目录")?;
            Ok((Path::new(&windows).join("System32").join(host), parameters))
        }
        None => Ok((path.to_path_buf(), arguments.to_owned())),
    }
}

#[cfg(test)]
mod tests {
    use super::launch_target;
    use std::path::Path;

    #[test]
    fn script_hosts_preserve_paths_and_arguments() {
        let (host, parameters) =
            launch_target(Path::new(r"C:\工具 目录\检查.PS1"), r#"-Name "测试 值""#).unwrap();
        assert!(host.ends_with("WindowsPowerShell/v1.0/powershell.exe"));
        assert_eq!(
            parameters,
            r#"-NoLogo -NoProfile -File "C:\工具 目录\检查.PS1" -Name "测试 值""#
        );
        assert!(!parameters.contains("Bypass"));
        for extension in ["bat", "cmd"] {
            let path = format!(r"C:\工具 & 目录\检查.{extension}");
            let (host, parameters) = launch_target(Path::new(&path), r#""参数 值""#).unwrap();
            assert!(host.ends_with("cmd.exe"));
            assert_eq!(parameters, format!("/d /s /c \"\"{path}\" \"参数 值\"\""));
        }
        for extension in ["vbs", "js"] {
            let path = format!(r"C:\工具\检查.{extension}");
            let (host, parameters) = launch_target(Path::new(&path), "/test").unwrap();
            assert!(host.ends_with("cscript.exe"));
            assert_eq!(parameters, format!("//nologo \"{path}\" /test"));
        }
        let path = Path::new(r"C:\工具\应用.exe");
        assert_eq!(
            launch_target(path, "--test").unwrap(),
            (path.to_path_buf(), "--test".into())
        );
    }
}
