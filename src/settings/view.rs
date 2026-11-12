//! Appearance preferences page. Every control commits immediately; there is
//! no save button, and enum choices persist stable English identifiers.
use super::{Backdrop, OpenMode, Preferences, ThemeChoice};
use crate::tools::{categories, store::CatalogStore};
use crate::ui::{
    self,
    theme::{control, layout, space},
};
use gpui_kit::{
    base::IndexPath,
    component::{
        Sizable, Theme, ThemeMode,
        button::Button,
        scroll::ScrollableElement,
        select::{Select, SelectEvent, SelectState},
        v_flex,
    },
    *,
};

const THEME_CHOICES: [(&str, ThemeChoice); 3] = [
    ("深色", ThemeChoice::Dark),
    ("浅色", ThemeChoice::Light),
    ("跟随系统", ThemeChoice::System),
];
const BACKDROP_CHOICES: [(&str, Backdrop); 2] =
    [("Acrylic", Backdrop::Acrylic), ("纯色", Backdrop::Solid)];

const OPEN_CHOICES: [(&str, OpenMode); 2] =
    [("双击", OpenMode::Double), ("单击", OpenMode::Single)];

pub(crate) struct SettingsPage {
    theme: Entity<SelectState<Vec<SharedString>>>,
    backdrop: Entity<SelectState<Vec<SharedString>>>,
    catalog: Entity<CatalogStore>,
    open_mode: Entity<SelectState<Vec<SharedString>>>,
    _open_subscription: Subscription,
    _theme_subscription: Subscription,
    _backdrop_subscription: Subscription,
}

impl SettingsPage {
    pub(crate) fn new(
        catalog: Entity<CatalogStore>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let preferences = super::read(cx);
        let open_mode = Self::select_state(&OPEN_CHOICES, preferences.open_mode, window, cx);
        let open_subscription = cx.subscribe_in(
            &open_mode,
            window,
            |_, state, event: &SelectEvent<Vec<SharedString>>, window, cx| {
                if let SelectEvent::Confirm(_) = event
                    && let Some(row) = state.read(cx).selected_index(cx).map(|path| path.row)
                    && let Some((_, choice)) = OPEN_CHOICES.get(row)
                {
                    commit(
                        Preferences {
                            open_mode: *choice,
                            ..super::read(cx)
                        },
                        window,
                        cx,
                    );
                }
            },
        );
        let theme = Self::select_state(&THEME_CHOICES, preferences.theme, window, cx);
        let backdrop = Self::select_state(&BACKDROP_CHOICES, preferences.backdrop, window, cx);
        let theme_subscription = cx.subscribe_in(
            &theme,
            window,
            |_, state, event: &SelectEvent<Vec<SharedString>>, window, cx| {
                if let SelectEvent::Confirm(_) = event
                    && let Some(row) = state.read(cx).selected_index(cx).map(|path| path.row)
                    && let Some((_, choice)) = THEME_CHOICES.get(row)
                {
                    commit(
                        Preferences {
                            theme: *choice,
                            ..super::read(cx)
                        },
                        window,
                        cx,
                    );
                }
            },
        );
        let backdrop_subscription = cx.subscribe_in(
            &backdrop,
            window,
            |_, state, event: &SelectEvent<Vec<SharedString>>, window, cx| {
                if let SelectEvent::Confirm(_) = event
                    && let Some(row) = state.read(cx).selected_index(cx).map(|path| path.row)
                    && let Some((_, choice)) = BACKDROP_CHOICES.get(row)
                {
                    commit(
                        Preferences {
                            backdrop: *choice,
                            ..super::read(cx)
                        },
                        window,
                        cx,
                    );
                }
            },
        );
        Self {
            theme,
            backdrop,
            catalog,
            open_mode,
            _open_subscription: open_subscription,
            _theme_subscription: theme_subscription,
            _backdrop_subscription: backdrop_subscription,
        }
    }

