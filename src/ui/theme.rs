//! Single theme entry point: edit spacing and geometry below, then the DARK /
//! LIGHT palettes. Views read semantic colors from cx.theme() and use short
//! names such as space::SM, text::BODY and layout::SIDEBAR_WIDTH.
use gpui_kit::{
    component::{ActiveTheme, Theme},
    *,
};

// 1. 共用尺度：间距、文字与几何尺寸。
pub(crate) mod space {
    use super::*;
    pub(crate) const NONE: Rems = rems(0.);
    pub(crate) const XS: Rems = rems(0.25);
    pub(crate) const SM: Rems = rems(0.5);
    pub(crate) const MD: Rems = rems(0.75);
    pub(crate) const LG: Rems = rems(1.);
    pub(crate) const SECTION: Rems = rems(1.5);
    pub(crate) const REGION: Rems = rems(2.5);
}

pub(crate) mod text {
    use super::*;
    pub(crate) const BASE: Pixels = px(15.);
    pub(crate) const CAPTION: Rems = rems(0.75);
    pub(crate) const BODY: Rems = rems(0.875);
    pub(crate) const TITLE: Rems = rems(1.25);
    pub(crate) const EMPHASIS: FontWeight = FontWeight::MEDIUM;
    pub(crate) const HEADING: FontWeight = FontWeight::SEMIBOLD;
}

/// Product geometry. Names include their role and axis so call sites are clear.
pub(crate) mod layout {
    use super::*;
    pub(crate) const SYSTEM_INFO_LABEL_WIDTH: Rems = rems(7.);
    pub(crate) const SYSTEM_INFO_MARKERS_WIDTH: Rems = rems(2.5);
    pub(crate) const SYSTEM_INFO_SUMMARY_ICON: Rems = rems(3.);
    pub(crate) const ICON_PICKER_WIDTH: Rems = rems(22.);
    pub(crate) const ICON_PICKER_HEIGHT: Rems = rems(15.);
    pub(crate) const ICON_PICKER_ROW_HEIGHT: Rems = rems(2.75);
    pub(crate) const ICON_PICKER_COLUMNS: usize = 6;
    pub(crate) const CATEGORY_LIST_HEIGHT: Rems = rems(12.);
    pub(crate) const CATEGORY_ROW_HEIGHT: Rems = rems(3.);
    pub(crate) const DIALOG_WIDTH: Rems = rems(34.);
    pub(crate) const DIALOG_RESERVED_HEIGHT: Rems = rems(14.);
    pub(crate) const ICON: Rems = rems(1.);
    pub(crate) const ICON_FEATURE: Rems = rems(1.5);
    pub(crate) const FOOTER_HEIGHT: Rems = rems(1.5);
    pub(crate) const CONTROL_HEIGHT: Rems = rems(2.);
    pub(crate) const NAV_ITEM_HEIGHT: Rems = rems(2.5);
    pub(crate) const TITLE_HEIGHT: Rems = rems(3.5);
    pub(crate) const BRAND_LOGO_SIZE: Rems = rems(1.75);
    pub(crate) const BRAND_HEIGHT: Rems = rems(4.);
    pub(crate) const EMPTY_STATE_HEIGHT: Rems = rems(18.);
    pub(crate) const SELECT_WIDTH: Rems = rems(10.);
    pub(crate) const SIDEBAR_WIDTH: Rems = rems(13.5);
    pub(crate) const SEARCH_WIDTH: Rems = rems(20.);
    pub(crate) const SIDEBAR_COLLAPSED_WIDTH: Rems = rems(4.);
}

/// Tool cards share the grid rhythm and adapt to the available width.
pub(crate) mod tool_card {
    use super::*;
    pub(crate) const MIN_WIDTH: Rems = rems(240. / 15.);
    pub(crate) const HEIGHT: Rems = rems(60. / 15.);
    pub(crate) const GAP: Rems = rems(8. / 15.);
    pub(crate) const PADDING_X: Rems = rems(16. / 15.);
    pub(crate) const ICON: Rems = rems(20. / 15.);
}

/// Standard controls use one native size so text, icons and hit areas agree.
pub(crate) mod control {
    use gpui_kit::component::Size;
    pub(crate) const STANDARD: Size = Size::Medium;
}

