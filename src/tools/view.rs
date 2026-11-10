//! Live catalog grids sharing one persistent catalog owner.
use super::{Tool, entry_icon, reorder::DragItem, store::CatalogStore, windows::ToolAction};
use crate::ui::{
    self,
    theme::{control, layout, space, text, tool_card},
    virtual_rows,
};
use gpui_kit::{
    assets::IconName,
    component::{
        ActiveTheme, Icon, Sizable, VirtualListScrollHandle,
        button::{Button, ButtonVariants},
        h_flex,
        input::{Input, InputEvent, InputState},
        menu::{ContextMenuExt, PopupMenuItem},
        v_flex,
    },
    prelude::FluentBuilder as _,
    *,
};

/// Static page identity plus the tool list it renders.
pub(crate) struct ToolsPage {
    key: SharedString,
    title: String,
    store: Entity<CatalogStore>,
    query: Entity<InputState>,
    scroll: VirtualListScrollHandle,
    columns: u16,
    selected: Option<String>,
    _query_subscription: Subscription,
    _catalog_subscription: Subscription,
}

impl ToolsPage {
    pub(crate) fn new(
        key: &str,
        title: &str,
        _detail: &str,
        store: Entity<CatalogStore>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let query = cx.new(|cx| InputState::new(window, cx).placeholder("搜索工具"));
        let subscription = cx.subscribe_in(&query, window, |_, _, event: &InputEvent, _, cx| {
            if matches!(event, InputEvent::Change) {
                cx.notify();
            }
        });
        Self {
            key: SharedString::from(key.to_owned()),
            title: title.to_owned(),
            _catalog_subscription: cx.observe(&store, |_, _, cx| cx.notify()),
            store,
            query,
            scroll: VirtualListScrollHandle::new(),
            columns: 2,
            selected: None,
            _query_subscription: subscription,
        }
    }

    pub(crate) fn rename(&mut self, title: &str, cx: &mut Context<Self>) {
        self.title = title.into();
        cx.notify();
    }

    fn execute(tool: Tool, action: ToolAction, window: &mut Window, cx: &mut App) {
        let message = match action {
            ToolAction::Reveal => format!("已打开 {} 的文件位置", tool.name),
            ToolAction::Link => format!("已打开 {} 的链接", tool.name),
            _ => format!("已打开 {}", tool.name),
        };
        window
            .spawn(cx, async move |cx| {
                let result = cx
                    .background_spawn(async move { super::windows::execute(&tool, action) })
                    .await;
                let _ = cx.update(|window, cx| match result {
                    Ok(()) => {
                        ui::notifications::push(
                            window,
                            component::notification::Notification::success(message),
                            cx,
                        );
                    }
                    Err(error) => ui::show_error(format!("无法打开：{error}"), window, cx),
                });
            })
            .detach();
    }

    fn visible_tools(&self, cx: &App) -> Vec<Tool> {
        let catalog = &self.store.read(cx).catalog;
        let tools = if self.key.as_ref() == "home" {
            &catalog.home
        } else {
            let Some(category) = catalog.category(&self.key) else {
                return Vec::new();
            };
            &category.tools
        };
        let query = self.query.read(cx).value().trim().to_lowercase();
        if query.is_empty() {
            return tools.clone();
        }
        tools
            .iter()
            .filter(|tool| {
                tool.name.to_lowercase().contains(&query)
                    || tool.description.to_lowercase().contains(&query)
            })
            .cloned()
            .collect()
    }

