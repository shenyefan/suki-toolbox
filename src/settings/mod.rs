//! Appearance preferences persisted in the executable directory's config.json.
use gpui_kit::{App, Global};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub(crate) mod view;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum ThemeChoice {
    Dark,
    Light,
    System,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Backdrop {
    Acrylic,
    Solid,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum OpenMode {
    Single,
    Double,
}

impl OpenMode {
    pub(crate) fn activates(self, keyboard: bool, count: usize) -> bool {
        keyboard
            || match self {
                Self::Single => count == 1,
                Self::Double => count == 2,
            }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct Preferences {
    pub(crate) theme: ThemeChoice,
    pub(crate) backdrop: Backdrop,
    pub(crate) open_mode: OpenMode,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            theme: ThemeChoice::Dark,
            backdrop: Backdrop::Acrylic,
            open_mode: OpenMode::Double,
        }
    }
}

struct Store {
    preferences: Preferences,
}
impl Global for Store {}

/// Install the loaded preferences; call once during bootstrap.
pub(crate) fn init(preferences: Preferences, cx: &mut App) {
    cx.set_global(Store { preferences });
}

pub(crate) fn read(cx: &App) -> Preferences {
    cx.global::<Store>().preferences
}

/// Commit new preferences as the single authoritative copy and persist them.
/// The caller owns applying window appearance and reporting write errors.
pub(crate) fn apply(preferences: Preferences, cx: &mut App) -> std::io::Result<()> {
    cx.global_mut::<Store>().preferences = preferences;
    persist(preferences)
}

fn file() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(|parent| parent.join("config.json")))
}

/// Load persisted preferences. Missing files or fields use defaults; damaged
/// files keep the original on disk and surface a readable message.
pub(crate) fn load() -> (Preferences, Option<String>) {
    let Some(path) = file() else {
        return (
            Preferences::default(),
            Some("无法定位程序目录，设置不会保存".into()),
        );
    };
    let Ok(raw) = std::fs::read_to_string(&path) else {
        return (Preferences::default(), None);
    };
    let value = match serde_json::from_str::<serde_json::Value>(&raw) {
        Ok(value) => value,
        Err(error) => {
            return (
                Preferences::default(),
                Some(format!("设置文件无法读取：{}（{error}）", path.display())),
            );
        }
    };
    match value
        .get("settings")
        .cloned()
        .map(serde_json::from_value)
        .transpose()
    {
        Ok(Some(preferences)) => (preferences, None),
        Ok(None) => (Preferences::default(), None),
        Err(error) => (
            Preferences::default(),
            Some(format!("设置文件无法读取：{}（{error}）", path.display())),
        ),
    }
}

fn persist(preferences: Preferences) -> std::io::Result<()> {
    let path = file().ok_or_else(|| std::io::Error::other("无法定位程序目录"))?;
    let mut value = if path.exists() {
        serde_json::from_str::<serde_json::Value>(&std::fs::read_to_string(&path)?)?
    } else {
        serde_json::json!({"categories": [], "tools": []})
    };
    value["settings"] = serde_json::to_value(preferences)?;
    std::fs::write(path, serde_json::to_vec_pretty(&value)?)
}

#[cfg(test)]
mod tests {
    use super::{Backdrop, Preferences, ThemeChoice};

    #[test]
    fn round_trips_current_format_with_english_enum_keys() {
        let preferences = Preferences::default();
        let raw = serde_json::to_string(&preferences).unwrap();
        assert_eq!(
            raw,
            r#"{"theme":"dark","backdrop":"acrylic","open_mode":"double"}"#
        );
        let parsed: Preferences = serde_json::from_str(&raw).unwrap();
        assert_eq!(parsed, preferences);
        assert_eq!(parsed.theme, ThemeChoice::Dark);
        assert_eq!(parsed.backdrop, Backdrop::Acrylic);
    }

    #[test]
    fn missing_fields_use_defaults_and_preserve_existing_values() {
        assert_eq!(
            serde_json::from_str::<Preferences>("{}").unwrap(),
            Preferences::default()
        );
        let parsed: Preferences = serde_json::from_str(r#"{"theme":"light"}"#).unwrap();
        assert_eq!(parsed.theme, ThemeChoice::Light);
        assert_eq!(parsed.open_mode, super::OpenMode::Double);
        assert_eq!(parsed.backdrop, Backdrop::Acrylic);
    }

    #[test]
    fn damaged_files_report_instead_of_falling_back_silently() {
        let error = serde_json::from_str::<Preferences>("broken settings").unwrap_err();
        assert!(!error.to_string().is_empty());
    }
    #[test]
    fn activation_mode_preserves_keyboard_and_does_not_launch_twice() {
        use super::OpenMode;
        assert!(OpenMode::Double.activates(true, 1));
        assert!(!OpenMode::Double.activates(false, 1));
        assert!(OpenMode::Double.activates(false, 2));
        assert!(OpenMode::Single.activates(false, 1));
        assert!(!OpenMode::Single.activates(false, 2));
    }
}