/// Native preferences rows shared by the settings page.
pub(crate) mod preferences {
    use super::*;
    pub(crate) const ROW_HEIGHT: Rems = rems(3.);
}

/// Export at twice the logical resolution for readable text and icons in PNG.
pub(crate) mod system_info_export {
    use gpui_kit::{
        App,
        component::{ActiveTheme, Theme},
    };

    pub(crate) const SCALE: f32 = 2.;
    pub(crate) const WIDTH: i32 = 1800;

    pub(crate) fn solid_theme(cx: &App) -> Theme {
        let mut theme = cx.theme().clone();
        super::apply(&mut theme, false);
        theme
    }
}

pub(crate) mod window {
    use super::*;
    // Platform window bounds are physical logical pixels, independent of UI zoom.
    pub(crate) const DEFAULT_WIDTH: Pixels = px(1180.);
    pub(crate) const DEFAULT_HEIGHT: Pixels = px(800.);
    pub(crate) const MIN_WIDTH: Pixels = px(880.);
    pub(crate) const MIN_HEIGHT: Pixels = px(600.);
}

pub(crate) mod radius {
    use super::*;
    pub(crate) const CONTROL: Pixels = px(10.);
    pub(crate) const SURFACE: Pixels = px(18.);
}

pub(crate) mod stroke {
    use super::*;
    pub(crate) const NONE: Pixels = px(0.);
    pub(crate) const HAIRLINE: Pixels = px(1.);
}

/// Windows Acrylic material opacities; smaller values stay more translucent.
pub(crate) mod opacity {
    pub(crate) const TRANSPARENT: f32 = 0.;
    pub(crate) const PREFERENCES_DARK: f32 = 0.72;
    pub(crate) const PREFERENCES_LIGHT: f32 = 0.72;
    pub(crate) const PREFERENCES_DIVIDER: f32 = 0.06;
    pub(crate) const SELECTED: f32 = 0.12;
    pub(crate) const ACTIVE_DARK: f32 = 0.55;
    pub(crate) const ACTIVE_LIGHT: f32 = 0.18;
    pub(crate) const PANEL: f32 = 0.3;
    pub(crate) const CONTROL_LIGHT: f32 = 0.65;
    pub(crate) const BUTTON_HOVER: f32 = 0.9;
    pub(crate) const BUTTON_PRESSED: f32 = 0.8;
    const CHROME_DARK: f32 = 0.73;
    const CHROME_LIGHT: f32 = 0.68;
    const WORKSPACE_DARK: f32 = 0.12;
    const WORKSPACE_LIGHT: f32 = 0.30;

    pub(super) fn chrome(dark: bool) -> f32 {
        if dark { CHROME_DARK } else { CHROME_LIGHT }
    }
    pub(super) fn workspace(dark: bool) -> f32 {
        if dark {
            WORKSPACE_DARK
        } else {
            WORKSPACE_LIGHT
        }
    }
}

// 2. 深浅调色板：同一个语义在两种外观中有独立取值。
#[derive(Clone, Copy)]
struct Palette {
    background: u32,
    foreground: u32,
    chrome: u32,
    workspace: u32,
    muted: u32,
    preferences: u32,
    muted_foreground: u32,
    border: u32,
    primary: u32,
    primary_foreground: u32,
    accent: u32,
    selected: u32,
    input: u32,
}
const DARK: Palette = Palette {
    background: 0x14161b,
    foreground: 0xf0f2f6,
    chrome: 0x0d1015,
    workspace: 0x191c22,
    muted: 0x20242c,
    preferences: 0x282b32,
    muted_foreground: 0xd2d8e3,
    border: 0x444b58,
    primary: 0xa7c4ff,
    primary_foreground: 0x152139,
    accent: 0x293346,
    selected: 0x44669c,
    input: 0x4e5768,
};
const LIGHT: Palette = Palette {
    background: 0xffffff,
    foreground: 0x252932,
    chrome: 0xffffff,
    workspace: 0xffffff,
    muted: 0xffffff,
    preferences: 0xffffff,
    muted_foreground: 0x29333f,
    border: 0xb7c0cd,
    primary: 0x315fa5,
    primary_foreground: 0xffffff,
    accent: 0xd4e0f3,
    selected: 0x315fa5,
    input: 0xa5b0c1,
};
fn palette(dark: bool) -> Palette {
    if dark { DARK } else { LIGHT }
}

