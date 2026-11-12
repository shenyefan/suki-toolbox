//! MI owns returned instances until the next GetInstance call. Copy selected
//! properties here, while the operation and its application are still alive.
use std::{
    collections::BTreeMap,
    ptr::{null, null_mut},
    sync::atomic::{AtomicBool, Ordering},
};
use windows::{Win32::System::Wmi::*, core::PCWSTR};

type Result<T> = std::result::Result<T, String>;

pub(super) enum Value {
    Text(String),
    Boolean(bool),
    Number(u64),
    Numbers(Vec<u64>),
}

pub(super) struct Record(BTreeMap<&'static str, Value>);

impl Record {
    pub(super) fn first_text(&self, fields: &[&str]) -> Option<String> {
        fields.iter().find_map(|field| self.text(field))
    }

    pub(super) fn boolean(&self, name: &str) -> Option<bool> {
        match self.0.get(name)? {
            Value::Boolean(value) => Some(*value),
            _ => None,
        }
    }

    pub(super) fn text(&self, name: &str) -> Option<String> {
        match self.0.get(name)? {
            Value::Text(value) => Some(value.clone()),
            _ => None,
        }
    }

    pub(super) fn number(&self, name: &str) -> Option<u64> {
        match self.0.get(name)? {
            Value::Number(value) => Some(*value),
            _ => None,
        }
    }

    pub(super) fn numbers(&self, name: &str) -> &[u64] {
        match self.0.get(name) {
            Some(Value::Numbers(value)) => value,
            _ => &[],
        }
    }
}

pub(super) struct Client {
    // Session must close before its application.
    session: Session,
    application: Application,
}

struct Application(MI_Application);
struct Session(MI_Session);
struct Options(MI_OperationOptions);
struct Operation {
    handle: MI_Operation,
    complete: bool,
}

// windows-rs represents MI function table entries as isize. The signatures
// below follow the installed Windows SDK mi.h, including MI_CALL (system ABI).
impl Client {
    pub(super) fn new() -> Result<Self> {
        unsafe {
            let mut application = Application(MI_Application::default());
            let mut error = null_mut();
            let result = MI_Application_InitializeV1(0, None, Some(&mut error), &mut application.0);
            let details = owned_error(error);
            check(result, &format!("初始化 MI{details}"))?;
            let create: unsafe extern "system" fn(
                *mut MI_Application,
                *const u16,
                *const u16,
                *mut MI_DestinationOptions,
                *mut MI_SessionCallbacks,
                *mut *mut MI_Instance,
                *mut MI_Session,
            ) -> MI_Result = std::mem::transmute((*application.0.ft).NewSession);
            let mut session = Session(MI_Session::default());
            let mut error = null_mut();
            let result = create(
                &mut application.0,
                windows::core::w!("WMIDCOM").as_ptr(),
                null(),
                null_mut(),
                null_mut(),
                &mut error,
                &mut session.0,
            );
            let details = owned_error(error);
            check(result, &format!("创建本地 MI 会话{details}"))?;
            Ok(Self {
                session,
                application,
            })
        }
    }

    pub(super) fn query(
        &mut self,
        namespace: &str,
        class: &str,
        fields: &'static [&'static str],
        filter: Option<&str>,
        timeout_seconds: u32,
        cancelled: &AtomicBool,
    ) -> Result<Vec<Record>> {
        if cancelled.load(Ordering::Relaxed) {
            return Err("已取消系统信息读取".into());
        }
        let context = format!("{namespace} / {class}");
        let mut query = format!("SELECT {} FROM {class}", fields.join(", "));
        if let Some(filter) = filter {
            query.push_str(" WHERE ");
            query.push_str(filter);
        }
        let namespace = wide(namespace);
        let query = wide(&query);
        unsafe {
            let create_options: unsafe extern "system" fn(
                *mut MI_Application,
                u8,
                *mut MI_OperationOptions,
            ) -> MI_Result = std::mem::transmute((*self.application.0.ft).NewOperationOptions);
            let mut options = Options(MI_OperationOptions::default());
            check(
                create_options(&mut self.application.0, 0, &mut options.0),
                &context,
            )?;
            let timeout: unsafe extern "system" fn(
                *mut MI_OperationOptions,
                *const u16,
                *const MI_Interval,
                u32,
            ) -> MI_Result = std::mem::transmute((*options.0.ft).SetInterval);
            check(
                timeout(
                    &mut options.0,
                    windows::core::w!("__MI_OPERATIONOPTIONS_TIMEOUT").as_ptr(),
                    &MI_Interval {
                        seconds: timeout_seconds,
                        ..Default::default()
                    },
                    0,
                ),
                &context,
            )?;
            let execute: unsafe extern "system" fn(
                *mut MI_Session,
                u32,
                *mut MI_OperationOptions,
                *const u16,
                *const u16,
                *const u16,
                *mut MI_OperationCallbacks,
                *mut MI_Operation,
            ) = std::mem::transmute((*self.session.0.ft).QueryInstances);
            let mut operation = Operation {
                handle: MI_Operation::default(),
                complete: false,
            };
            execute(
                &mut self.session.0,
                0,
                &mut options.0,
                namespace.as_ptr(),
                windows::core::w!("WQL").as_ptr(),
                query.as_ptr(),
                null_mut(),
                &mut operation.handle,
            );
            let next: unsafe extern "system" fn(
                *mut MI_Operation,
                *mut *const MI_Instance,
                *mut u8,
                *mut MI_Result,
                *mut *const u16,
                *mut *const MI_Instance,
            ) -> MI_Result = std::mem::transmute((*operation.handle.ft).GetInstance);
            let mut records = Vec::new();
            loop {
                if cancelled.load(Ordering::Relaxed) {
                    return Err("已取消系统信息读取".into());
                }
                let mut instance = null();
                let mut more = 0;
                let mut result = MI_RESULT_OK;
                let mut message = null();
                let mut details = null();
                check(
                    next(
                        &mut operation.handle,
                        &mut instance,
                        &mut more,
                        &mut result,
                        &mut message,
                        &mut details,
                    ),
                    &context,
                )?;
                operation.complete = more == 0;
                let message = read_string(message);
                let details = error_description(details);
                let error = if details.trim() == message.trim() {
                    format!("{context} {message}")
                } else {
                    format!("{context} {message}{details}")
                };
                check(result, error.trim())?;
                if !instance.is_null() {
                    let mut record = BTreeMap::new();
                    for &name in fields {
                        if let Some(value) = element(instance, name)
                            .map_err(|error| format!("{context}.{name} {error}"))?
                        {
                            record.insert(name, value);
                        }
                    }
                    records.push(Record(record));
                }
                if operation.complete {
                    return Ok(records);
                }
            }
        }
    }
}