    fn card(
        &self,
        tool: &Tool,
        earlier: Option<String>,
        later: Option<String>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let launch = tool.clone();
        let selected = self.selected.as_ref() == Some(&tool.id);
        let target = tool.clone();
        let right_id = tool.id.clone();
        let store = self.store.clone();
        let pending = self.store.read(cx).saving;
        let drag = DragItem::Tool {
            page: self.key.to_string(),
            id: tool.id.clone(),
            label: tool.name.clone(),
        };
        let drag_page = self.key.to_string();
        let accept_page = drag_page.clone();
        let drop_page = drag_page.clone();
        let drop_target = tool.id.clone();
        let page = self.key.to_string();
        let icon = if !tool.icon.is_empty() {
            let path = self
                .store
                .read(cx)
                .config_file
                .as_ref()
                .map(|file| super::resolve(&file.directory, &tool.icon))
                .unwrap_or_else(|_| tool.icon.clone().into());
            img(path)
                .size(tool_card::ICON)
                .object_fit(ObjectFit::Contain)
                .flex_shrink_0()
                .into_any_element()
        } else if let Some(icon) = tool
            .exe
            .as_ref()
            .and_then(|path| self.store.read(cx).icons.get(path))
            .and_then(Option::as_ref)
        {
            img(icon.clone())
                .size(tool_card::ICON)
                .flex_shrink_0()
                .into_any_element()
        } else {
            Icon::new(if tool.exe.is_some() {
                entry_icon()
            } else {
                IconName::Link
            })
            .size(tool_card::ICON)
            .flex_shrink_0()
            .into_any_element()
        };
        let card = ui::selectable_card(
            Button::new(SharedString::from(format!("tool-{}", tool.id)))
                .ghost()
                .with_size(control::STANDARD)
                .h_full()
                .min_w_0()
                .px(tool_card::PADDING_X)
                .justify_center()
                .rounded(cx.theme().radius)
                .accessibility_label(tool.name.clone())
                .child(
                    h_flex()
                        .w_full()
                        .min_w_0()
                        .gap(space::SM)
                        .child(icon)
                        .child(
                            v_flex()
                                .min_w_0()
                                .flex_1()
                                .gap(space::XS)
                                .child(
                                    div()
                                        .min_w_0()
                                        .truncate()
                                        .font_weight(text::EMPHASIS)
                                        .child(tool.name.clone()),
                                )
                                .when(!tool.description.is_empty(), |column| {
                                    column.child(
                                        div()
                                            .min_w_0()
                                            .truncate()
                                            .text_size(text::CAPTION)
                                            .text_color(cx.theme().muted_foreground)
                                            .child(tool.description.clone()),
                                    )
                                }),
                        ),
                ),
            selected,
            cx,
        )
        .drag_over::<DragItem>(move |style, drag, _, cx| {
            if matches!(drag, DragItem::Tool {page, ..} if *page == drag_page) {
                style
                    .border_color(cx.theme().drag_border)
                    .bg(cx.theme().accent)
            } else {
                style
            }
        })
        .on_click(cx.listener(move |this, event: &ClickEvent, window, cx| {
            this.selected = Some(launch.id.clone());
            cx.notify();
            if crate::settings::read(cx)
                .open_mode
                .activates(event.is_keyboard(), event.click_count())
            {
                Self::execute(launch.clone(), ToolAction::Open, window, cx);
            }
        }))
        .on_mouse_down(
            MouseButton::Right,
            cx.listener(move |this, _, _, cx| {
                this.selected = Some(right_id.clone());
                cx.notify();
            }),
        )
        .context_menu(move |menu, _, cx| {
            let open = target.clone();
            let admin = target.clone();
            let reveal = target.clone();
            let link = target.clone();
            let edit = target.clone();
            let favorite = target.clone();
            let remove = target.clone();
            let edit_store = store.clone();
            let favorite_store = store.clone();
            let remove_store = store.clone();
            let pending = store.read(cx).saving;
            let category = store
                .read(cx)
                .catalog
                .categories
                .iter()
                .find(|category| category.tools.iter().any(|tool| tool.id == target.id))
                .map(|category| category.id.clone());
            menu.item(PopupMenuItem::new("打开").icon(IconName::Play).on_click(
                move |_, window, cx| Self::execute(open.clone(), ToolAction::Open, window, cx),
            ))
            .item(
                PopupMenuItem::new("以管理员身份打开")
                    .icon(IconName::ShieldCheck)
                    .disabled(target.exe.is_none())
                    .on_click(move |_, window, cx| {
                        Self::execute(admin.clone(), ToolAction::Administrator, window, cx)
                    }),
            )
            .item(
                PopupMenuItem::new("打开文件所在位置")
                    .icon(IconName::FolderOpen)
                    .disabled(target.exe.is_none())
                    .on_click(move |_, window, cx| {
                        Self::execute(reveal.clone(), ToolAction::Reveal, window, cx)
                    }),
            )
            .item(
                PopupMenuItem::new("打开链接")
                    .icon(IconName::ExternalLink)
                    .disabled(target.link.is_empty())
                    .on_click(move |_, window, cx| {
                        Self::execute(link.clone(), ToolAction::Link, window, cx)
                    }),
            )
            .separator()
            .item(
                PopupMenuItem::new(if target.favorite {
                    "取消收藏"
                } else {
                    "收藏"
                })
                .icon(IconName::Star)
                .disabled(pending)
                .on_click(move |_, window, cx| {
                    let mut next = favorite_store.read(cx).catalog.clone();
                    if let Some(tool) = next
                        .categories
                        .iter_mut()
                        .flat_map(|category| &mut category.tools)
                        .find(|tool| tool.id == favorite.id)
                    {
                        tool.favorite = !tool.favorite;
                    }
                    favorite_store.update(cx, |store, cx| store.save(next, false, window, cx));
                }),
            )
            .item({
                let owner = store.clone();
                let source = target.id.clone();
                let page = page.clone();
                let earlier = earlier.clone();
                PopupMenuItem::new("前移")
                    .icon(IconName::ArrowUp)
                    .disabled(pending || earlier.is_none())
                    .on_click(move |_, window, cx| {
                        if let Some(target) = &earlier {
                            owner.update(cx, |store, cx| {
                                store.reorder_tool(&page, &source, target, window, cx)
                            });
                        }
                    })
            })
            .item({
                let owner = store.clone();
                let source = target.id.clone();
                let page = page.clone();
                let later = later.clone();
                PopupMenuItem::new("后移")
                    .icon(IconName::ArrowDown)
                    .disabled(pending || later.is_none())
                    .on_click(move |_, window, cx| {
                        if let Some(target) = &later {
                            owner.update(cx, |store, cx| {
                                store.reorder_tool(&page, &source, target, window, cx)
                            });
                        }
                    })
            })
            .item(
                PopupMenuItem::new("编辑…")
                    .icon(IconName::Pencil)
                    .disabled(pending)
                    .on_click(move |_, window, cx| {
                        super::editor::open(
                            edit_store.clone(),
                            Some(edit.clone()),
                            category.clone(),
                            window,
                            cx,
                        )
                    }),
            )
            .item(
                PopupMenuItem::new("删除")
                    .icon(IconName::Trash)
                    .disabled(pending)
                    .on_click(move |_, window, cx| {
                        let mut next = remove_store.read(cx).catalog.clone();
                        for category in &mut next.categories {
                            category.tools.retain(|tool| tool.id != remove.id);
                        }
                        remove_store.update(cx, |store, cx| store.save(next, false, window, cx));
                    }),
            )
        });
        div().id(SharedString::from(format!("tool-drag-{}", tool.id))).size_full()
            .child(card)
            .when(!pending, |row| row
                .on_drag(drag, |drag, _, _, cx| cx.new(|_| drag.clone()))
                .can_drop(move |drag, _, _| matches!(drag.downcast_ref::<DragItem>(), Some(DragItem::Tool {page, ..}) if *page == accept_page))
                .on_drop(cx.listener(move |this, drag: &DragItem, window, cx| {
                    if let DragItem::Tool {page, id, ..} = drag && *page == drop_page {
                        this.store.update(cx, |store, cx| store.reorder_tool(page, id, &drop_target, window, cx));
                    }
                })))
            .into_any_element()
    }
}

