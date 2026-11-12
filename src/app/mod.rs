use crate::settings::view::SettingsPage;
use crate::tools::{Catalog, view::ToolsPage};
use crate::ui::theme::{layout, opacity, space, stroke, text};
use gpui_kit::prelude::FluentBuilder as _;
pub(crate) mod window;
use crate::ui;
use gpui_kit::{
    component::{
        ActiveTheme, Icon, Selectable, Sizable, TitleBar,
        button::{Button, ButtonVariants},
        h_flex,
        scroll::ScrollableElement,
        v_flex,
    },
    *,
};

actions!(toolbox, [Quit, ToggleSidebar]);

mod navigation;
use navigation::Page;
use std::sync::{Arc, LazyLock};

pub(crate) struct ToolboxApp {
    page: Page,
    catalog: Catalog,
    collapsed: bool,
    focus: FocusHandle,
    tools_pages: Vec<(Page, Entity<ToolsPage>)>,
    settings_page: Entity<SettingsPage>,
    system_page: Entity<crate::system_info::SystemInfoPage>,
    _catalog_subscription: Subscription,
}

impl ToolboxApp {
    pub(crate) fn new(
        catalog: Catalog,
        config_missing: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        window.set_window_title("Suki Toolbox");
        let focus = cx.focus_handle();
        focus.focus(window, cx);
        cx.bind_keys([
            KeyBinding::new("ctrl-q", Quit, None),
            KeyBinding::new("ctrl-b", ToggleSidebar, Some("Toolbox")),
        ]);
        let store = cx.new(|_| crate::tools::store::CatalogStore::new(catalog.clone()));
        let mut tools_pages = vec![(
            Page::Home,
            cx.new(|cx| ToolsPage::new("home", "收藏", "", store.clone(), window, cx)),
        )];
        tools_pages.extend(catalog.categories.iter().map(|category| {
            let page = Page::Category(category.id.clone());
            (
                page,
                cx.new(|cx| {
                    ToolsPage::new(
                        &category.id,
                        &category.label,
                        &category.detail,
                        store.clone(),
                        window,
                        cx,
                    )
                }),
            )
        }));
        store.update(cx, |store, cx| store.load_icons(window, cx));
        let system_page = cx.new(|cx| crate::system_info::SystemInfoPage::new(window, cx));
        let settings_page = cx.new(|cx| SettingsPage::new(store.clone(), window, cx));
        let catalog_subscription = cx.observe_in(&store, window, |this, store, window, cx| {
            this.catalog = store.read(cx).catalog.clone();
            this.tools_pages.retain(|(page, _)| match page {
                Page::Category(id) => this.catalog.category(id).is_some(),
                _ => true,
            });
            if let Page::Category(id) = &this.page
                && this.catalog.category(id).is_none()
            {
                this.page = Page::Home;
            }
            for category in &this.catalog.categories {
                let page = Page::Category(category.id.clone());
                if !this.tools_pages.iter().any(|(key, _)| key == &page) {
                    let view = cx.new(|cx| {
                        ToolsPage::new(
                            &category.id,
                            &category.label,
                            &category.detail,
                            store.clone(),
                            window,
                            cx,
                        )
                    });
                    this.tools_pages.push((page, view));
                } else if let Some((_, view)) =
                    this.tools_pages.iter().find(|(key, _)| key == &page)
                {
                    view.update(cx, |view, cx| view.rename(&category.label, cx));
                }
            }
            cx.notify();
        });
        Self {
            _catalog_subscription: catalog_subscription,
            page: if config_missing || catalog.home.is_empty() {
                Page::SystemInfo
            } else {
                Page::Home
            },
            catalog,
            collapsed: false,
            focus,
            tools_pages,
            settings_page,
            system_page,
        }
    }

    fn routes(&self) -> Vec<Page> {
        let mut routes = vec![Page::Home, Page::SystemInfo];
        routes.extend(
            self.catalog
                .categories
                .iter()
                .map(|category| Page::Category(category.id.clone())),
        );
        routes.push(Page::Settings);
        routes
    }