/// Solid mode replaces the Acrylic material with opaque palette surfaces.
fn translucent(cx: &App) -> bool {
    crate::settings::read(cx).backdrop != crate::settings::Backdrop::Solid
}

pub(crate) fn window_tint(cx: &App) -> Hsla {
    let color = cx.theme().sidebar;
    if translucent(cx) {
        color.opacity(opacity::chrome(cx.theme().is_dark()))
    } else {
        color
    }
}
pub(crate) fn workspace_tint(cx: &App) -> Hsla {
    if translucent(cx) {
        let p = palette(cx.theme().is_dark());
        Hsla::from(rgb(p.workspace)).opacity(opacity::workspace(cx.theme().is_dark()))
    } else {
        cx.theme().background
    }
}

// 3. 唯一初始化入口，将产品主题映射到 GPUI Kit。
/// Apply the product theme and synchronize the native window appearance.
pub(crate) fn configure_theme(cx: &mut App) {
    cx.set_window_appearance(Some(if cx.theme().is_dark() {
        WindowAppearance::Dark
    } else {
        WindowAppearance::Light
    }));
    let translucent = translucent(cx);
    Theme::update(cx, |theme| apply(theme, translucent));
}

/// Resolve each theme completely so switching appearance cannot retain colors
/// or interactive states from the previous palette.
fn apply(theme: &mut Theme, translucent: bool) {
    let mut p = palette(theme.is_dark());
    if !translucent && !theme.is_dark() {
        p.chrome = 0xf0f2f6;
        p.muted = 0xf5f6f9;
    }
    theme.font_size = text::BASE;
    // Keep focus inside scroll and virtual row boundaries, retaining the native border.
    theme.focus_ring = false;
    theme.notification.placement = Anchor::BottomRight;
    theme.radius = radius::CONTROL;
    theme.radius_lg = radius::SURFACE;
    theme.colors.background = if !translucent {
        rgb(p.background).into()
    } else if theme.is_dark() {
        Hsla::from(rgb(p.foreground)).opacity(opacity::SELECTED)
    } else {
        Hsla::from(rgb(p.background)).opacity(opacity::CONTROL_LIGHT)
    };
    theme.colors.foreground = rgb(p.foreground).into();
    theme.colors.sidebar = rgb(p.chrome).into();
    theme.colors.sidebar_foreground = rgb(p.muted_foreground).into();
    let selected = Hsla::from(rgb(p.selected)).opacity(if theme.is_dark() {
        opacity::ACTIVE_DARK
    } else {
        opacity::ACTIVE_LIGHT
    });
    theme.colors.sidebar_accent = selected;
    theme.colors.sidebar_accent_foreground = theme.colors.foreground;
    theme.colors.muted = rgb(p.muted).into();
    theme.colors.muted_foreground = rgb(p.muted_foreground).into();
    theme.colors.border = rgb(p.border).into();
    theme.colors.primary = rgb(p.primary).into();
    theme.colors.primary_foreground = rgb(p.primary_foreground).into();
    theme.colors.input = rgb(p.input).into();
    theme.colors.accent = Hsla::from(rgb(p.accent)).opacity(opacity::PANEL);
    theme.colors.accent_foreground = theme.colors.foreground;
    theme.colors.list_active = selected;
    theme.colors.list_active_border = theme.colors.primary;
    theme.colors.list_hover = theme.colors.foreground.opacity(opacity::SELECTED);
    theme.colors.list_head = theme.colors.muted.opacity(opacity::PANEL);
    theme.colors.list_even = theme.colors.background.opacity(opacity::TRANSPARENT);
    theme.colors.popover = rgb(p.background).into();
    theme.colors.popover_foreground = theme.colors.foreground;
    theme.colors.ring = theme.colors.primary;
    theme.colors.selection = theme.colors.primary.opacity(opacity::PANEL);
    theme.colors.title_bar = theme.colors.sidebar;
    theme.colors.title_bar_border = theme.colors.border.opacity(opacity::TRANSPARENT);
    theme.colors.sidebar_border = theme.colors.title_bar_border;
    theme.colors.button_foreground = theme.colors.foreground;
    theme.colors.button_hover = theme.colors.list_hover;
    theme.colors.button_active = selected;
    theme.colors.button_primary = theme.colors.primary;
    theme.colors.button_primary_foreground = theme.colors.primary_foreground;
    theme.colors.button_primary_hover = theme.colors.primary.opacity(opacity::BUTTON_HOVER);
    theme.colors.button_primary_active = theme.colors.primary.opacity(opacity::BUTTON_PRESSED);
    theme.colors.tab = theme.colors.background;
    theme.colors.tab_bar = theme.colors.background;
    theme.colors.tab_bar_segmented = theme.colors.muted.opacity(opacity::PANEL);
    theme.colors.tab_active = selected;
    theme.colors.tab_foreground = theme.colors.muted_foreground;
    theme.colors.tab_active_foreground = theme.colors.foreground;
    theme.colors.secondary = theme.colors.muted.opacity(opacity::PANEL);
    theme.colors.secondary_foreground = theme.colors.foreground;
    theme.colors.secondary_hover = theme.colors.accent;
    theme.colors.secondary_active = selected;
}

