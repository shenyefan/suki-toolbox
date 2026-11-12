use super::{Tool, store::CatalogStore};
use crate::ui::{
    self,
    theme::{control, space},
};
use gpui_kit::{
    base::IndexPath,
    component::{
        Disableable, Sizable, WindowExt,
        button::Button,
        form::{Field, Form},
        input::{Input, InputState},
        select::{Select, SelectState},
    },
    *,
};

struct ToolEditor {
    store: Entity<CatalogStore>,
    original: Option<Tool>,
    fields: Vec<Entity<InputState>>,
    category: Entity<SelectState<Vec<SharedString>>>,
    picker: Option<Task<()>>,
    uploaded_icon: Option<std::path::PathBuf>,
    _subscription: Subscription,
    _icon_subscription: Subscription,
}

const LABELS: [&str; 6] = ["标题", "路径", "参数", "链接", "图标", "说明"];

pub(super) fn open(
    store: Entity<CatalogStore>,
    tool: Option<Tool>,
    category: Option<String>,
    window: &mut Window,
    cx: &mut App,
) {
    let title = if tool.is_some() {
        "编辑工具"
    } else {
        "新建工具"
    };
    let editor = cx.new(|cx| {
        let values = tool
            .as_ref()
            .map(|tool| {
                vec![
                    tool.name.clone(),
                    tool.exe
                        .as_ref()
                        .map(|path| {
                            store
                                .read(cx)
                                .config_file
                                .as_ref()
                                .map(|file| super::config_path(&file.directory, path))
                                .unwrap_or_else(|_| path.display().to_string())
                        })
                        .unwrap_or_default(),
                    tool.arguments.clone(),
                    tool.link.clone(),
                    tool.icon.clone(),
                    tool.description.clone(),
                ]
            })
            .unwrap_or_else(|| vec![String::new(); 6]);
        let fields: Vec<Entity<InputState>> = values
            .into_iter()
            .map(|value| cx.new(|cx| InputState::new(window, cx).default_value(value)))
            .collect();
        let categories = &store.read(cx).catalog.categories;
        let selected = categories
            .iter()
            .position(|item| Some(&item.id) == category.as_ref())
            .map(IndexPath::new);
        let choices = categories
            .iter()
            .map(|item| SharedString::from(item.label.clone()))
            .collect();
        let category = cx.new(|cx| SelectState::new(choices, selected, window, cx));
        let subscription = cx.observe(&store, |_, _, cx| cx.notify());
        let icon_subscription = cx.observe(&fields[4], |_, _, cx| cx.notify());
        ToolEditor {
            store,
            original: tool,
            fields,
            category,
            _subscription: subscription,
            _icon_subscription: icon_subscription,
            picker: None,
            uploaded_icon: None,
        }
    });
    window.open_dialog(cx, move |dialog, window, cx| {
        let confirm = editor.clone();
        let cancel = editor.clone();
        let pending = editor.read(cx).store.read(cx).saving;
        ui::modal::frame(dialog, title, window, cx)
            .content(ui::modal::content("tool-editor-scroll", editor.clone()))
            .footer(ui::modal::footer(pending))
            .on_ok(move |_, window, cx| {
                confirm.update(cx, |this, cx| this.save(window, cx));
                false
            })
            .on_cancel(move |_, _, cx| !cancel.read(cx).store.read(cx).saving)
    });
}

