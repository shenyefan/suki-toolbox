"""Collect both Windows packages and create/update the tag's draft release.

Usage: release.py VERSION TAG PACKAGES_DIR (GH_TOKEN and GH_REPO required)
"""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys


def collect(packages, output, version):
    output.mkdir(parents=True, exist_ok=True)
    names = []
    for channel in ("win-x64", "win-arm64"):
        for name in (f"SukiToolbox-{version}-{channel}-Portable.zip", f"build.{channel}.json"):
            source = packages / f"packages-{channel}" / name
            if not source.is_file() or source.stat().st_size == 0:
                raise ValueError(f"Missing release artifact: {source}")
            shutil.copy2(source, output / name)
            names.append(name)
    checksums = "".join(
        f"{hashlib.sha256((output / name).read_bytes()).hexdigest()}  {name}\n"
        for name in sorted(names)
    )
    (output / "SHA256SUMS").write_text(checksums, encoding="utf-8")
    return names + ["SHA256SUMS"]


def main():
    version, tag, packages = sys.argv[1:]
    output = Path("Releases")
    names = collect(Path(packages), output, version)
    repository = os.environ["GH_REPO"]
    base = f"https://github.com/{repository}/releases/download/{tag}"
    notes = ["## 下载\n", "| 平台 | 便携包 |", "| --- | --- |"]
    for channel, label in (("win-x64", "Windows x64"), ("win-arm64", "Windows ARM64")):
        name = f"SukiToolbox-{version}-{channel}-Portable.zip"
        notes.append(f"| {label} | [{name}]({base}/{name}) |")
    notes.extend([
        "\n## 使用说明\n",
        "- 解压到可写目录运行，无须安装；工具软件需自行添加。",
        "- 配置位于程序旁的 `config.json`，上传图标位于 `icons/`。",
        "- 更新时退出程序并替换 EXE，保留 `config.json`、`tools/` 和 `icons/`。",
        "- `SHA256SUMS` 提供下载文件校验；`build.*.json` 记录版本、架构和源码提交。",
        "\n## 变更记录\n",
    ])
    changes = subprocess.check_output(
        ["gh", "api", f"repos/{repository}/releases/generate-notes", "-f", f"tag_name={tag}", "--jq", ".body"],
        text=True, encoding="utf-8",
    )
    notes.append(changes)
    notes_file = Path("release-notes.md")
    notes_file.write_text("\n".join(notes) + "\n", encoding="utf-8")
    existing = subprocess.run(
        ["gh", "release", "view", tag, "--json", "isDraft,assets"],
        capture_output=True, text=True, encoding="utf-8",
    )
    if existing.returncode == 0:
        release = json.loads(existing.stdout)
        if not release["isDraft"]:
            raise ValueError("Refusing to change a published release")
        subprocess.run(
            ["gh", "release", "edit", tag, "--title", f"Suki Toolbox {tag}", "--notes-file", str(notes_file)],
            check=True,
        )
        for asset in release["assets"]:
            if asset["name"] not in names:
                subprocess.run(["gh", "release", "delete-asset", tag, asset["name"], "--yes"], check=True)
    else:
        subprocess.run(
            ["gh", "release", "create", tag, "--verify-tag", "--draft", "--title", f"Suki Toolbox {tag}", "--notes-file", str(notes_file)],
            check=True,
        )
    subprocess.run(["gh", "release", "upload", tag, *[str(output / name) for name in names], "--clobber"], check=True)


if __name__ == "__main__":
    main()
