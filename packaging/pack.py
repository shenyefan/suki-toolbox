"""Create a portable Windows archive without developer or user configuration.

Usage: pack.py VERSION TARGET OUTPUT_DIR
"""
import json
from pathlib import Path
import subprocess
import sys
from zipfile import ZIP_DEFLATED, ZipFile

ROOT = Path(__file__).resolve().parents[1]
CHANNELS = {
    "x86_64-pc-windows-msvc": "win-x64",
    "aarch64-pc-windows-msvc": "win-arm64",
}


def pack(binary, readme, output, version, target, revision):
    channel = CHANNELS[target]
    output.mkdir(parents=True, exist_ok=True)
    archive = output / f"SukiToolbox-{version}-{channel}-Portable.zip"
    with ZipFile(archive, "w", compression=ZIP_DEFLATED) as package:
        package.write(binary, "suki-toolbox.exe")
        package.write(readme, "README.md")
    metadata = {
        "version": version,
        "target": target,
        "channel": channel,
        "revision": revision,
        "format": "portable",
    }
    (output / f"build.{channel}.json").write_text(
        json.dumps(metadata, indent=2) + "\n", encoding="utf-8"
    )
    return archive


if __name__ == "__main__":
    version, target, output = sys.argv[1:]
    revision = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip()
    pack(
        ROOT / "target" / target / "release" / "suki-toolbox.exe",
        ROOT / "packaging" / "README.md",
        Path(output),
        version,
        target,
        revision,
    )