    fn select_state<T: PartialEq + Copy>(
        choices: &[(&str, T)],
        current: T,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<SelectState<Vec<SharedString>>> {
        let index = choices
            .iter()
            .position(|(_, value)| *value == current)
            .unwrap_or(0);
        let items = choices
            .iter()
            .map(|(label, _)| SharedString::from(*label))
            .collect();
        cx.new(|cx| SelectState::new(items, Some(IndexPath::new(index)), window, cx))
    }
}

/// One commit path: store the authoritative copy, persist it, then apply the
/// appearance change so every window and control follows the same values.
fn commit(preferences: Preferences, window: &mut Window, cx: &mut App) {
    let previous = super::read(cx);
    if let Err(error) = super::apply(preferences, cx) {
        ui::show_error(format!("设置保存失败：{error}"), window, cx);
    }
    match preferences.theme {
        ThemeChoice::Dark => Theme::change(ThemeMode::Dark, Some(window), cx),
        ThemeChoice::Light => Theme::change(ThemeMode::Light, Some(window), cx),
        ThemeChoice::System => Theme::sync_system_appearance(Some(window), cx),
    }
    ui::configure_theme(cx);
    if previous.backdrop != preferences.backdrop {
        window.set_background_appearance(crate::app::window::background(preferences.backdrop));
    }
}

impl Render for SettingsPage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let content = v_flex()
            .w_full()
            .gap(space::SECTION)
            .child(ui::preferences_group(
                "外观",
                vec![
                    ui::preference_row(
                        "主题",
                        "选择深色、浅色或跟随系统",
                        div().w(layout::SELECT_WIDTH).flex_shrink_0().child(
                            Select::new(&self.theme)
                                .id("appearance-theme")
                                .w_full()
                                .with_size(control::STANDARD)
                                .accessibility_label("主题"),
                        ),
                        cx,
                    )
                    .into_any_element(),
                    ui::preference_row(
                        "背景材质",
                        "Acrylic 磨砂或纯色背景",
                        div().w(layout::SELECT_WIDTH).flex_shrink_0().child(
                            Select::new(&self.backdrop)
                                .id("appearance-backdrop")
                                .w_full()
                                .with_size(control::STANDARD)
                                .accessibility_label("背景材质"),
                        ),
                        cx,
                    )
                    .into_any_element(),
                ],
                cx,
            ))
            .child(ui::preferences_group(
                "操作",
                vec![
                    ui::preference_row(
                        "打开方式",
                        "",
                        div().w(layout::SELECT_WIDTH).flex_shrink_0().child(
                            Select::new(&self.open_mode)
                                .id("tool-open-mode")
                                .w_full()
                                .with_size(control::STANDARD)
                                .accessibility_label("打开方式"),
                        ),
                        cx,
                    )
                    .into_any_element(),
                ],
                cx,
            ))
            .child(ui::preferences_group(
                "工具分类",
                vec![
                    ui::preference_row(
                        "分类",
                        "",
                        Button::new("manage-categories")
                            .outline()
                            .with_size(control::STANDARD)
                            .icon(gpui_kit::assets::IconName::FolderCog)
                            .label("管理分类…")
                            .on_click(cx.listener(|this, _, window, cx| {
                                categories::open_manager(this.catalog.clone(), window, cx);
                            })),
                        cx,
                    )
                    .into_any_element(),
                ],
                cx,
            ))
            .child(ui::preferences_group(
                "关于",
                vec![
                    ui::preference_row(
                        "Suki Toolbox",
                        "",
                        ui::muted(env!("CARGO_PKG_VERSION"), cx),
                        cx,
                    )
                    .into_any_element(),
                    ui::preference_row("版权", "", ui::muted("Copyright © Moenya", cx), cx)
                        .into_any_element(),
                ],
                cx,
            ));
        ui::workspace()
            .child(ui::workspace_header("设置", "", cx))
            .child(
                div()
                    .id("settings-scroll")
                    .flex_1()
                    .min_h_0()
                    .min_w_0()
                    .overflow_y_scrollbar()
                    .child(content),
            )
    }
}
