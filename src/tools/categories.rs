//! Category maintenance, backed by the same catalog as the tool editor.
use super::{Category, reorder::DragItem, store::CatalogStore};
use crate::ui::{
    self,
    file_field::{self, PathKind},
    theme::{control, layout, space},
};
use gpui_kit::{
    assets::IconName,
    component::{
        ActiveTheme, Disableable, Icon, Selectable, Sizable, VirtualListScrollHandle, WindowExt,
        button::{Button, ButtonVariants},
        dialog::{DialogAction, DialogFooter},
        form::{Field, Form},
        h_flex,
        input::{Input, InputEvent, InputState},
        menu::{ContextMenuExt, PopupMenuItem},
        popover::Popover,
        v_flex,
    },
    prelude::FluentBuilder as _,
    *,
};

pub(crate) fn open_manager(store: Entity<CatalogStore>, window: &mut Window, cx: &mut App) {
    let panel = cx.new(|cx| CategoryManager::new(store.clone(), cx));
    window.open_dialog(cx, move |dialog, window, cx| {
        let pending = store.read(cx).saving;
        let cancel = store.clone();
        ui::modal::frame(dialog, "工具分类", window, cx)
            .child(panel.clone())
            .footer(
                DialogFooter::new().justify_end().child(
                    Button::new("category-manager-close")
                        .outline()
                        .label("完成")
                        .with_size(control::STANDARD)
                        .disabled(pending)
                        .on_click(|_, window, cx| window.close_dialog(cx)),
                ),
            )
            .on_cancel(move |_, _, cx| !cancel.read(cx).saving)
    });
}

struct CategoryManager {
    store: Entity<CatalogStore>,
    scroll: VirtualListScrollHandle,
    _subscription: Subscription,
}

impl CategoryManager {
    pub(crate) fn new(store: Entity<CatalogStore>, cx: &mut Context<Self>) -> Self {
        let subscription = cx.observe(&store, |_, _, cx| cx.notify());
        Self {
            store,
            scroll: VirtualListScrollHandle::new(),
            _subscription: subscription,
        }
    }
}

