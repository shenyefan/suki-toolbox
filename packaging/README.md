# Suki Toolbox

Windows 工具箱 · Copyright © Moenya

解压到可写目录后运行 `suki-toolbox.exe`。程序图标和界面资源已嵌入 EXE，无须单独下载。

- `win-x64` 适用于 Intel / AMD 64 位 Windows，`win-arm64` 适用于 ARM64 Windows。
- 工具软件需自行放入 `tools/<分类>/<软件名>/`；压缩包不包含第三方工具。
- 首次运行没有 `config.json` 时会扫描工具目录，分类少于 15 个时自动生成配置。支持 EXE、BAT、CMD、PS1、VBS、JS。
- 配置、收藏及外观保存在程序旁的 `config.json`，上传的图标保存在 `icons/`。请选择可写目录。
- 首次运行或没有收藏时，默认显示系统信息。
- 更新时退出程序，用新版 EXE 替换旧版，保留原有 `config.json`、`tools/` 和 `icons/`。当前未接入应用内更新。

项目：https://github.com/shenyefan/suki-toolbox
