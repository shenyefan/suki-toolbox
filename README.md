> **自用 Vibe Coding 项目，请勿传播**

<p align="center">
  <img src="assets/logo.png" width="112" alt="Suki Toolbox 图标" />
</p>

<h1 align="center">Suki Toolbox</h1>

<p align="center">基于 Rust 与 GPUI Kit 的 Windows 桌面工具箱</p>

<p align="center">
  <a href="../../releases">下载</a> ·
  <a href="#工具目录与配置">工具配置</a> ·
  <a href="#构建与发布">构建与发布</a> ·
  <a href="#开源致谢">开源致谢</a>
</p>

## 简介

Suki Toolbox 使用 GPUI Kit 构建原生桌面界面，集中管理本地程序、脚本和网页入口，提供工具分类、收藏、搜索和系统信息概览。

项目目前仅面向 Windows。工具软件由用户自行添加，发布包不附带第三方工具；工具下载、应用内更新、专项硬件检测和进程持续状态跟踪尚未接入。

## 主要功能

- **工具管理**：新建、编辑、删除工具，维护分类，搜索名称与说明。
- **工具启动**：打开程序、运行脚本、以管理员身份打开、定位文件和打开网页；支持 EXE、BAT、CMD、PS1、VBS、JS。
- **收藏与排序**：收藏常用工具，拖动调整工具与分类顺序，收藏页单独保存排列。
- **图标管理**：导入本地图片、引用图标路径、后台提取 EXE 图标，分类图标支持搜索选择。
- **系统信息**：查看型号、Windows 版本、运行时间，以及主板、处理器、内存、显卡、显示器、硬盘、电池、声卡和网卡概览，支持刷新与复制。
- **外观与操作**：深色、浅色、跟随系统，Acrylic / 纯色背景，以及单击 / 双击打开方式。

系统信息通过 Windows MI、DXGI 等系统接口读取。未读到的字段显示未知，无电池时省略电池行。工具打开成功表示系统接受了启动请求，不代表程序持续监控工具进程。

## 下载与使用

在 [GitHub Releases](../../releases) 中选择对应架构的便携包。仓库中的发布工作流配置了以下产物，是否已有可用版本以 Releases 页面为准：

| 平台 | 架构 | 文件形式 |
| --- | --- | --- |
| Windows | Intel / AMD x64 | `SukiToolbox-<版本>-win-x64-Portable.zip` |
| Windows | ARM64 | `SukiToolbox-<版本>-win-arm64-Portable.zip` |

解压到可写目录，运行 `suki-toolbox.exe`。首次运行没有配置或没有收藏时，默认进入系统信息；已有配置且存在收藏时，默认进入收藏。

工具、分类和收藏可在界面中维护，也可先按下方目录结构放入工具，再首次运行生成配置。配置和上传的图标均保存在程序旁，移动工具箱时应一起保留。

更新时退出程序，用新版 EXE 替换旧版，保留 `config.json`、`tools/` 和 `icons/`。当前通过手动替换更新，不包含安装程序或应用内更新。

## 工具目录与配置

程序优先读取自身目录下的 `config.json`，只有文件缺失时才扫描 `tools`：

```text
Suki Toolbox/
  suki-toolbox.exe
  config.json
  icons/
  tools/
    硬件检测/
      CPU-Z/
        cpuz.exe
    脚本/
      清理缓存/
        清理缓存.ps1
```

`tools` 一级分类目录少于 15 个时自动生成配置，达到或超过 15 个时提示并停止自动生成。分类名和工具名来自目录名称，保留空分类；软件目录会继续检查两层子目录，优先选择同名、层级较浅的启动文件，再比较文件大小。

自动生成的工具说明为空，默认收藏每个非空分类的首个工具。没有 `tools` 目录时生成空配置；已有配置不会重新扫描或覆盖，格式错误会明确提示。

配置格式如下，设置和工具目录共用一个文件：

```json
{
  "settings": {
    "theme": "dark",
    "backdrop": "acrylic",
    "open_mode": "double"
  },
  "favorite_order": ["cpu-z"],
  "categories": [
    { "name": "硬件检测", "path": "tools/硬件检测", "icon": "Cpu" }
  ],
  "tools": [
    {
      "id": "cpu-z",
      "title": "CPU-Z",
      "category": "硬件检测",
      "path": "tools/硬件检测/CPU-Z/cpuz.exe",
      "arguments": "",
      "link": "https://www.cpuid.com/softwares/cpu-z.html",
      "icon": "",
      "description": "",
      "favorite": true
    }
  ]
}
```

工具 ID 唯一且非空，分类对应分类名称，文件路径和链接至少填写一项。界面新建工具时自动生成 UUID，改名和移动分类保留 ID；收藏排列独立保存在 `favorite_order`。删除工具或分类只移除配置条目，保留磁盘文件。

