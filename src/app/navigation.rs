use gpui_kit::assets::IconName;

/// Category pages reference their catalog entry by stable id.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) enum Page {
    Home,
    SystemInfo,
    Category(String),
    Settings,
}

impl Page {
    pub(crate) fn id(&self) -> String {
        match self {
            Self::Home => "home".into(),
            Self::SystemInfo => "system-information".into(),
            Self::Category(id) => id.clone(),
            Self::Settings => "settings".into(),
        }
    }
    pub(crate) fn icon(&self) -> IconName {
        match self {
            Self::Home => IconName::Star,
            Self::SystemInfo => IconName::Monitor,
            Self::Category(_) => crate::tools::entry_icon(),
            Self::Settings => IconName::Settings,
        }
    }
}
