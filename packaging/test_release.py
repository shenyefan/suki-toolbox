"""Release contract tests; these do not execute binaries or contact GitHub."""
import hashlib
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
from zipfile import ZipFile

from pack import CHANNELS, pack
import release
from release import collect
from version import validate


class ReleaseTests(unittest.TestCase):
    def test_tag_must_match_stable_package_version(self):
        self.assertEqual(validate("0.1.0", "refs/tags/v0.1.0", "v0.1.0"), "0.1.0")
        self.assertEqual(validate("0.1.0", "refs/heads/main", "main"), "0.1.0")
        with self.assertRaises(ValueError):
            validate("0.1.0", "refs/tags/v0.2.0", "v0.2.0")
        with self.assertRaises(ValueError):
            validate("0.1.0-beta.1", "refs/heads/main", "main")

    def test_packages_exclude_local_configuration_and_checksums_match(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            binary = root / "suki-toolbox.exe"
            binary.write_bytes(b"fixture executable")
            readme = root / "README.md"
            readme.write_text("portable instructions", encoding="utf-8")
            (root / "config.json").write_text('{"private": true}', encoding="utf-8")
            packages = root / "packages"
            for target, channel in CHANNELS.items():
                output = packages / f"packages-{channel}"
                archive = pack(binary, readme, output, "0.1.0", target, "test-revision")
                with ZipFile(archive) as package:
                    self.assertEqual(set(package.namelist()), {"suki-toolbox.exe", "README.md"})
                    self.assertEqual(package.read("suki-toolbox.exe"), binary.read_bytes())
                metadata = json.loads((output / f"build.{channel}.json").read_text())
                self.assertEqual(metadata["target"], target)
                self.assertEqual(metadata["revision"], "test-revision")
            output = root / "Releases"
            names = collect(packages, output, "0.1.0")
            self.assertEqual(len(names), 5)
            for line in (output / "SHA256SUMS").read_text().splitlines():
                digest, name = line.split("  ")
                self.assertEqual(digest, hashlib.sha256((output / name).read_bytes()).hexdigest())
            with self.assertRaises(ValueError):
                collect(packages, root / "incomplete", "0.2.0")

    def test_published_release_is_never_modified(self):
        from subprocess import CompletedProcess

        with tempfile.TemporaryDirectory() as temporary:
            notes = Path(temporary) / "release-notes.md"
            existing = CompletedProcess([], 0, '{"isDraft": false, "assets": []}', "")
            with (
                patch("release.sys.argv", ["release.py", "0.1.0", "v0.1.0", "packages"]),
                patch.dict("release.os.environ", {"GH_REPO": "owner/repo"}),
                patch("release.collect", return_value=["archive.zip"]),
                patch("release.Path", side_effect=lambda value: notes if value == "release-notes.md" else Path(value)),
                patch("release.subprocess.check_output", return_value="changes"),
                patch("release.subprocess.run", return_value=existing) as commands,
            ):
                with self.assertRaisesRegex(ValueError, "published release"):
                    release.main()
                self.assertEqual(commands.call_count, 1)
                self.assertEqual(commands.call_args.args[0][1:3], ["release", "view"])


if __name__ == "__main__":
    unittest.main()
