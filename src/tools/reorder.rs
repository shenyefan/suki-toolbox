//! Native drag payload and the shared preview for catalog reordering.
use crate::ui::theme::{space, text};
use gpui_kit::{component::ActiveTheme, *};

#[derive(Clone)]
pub(super) enum DragItem {
    Category {
        id: String,
        label: String,
    },
    Tool {
        page: String,
        id: String,
        label: String,
    },
}

impl Render for DragItem {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let label = match self {
            Self::Category { label, .. } | Self::Tool { label, .. } => label.clone(),
        };
        div()
            .px(space::LG)
            .py(space::SM)
            .rounded(cx.theme().radius)
            .bg(cx.theme().popover)
            .text_color(cx.theme().popover_foreground)
            .text_size(text::BODY)
            .child(label)
    }
}

#[cfg(test)]
mod tests {
    use crate::tools::{Catalog, Category, ConfigFile, Tool, store::CatalogStore, view::ToolsPage};
    use gpui_kit::{AppContext as _, TestAppContext, WindowOptions, test::TestWindowExt};
    use std::sync::Arc;

    fn fixture(
        cx: &mut TestAppContext,
    ) -> (
        tempfile::TempDir,
        Arc<ConfigFile>,
        gpui_kit::Entity<CatalogStore>,
    ) {
        let directory = tempfile::tempdir().unwrap();
        let file = Arc::new(ConfigFile::in_directory(directory.path()));
        let tool = |id: &str, favorite| Tool {
            id: id.into(),
            name: id.into(),
            link: "https://example.com".into(),
            favorite,
            ..Default::default()
        };
        let mut catalog = Catalog {
            categories: vec![
                Category {
                    id: "a".into(),
                    label: "A".into(),
                    tools: vec![tool("a1", true), tool("a2", false), tool("a3", true)],
                    ..Default::default()
                },
                Category {
                    id: "b".into(),
                    label: "B".into(),
                    tools: vec![tool("b1", true)],
                    ..Default::default()
                },
                Category {
                    id: "c".into(),
                    label: "C".into(),
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        catalog.refresh_home();
        file.save(&catalog).unwrap();
        cx.update(|cx| {
            gpui_kit::init(cx);
            crate::ui::notifications::init(cx);
            crate::settings::init(crate::settings::Preferences::default(), cx);
            cx.set_reduce_motion(true);
        });
        let store = cx.new(|_| {
            let mut store = CatalogStore::new(catalog);
            store.config_file = Ok(file.clone());
            store
        });
        (directory, file, store)
    }

    #[gpui_kit::test]
    fn category_manager_drag_persists_order_and_keeps_dialog_open(cx: &mut TestAppContext) {
        let (_directory, file, store) = fixture(cx);
        let (handle, _) = cx.update(|cx| {
            gpui_kit::open_window(WindowOptions::default(), cx, |window, cx| {
                cx.new(|cx| crate::settings::view::SettingsPage::new(store.clone(), window, cx))
            })
            .unwrap()
        });
        cx.update_window(handle, |_, window, cx| {
            window.render_frame(cx);
            assert!(window.try_find("category-new").is_none());
            window.click("manage-categories", cx);
        })
        .unwrap();
        cx.run_until_parked();
        cx.update_window(handle, |_, window, cx| {
            window.render_frame(cx);
            // The label is inside the draggable native row; child buttons keep their own behavior.
            let source = window.find("category-edit-a").bounds();
            let target = window.find("category-edit-c").bounds();
            let from = gpui_kit::point(source.left() - source.size.width, source.center().y);
            let to = gpui_kit::point(target.left() - target.size.width, target.center().y);
            TestWindowExt::drag(window, from, to, cx);
        })
        .unwrap();
        cx.run_until_parked();
        store.read_with(cx, |store, _| {
            assert_eq!(
                store
                    .catalog
                    .categories
                    .iter()
                    .map(|item| item.id.as_str())
                    .collect::<Vec<_>>(),
                ["b", "c", "a"]
            )
        });
        let (loaded, error) = file.load();
        assert!(error.is_none());
        assert_eq!(
            loaded
                .categories
                .iter()
                .map(|item| item.label.as_str())
                .collect::<Vec<_>>(),
            ["B", "C", "A"]
        );
        cx.update_window(handle, |_, window, cx| {
            window.render_frame(cx);
            assert!(window.find("category-manager-close").visible());
            assert!(crate::ui::notifications::current(window, cx).is_empty());
        })
        .unwrap();
    }

    #[gpui_kit::test]
    fn tool_card_drag_reorders_tools_without_launching(cx: &mut TestAppContext) {
        let (_directory, file, store) = fixture(cx);
        let (handle, _) = cx.update(|cx| {
            gpui_kit::open_window(WindowOptions::default(), cx, |window, cx| {
                cx.new(|cx| ToolsPage::new("a", "A", "", store.clone(), window, cx))
            })
            .unwrap()
        });
        cx.update_window(handle, |_, window, cx| {
            window.render_frame(cx);
            let from = window.find("tool-a1").bounds().center();
            let to = window.find("tool-a3").bounds().center();
            TestWindowExt::drag(window, from, to, cx);
        })
        .unwrap();
        cx.run_until_parked();
        store.read_with(cx, |store, _| {
            assert_eq!(
                store.catalog.categories[0]
                    .tools
                    .iter()
                    .map(|tool| tool.id.as_str())
                    .collect::<Vec<_>>(),
                ["a2", "a3", "a1"]
            )
        });
        let (loaded, error) = file.load();
        assert!(error.is_none());
        assert_eq!(
            loaded.categories[0]
                .tools
                .iter()
                .map(|tool| tool.id.as_str())
                .collect::<Vec<_>>(),
            ["a2", "a3", "a1"]
        );
        cx.update_window(handle, |_, window, cx| {
            window.render_frame(cx);
            assert!(
                crate::ui::notifications::current(window, cx).is_empty(),
                "dragging must not activate a tool"
            );
        })
        .unwrap();
    }

    #[gpui_kit::test]
    fn favorite_drag_has_its_own_persisted_order(cx: &mut TestAppContext) {
        let (_directory, file, store) = fixture(cx);
        let (handle, _) = cx.update(|cx| {
            gpui_kit::open_window(WindowOptions::default(), cx, |window, cx| {
                cx.new(|cx| ToolsPage::new("home", "收藏", "", store.clone(), window, cx))
            })
            .unwrap()
        });
        cx.update_window(handle, |_, window, cx| {
            window.render_frame(cx);
            let from = window.find("tool-b1").bounds().center();
            let to = window.find("tool-a1").bounds().center();
            TestWindowExt::drag(window, from, to, cx);
        })
        .unwrap();
        cx.run_until_parked();
        let (mut loaded, error) = file.load();
        assert!(error.is_none());
        assert_eq!(
            loaded
                .home
                .iter()
                .map(|tool| tool.id.as_str())
                .collect::<Vec<_>>(),
            ["b1", "a1", "a3"]
        );
        assert_eq!(
            loaded.categories[0]
                .tools
                .iter()
                .map(|tool| tool.id.as_str())
                .collect::<Vec<_>>(),
            ["a1", "a2", "a3"]
        );
        assert!(!loaded.reorder_tool("home", "b1", "b1"));
        assert!(!loaded.reorder_tool("a", "b1", "a1"));
        loaded.categories[0].tools[1].favorite = true;
        loaded.categories[0].tools[0].favorite = false;
        loaded.refresh_home();
        assert_eq!(loaded.favorite_order, ["b1", "a3", "a2"]);
    }
}