impl ToolEditor {
    fn save(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.store.read(cx).saving {
            return;
        }
        let values: Vec<String> = self
            .fields
            .iter()
            .map(|field| field.read(cx).value().to_string())
            .collect();
        if values[0].trim().is_empty()
            || (values[1].trim().is_empty() && values[3].trim().is_empty())
        {
            ui::show_error("标题不能为空，路径和链接至少填写一项".into(), window, cx);
            return;
        }
        let category = self
            .category
            .read(cx)
            .selected_value()
            .map(ToString::to_string);
        let Some(category) = category else {
            ui::show_error("请先在设置中添加分类并选择分类".into(), window, cx);
            return;
        };
        let file = match self.store.read(cx).config_file.clone() {
            Ok(file) => file,
            Err(error) => {
                ui::show_error(error, window, cx);
                return;
            }
        };
        let mut next = self.store.read(cx).catalog.clone();
        let mut tool = self.original.clone().unwrap_or_else(|| Tool {
            id: uuid::Uuid::new_v4().to_string(),
            ..Default::default()
        });
        tool.name = values[0].trim().into();
        tool.exe = (!values[1].trim().is_empty())
            .then(|| super::resolve(&file.directory, values[1].trim()));
        tool.arguments = values[2].clone();
        tool.link = values[3].trim().into();
        tool.icon = values[4].trim().into();
        tool.description = values[5].clone();
        let upload = self
            .uploaded_icon
            .as_ref()
            .filter(|path| path.to_string_lossy() == tool.icon)
            .map(|path| (tool.id.clone(), path.clone()));
        for item in &mut next.categories {
            item.tools.retain(|item| item.id != tool.id);
        }
        if let Some(target) = next
            .categories
            .iter_mut()
            .find(|item| item.label == category)
        {
            // Retain configured ordering when editing within the same category.
            let original_ix = self
                .store
                .read(cx)
                .catalog
                .categories
                .iter()
                .find(|item| item.label == category)
                .and_then(|item| item.tools.iter().position(|item| item.id == tool.id));
            target
                .tools
                .insert(original_ix.unwrap_or(target.tools.len()), tool);
        }
        self.store.update(cx, |store, cx| {
            store.save_with_icon(next, upload, window, cx)
        });
    }
}

impl Render for ToolEditor {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let pending = self.store.read(cx).saving;
        let mut form = Form::vertical()
            .w_full()
            .with_size(control::STANDARD)
            .gap(space::MD)
            .child(
                Field::new().label("标题").child(
                    Input::new(&self.fields[0])
                        .id("tool-title")
                        .aria_label("标题")
                        .w_full()
                        .with_size(control::STANDARD)
                        .disabled(pending),
                ),
            )
            .child(
                Field::new().label("分类").child(
                    Select::new(&self.category)
                        .id("tool-category")
                        .accessibility_label("分类")
                        .placeholder("选择分类")
                        .w_full()
                        .with_size(control::STANDARD)
                        .disabled(pending),
                ),
            );
        for (ix, label) in LABELS.iter().enumerate().skip(1) {
            let input = if ix == 4 {
                let value = self.fields[4].read(cx).value().to_string();
                let preview = if value.is_empty() {
                    gpui_kit::component::Icon::new(gpui_kit::assets::IconName::Image)
                        .size(ui::theme::tool_card::ICON)
                        .into_any_element()
                } else {
                    let path = self
                        .store
                        .read(cx)
                        .config_file
                        .as_ref()
                        .map(|file| super::resolve(&file.directory, &value))
                        .unwrap_or_else(|_| value.into());
                    img(path)
                        .size(ui::theme::tool_card::ICON)
                        .object_fit(ObjectFit::Contain)
                        .into_any_element()
                };
                ui::file_field::preview_field(
                    "tool-icon",
                    "图标路径",
                    &self.fields[4],
                    pending,
                    Button::new("tool-icon-upload")
                        .outline()
                        .with_size(control::STANDARD)
                        .child(preview)
                        .accessibility_label("上传工具图标")
                        .tooltip("上传图片…")
                        .disabled(pending)
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.picker = Some(ui::file_field::choose(
                                this.fields[4].clone(),
                                ui::file_field::PathKind::Image,
                                |this, path, _, _| {
                                    this.uploaded_icon = Some(path.clone());
                                    Ok(path.display().to_string())
                                },
                                window,
                                cx,
                            ));
                        })),
                )
                .into_any_element()
            } else if ix == 1 {
                ui::file_field::field(
                    "tool-path",
                    label,
                    &self.fields[ix],
                    pending,
                    cx.listener(move |this, _, window, cx| {
                        let kind = ui::file_field::PathKind::File;
                        let file = this.store.read(cx).config_file.clone();
                        this.picker = Some(ui::file_field::choose(
                            this.fields[ix].clone(),
                            kind,
                            move |_, path, _, _| {
                                file.map(|file| super::config_path(&file.directory, &path))
                            },
                            window,
                            cx,
                        ));
                    }),
                )
                .into_any_element()
            } else {
                Input::new(&self.fields[ix])
                    .id(SharedString::from(format!("tool-field-{ix}")))
                    .aria_label((*label).to_owned())
                    .w_full()
                    .with_size(control::STANDARD)
                    .disabled(pending)
                    .into_any_element()
            };
            form = form.child(Field::new().label((*label).to_owned()).child(input));
        }
        form
    }
}