pub(crate) fn preferences_tint(cx: &App) -> Hsla {
    let dark = cx.theme().is_dark();
    let color: Hsla = if translucent(cx) {
        rgb(palette(dark).preferences).into()
    } else {
        cx.theme().muted
    };
    color.opacity(if dark {
        opacity::PREFERENCES_DARK
    } else {
        opacity::PREFERENCES_LIGHT
    })
}

#[cfg(test)]
mod tests {
    use super::{opacity, palette};

    fn channels(hex: u32) -> [f32; 3] {
        [
            ((hex >> 16) & 255) as f32,
            ((hex >> 8) & 255) as f32,
            (hex & 255) as f32,
        ]
        .map(|v| v / 255.)
    }
    fn blend(top: [f32; 3], a: f32, bottom: [f32; 3]) -> [f32; 3] {
        std::array::from_fn(|i| top[i] * a + bottom[i] * (1. - a))
    }
    fn luminance(c: [f32; 3]) -> f32 {
        let c = c.map(|v| {
            if v <= 0.04045 {
                v / 12.92
            } else {
                ((v + 0.055) / 1.055).powf(2.4)
            }
        });
        c[0] * 0.2126 + c[1] * 0.7152 + c[2] * 0.0722
    }
    fn contrast(a: [f32; 3], b: [f32; 3]) -> f32 {
        let (a, b) = (luminance(a), luminance(b));
        (a.max(b) + 0.05) / (a.min(b) + 0.05)
    }

    #[test]
    fn text_remains_readable_over_acrylic_and_solid_surfaces_in_both_themes() {
        for dark in [true, false] {
            let p = palette(dark);
            for backdrop in [0x000000, 0xffffff] {
                let chrome = blend(
                    channels(p.chrome),
                    opacity::chrome(dark),
                    channels(backdrop),
                );
                let workspace = blend(channels(p.workspace), opacity::workspace(dark), chrome);
                for surface in [
                    chrome,
                    workspace,
                    channels(p.background),
                    blend(
                        channels(if dark { p.foreground } else { p.background }),
                        if dark {
                            opacity::SELECTED
                        } else {
                            opacity::CONTROL_LIGHT
                        },
                        workspace,
                    ),
                    blend(channels(p.accent), opacity::PANEL, workspace),
                    blend(
                        channels(p.selected),
                        if dark {
                            opacity::ACTIVE_DARK
                        } else {
                            opacity::ACTIVE_LIGHT
                        },
                        chrome,
                    ),
                    blend(
                        channels(p.preferences),
                        if dark {
                            opacity::PREFERENCES_DARK
                        } else {
                            opacity::PREFERENCES_LIGHT
                        },
                        workspace,
                    ),
                ] {
                    for text in [p.foreground, p.muted_foreground] {
                        assert!(
                            contrast(channels(text), surface) >= 4.5,
                            "theme dark={dark}, text={text:x}, contrast={}",
                            contrast(channels(text), surface)
                        );
                    }
                }
            }
        }
    }
}