fn check(result: MI_Result, context: &str) -> Result<()> {
    if result == MI_RESULT_OK {
        Ok(())
    } else {
        Err(format!("{context}（MI_Result {}）", result.0))
    }
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}

unsafe fn read_string(value: *const u16) -> String {
    if value.is_null() {
        String::new()
    } else {
        unsafe {
            PCWSTR(value)
                .to_string()
                .unwrap_or_else(|_| "无效 UTF-16 文本".into())
        }
    }
}

unsafe fn element(instance: *const MI_Instance, name: &str) -> Result<Option<Value>> {
    unsafe {
        let get: unsafe extern "system" fn(
            *const MI_Instance,
            *const u16,
            *mut MI_Value,
            *mut MI_Type,
            *mut u32,
            *mut u32,
        ) -> MI_Result = std::mem::transmute((*(*instance).ft).GetElement);
        let mut value = MI_Value::default();
        let mut kind = MI_Type::default();
        let mut flags = 0;
        check(
            get(
                instance,
                wide(name).as_ptr(),
                &mut value,
                &mut kind,
                &mut flags,
                null_mut(),
            ),
            "读取属性",
        )?;
        if flags & MI_FLAG_NULL != 0 {
            return Ok(None);
        }
        let value = match kind {
            MI_BOOLEAN => Value::Boolean(value.boolean != 0),
            MI_STRING => {
                let text = read_string(value.string).trim().to_owned();
                if text.is_empty() {
                    return Ok(None);
                }
                Value::Text(text)
            }
            MI_UINT8 => Value::Number(value.uint8.into()),
            MI_UINT16 => Value::Number(value.uint16.into()),
            MI_UINT32 => Value::Number(value.uint32.into()),
            MI_UINT64 => Value::Number(value.uint64),
            MI_UINT16A => Value::Numbers(copy_array(value.uint16a.data, value.uint16a.size)),
            MI_UINT32A => Value::Numbers(copy_array(value.uint32a.data, value.uint32a.size)),
            _ => return Err(format!("不支持的 MI 属性类型 {}", kind.0)),
        };
        Ok(Some(value))
    }
}