    fn page_label(&self, page: &Page) -> String {
        match page {
            Page::Home => "收藏".into(),
            Page::SystemInfo => "系统信息".into(),
            Page::Category(id) => self
                .catalog
                .category(id)
                .map(|category| category.label.clone())
                .unwrap_or_else(|| id.clone()),
            Page::Settings => "设置".into(),
        }
    }

    fn navigate(&mut self, page: Page, cx: &mut Context<Self>) {
        if self.page != page {
            self.page = page;
            cx.notify();
        }
    }

    /// Cross-fade weights between the outgoing and incoming page layers.
    fn page_weights(&self, window: &mut Window, cx: &mut Context<Self>) -> Vec<(Page, f32)> {
        let motion = cx.theme().motion_tokens();
        let policy = base::motion::Transition::new(motion.duration_normal)
            .easing(motion.easing_move.clone());
        let current = self.page.clone();
        let mut layers: Vec<_> = self
            .routes()
            .into_iter()
            .map(|page| {
                let weight = base::motion::transition(
                    (
                        ElementId::from("page-handoff"),
                        SharedString::from(page.id()),
                    ),
                    if page == current { 1.0 } else { 0.0 },
                    policy.clone(),
                    window,
                    cx,
                );
                (page, weight)
            })
            .filter(|(page, weight)| *weight > 0. || *page == current)
            .collect();
        let total: f32 = layers.iter().map(|(_, weight)| *weight).sum();
        if total > 0. {
            for (_, weight) in &mut layers {
                *weight /= total;
            }
        }
        layers
    }

    fn nav_item(&self, page: Page, cx: &mut Context<Self>) -> Button {
        let collapsed = self.collapsed;
        let label = self.page_label(&page);
        let selected = self.page == page;
        let icon_name = match &page {
            Page::Category(id) => self
                .catalog
                .category(id)
                .and_then(|category| crate::tools::library_icon(&category.icon)),
            _ => None,
        }
        .unwrap_or_else(|| page.icon());
        let icon = Icon::new(icon_name)
            .size(layout::ICON)
            .flex_shrink_0()
            .into_any_element();
        Button::new(SharedString::from(format!("nav-{}", page.id())))
            .ghost()
            .small()
            .w_full()
            .h(layout::NAV_ITEM_HEIGHT)
            .flex_shrink_0()
            .px(space::MD)
            .overflow_hidden()
            .rounded(cx.theme().radius)
            .selected(selected)
            .accessibility_label(label.clone())
            .when(collapsed, |button| button.tooltip(label.clone()))
            .child(
                h_flex()
                    .w_full()
                    .min_w_0()
                    .gap(space::MD)
                    .when(collapsed, |row| row.justify_center())
                    .child(icon)
                    .when(!collapsed, |row| {
                        row.child(
                            div()
                                .min_w_0()
                                .overflow_hidden()
                                .whitespace_nowrap()
                                .child(label),
                        )
                    }),
            )
            .on_click(cx.listener(move |this, _, _, cx| this.navigate(page.clone(), cx)))
    }

