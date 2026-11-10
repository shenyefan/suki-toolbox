"""Validate the release tag against the current Cargo package version."""
import os
from pathlib import Path
import re
import tomllib

ROOT = Path(__file__).resolve().parents[1]


def validate(version, ref, tag):
    if not re.fullmatch(r"\d+\.\d+\.\d+", version):
        raise ValueError("Only stable x.y.z versions are supported")
    if ref.startswith("refs/tags/") and tag != f"v{version}":
        raise ValueError(f"Tag must match Cargo.toml version v{version}")
    return version


if __name__ == "__main__":
    package = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))["package"]
    version = validate(package["version"], os.environ["GITHUB_REF"], os.environ["GITHUB_REF_NAME"])
    print(f"version={version}")