impl Render for CategoryManager {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let pending = self.store.read(cx).saving;
        let categories = self.store.read(cx).catalog.categories.clone();
        let count = categories.len();
        let list =
            if count == 0 {
                ui::muted("暂无分类", cx)
                    .px(space::LG)
                    .py(space::MD)
                    .into_any_element()
            } else {
                let height = layout::CATEGORY_ROW_HEIGHT.to_pixels(window.rem_size());
                div()
                    .h(layout::CATEGORY_LIST_HEIGHT
                        .to_pixels(window.rem_size())
                        .min(height * count as f32))
                    .child(ui::virtual_rows::rows(
                        cx.entity(),
                        "category-list",
                        vec![height; count],
                        &self.scroll,
                        move |this, ix, _, cx| {
                            let category = categories[ix].clone();
                            let edit = category.clone();
                            let remove = category.clone();
                            let target = category.id.clone();
                            let drag = DragItem::Category {
                                id: category.id.clone(),
                                label: category.label.clone(),
                            };
                            let earlier = ix
                                .checked_sub(1)
                                .and_then(|index| categories.get(index))
                                .map(|item| item.id.clone());
                            let later = categories.get(ix + 1).map(|item| item.id.clone());
                            let menu_store = this.store.clone();
                            h_flex()
                                .id(SharedString::from(format!("category-row-{}", category.id)))
                                .size_full()
                                .px(space::LG)
                                .gap(space::SM)
                                .child(Icon::new(IconName::GripVertical).size(layout::ICON))
                                .child(div().flex_1().min_w_0().truncate().child(category.label))
                                .child(
                                    Button::new(SharedString::from(format!(
                                        "category-edit-{}",
                                        category.id
                                    )))
                                    .ghost()
                                    .icon(IconName::Pencil)
                                    .with_size(control::STANDARD)
                                    .accessibility_label(format!("编辑分类{}", edit.label))
                                    .tooltip("编辑分类…")
                                    .disabled(pending)
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        open(this.store.clone(), Some(edit.clone()), window, cx)
                                    })),
                                )
                                .child(
                                    Button::new(SharedString::from(format!(
                                        "category-delete-{}",
                                        category.id
                                    )))
                                    .ghost()
                                    .icon(IconName::Trash)
                                    .with_size(control::STANDARD)
                                    .accessibility_label(format!("删除分类{}", remove.label))
                                    .tooltip("删除分类…")
                                    .disabled(pending)
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        delete(this.store.clone(), remove.clone(), window, cx)
                                    })),
                                )
                                .when(!pending, |row| {
                                    row.on_drag(drag, |drag, _, _, cx| cx.new(|_| drag.clone()))
                                        .drag_over::<DragItem>(|style, drag, _, cx| {
                                            if matches!(drag, DragItem::Category { .. }) {
                                                style.bg(cx.theme().accent)
                                            } else {
                                                style
                                            }
                                        })
                                        .on_drop(cx.listener(
                                            move |this, drag: &DragItem, window, cx| {
                                                if let DragItem::Category { id, .. } = drag {
                                                    this.store.update(cx, |store, cx| {
                                                        store.reorder_category(
                                                            id, &target, window, cx,
                                                        )
                                                    });
                                                }
                                            },
                                        ))
                                })
                                .context_menu(move |menu, _, _| {
                                    let up_store = menu_store.clone();
                                    let down_store = menu_store.clone();
                                    let up_source = category.id.clone();
                                    let down_source = category.id.clone();
                                    let up = earlier.clone();
                                    let down = later.clone();
                                    menu.item(
                                        PopupMenuItem::new("上移")
                                            .icon(IconName::ArrowUp)
                                            .disabled(pending || up.is_none())
                                            .on_click(move |_, window, cx| {
                                                if let Some(target) = &up {
                                                    up_store.update(cx, |store, cx| {
                                                        store.reorder_category(
                                                            &up_source, target, window, cx,
                                                        )
                                                    });
                                                }
                                            }),
                                    )
                                    .item(
                                        PopupMenuItem::new("下移")
                                            .icon(IconName::ArrowDown)
                                            .disabled(pending || down.is_none())
                                            .on_click(move |_, window, cx| {
                                                if let Some(target) = &down {
                                                    down_store.update(cx, |store, cx| {
                                                        store.reorder_category(
                                                            &down_source,
                                                            target,
                                                            window,
                                                            cx,
                                                        )
                                                    });
                                                }
                                            }),
                                    )
                                })
                                .into_any_element()
                        },
                    ))
                    .into_any_element()
            };
        v_flex()
            .w_full()
            .gap(space::SM)
            .child(
                h_flex().px(space::LG).child(div().flex_1()).child(
                    Button::new("category-new")
                        .outline()
                        .icon(IconName::Plus)
                        .with_size(control::STANDARD)
                        .label("新建分类…")
                        .disabled(pending)
                        .on_click(cx.listener(|this, _, window, cx| {
                            open(this.store.clone(), None, window, cx)
                        })),
                ),
            )
            .child(
                div()
                    .rounded(cx.theme().radius_lg)
                    .bg(ui::theme::preferences_tint(cx))
                    .child(list),
            )
    }
}

struct CategoryEditor {
    store: Entity<CatalogStore>,
    original: Option<Category>,
    name: Entity<InputState>,
    path: Entity<InputState>,
    icon: Entity<InputState>,
    icon_search: Entity<InputState>,
    icon_scroll: VirtualListScrollHandle,
    _search_subscription: Subscription,
    picker: Option<Task<()>>,
    _subscription: Subscription,
    _icon_subscription: Subscription,
}

