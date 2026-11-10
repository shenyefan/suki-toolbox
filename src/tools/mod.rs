pub(crate) mod categories;
mod editor;
mod executable_icon;
mod reorder;
pub(crate) mod store;
pub(crate) mod view;
mod windows;

use gpui_kit::assets::IconName;
use serde::{Deserialize, Serialize};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Tool {
    /// Opaque identity retained across renames and category moves.
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) exe: Option<PathBuf>,
    pub(crate) arguments: String,
    pub(crate) link: String,
    pub(crate) icon: String,
    pub(crate) favorite: bool,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Category {
    pub(crate) id: String,
    pub(crate) label: String,
    pub(crate) detail: String,
    pub(crate) path: String,
    pub(crate) icon: String,
    pub(crate) tools: Vec<Tool>,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Catalog {
    pub(crate) categories: Vec<Category>,
    pub(crate) home: Vec<Tool>,
    pub(crate) favorite_order: Vec<String>,
}

impl Catalog {
    pub(crate) fn refresh_home(&mut self) {
        self.home = self
            .categories
            .iter()
            .flat_map(|category| &category.tools)
            .filter(|tool| tool.favorite)
            .cloned()
            .collect();
        let ranks: std::collections::HashMap<_, _> = self
            .favorite_order
            .iter()
            .enumerate()
            .map(|(index, id)| (id.as_str(), index))
            .collect();
        self.home
            .sort_by_key(|tool| ranks.get(tool.id.as_str()).copied().unwrap_or(usize::MAX));
        self.favorite_order = self.home.iter().map(|tool| tool.id.clone()).collect();
    }

    pub(crate) fn reorder_category(&mut self, source: &str, target: &str) -> bool {
        let positions = (
            self.categories.iter().position(|item| item.id == source),
            self.categories.iter().position(|item| item.id == target),
        );
        if let (Some(from), Some(to)) = positions
            && from != to
        {
            let item = self.categories.remove(from);
            self.categories.insert(to, item);
            return true;
        }
        false
    }

    pub(crate) fn reorder_tool(&mut self, page: &str, source: &str, target: &str) -> bool {
        if page == "home" {
            self.refresh_home();
            let positions = (
                self.favorite_order.iter().position(|id| id == source),
                self.favorite_order.iter().position(|id| id == target),
            );
            if let (Some(from), Some(to)) = positions
                && from != to
            {
                let id = self.favorite_order.remove(from);
                self.favorite_order.insert(to, id);
                self.refresh_home();
                return true;
            }
        } else if let Some(category) = self.categories.iter_mut().find(|item| item.id == page) {
            let positions = (
                category.tools.iter().position(|tool| tool.id == source),
                category.tools.iter().position(|tool| tool.id == target),
            );
            if let (Some(from), Some(to)) = positions
                && from != to
            {
                let tool = category.tools.remove(from);
                category.tools.insert(to, tool);
                return true;
            }
        }
        false
    }

    pub(crate) fn category(&self, id: &str) -> Option<&Category> {
        self.categories.iter().find(|category| category.id == id)
    }
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ConfigRoot {
    #[serde(default)]
    settings: ConfigSettings,
    #[serde(default)]
    favorite_order: Vec<String>,
    categories: Vec<ConfigCategory>,
    tools: Vec<ConfigTool>,
}

#[derive(Deserialize, Serialize, Clone, Copy)]
#[serde(default)]
struct ConfigSettings {
    theme: crate::settings::ThemeChoice,
    backdrop: crate::settings::Backdrop,
    open_mode: crate::settings::OpenMode,
}

impl Default for ConfigSettings {
    fn default() -> Self {
        Self {
            theme: crate::settings::ThemeChoice::Dark,
            backdrop: crate::settings::Backdrop::Acrylic,
            open_mode: crate::settings::OpenMode::Double,
        }
    }
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ConfigCategory {
    name: String,
    #[serde(default)]
    path: String,
    #[serde(default)]
    icon: String,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ConfigTool {
    id: String,
    title: String,
    category: String,
    #[serde(default)]
    path: String,
    #[serde(default)]
    arguments: String,
    #[serde(default)]
    link: String,
    #[serde(default)]
    icon: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    favorite: bool,
}

/// The catalog configuration adjacent to the executable.
pub(crate) struct ConfigFile {
    directory: PathBuf,
    path: PathBuf,
}

impl ConfigFile {
    pub(crate) fn discover() -> io::Result<Self> {
        let exe = std::env::current_exe()?;
        let directory = exe
            .parent()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "无法定位程序目录"))?;
        Ok(Self::in_directory(directory))
    }

    pub(crate) fn in_directory(directory: &Path) -> Self {
        Self {
            directory: directory.to_path_buf(),
            path: directory.join("config.json"),
        }
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    /// Generate without replacing an existing configuration, including one created
    /// after the caller checked `exists`.
    pub(crate) fn save_new(&self, catalog: &Catalog) -> io::Result<()> {
        self.write(catalog, true)
    }

    pub(crate) fn save(&self, catalog: &Catalog) -> io::Result<()> {
        self.write(catalog, false)
    }

    fn write(&self, catalog: &Catalog, new: bool) -> io::Result<()> {
        let mut catalog = catalog.clone();
        self.prepare(&mut catalog)?;
        catalog.refresh_home();
        let config = ConfigRoot {
            settings: read_settings(&self.path).unwrap_or_default(),
            favorite_order: catalog.favorite_order.clone(),
            categories: catalog
                .categories
                .iter()
                .map(|category| ConfigCategory {
                    name: category.label.clone(),
                    path: category.path.clone(),
                    icon: category.icon.clone(),
                })
                .collect(),
            tools: catalog
                .categories
                .iter()
                .flat_map(|category| {
                    category.tools.iter().map(|tool| ConfigTool {
                        id: tool.id.clone(),
                        title: tool.name.clone(),
                        category: category.label.clone(),
                        path: tool
                            .exe
                            .as_ref()
                            .map(|path| config_path(&self.directory, path))
                            .unwrap_or_default(),
                        arguments: tool.arguments.clone(),
                        link: tool.link.clone(),
                        icon: tool.icon.clone(),
                        description: tool.description.clone(),
                        favorite: tool.favorite,
                    })
                })
                .collect(),
        };
        let bytes = serde_json::to_vec_pretty(&config)?;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(new)
            .create(!new)
            .truncate(!new)
            .open(self.path())?;
        file.write_all(&bytes)
    }

    /// Normalize stored paths and validate component-library icon names.
    fn prepare(&self, catalog: &mut Catalog) -> io::Result<()> {
        for category in &mut catalog.categories {
            category.path = config_path(&self.directory, Path::new(&category.path));
            if !category.icon.is_empty() && library_icon(&category.icon).is_none() {
                return Err(io::Error::other(format!(
                    "未知组件库图标：{}",
                    category.icon
                )));
            }
            for tool in &mut category.tools {
                tool.icon = config_path(&self.directory, Path::new(&tool.icon));
            }
        }
        Ok(())
    }

    fn import_icon(&self, source: &Path) -> io::Result<String> {
        let extension = source
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("png");
        let relative = format!("icons/{}.{}", uuid::Uuid::new_v4(), extension);
        std::fs::create_dir_all(self.directory.join("icons"))?;
        std::fs::copy(source, self.directory.join(&relative))?;
        Ok(relative)
    }

    fn generate(&self) -> (Catalog, Option<String>) {
        let root = self.directory.join("tools");
        let entries = match std::fs::read_dir(&root) {
            Ok(entries) => entries,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                let catalog = Catalog::default();
                return match self.save_new(&catalog) {
                    Ok(()) => (catalog, None),
                    Err(error) => (catalog, Some(format!("无法生成 config.json：{error}"))),
                };
            }
            Err(error) => {
                return (
                    Catalog::default(),
                    Some(format!("无法读取工具目录：{error}")),
                );
            }
        };
        let count = entries
            .filter_map(Result::ok)
            .filter(|entry| entry.path().is_dir())
            .count();
        if count >= 15 {
            return (
                Catalog::default(),
                Some(format!(
                    "tools 中有 {count} 个分类目录，达到 15 个，未自动生成 config.json"
                )),
            );
        }
        let catalog = scan(&root);
        match self.save_new(&catalog) {
            Ok(()) => (catalog, None),
            Err(error) => (catalog, Some(format!("无法生成 config.json：{error}"))),
        }
    }

    fn load(&self) -> (Catalog, Option<String>) {
        match std::fs::read_to_string(self.path()) {
            Ok(raw) => match serde_json::from_str::<ConfigRoot>(&raw) {
                Ok(config) => match from_config(&self.directory, config) {
                    Ok(catalog) => (catalog, None),
                    Err(error) => (
                        Catalog::default(),
                        Some(format!("config.json 格式错误（{error}）")),
                    ),
                },
                Err(error) => (
                    Catalog::default(),
                    Some(format!("config.json 格式错误（{error}）")),
                ),
            },
            Err(error) if error.kind() == io::ErrorKind::NotFound => self.generate(),
            Err(error) => (
                Catalog::default(),
                Some(format!("config.json 无法读取（{error}）")),
            ),
        }
    }
}

fn read_settings(path: &Path) -> Option<ConfigSettings> {
    let raw = std::fs::read_to_string(path).ok()?;
    serde_json::from_str::<ConfigRoot>(&raw)
        .ok()
        .map(|root| root.settings)
}

/// Only a missing configuration permits scanning the tools directory.
pub(crate) fn load() -> (Catalog, Option<String>, bool) {
    match ConfigFile::discover() {
        Ok(file) => {
            let missing = !file.path().exists();
            let (catalog, error) = file.load();
            (catalog, error, missing)
        }
        Err(error) => (
            Catalog::default(),
            Some(format!("无法定位程序目录（{error}）")),
            true,
        ),
    }
}

fn from_config(exe_dir: &Path, config: ConfigRoot) -> Result<Catalog, String> {
    let mut catalog = Catalog {
        favorite_order: config.favorite_order,
        ..Default::default()
    };
    for category in config.categories {
        if category.name.trim().is_empty()
            || catalog
                .categories
                .iter()
                .any(|item| item.label == category.name)
        {
            return Err("分类名称不能为空或重复".into());
        }
        let category_path = config_path(exe_dir, Path::new(&category.path));
        if !category.icon.is_empty() && library_icon(&category.icon).is_none() {
            return Err(format!("未知组件库图标：{}", category.icon));
        }
        catalog.categories.push(Category {
            id: category.name.clone(),
            label: category.name,
            path: category_path,
            icon: category.icon,
            ..Default::default()
        });
    }
    let mut ids = std::collections::HashSet::new();
    for tool in config.tools {
        if tool.id.is_empty() || !ids.insert(tool.id.clone()) || tool.title.trim().is_empty() {
            return Err("工具 ID 必须唯一且非空，标题不能为空".into());
        }
        let category = catalog
            .categories
            .iter_mut()
            .find(|category| category.label == tool.category)
            .ok_or_else(|| format!("工具“{}”的分类不存在：{}", tool.title, tool.category))?;
        if tool.path.trim().is_empty() && tool.link.trim().is_empty() {
            return Err(format!("工具“{}”的路径和链接不能同时为空", tool.title));
        }
        category.tools.push(Tool {
            id: tool.id,
            name: tool.title,
            description: tool.description,
            exe: (!tool.path.is_empty()).then(|| resolve(exe_dir, &tool.path)),
            arguments: tool.arguments,
            link: tool.link,
            icon: tool.icon,
            favorite: tool.favorite,
        });
    }
    catalog.refresh_home();
    Ok(catalog)
}

/// Scan each tool folder for a Windows executable or supported script.
fn scan(root: &Path) -> Catalog {
    let mut categories = Vec::new();
    let Ok(category_entries) = std::fs::read_dir(root) else {
        return Catalog::default();
    };
    let mut category_dirs: Vec<_> = category_entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();
    category_dirs.sort();
    for category_dir in category_dirs {
        let label = file_name(&category_dir);
        let mut tools = Vec::new();
        if let Ok(tool_entries) = std::fs::read_dir(&category_dir) {
            let mut tool_dirs: Vec<_> = tool_entries
                .flatten()
                .map(|entry| entry.path())
                .filter(|path| path.is_dir())
                .collect();
            tool_dirs.sort();
            for tool_dir in tool_dirs {
                let name = file_name(&tool_dir);
                if let Some(exe) = find_launch_file(&tool_dir, &name) {
                    tools.push(Tool {
                        id: format!("{label}/{name}"),
                        description: String::new(),
                        name,
                        exe: Some(exe),
                        favorite: tools.is_empty(),
                        ..Default::default()
                    });
                }
            }
        }
        categories.push(Category {
            id: label.clone(),
            detail: format!("扫描自 {}", root.display()),
            path: category_dir
                .strip_prefix(root.parent().unwrap_or(root))
                .unwrap_or(&category_dir)
                .to_string_lossy()
                .into_owned(),
            icon: String::new(),
            label,
            tools,
        });
    }
    let home = categories
        .iter()
        .filter_map(|category| category.tools.first())
        .cloned()
        .collect();
    Catalog {
        categories,
        home,
        ..Default::default()
    }
}

/// Pick one launch file per tool folder: an exact name match at the shallowest depth
/// wins, then the larger one.
fn find_launch_file(tool_dir: &Path, tool_name: &str) -> Option<PathBuf> {
    let mut candidates: Vec<(bool, usize, u64, PathBuf)> = Vec::new();
    let mut queue = vec![(tool_dir.to_path_buf(), 0usize)];
    while let Some((dir, depth)) = queue.pop() {
        if depth > 2 {
            continue;
        }
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                queue.push((path, depth + 1));
            } else if path.extension().is_some_and(|ext| {
                ["exe", "bat", "cmd", "ps1", "vbs", "js"]
                    .iter()
                    .any(|allowed| ext.eq_ignore_ascii_case(allowed))
            }) {
                let name_match = path
                    .file_stem()
                    .is_some_and(|stem| stem.eq_ignore_ascii_case(tool_name));
                let size = entry.metadata().map(|meta| meta.len()).unwrap_or(0);
                candidates.push((name_match, depth, size, path));
            }
        }
    }
    candidates.sort_by(|a, b| {
        b.0.cmp(&a.0)
            .then(a.1.cmp(&b.1))
            .then(b.2.cmp(&a.2))
            .then(a.3.cmp(&b.3))
    });
    candidates.into_iter().next().map(|(_, _, _, path)| path)
}