    fn sidebar(&self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        static LOGO: LazyLock<Arc<Image>> = LazyLock::new(|| {
            Arc::new(Image::from_bytes(
                ImageFormat::Svg,
                include_bytes!("../../assets/logo-mark.svg").to_vec(),
            ))
        });
        let motion = cx.theme().motion_tokens();
        let expanded = base::motion::transition(
            "sidebar-expanded",
            if self.collapsed { 0.0_f32 } else { 1.0 },
            base::motion::Transition::new(motion.duration_normal)
                .easing(motion.easing_move.clone()),
            window,
            cx,
        );
        let collapsed_width = layout::SIDEBAR_COLLAPSED_WIDTH.to_pixels(window.rem_size());
        let expanded_width = layout::SIDEBAR_WIDTH.to_pixels(window.rem_size());
        let width = collapsed_width + (expanded_width - collapsed_width) * expanded;
        let brand_inset =
            (collapsed_width - layout::BRAND_LOGO_SIZE.to_pixels(window.rem_size())) / 2.;
        let brand_inset =
            brand_inset + (space::LG.to_pixels(window.rem_size()) - brand_inset) * expanded;
        let category_pages: Vec<Page> = self
            .catalog
            .categories
            .iter()
            .map(|category| Page::Category(category.id.clone()))
            .collect();
        let mut navigation = v_flex().gap(space::LG).px(space::MD).pb(space::MD);
        for pages in [vec![Page::Home, Page::SystemInfo], category_pages] {
            navigation = navigation.child(
                v_flex()
                    .gap(space::XS)
                    .children(pages.into_iter().map(|page| self.nav_item(page, cx))),
            );
        }
        v_flex()
            .id("sidebar")
            .w(width)
            .overflow_hidden()
            .flex_shrink_0()
            .h_full()
            .child(
                h_flex()
                    .h(layout::BRAND_HEIGHT)
                    .flex_shrink_0()
                    .px(brand_inset)
                    .gap(space::SM)
                    .overflow_hidden()
                    .child(
                        Button::new("toggle-sidebar-logo")
                            .ghost()
                            .p(space::NONE)
                            .size(layout::BRAND_LOGO_SIZE)
                            .flex_shrink_0()
                            .accessibility_label(if self.collapsed {
                                "展开侧栏"
                            } else {
                                "收起侧栏"
                            })
                            .tooltip("切换侧栏 · Ctrl+B")
                            .child(img(LOGO.clone()).size_full())
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.collapsed = !this.collapsed;
                                cx.notify();
                            })),
                    )
                    .child(
                        div()
                            .min_w_0()
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .font_weight(text::HEADING)
                            .child("Suki Toolbox"),
                    ),
            )
            .child(
                div()
                    .id("main-navigation-scroll")
                    .flex_1()
                    .min_h(space::NONE)
                    .overflow_y_scrollbar()
                    .child(navigation),
            )
            .child(
                v_flex()
                    .flex_shrink_0()
                    .px(space::MD)
                    .pb(space::MD)
                    .child(self.nav_item(Page::Settings, cx)),
            )
    }

    fn content(&self, route: &Page) -> AnyElement {
        match route {
            Page::Settings => self.settings_page.clone().into_any_element(),
            Page::SystemInfo => self.system_page.clone().into_any_element(),
            page => self
                .tools_pages
                .iter()
                .find(|(key, _)| key == page)
                .map(|(_, view)| view.clone().into_any_element())
                .unwrap_or_else(|| div().into_any_element()),
        }
    }
}