fn open(
    store: Entity<CatalogStore>,
    original: Option<Category>,
    window: &mut Window,
    cx: &mut App,
) {
    let title = if original.is_some() {
        "编辑分类"
    } else {
        "新建分类"
    };
    let editor = cx.new(|cx| {
        let category = original.clone().unwrap_or_default();
        let name = cx.new(|cx| InputState::new(window, cx).default_value(category.label));
        let path = cx.new(|cx| InputState::new(window, cx).default_value(category.path));
        let icon = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("例如 Cpu、Monitor、Folder")
                .default_value(category.icon)
        });
        let icon_search = cx.new(|cx| InputState::new(window, cx).placeholder("搜索图标名称"));
        let search_subscription = cx.subscribe(
            &icon_search,
            |this: &mut CategoryEditor, _, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Change) {
                    this.icon_scroll.scroll_to_item(0, ScrollStrategy::Top);
                    cx.notify();
                }
            },
        );
        let subscription = cx.observe(&store, |_, _, cx| cx.notify());
        let icon_subscription = cx.observe(&icon, |_, _, cx| cx.notify());
        CategoryEditor {
            store,
            original,
            name,
            path,
            icon,
            icon_search,
            icon_scroll: VirtualListScrollHandle::new(),
            _search_subscription: search_subscription,
            picker: None,
            _subscription: subscription,
            _icon_subscription: icon_subscription,
        }
    });
    window.open_dialog(cx, move |dialog, window, cx| {
        let confirm = editor.clone();
        let cancel = editor.clone();
        let pending = editor.read(cx).store.read(cx).saving;
        ui::modal::frame(dialog, title, window, cx)
            .content(ui::modal::content("category-editor-scroll", editor.clone()))
            .footer(ui::modal::footer(pending))
            .on_ok(move |_, window, cx| {
                confirm.update(cx, |this, cx| this.save(window, cx));
                false
            })
            .on_cancel(move |_, _, cx| !cancel.read(cx).store.read(cx).saving)
    });
}

impl CategoryEditor {
    fn save(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.store.read(cx).saving {
            return;
        }
        let name = self.name.read(cx).value().trim().to_string();
        let mut next = self.store.read(cx).catalog.clone();
        if name.is_empty()
            || next.categories.iter().any(|category| {
                category.label == name
                    && self
                        .original
                        .as_ref()
                        .is_none_or(|original| original.id != category.id)
            })
        {
            ui::show_error("分类名称不能为空或重复".into(), window, cx);
            return;
        }
        let mut category = self.original.clone().unwrap_or_else(|| Category {
            id: uuid::Uuid::new_v4().to_string(),
            ..Default::default()
        });
        category.label = name;
        category.path = self.path.read(cx).value().trim().into();
        category.icon = self.icon.read(cx).value().trim().into();
        if !category.icon.is_empty() && super::library_icon(&category.icon).is_none() {
            ui::show_error(format!("未知组件库图标：{}", category.icon), window, cx);
            return;
        }
        if let Some(existing) = next
            .categories
            .iter_mut()
            .find(|existing| existing.id == category.id)
        {
            *existing = category;
        } else {
            next.categories.push(category);
        }
        self.store
            .update(cx, |store, cx| store.save(next, true, window, cx));
    }
}