impl Render for ToolsPage {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let filtered = !self.query.read(cx).value().trim().is_empty();
        let tools = self.visible_tools(cx);
        let count = tools.len();
        let columns = usize::from(self.columns.max(1));
        let row_height = (tool_card::HEIGHT + tool_card::GAP).to_pixels(window.rem_size());
        let body = if count == 0 {
            let heading = if filtered {
                "没有匹配的工具"
            } else if self.key.as_ref() == "home" {
                "暂无收藏"
            } else {
                "暂无工具"
            };
            let empty = ui::empty(IconName::Search, heading, cx).id("tools-empty");
            #[cfg(test)]
            let empty = {
                use gpui_kit::test::TestSupportExt as _;
                empty.test_support()
            };
            empty.into_any_element()
        } else {
            virtual_rows::rows(
                cx.entity(),
                (ElementId::from("tool-grid"), self.key.clone()),
                vec![row_height; count.div_ceil(columns)],
                &self.scroll,
                move |this, row, _, cx| {
                    let start = row * columns;
                    let end = (start + columns).min(tools.len());
                    div()
                        .h(row_height)
                        .pb(tool_card::GAP)
                        .grid()
                        .grid_cols(columns as u16)
                        .gap(tool_card::GAP)
                        .children((start..end).map(|index| {
                            let earlier = index
                                .checked_sub(1)
                                .and_then(|index| tools.get(index))
                                .map(|tool| tool.id.clone());
                            let later = tools.get(index + 1).map(|tool| tool.id.clone());
                            this.card(&tools[index], earlier, later, cx)
                        }))
                        .into_any_element()
                },
            )
        };
        let owner = cx.weak_entity();
        let current_columns = self.columns;
        let new_store = self.store.clone();
        let category = (self.key.as_ref() != "home").then(|| self.key.to_string());
        ui::workspace()
            .id((ElementId::from("tools-workspace"), self.key.clone()))
            .child(ui::workspace_header(
                &self.title,
                self.selected
                    .as_ref()
                    .and_then(|id| {
                        self.store
                            .read(cx)
                            .catalog
                            .categories
                            .iter()
                            .flat_map(|category| &category.tools)
                            .find(|tool| &tool.id == id)
                    })
                    .map(|tool| tool.description.as_str())
                    .unwrap_or(""),
                cx,
            ))
            .child(
                ui::workspace_toolbar().child(
                    Input::new(&self.query)
                        .aria_label(format!("搜索{}", self.title))
                        .with_size(control::STANDARD)
                        .w(layout::SEARCH_WIDTH)
                        .flex_1()
                        .min_w(space::NONE)
                        .prefix(Icon::new(IconName::Search).size(layout::ICON))
                        .cleanable(true),
                ),
            )
            .child(
                ui::workspace_body()
                    .on_children_prepainted(move |bounds, window, cx| {
                        let Some(bounds) = bounds.first() else {
                            return;
                        };
                        let gap = tool_card::GAP.to_pixels(window.rem_size());
                        let min = tool_card::MIN_WIDTH.to_pixels(window.rem_size());
                        let columns = ui::adaptive_columns(bounds.size.width, min, gap) as u16;
                        if columns == current_columns {
                            return;
                        }
                        // Defer the geometry-derived update; never mutate an entity during paint.
                        let owner = owner.clone();
                        window.defer(cx, move |_, cx| {
                            let _ = owner.update(cx, |this, cx| {
                                if this.columns != columns {
                                    this.columns = columns;
                                    cx.notify();
                                }
                            });
                        });
                    })
                    .child(body),
            )
            .child(
                ui::workspace_footer()
                    .child(ui::muted(format!("{count} 个工具"), cx))
                    .when(filtered, |row| row.child(ui::muted("已按关键词筛选", cx))),
            )
            .context_menu(move |menu, _, cx| {
                let store = new_store.clone();
                let category = category.clone();
                menu.item(
                    PopupMenuItem::new("新建…")
                        .icon(IconName::Plus)
                        .disabled(store.read(cx).saving)
                        .on_click(move |_, window, cx| {
                            super::editor::open(store.clone(), None, category.clone(), window, cx)
                        }),
                )
            })
    }
}
