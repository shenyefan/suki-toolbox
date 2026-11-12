use super::{Catalog, ConfigFile};
use crate::ui;
use gpui_kit::*;

/// The sole catalog owner. Publish changes only after persistence succeeds.
pub(crate) struct CatalogStore {
    pub(crate) catalog: Catalog,
    pub(crate) saving: bool,
    task: Option<Task<()>>,
    icons_task: Option<Task<()>>,
    pub(crate) icons:
        std::collections::HashMap<std::path::PathBuf, Option<std::sync::Arc<RenderImage>>>,
    pub(crate) config_file: Result<std::sync::Arc<ConfigFile>, String>,
}

impl CatalogStore {
    pub(crate) fn new(catalog: Catalog) -> Self {
        Self {
            catalog,
            saving: false,
            task: None,
            icons_task: None,
            icons: Default::default(),
            config_file: ConfigFile::discover()
                .map(std::sync::Arc::new)
                .map_err(|error| error.to_string()),
        }
    }

    pub(crate) fn save(
        &mut self,
        next: Catalog,
        close_dialog: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.persist(next, close_dialog, None, window, cx);
    }

    pub(super) fn reorder_category(
        &mut self,
        source: &str,
        target: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.saving {
            return;
        }
        let mut next = self.catalog.clone();
        if next.reorder_category(source, target) {
            self.save(next, false, window, cx);
        }
    }

    pub(super) fn reorder_tool(
        &mut self,
        page: &str,
        source: &str,
        target: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.saving {
            return;
        }
        let mut next = self.catalog.clone();
        if next.reorder_tool(page, source, target) {
            self.save(next, false, window, cx);
        }
    }

    pub(super) fn save_with_icon(
        &mut self,
        next: Catalog,
        upload: Option<(String, std::path::PathBuf)>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.persist(next, true, upload, window, cx);
    }

    pub(crate) fn load_icons(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let paths: std::collections::HashSet<_> = self
            .catalog
            .categories
            .iter()
            .flat_map(|category| &category.tools)
            .filter(|tool| tool.icon.is_empty())
            .filter_map(|tool| tool.exe.clone())
            .filter(|path| {
                path.extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("exe"))
            })
            .collect();
        self.icons.retain(|path, _| paths.contains(path));
        let missing: Vec<_> = paths
            .into_iter()
            .filter(|path| !self.icons.contains_key(path))
            .collect();
        if missing.is_empty() {
            return;
        }
        self.icons_task = Some(cx.spawn_in(window, async move |this, cx| {
            let icons = cx
                .background_spawn(async move {
                    missing
                        .into_iter()
                        .map(|path| {
                            let icon = super::executable_icon::extract(&path);
                            (path, icon)
                        })
                        .collect::<Vec<_>>()
                })
                .await;
            let _ = this.update_in(cx, |this, _, cx| {
                this.icons.extend(icons);
                cx.notify();
            });
        }));
    }

    fn persist(
        &mut self,
        mut next: Catalog,
        close_dialog: bool,
        upload: Option<(String, std::path::PathBuf)>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.saving {
            return;
        }
        self.saving = true;
        cx.notify();
        let file = self.config_file.clone();
        self.task = Some(cx.spawn_in(window, async move |this, cx| {
            let (next, result) = cx
                .background_spawn(async move {
                    let result = file.and_then(|file| {
                        if let Some((id, source)) = upload {
                            let tool = next
                                .categories
                                .iter_mut()
                                .flat_map(|category| &mut category.tools)
                                .find(|tool| tool.id == id)
                                .ok_or("工具已不存在")?;
                            tool.icon = file
                                .import_icon(&source)
                                .map_err(|error| error.to_string())?;
                        }
                        file.prepare(&mut next)
                            .and_then(|()| {
                                next.refresh_home();
                                file.save(&next)
                            })
                            .map_err(|error| error.to_string())
                    });
                    (next, result)
                })
                .await;
            let _ = this.update_in(cx, |this, window, cx| {
                this.saving = false;
                match result {
                    Ok(warning) => {
                        this.catalog = next;
                        this.load_icons(window, cx);
                        if close_dialog {
                            use gpui_kit::component::WindowExt as _;
                            window.close_dialog(cx);
                        }
                        if let Some(warning) = warning {
                            ui::show_error(warning, window, cx);
                        }
                    }
                    Err(error) => ui::show_error(format!("工具配置保存失败：{error}"), window, cx),
                }
                cx.notify();
            });
        }));
    }
}