impl Render for CategoryEditor {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let pending = self.store.read(cx).saving;
        Form::vertical()
            .w_full()
            .with_size(control::STANDARD)
            .gap(space::MD)
            .child(
                Field::new().label("分类名称").child(
                    Input::new(&self.name)
                        .id("category-name")
                        .aria_label("分类名称")
                        .w_full()
                        .with_size(control::STANDARD)
                        .disabled(pending),
                ),
            )
            .child(Field::new().label("路径").child(file_field::field(
                "category-path",
                "分类路径",
                &self.path,
                pending,
                cx.listener(|this, _, window, cx| {
                    let file = this.store.read(cx).config_file.clone();
                    this.picker = Some(file_field::choose(
                        this.path.clone(),
                        PathKind::Folder,
                        move |_, path, _, _| {
                            file.map(|file| super::config_path(&file.directory, &path))
                        },
                        window,
                        cx,
                    ));
                }),
            )))
            .child(
                Field::new()
                    .label("图标名称")
                    .child(file_field::preview_field(
                        "category-icon",
                        "分类图标名称",
                        &self.icon,
                        pending,
                        Popover::new("category-icon-picker")
                            .anchor(Anchor::TopRight)
                            .track_focus(&self.icon_search.read(cx).focus_handle(cx))
                            .trigger(
                                Button::new("category-icon-preview")
                                    .outline()
                                    .with_size(control::STANDARD)
                                    .icon(
                                        super::library_icon(self.icon.read(cx).value().trim())
                                            .unwrap_or(IconName::Folder),
                                    )
                                    .accessibility_label("选择分类图标")
                                    .tooltip("选择图标…")
                                    .disabled(pending),
                            )
                            .content({
                                let input = self.icon.clone();
                                let search = self.icon_search.clone();
                                let scroll = self.icon_scroll.clone();
                                move |_, window, cx| {
                                    let query = search.read(cx).value().trim().to_lowercase();
                                    let icons: Vec<_> = IconName::ALL
                                        .iter()
                                        .copied()
                                        .filter(|icon| {
                                            format!("{icon:?}").to_lowercase().contains(&query)
                                        })
                                        .collect();
                                    let selected = input.read(cx).value().to_string();
                                    let input = input.clone();
                                    let rows = icons.len().div_ceil(layout::ICON_PICKER_COLUMNS);
                                    let row_height =
                                        layout::ICON_PICKER_ROW_HEIGHT.to_pixels(window.rem_size());
                                    let list = if rows == 0 {
                                        ui::muted("无匹配图标", cx).into_any_element()
                                    } else {
                                        div()
                                            .h(layout::ICON_PICKER_HEIGHT)
                                            .child(ui::virtual_rows::rows(
                                                cx.entity(),
                                                "library-icons",
                                                vec![row_height; rows],
                                                &scroll,
                                                move |_, ix, _, cx| {
                                                    h_flex()
                                                    .size_full()
                                                    .gap(space::XS)
                                                    .children(
                                                        icons
                                                            .iter()
                                                            .skip(ix * layout::ICON_PICKER_COLUMNS)
                                                            .take(layout::ICON_PICKER_COLUMNS)
                                                            .map(|icon| {
                                                                let name = format!("{icon:?}");
                                                                let input = input.clone();
                                                                Button::new(SharedString::from(
                                                                    format!("library-icon-{name}"),
                                                                ))
                                                                .ghost()
                                                                .with_size(control::STANDARD)
                                                                .flex_1()
                                                                .min_w_0()
                                                                .icon(*icon)
                                                                .selected(selected == name)
                                                                .accessibility_label(name.clone())
                                                                .tooltip(name.clone())
                                                                .on_click(cx.listener(
                                                                    move |state, _, window, cx| {
                                                                        input.update(
                                                                            cx,
                                                                            |input, cx| {
                                                                                input.set_value(
                                                                                    name.clone(),
                                                                                    window,
                                                                                    cx,
                                                                                )
                                                                            },
                                                                        );
                                                                        state.dismiss(window, cx);
                                                                    },
                                                                ))
                                                            }),
                                                    )
                                                    .children(
                                                        (0..layout::ICON_PICKER_COLUMNS
                                                            .saturating_sub(
                                                            icons.len().saturating_sub(
                                                                ix * layout::ICON_PICKER_COLUMNS,
                                                            ),
                                                        ))
                                                            .map(|_| div().flex_1()),
                                                    )
                                                    .into_any_element()
                                                },
                                            ))
                                            .into_any_element()
                                    };
                                    v_flex()
                                        .id("category-icon-panel")
                                        .w(layout::ICON_PICKER_WIDTH)
                                        .gap(space::SM)
                                        .child(
                                            Input::new(&search)
                                                .id("category-icon-search")
                                                .aria_label("搜索图标名称")
                                                .w_full()
                                                .with_size(control::STANDARD),
                                        )
                                        .child(list)
                                }
                            }),
                    )),
            )
    }
}