/// Keep application-owned files portable and preserve external file locations.
fn config_path(base: &Path, path: &Path) -> String {
    if path.as_os_str().is_empty() {
        return String::new();
    }
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        base.join(path)
    };
    let mut normalized = PathBuf::new();
    for component in absolute.components() {
        match component {
            std::path::Component::ParentDir => {
                normalized.pop();
            }
            std::path::Component::CurDir => {}
            _ => normalized.push(component.as_os_str()),
        }
    }
    match normalized.strip_prefix(base) {
        Ok(relative) => relative.to_string_lossy().replace('\\', "/"),
        Err(_) => normalized.to_string_lossy().into_owned(),
    }
}

pub(crate) fn library_icon(name: &str) -> Option<IconName> {
    IconName::ALL
        .iter()
        .copied()
        .find(|icon| format!("{icon:?}") == name)
}

fn resolve(base: &Path, relative: &str) -> PathBuf {
    let path = PathBuf::from(relative);
    if path.is_absolute() {
        path
    } else {
        base.join(path)
    }
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// Every catalog entry shares one icon; the directory drives everything else.
pub(crate) fn entry_icon() -> IconName {
    IconName::Box
}

#[cfg(test)]
mod tests {
    use super::{ConfigFile, find_launch_file};
    use std::fs;
    use std::path::{Path, PathBuf};

    fn write(path: &Path, bytes: &[u8]) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }

    #[test]
    fn scan_picks_the_exe_matching_the_tool_folder() {
        let root = tempfile::tempdir().unwrap();
        let tool_dir = root.path().join("硬件检测/CPU-Z");
        write(&tool_dir.join("launcher.exe"), b"x");
        write(&tool_dir.join("cpuz.exe"), b"xx");
        let picked = find_launch_file(&tool_dir, "CPU-Z").unwrap();
        assert!(picked.ends_with("cpuz.exe"), "picked {}", picked.display());
    }

    #[test]
    fn nested_exes_prefer_shallow_name_matches() {
        let root = tempfile::tempdir().unwrap();
        let tool_dir = root.path().join("AIDA64");
        write(&tool_dir.join("bin/x64/aida64.exe"), b"xxx");
        write(&tool_dir.join("aida64.exe"), b"xx");
        let picked = find_launch_file(&tool_dir, "AIDA64").unwrap();
        assert!(picked.ends_with("aida64.exe"));
        assert_eq!(
            picked.components().count(),
            tool_dir.components().count() + 1
        );
    }

    #[test]
    fn generated_config_discovers_scripts_and_keeps_relative_paths() {
        let root = tempfile::tempdir().unwrap();
        for extension in ["bat", "CMD", "ps1", "vbs", "js"] {
            write(
                &root
                    .path()
                    .join(format!("tools/脚本/{extension}/启动.{extension}")),
                b"script",
            );
        }
        write(&root.path().join("tools/脚本/说明/readme.txt"), b"text");
        let file = ConfigFile::in_directory(root.path());
        let (catalog, error) = file.load();
        assert!(error.is_none(), "{error:?}");
        assert_eq!(catalog.categories[0].tools.len(), 5);
        let raw: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(file.path()).unwrap()).unwrap();
        for tool in raw["tools"].as_array().unwrap() {
            assert!(Path::new(tool["path"].as_str().unwrap()).is_relative());
        }
        let (loaded, error) = file.load();
        assert!(error.is_none());
        assert_eq!(loaded.categories[0].tools, catalog.categories[0].tools);
    }

    #[test]
    fn edited_catalog_round_trips_every_field_and_empty_categories() {
        let root = tempfile::tempdir().unwrap();
        write(&root.path().join("tools/CPU/Tool/run.exe"), b"exe");
        fs::create_dir_all(root.path().join("tools/Empty")).unwrap();
        let file = ConfigFile::in_directory(root.path());
        let (mut catalog, error) = file.load();
        assert!(error.is_none());
        let tool = &mut catalog.categories[0].tools[0];
        tool.name = "Renamed".into();
        tool.arguments = "--flag \"two words\"".into();
        tool.link = "https://example.com".into();
        tool.icon = "icons/tool.png".into();
        tool.description = "说明".into();
        tool.favorite = true;
        let expected = tool.clone();
        file.save(&catalog).unwrap();
        assert!(file.save_new(&catalog).is_err());
        let (mut loaded, error) = file.load();
        assert!(error.is_none());
        assert_eq!(loaded.categories[0].tools[0], expected);
        assert!(loaded.categories[1].tools.is_empty());
        loaded.categories[0].tools.clear();
        loaded.refresh_home();
        file.save(&loaded).unwrap();
        let (deleted, error) = file.load();
        assert!(error.is_none());
        assert!(deleted.home.is_empty());
        assert!(deleted.categories[0].tools.is_empty());
        assert!(root.path().join("tools/CPU/Tool/run.exe").exists());
    }

    #[test]
    fn saved_paths_and_uploaded_icons_survive_moving_the_application() {
        let root = tempfile::tempdir().unwrap();
        let original = root.path().join("original");
        write(&original.join("tools/CPU/Tool/run.exe"), b"exe");
        let image = root.path().join("selected.png");
        write(&image, b"image upload bytes");
        let file = ConfigFile::in_directory(&original);
        let (mut catalog, error) = file.load();
        assert!(error.is_none());
        catalog.categories[0].path = original.join("tools/CPU").display().to_string();
        catalog.categories[0].icon = "Cpu".into();
        catalog.categories[0].tools[0].icon = file.import_icon(&image).unwrap();
        file.prepare(&mut catalog).unwrap();
        let icon = catalog.categories[0].tools[0].icon.clone();
        assert!(icon.starts_with("icons/"));
        assert_eq!(
            fs::read(original.join(&icon)).unwrap(),
            b"image upload bytes"
        );
        file.save(&catalog).unwrap();
        let saved: serde_json::Value =
            serde_json::from_slice(&fs::read(file.path()).unwrap()).unwrap();
        assert!(!Path::new(saved["categories"][0]["path"].as_str().unwrap()).is_absolute());
        assert!(!Path::new(saved["tools"][0]["path"].as_str().unwrap()).is_absolute());
        assert!(!Path::new(saved["tools"][0]["icon"].as_str().unwrap()).is_absolute());
        let moved = root.path().join("moved");
        fs::rename(&original, &moved).unwrap();
        let (loaded, error) = ConfigFile::in_directory(&moved).load();
        assert!(error.is_none(), "{error:?}");
        assert!(
            loaded.categories[0].tools[0]
                .exe
                .as_ref()
                .unwrap()
                .is_file()
        );
        assert!(moved.join(&loaded.categories[0].tools[0].icon).is_file());
    }

    #[test]
    fn internal_paths_are_relative_and_external_paths_remain_absolute() {
        let root = tempfile::tempdir().unwrap();
        let directory = root.path().join("app");
        let internal = directory.join("tools/run.exe");
        let external = root.path().join("external/run.exe");
        assert_eq!(
            PathBuf::from(super::config_path(&directory, &internal)),
            PathBuf::from("tools/run.exe")
        );
        assert_eq!(
            PathBuf::from(super::config_path(&directory, &external)),
            external
        );
        assert_eq!(
            PathBuf::from(super::config_path(
                &directory,
                Path::new("../external/run.exe")
            )),
            external
        );
        assert_eq!(
            super::config_path(&directory, Path::new(r"Z:\outside\run.exe")),
            r"Z:\outside\run.exe"
        );
        let file = ConfigFile::in_directory(&directory);
        fs::create_dir_all(&directory).unwrap();
        let catalog = super::Catalog {
            categories: vec![super::Category {
                label: "外部工具".into(),
                path: external.parent().unwrap().display().to_string(),
                tools: vec![super::Tool {
                    id: "external".into(),
                    name: "工具".into(),
                    exe: Some(external.clone()),
                    icon: external
                        .parent()
                        .unwrap()
                        .join("custom.png")
                        .display()
                        .to_string(),
                    ..Default::default()
                }],
                ..Default::default()
            }],
            ..Default::default()
        };
        file.save(&catalog).unwrap();
        let (loaded, error) = file.load();
        assert!(error.is_none());
        assert_eq!(loaded.categories[0].tools[0].exe.as_ref(), Some(&external));
        assert!(Path::new(&loaded.categories[0].path).is_absolute());
        assert!(Path::new(&loaded.categories[0].tools[0].icon).is_absolute());
        assert!(
            !directory.join("icons").exists(),
            "typed icon paths must not import files"
        );
    }

    #[test]
    fn category_icons_are_component_library_names() {
        assert_eq!(
            super::library_icon("Cpu"),
            Some(gpui_kit::assets::IconName::Cpu)
        );
        assert_eq!(
            super::library_icon("Folder"),
            Some(gpui_kit::assets::IconName::Folder)
        );
        assert!(super::library_icon("icons/image.png").is_none());
        assert!(super::library_icon("UnknownIcon").is_none());
    }

    #[test]
    fn malformed_configuration_never_falls_back_to_scanning() {
        let root = tempfile::tempdir().unwrap();
        write(&root.path().join("tools/CPU/Tool/run.exe"), b"exe");
        let file = ConfigFile::in_directory(root.path());
        for raw in [
            r#"{"categories":[],"tools":[{"id":"a","title":"A","category":"missing"}]}"#,
            r#"{"categories":[{"label":"legacy"}]}"#,
            "{broken",
        ] {
            write(file.path(), raw.as_bytes());
            let (catalog, error) = file.load();
            assert!(error.is_some());
            assert!(catalog.categories.is_empty());
        }
    }
    #[test]
    fn auto_generation_stops_at_fifteen_category_directories() {
        for count in [14, 15, 16] {
            let root = tempfile::tempdir().unwrap();
            for ix in 0..count {
                fs::create_dir_all(root.path().join(format!("tools/Category-{ix}"))).unwrap();
            }
            let file = ConfigFile::in_directory(root.path());
            let (catalog, error) = file.load();
            assert_eq!(file.path().exists(), count < 15);
            assert_eq!(error.is_some(), count >= 15);
            assert_eq!(catalog.categories.len(), if count < 15 { count } else { 0 });
        }
    }

    #[test]
    fn generated_tools_have_no_description_and_existing_config_wins() {
        let root = tempfile::tempdir().unwrap();
        write(&root.path().join("tools/CPU/Tool/run.exe"), b"exe");
        let file = ConfigFile::in_directory(root.path());
        let (catalog, error) = file.load();
        assert!(error.is_none());
        assert!(file.path().exists());
        assert!(catalog.categories[0].tools[0].description.is_empty());
        for ix in 0..16 {
            fs::create_dir_all(root.path().join(format!("tools/New-{ix}"))).unwrap();
        }
        let (loaded, error) = file.load();
        assert!(error.is_none());
        assert_eq!(loaded.categories.len(), 1);
    }
}