| 设置 | 可选值 | 默认值 |
| --- | --- | --- |
| `theme` | `dark`、`light`、`system` | `dark` |
| `backdrop` | `acrylic`、`solid` | `acrylic` |
| `open_mode` | `single`、`double` | `double` |

程序目录内的文件和目录保存为相对路径，目录外保存为绝对路径。相对路径以 EXE 所在目录为基准，与启动时的工作目录无关。上传图标在保存时复制到 `icons/`，手工填写图标路径只保存引用。

`arguments` 保留用户填写的参数，启动工作目录为工具所在目录。脚本使用对应解释器，BAT / CMD 参数遵循 cmd 语法，PS1 使用 Windows PowerShell 并遵循系统执行策略。网页链接需使用 `http://` 或 `https://`。

## 构建与发布

准备支持 Rust 2024 Edition 的 Rust 工具链，以及 Visual Studio Build Tools 的 C++ 桌面开发组件和 Windows SDK。Windows 使用 MSVC 工具链，SDK 需提供资源编译器与 `fxc.exe`。

在已初始化 MSVC 环境的 PowerShell 中执行：

```powershell
# 开发运行
cargo run --locked

# 构建发布版本
cargo build --locked --release --bin suki-toolbox
```

开发运行时，配置目录为 `target/debug/`；发布构建默认位于 `target/release/`。应用标识、界面图形及 Windows 文件属性在构建时嵌入程序。

[GitHub Actions](.github/workflows/release.yml) 支持推送 `v*` 标签和手动运行。推送标签时，标签须与 Cargo 包版本一致，例如 `Cargo.toml` 为 `0.1.0`，标签就应为 `v0.1.0`；目前只接受稳定的 `x.y.z` 版本。

工作流执行格式检查、严格 Clippy、Rust 测试和发布脚本测试，使用匹配架构的 MSVC 工具链分别构建 Windows x64 / ARM64，并静态链接 C 运行库。每个架构生成便携 ZIP 和记录版本、架构、源码提交的 `build.*.json`，Actions artifacts 保留 14 天。

所有检查与构建成功后，标签任务汇总两个架构，生成 `SHA256SUMS` 和带下载链接、使用说明及自动变更记录的 **Release 草稿**，供检查后手动发布。重跑可更新同一标签的草稿，已经正式发布的版本不会被覆盖。工作流使用仓库自带的 `GITHUB_TOKEN`，无须额外配置个人令牌。

手动选择分支运行时只上传构建产物；选择标签运行时也会创建或更新对应的 Release 草稿。首次远端构建和 ARM64 实机运行仍需验证。

本地打包使用 Python 3.11+，例如：

```powershell
$env:RUSTFLAGS = "-C target-feature=+crt-static"
cargo build --locked --release --target x86_64-pc-windows-msvc --bin suki-toolbox
python packaging/pack.py 0.1.0 x86_64-pc-windows-msvc target/packages
```

版本参数应与 Cargo 版本一致。ARM64 需改用 `aarch64-pc-windows-msvc` 并准备对应交叉编译环境。便携包只包含 EXE 和[使用说明](packaging/README.md)，不打包开发者本地配置或工具目录。

## 项目约定与检查

[AGENTS.md](AGENTS.md) 定义架构、状态所有权、系统边界和验证责任；[DESIGN.md](DESIGN.md) 定义视觉与交互规范。

```powershell
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
python -m unittest discover -s packaging -p "test_*.py"
```

自动测试与真实窗口验收分别记录；编译通过不代表所有显示尺寸、主题和本机硬件均已完成验证。

## 开源致谢

感谢以下开源项目及其贡献者。Suki Toolbox 使用了这些项目的框架、组件、图标或实现参考：

| 项目 | 用途 |
| --- | --- |
| [Rust](https://github.com/rust-lang/rust) | 开发语言与工具链 |
| [GPUI Kit](https://github.com/longbridge/gpui-kit) | 桌面框架与组件整合 |
| [GPUI / Zed](https://github.com/zed-industries/zed) | 原生界面与渲染基础 |
| [GPUI Component](https://github.com/longbridge/gpui-component) | 界面组件 |
| [Lucide](https://github.com/lucide-icons/lucide) | 组件图标 |
| [windows-rs](https://github.com/microsoft/windows-rs) | Windows 系统接口 |
| [TubaTools](https://github.com/luolangaga/tubatools) | 系统信息读取与展示参考 |

Rust 依赖的版本和来源以 [Cargo.lock](Cargo.lock) 为准，各第三方项目遵循其各自许可。

Copyright © Moenya