#[cfg(test)]
mod tests {
    use super::open;
    use crate::tools::{Catalog, Category, ConfigFile, store::CatalogStore, view::ToolsPage};
    use gpui_kit::{
        AppContext as _, Bounds, TestAppContext, WindowBounds, WindowOptions, px, size,
        test::TestWindowExt,
    };
    use std::sync::Arc;

    #[gpui_kit::test]
    fn icon_upload_is_imported_on_save_and_cancel_preserves_disk(cx: &mut TestAppContext) {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("selected.png");
        std::fs::write(&source, b"selected image").unwrap();
        let file = Arc::new(ConfigFile::in_directory(directory.path()));
        cx.update(|cx| {
            gpui_kit::init(cx);
            crate::settings::init(crate::settings::Preferences::default(), cx);
            crate::ui::notifications::init(cx);
            cx.set_reduce_motion(true);
        });
        let tool = crate::tools::Tool {
            id: "site".into(),
            name: "网站".into(),
            link: "https://example.com".into(),
            ..Default::default()
        };
        let store = cx.new(|_| {
            let mut store = CatalogStore::new(Catalog {
                categories: vec![Category {
                    id: "web".into(),
                    label: "网站".into(),
                    tools: vec![tool.clone()],
                    ..Default::default()
                }],
                ..Default::default()
            });
            store.config_file = Ok(file.clone());
            store
        });
        let (handle, _) = cx.update(|cx| {
            gpui_kit::open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                        Default::default(),
                        size(px(880.), px(900.)),
                    ))),
                    ..Default::default()
                },
                cx,
                |window, cx| {
                    cx.new(|cx| ToolsPage::new("web", "网站", "", store.clone(), window, cx))
                },
            )
            .unwrap()
        });
        for save in [false, true] {
            cx.update_window(handle, |_, window, cx| {
                open(
                    store.clone(),
                    Some(tool.clone()),
                    Some("web".into()),
                    window,
                    cx,
                );
                window.render_frame(cx);
                assert!(window.find("tool-icon").visible());
                window.click("tool-icon-upload", cx);
            })
            .unwrap();
            assert!(cx.did_prompt_for_paths());
            cx.simulate_path_prompt_response(|options| {
                assert!(options.files);
                Some(vec![source.clone()])
            });
            cx.run_until_parked();
            assert!(!directory.path().join("icons").exists());
            cx.update_window(handle, |_, window, cx| {
                window.render_frame(cx);
                window.click(if save { "dialog-save" } else { "dialog-cancel" }, cx);
            })
            .unwrap();
            cx.run_until_parked();
        }
        let icon = store.read_with(cx, |store, _| {
            store.catalog.categories[0].tools[0].icon.clone()
        });
        assert!(icon.starts_with("icons/"));
        assert_eq!(
            std::fs::read(directory.path().join(&icon)).unwrap(),
            b"selected image"
        );
        let (loaded, error) = file.load();
        assert!(error.is_none());
        assert_eq!(loaded.categories[0].tools[0].icon, icon);
        cx.update_window(handle, |_, window, cx| {
            open(
                store.clone(),
                Some(loaded.categories[0].tools[0].clone()),
                Some("web".into()),
                window,
                cx,
            );
            window.render_frame(cx);
            window.click("tool-icon", cx);
            window.press("ctrl-a", cx);
            window.press("backspace", cx);
            window.click("dialog-save", cx);
        })
        .unwrap();
        cx.run_until_parked();
        assert!(store.read_with(cx, |store, _| {
            store.catalog.categories[0].tools[0].icon.is_empty()
        }));
    }

    #[gpui_kit::test]
    fn website_tool_saves_favorites_and_deletes_through_native_controls(cx: &mut TestAppContext) {
        let directory = tempfile::tempdir().unwrap();
        let file = Arc::new(ConfigFile::in_directory(directory.path()));
        cx.update(|cx| {
            gpui_kit::init(cx);
            crate::settings::init(crate::settings::Preferences::default(), cx);
            crate::ui::notifications::init(cx);
            cx.set_reduce_motion(true);
        });
        let store = cx.new(|_| {
            let mut store = CatalogStore::new(Catalog {
                categories: vec![Category {
                    id: "web".into(),
                    label: "网站".into(),
                    ..Default::default()
                }],
                ..Default::default()
            });
            store.config_file = Ok(file.clone());
            store
        });
        let (handle, _) = cx.update(|cx| {
            gpui_kit::open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                        Default::default(),
                        size(px(1180.), px(900.)),
                    ))),
                    ..Default::default()
                },
                cx,
                |window, cx| {
                    cx.new(|cx| ToolsPage::new("web", "网站", "", store.clone(), window, cx))
                },
            )
            .unwrap()
        });
        cx.update_window(handle, |_, window, cx| {
            open(store.clone(), None, Some("web".into()), window, cx);
            window.render_frame(cx);
            window.click("tool-title", cx);
            window.input("工具网站", cx);
            window.click("tool-field-3", cx);
            window.input("https://example.com/tools?a=1&b=2", cx);
            window.click("tool-field-5", cx);
            window.input("工具说明", cx);
            window.click("dialog-save", cx);
        })
        .unwrap();
        cx.run_until_parked();
        let id = store.read_with(cx, |store, _| {
            assert!(!store.saving);
            let tool = &store.catalog.categories[0].tools[0];
            assert_eq!(tool.name, "工具网站");
            assert!(tool.exe.is_none());
            assert_eq!(tool.link, "https://example.com/tools?a=1&b=2");
            assert_eq!(tool.description, "工具说明");
            tool.id.clone()
        });
        let (loaded, error) = file.load();
        assert!(error.is_none());
        assert_eq!(loaded.categories[0].tools[0].id, id);
        // A settings fallback must still publish the saved favorite and notify the user.
        let mut config: serde_json::Value =
            serde_json::from_slice(&std::fs::read(file.path()).unwrap()).unwrap();
        config["settings"]["theme"] = "invalid".into();
        std::fs::write(file.path(), serde_json::to_vec(&config).unwrap()).unwrap();
        cx.update_window(handle, |_, window, cx| {
            window.render_frame(cx);
            assert!(window.try_find("dialog-save").is_none());
            window.right_click(gpui_kit::SharedString::from(format!("tool-{id}")), cx);
        })
        .unwrap();
        cx.run_until_parked();
        cx.update_window(handle, |_, window, cx| {
            window.render_frame(cx);
            let mut menu = window.within("popup-menu");
            for _ in 0..3 {
                menu.press("down", cx);
            }
            assert_eq!(menu.find_by_label("收藏").selected(), Some(true));
            menu.press("enter", cx);
        })
        .unwrap();
        cx.run_until_parked();
        store.read_with(cx, |store, _| assert_eq!(store.catalog.home[0].id, id));
        cx.update_window(handle, |_, window, cx| {
            window.render_frame(cx);
            assert!(!crate::ui::notifications::current(window, cx).is_empty());
            window.right_click(gpui_kit::SharedString::from(format!("tool-{id}")), cx);
        })
        .unwrap();
        cx.run_until_parked();
        cx.update_window(handle, |_, window, cx| {
            window.render_frame(cx);
            let mut menu = window.within("popup-menu");
            for _ in 0..5 {
                menu.press("down", cx);
            }
            assert_eq!(menu.find_by_label("删除").selected(), Some(true));
            menu.press("enter", cx);
        })
        .unwrap();
        cx.run_until_parked();
        let (loaded, error) = file.load();
        assert!(error.is_none());
        assert!(loaded.categories[0].tools.is_empty());
        assert!(loaded.home.is_empty());
    }
}