fn delete(store: Entity<CatalogStore>, category: Category, window: &mut Window, cx: &mut App) {
    window.open_dialog(cx, move |dialog, window, cx| {
        let pending = store.read(cx).saving;
        let confirm = store.clone();
        let cancel = store.clone();
        let id = category.id.clone();
        ui::modal::frame(
            dialog,
            format!("删除分类“{}”？", category.label),
            window,
            cx,
        )
        .child(ui::muted(
            format!("同时移除 {} 个工具条目，保留磁盘文件", category.tools.len()),
            cx,
        ))
        .footer(
            DialogFooter::new()
                .justify_end()
                .gap(space::SM)
                .child(
                    Button::new("category-delete-cancel")
                        .outline()
                        .icon(IconName::X)
                        .label("取消")
                        .disabled(pending)
                        .on_click(|_, window, cx| window.close_dialog(cx)),
                )
                .child(
                    div().flex_none().child(
                        DialogAction::new().child(
                            Button::new("category-delete-confirm")
                                .danger()
                                .icon(IconName::Trash)
                                .label("删除")
                                .disabled(pending)
                                .loading(pending),
                        ),
                    ),
                ),
        )
        .on_ok(move |_, window, cx| {
            let mut next = confirm.read(cx).catalog.clone();
            next.categories.retain(|category| category.id != id);
            confirm.update(cx, |store, cx| store.save(next, true, window, cx));
            false
        })
        .on_cancel(move |_, _, cx| !cancel.read(cx).saving)
    });
}

#[cfg(test)]
mod tests {
    use crate::tools::{Catalog, ConfigFile, store::CatalogStore};
    use gpui_kit::{
        AppContext as _, Bounds, ElementId, SharedString, TestAppContext, WindowBounds,
        WindowOptions, px, size, test::TestWindowExt,
    };
    use std::sync::Arc;