unsafe fn copy_array<T: Copy + Into<u64>>(data: *const T, size: u32) -> Vec<u64> {
    if size == 0 {
        Vec::new()
    } else {
        unsafe {
            std::slice::from_raw_parts(data, size as usize)
                .iter()
                .map(|&value| value.into())
                .collect()
        }
    }
}

unsafe fn error_description(instance: *const MI_Instance) -> String {
    if instance.is_null() {
        return String::new();
    }
    match unsafe { element(instance, "Message") } {
        Ok(Some(Value::Text(message))) => format!(" {message}"),
        _ => String::new(),
    }
}

unsafe fn owned_error(instance: *mut MI_Instance) -> String {
    if instance.is_null() {
        return String::new();
    }
    unsafe {
        let description = error_description(instance);
        let delete: unsafe extern "system" fn(*mut MI_Instance) -> MI_Result =
            std::mem::transmute((*(*instance).ft).Delete);
        delete(instance);
        description
    }
}

impl Drop for Application {
    fn drop(&mut self) {
        if !self.0.ft.is_null() {
            unsafe {
                let close: unsafe extern "system" fn(*mut MI_Application) -> MI_Result =
                    std::mem::transmute((*self.0.ft).Close);
                close(&mut self.0);
            }
        }
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        if !self.0.ft.is_null() {
            unsafe {
                let close: unsafe extern "system" fn(
                    *mut MI_Session,
                    *mut std::ffi::c_void,
                    Option<unsafe extern "system" fn(*mut std::ffi::c_void)>,
                ) -> MI_Result = std::mem::transmute((*self.0.ft).Close);
                close(&mut self.0, null_mut(), None);
            }
        }
    }
}

impl Drop for Options {
    fn drop(&mut self) {
        if !self.0.ft.is_null() {
            unsafe {
                let delete: unsafe extern "system" fn(*mut MI_OperationOptions) =
                    std::mem::transmute((*self.0.ft).Delete);
                delete(&mut self.0);
            }
        }
    }
}

impl Drop for Operation {
    fn drop(&mut self) {
        if !self.handle.ft.is_null() {
            unsafe {
                if !self.complete {
                    let cancel: unsafe extern "system" fn(
                        *mut MI_Operation,
                        MI_CancellationReason,
                    ) -> MI_Result = std::mem::transmute((*self.handle.ft).Cancel);
                    cancel(&mut self.handle, MI_REASON_NONE);
                    // MI requires the final result to be consumed before Close,
                    // including when a cancelled query or property read fails.
                    let next: unsafe extern "system" fn(
                        *mut MI_Operation,
                        *mut *const MI_Instance,
                        *mut u8,
                        *mut MI_Result,
                        *mut *const u16,
                        *mut *const MI_Instance,
                    ) -> MI_Result = std::mem::transmute((*self.handle.ft).GetInstance);
                    let mut more = 1;
                    while more != 0 {
                        let mut instance = null();
                        if next(
                            &mut self.handle,
                            &mut instance,
                            &mut more,
                            null_mut(),
                            null_mut(),
                            null_mut(),
                        ) != MI_RESULT_OK
                        {
                            break;
                        }
                    }
                }
                let close: unsafe extern "system" fn(*mut MI_Operation) -> MI_Result =
                    std::mem::transmute((*self.handle.ft).Close);
                close(&mut self.handle);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_use_the_first_available_property_and_allow_all_to_be_absent() {
        let mut record = Record(BTreeMap::from([
            ("Caption", Value::Text("备用名称".into())),
            ("Description", Value::Text("设备描述".into())),
        ]));
        assert_eq!(
            record
                .first_text(&["Name", "Caption", "Description"])
                .as_deref(),
            Some("备用名称")
        );
        record.0.remove("Caption");
        assert_eq!(
            record
                .first_text(&["Name", "Caption", "Description"])
                .as_deref(),
            Some("设备描述")
        );
        record.0.clear();
        assert!(
            record
                .first_text(&["Name", "Caption", "Description"])
                .is_none()
        );
    }
}