impl Render for ToolboxApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .id("toolbox-window")
            .key_context("Toolbox")
            .track_focus(&self.focus)
            .size_full()
            .text_size(text::BODY)
            .bg(ui::window_tint(cx))
            .text_color(cx.theme().foreground)
            .on_action(cx.listener(|this, _: &ToggleSidebar, _, cx| {
                this.collapsed = !this.collapsed;
                cx.notify();
            }))
            .on_action(|_: &Quit, _, cx| cx.quit())
            .child(
                TitleBar::new()
                    .border(stroke::NONE)
                    .bg(cx.theme().background.opacity(opacity::TRANSPARENT))
                    .child(
                        h_flex()
                            .flex_1()
                            .min_w_0()
                            .pr(space::SM)
                            .when(self.collapsed, |bar| {
                                bar.child(
                                    ui::muted("Suki Toolbox", cx)
                                        .id("window-title")
                                        .text_size(text::BODY),
                                )
                            })
                            .child(div().flex_1()),
                    ),
            )
            .child(
                h_flex()
                    .items_stretch()
                    .flex_1()
                    .min_h(space::NONE)
                    .w_full()
                    .child(self.sidebar(window, cx))
                    .child(
                        v_flex()
                            .flex_1()
                            .min_w(space::NONE)
                            .min_h(space::NONE)
                            .mr(space::SM)
                            .mb(space::SM)
                            .rounded(cx.theme().radius_lg)
                            .bg(ui::workspace_tint(cx))
                            .overflow_hidden()
                            .child({
                                let layers = self.page_weights(window, cx);
                                div().id("page-transition").size_full().relative().children(
                                    layers.into_iter().map(|(page, weight)| {
                                        div()
                                            .id((
                                                ElementId::from("page-layer"),
                                                SharedString::from(page.id()),
                                            ))
                                            .absolute()
                                            .size_full()
                                            .opacity(weight)
                                            .child(self.content(&page))
                                    }),
                                )
                            }),
                    ),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::{Page, ToolboxApp};
    use crate::tools::{Catalog, Category, Tool};
    use gpui_kit::test::TestWindowExt;
    use gpui_kit::{AppContext as _, TestAppContext, px};

    fn catalog() -> Catalog {
        let tool = |name: &str| Tool {
            id: format!("CPU 工具/{name}"),
            name: name.into(),
            description: "tools/硬件检测/…".into(),
            exe: None,
            ..Default::default()
        };
        Catalog {
            categories: vec![Category {
                id: "CPU-工具".into(),
                label: "CPU 工具".into(),
                detail: String::new(),
                tools: vec![tool("CPU-Z"), tool("Core Temp")],
                ..Default::default()
            }],
            home: vec![tool("CPU-Z")],
            ..Default::default()
        }
    }

    fn open(cx: &mut TestAppContext, catalog: Catalog) -> gpui_kit::AnyWindowHandle {
        open_catalog(cx, catalog, false)
    }

    fn open_catalog(
        cx: &mut TestAppContext,
        catalog: Catalog,
        config_missing: bool,
    ) -> gpui_kit::AnyWindowHandle {
        cx.update(|cx| {
            gpui_kit::init(cx);
            crate::ui::notifications::init(cx);
            crate::settings::init(crate::settings::Preferences::default(), cx);
            cx.set_reduce_motion(true);
        });
        let (handle, _) = cx.update(|cx| {
            gpui_kit::open_window(gpui_kit::WindowOptions::default(), cx, |window, cx| {
                cx.new(|cx| ToolboxApp::new(catalog, config_missing, window, cx))
            })
            .unwrap()
        });
        cx.run_until_parked();
        handle
    }

    #[gpui_kit::test]
    fn missing_config_starts_on_system_info_even_with_supplied_favorites(cx: &mut TestAppContext) {
        let handle = open_catalog(cx, catalog(), true);
        cx.update_window(handle, |_, window, cx| {
            window.render_frame(cx);
            assert!(window.find("refresh-system-information").visible());
            window.click("nav-home", cx);
            window.render_frame(cx);
            assert!(window.find("tool-CPU 工具/CPU-Z").visible());
        })
        .unwrap();
    }

    #[gpui_kit::test]
    fn sidebar_follows_the_catalog_and_reaches_settings(cx: &mut TestAppContext) {
        let handle = open(cx, catalog());
        cx.update_window(handle, |_, window, cx| {
            window.render_frame(cx);
            assert!(window.find("nav-home").bounds().size.width > px(0.));
            assert!(window.find("tool-CPU 工具/CPU-Z").bounds().size.width > px(0.));
            assert!(window.find_all("tool-CPU 工具/Core Temp").is_empty());
            window.click("nav-CPU-工具", cx);
        })
        .unwrap();
        cx.run_until_parked();
        cx.update_window(handle, |_, window, cx| {
            window.render_frame(cx);
            assert!(window.find("tool-CPU 工具/Core Temp").bounds().size.width > px(0.));
            window.click("nav-settings", cx);
        })
        .unwrap();
        cx.run_until_parked();
        cx.update_window(handle, |_, window, cx| {
            window.render_frame(cx);
            assert!(window.find_all("tool-CPU 工具/CPU-Z").is_empty());
        })
        .unwrap();
    }

    #[gpui_kit::test]
    fn empty_catalog_keeps_navigation_and_shows_the_hint(cx: &mut TestAppContext) {
        let handle = open(cx, Catalog::default());
        cx.update_window(handle, |_, window, cx| {
            window.render_frame(cx);
            assert!(window.find("nav-home").bounds().size.width > px(0.));
            assert!(window.find("refresh-system-information").visible());
            window.click("nav-home", cx);
            window.render_frame(cx);
            assert!(window.find("tools-empty").bounds().size.width > px(0.));
            assert!(window.find_all("nav-settings").len() == 1);
        })
        .unwrap();
        assert_eq!(Page::Home.id(), "home");
    }
    #[gpui_kit::test]
    fn tool_context_menu_opens_editor_and_escape_discards_changes(cx: &mut TestAppContext) {
        let handle = open(cx, catalog());
        cx.update_window(handle, |_, window, cx| {
            window.render_frame(cx);
            window.right_click("tool-CPU 工具/CPU-Z", cx);
        })
        .unwrap();
        cx.run_until_parked();
        cx.update_window(handle, |_, window, cx| {
            window.render_frame(cx);
            let mut menu = window.within("popup-menu");
            assert!(menu.find_by_label("打开").visible());
            assert!(menu.find_by_label("以管理员身份打开").visible());
            assert!(menu.find_by_label("打开文件所在位置").visible());
            assert!(menu.find_by_label("打开链接").visible());
            assert!(menu.find_by_label("收藏").visible());
            assert!(menu.find_by_label("删除").visible());
            assert!(menu.try_find_by_label("新建…").is_none());
            for _ in 0..3 {
                menu.press("down", cx);
            }
            assert_eq!(menu.find_by_label("编辑…").selected(), Some(true));
            menu.press("enter", cx);
        })
        .unwrap();
        cx.run_until_parked();
        cx.update_window(handle, |_, window, cx| {
            window.render_frame(cx);
            assert!(window.find("dialog-save").visible());
            assert!(window.find("tool-title").visible());
            window.click("tool-title", cx);
            window.press("ctrl-a", cx);
            window.input("未保存的标题", cx);
            window.press("escape", cx);
        })
        .unwrap();
        cx.run_until_parked();
        cx.update_window(handle, |_, window, cx| {
            window.render_frame(cx);
            assert!(window.try_find("dialog-save").is_none());
            assert!(window.find("tool-CPU 工具/CPU-Z").visible());
        })
        .unwrap();
    }

    #[gpui_kit::test]
    fn empty_workspace_can_create_and_invalid_submit_keeps_form(cx: &mut TestAppContext) {
        let handle = open(cx, Catalog::default());
        cx.update_window(handle, |_, window, cx| {
            window.render_frame(cx);
            window.click("nav-home", cx);
            window.render_frame(cx);
            window.right_click("tools-empty", cx);
        })
        .unwrap();
        cx.run_until_parked();
        cx.update_window(handle, |_, window, cx| {
            window.render_frame(cx);
            let mut menu = window.within("popup-menu");
            assert!(menu.find_by_label("新建…").visible());
            menu.press("down", cx);
            menu.press("enter", cx);
        })
        .unwrap();
        cx.run_until_parked();
        cx.update_window(handle, |_, window, cx| {
            window.render_frame(cx);
            assert!(window.find("tool-category").visible());
            window.click("dialog-save", cx);
            assert!(window.find("dialog-save").visible());
            assert!(!crate::ui::notifications::current(window, cx).is_empty());
            window.click("dialog-cancel", cx);
        })
        .unwrap();
        cx.run_until_parked();
        cx.update_window(handle, |_, window, cx| {
            window.render_frame(cx);
            assert!(window.try_find("dialog-save").is_none());
            assert!(window.find("tools-empty").visible());
        })
        .unwrap();
    }
}