    #[gpui_kit::test]
    fn settings_maintains_categories_and_native_folder_prompt(cx: &mut TestAppContext) {
        let directory = tempfile::tempdir().unwrap();
        let file = Arc::new(ConfigFile::in_directory(directory.path()));
        cx.update(|cx| {
            gpui_kit::init(cx);
            crate::settings::init(crate::settings::Preferences::default(), cx);
            crate::ui::notifications::init(cx);
            cx.set_reduce_motion(true);
        });
        let store = cx.new(|_| {
            let mut store = CatalogStore::new(Catalog::default());
            store.config_file = Ok(file.clone());
            store
        });
        let (handle, _) = cx.update(|cx| {
            gpui_kit::open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                        Default::default(),
                        size(px(880.), px(700.)),
                    ))),
                    ..Default::default()
                },
                cx,
                |window, cx| {
                    cx.new(|cx| crate::settings::view::SettingsPage::new(store.clone(), window, cx))
                },
            )
            .unwrap()
        });
        cx.run_until_parked();
        cx.update_window(handle, |_, window, cx| {
            window.render_frame(cx);
            assert!(window.find("tool-open-mode").visible());
            assert!(window.try_find("save-tool-config").is_none());
            assert!(window.try_find("category-new").is_none());
            window.click("manage-categories", cx);
            window.click("category-new", cx);
            window.click("category-name", cx);
            window.input("硬件检测", cx);
            window.click((ElementId::from("category-path"), "choose"), cx);
        })
        .unwrap();
        assert!(cx.did_prompt_for_paths());
        cx.simulate_path_prompt_response(|options| {
            assert!(options.directories);
            assert!(!options.files);
            Some(vec![directory.path().join("工具")])
        });
        cx.run_until_parked();
        cx.update_window(handle, |_, window, cx| {
            window.render_frame(cx);
            let name = window.find("category-name").bounds();
            let path = window.find("category-path").bounds();
            let cancel = window.find("dialog-cancel").bounds();
            let commit = window.find("dialog-save").bounds();
            assert!((name.left() - path.left()).abs() <= px(1.));
            assert!((commit.center().y - cancel.center().y).abs() <= px(1.));
            window.click("dialog-save", cx);
        })
        .unwrap();
        cx.run_until_parked();
        let id = store.read_with(cx, |store, _| {
            let category = &store.catalog.categories[0];
            assert_eq!(category.label, "硬件检测");
            assert_eq!(category.path, "工具");
            category.id.clone()
        });
        cx.update_window(handle, |_, window, cx| {
            window.render_frame(cx);
            window.click(SharedString::from(format!("category-edit-{id}")), cx);
            window.click("category-name", cx);
            window.press("ctrl-a", cx);
            window.input("处理器", cx);
            window.click("category-icon-preview", cx);
        })
        .unwrap();
        cx.run_until_parked();
        cx.update_window(handle, |_, window, cx| {
            window.render_frame(cx);
            assert!(window.find("category-icon-search").visible());
            window.scroll(
                "library-icon-AArrowDown",
                gpui_kit::ScrollDelta::Pixels(gpui_kit::point(px(0.), px(-600.))),
                cx,
            );
        })
        .unwrap();
        cx.run_until_parked();
        cx.update_window(handle, |_, window, cx| {
            window.render_frame(cx);
            assert!(window.try_find("library-icon-AArrowDown").is_none());
            window.click("category-icon-search", cx);
            window.press("right", cx);
        })
        .unwrap();
        cx.run_until_parked();
        cx.update_window(handle, |_, window, cx| {
            window.render_frame(cx);
            assert!(
                window.try_find("library-icon-AArrowDown").is_none(),
                "focus or caret updates must preserve scroll position"
            );
            window.press("escape", cx);
        })
        .unwrap();
        cx.run_until_parked();
        cx.update_window(handle, |_, window, cx| {
            window.render_frame(cx);
            assert!(window.try_find("category-icon-search").is_none());
            assert!(window.find("category-name").visible());
            window.click("category-icon-preview", cx);
        })
        .unwrap();
        cx.run_until_parked();
        cx.update_window(handle, |_, window, cx| {
            window.render_frame(cx);
            window.click("category-icon-search", cx);
            window.input("cpu", cx);
        })
        .unwrap();
        cx.run_until_parked();
        cx.update_window(handle, |_, window, cx| {
            window.render_frame(cx);
            assert!(window.find("library-icon-Cpu").visible());
            window.click("library-icon-Cpu", cx);
        })
        .unwrap();
        cx.run_until_parked();
        cx.update_window(handle, |_, window, cx| {
            window.render_frame(cx);
            assert!(window.try_find("category-icon-search").is_none());
            window.click("dialog-save", cx);
        })
        .unwrap();
        cx.run_until_parked();
        store.read_with(cx, |store, _| {
            assert_eq!(store.catalog.categories[0].id, id);
            assert_eq!(store.catalog.categories[0].label, "处理器");
            assert_eq!(store.catalog.categories[0].icon, "Cpu");
        });
        let (loaded, error) = file.load();
        assert!(error.is_none());
        assert_eq!(loaded.categories[0].label, "处理器");
        cx.update_window(handle, |_, window, cx| {
            window.render_frame(cx);
            window.click(SharedString::from(format!("category-delete-{id}")), cx);
            window.click("category-delete-confirm", cx);
        })
        .unwrap();
        cx.run_until_parked();
        assert!(store.read_with(cx, |store, _| store.catalog.categories.is_empty()));
        let (loaded, error) = file.load();
        assert!(error.is_none());
        assert!(loaded.categories.is_empty());
    }
}
