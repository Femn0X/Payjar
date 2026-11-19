import argparse
import contextlib
import hashlib
import io
import json
import os
import tempfile
import unittest
import zipfile
from pathlib import Path
from unittest.mock import patch

import pjpm


def release(name, version, dependencies=None, commit=None, path=None):
    return {"name": name, "version": version, "dependencies": dependencies or {},
            "language": "python", "manifest": "pjpackage.toml",
            "commit": commit, "path": path or name}


class ManifestTests(unittest.TestCase):
    def tearDown(self):
        pjpm.registry_index.cache_clear()

    def test_v2_manifest(self):
        with tempfile.TemporaryDirectory() as d:
            p = Path(d) / "pjpackage.toml"
            p.write_text('[package]\nschema=2\nname="demo"\nversion="1.2.3"\nlanguage="python"\n[dependencies]\n')
            m = pjpm.load_manifest(Path(d))
            self.assertEqual(m["name"], "demo")
            self.assertEqual(m["format"], "v2")

    def test_legacy_manifest(self):
        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            (root / "package.toml").write_text('description=""\nlanguage="payjar"\nbuild="PJC"\nversion="1.0"\n')
            self.assertEqual(pjpm.load_manifest(root)["format"], "legacy")

    def test_init_pj_fallback(self):
        with tempfile.TemporaryDirectory() as d:
            (Path(d) / "init.pj").write_text('print("ok")')
            self.assertEqual(pjpm.load_manifest(Path(d))["format"], "legacy-init")

    def test_reject_unsafe_version_paths(self):
        for version in ("../outside", "1.0/../../outside", "1.0\\\\unsafe", ".", ".."):
            with self.subTest(version=version), self.assertRaises(pjpm.PjpmError):
                pjpm.safe_version(version)

    def test_v2_manifest_accepts_build_metadata(self):
        with tempfile.TemporaryDirectory() as directory:
            (Path(directory) / "pjpackage.toml").write_text(chr(10).join([
                "[package]", "schema=2", 'name="demo"',
                'version="1.2.3+build.4"', 'language="python"', ""
            ]))
            self.assertEqual(pjpm.load_manifest(Path(directory))["version"], "1.2.3+build.4")

    def test_search_exact_legacy_version(self):
        fake_catalog = [
            release("Geometry", "2.0.0"),
            release("Geometry", "1.0"),
        ]
        with patch.object(pjpm, "registry_dirs", return_value=["Geometry"]), \
             patch.object(pjpm, "registry_candidates", return_value=fake_catalog):
            out = io.StringIO()
            with contextlib.redirect_stdout(out):
                pjpm.cmd_search(argparse.Namespace(query="Geometry@1.0"))
        self.assertEqual(out.getvalue().strip(), "Geometry 1.0 [python]")

    def test_search_ranges_include_matching_historical_versions(self):
        fake_catalog = [
            release("example", "2.0.0"),
            release("example", "1.3.0"),
            release("example", "1.0.0"),
        ]
        with patch.object(pjpm, "registry_dirs", return_value=["example"]), \
             patch.object(pjpm, "registry_candidates", return_value=fake_catalog):
            out = io.StringIO()
            with contextlib.redirect_stdout(out):
                pjpm.cmd_search(argparse.Namespace(query="exa@^1.0.0"))
        self.assertIn("example 1.3.0", out.getvalue())
        self.assertIn("example 1.0.0", out.getvalue())
        self.assertNotIn("example 2.0.0", out.getvalue())

    def test_info_exact_version(self):
        fake_catalog = [release("Geometry", "2.0.0"), release("Geometry", "1.0")]
        with patch.object(pjpm, "registry_candidates", return_value=fake_catalog):
            out = io.StringIO()
            with contextlib.redirect_stdout(out):
                pjpm.cmd_info(argparse.Namespace(package="Geometry@1.0"))
        data = json.loads(out.getvalue())
        self.assertEqual(data["version"], "1.0")
        self.assertEqual(data["available_versions"], ["2.0.0", "1.0"])

    def test_add_outside_project_has_actionable_error(self):
        with tempfile.TemporaryDirectory() as directory:
            old_dir = os.getcwd()
            try:
                os.chdir(directory)
                with self.assertRaisesRegex(pjpm.PjpmError, "pjpm init"):
                    pjpm.cmd_add(argparse.Namespace(package="Geometry@1.0"))
            finally:
                os.chdir(old_dir)

    def write_config(self, version='"1.0"'):
        Path("config.toml").write_text(chr(10).join([
            "[dependencies]", "Geometry = " + version, ""
        ]))

    def test_config_only_dependencies(self):
        with tempfile.TemporaryDirectory() as directory:
            cwd = os.getcwd()
            try:
                os.chdir(directory)
                self.write_config()
                self.assertEqual(pjpm.project_dependencies(), {"Geometry": "1.0"})
            finally:
                os.chdir(cwd)

    def test_config_rejects_non_string_constraints(self):
        with tempfile.TemporaryDirectory() as directory:
            cwd = os.getcwd()
            try:
                os.chdir(directory)
                self.write_config("1.0")
                with self.assertRaisesRegex(pjpm.PjpmError, "constraint string"):
                    pjpm.project_dependencies()
            finally:
                os.chdir(cwd)

    def test_sync_skips_existing_local_package(self):
        with tempfile.TemporaryDirectory() as directory:
            cwd = os.getcwd()
            try:
                os.chdir(directory)
                self.write_config()
                Path("Geometry").mkdir()
                Path("Geometry/init.pj").write_text("package Geometry;")
                with patch.object(pjpm, "resolve_graph") as resolve:
                    pjpm.cmd_sync(argparse.Namespace())
                resolve.assert_not_called()
                self.assertFalse(Path(".pjpm").exists())
            finally:
                os.chdir(cwd)

    def test_sync_installs_only_when_missing_and_caches_lock(self):
        with tempfile.TemporaryDirectory() as directory:
            cwd = os.getcwd()
            try:
                os.chdir(directory)
                self.write_config()
                meta = release("Geometry", "1.0", commit="a" * 40)
                record = {"name": "Geometry", "version": "1.0", "registry": pjpm.REGISTRY,
                          "commit": "a" * 40, "path": "Geometry", "sha256": "e" * 64}

                def fake_install(_meta, expected_sha256=None):
                    root = Path(".pjpm/packages/Geometry/1.0")
                    root.mkdir(parents=True, exist_ok=True)
                    (root / "init.pj").write_text("package Geometry;")
                    (root / ".pjpm-source.json").write_text(json.dumps(record))
                    return record

                with patch.object(pjpm, "resolve_graph", return_value=[meta]) as resolve, \
                     patch.object(pjpm, "install_package", side_effect=fake_install) as install:
                    pjpm.cmd_sync(argparse.Namespace())
                    pjpm.cmd_sync(argparse.Namespace())
                self.assertEqual(resolve.call_count, 1)
                self.assertEqual(install.call_count, 1)
                self.assertEqual(pjpm.read_lock()["project_dependencies"], {"Geometry": "1.0"})
                self.assertTrue(Path(".pjpm/packages/Geometry/1.0/init.pj").is_file())
            finally:
                os.chdir(cwd)

    def test_version_constraints(self):
        self.assertTrue(pjpm.version_ok("1.4.0", "^1.2.0"))
        self.assertFalse(pjpm.version_ok("2.0.0", "^1.2.0"))
        self.assertTrue(pjpm.version_ok("1.4.2", "~1.4.0"))
        self.assertTrue(pjpm.version_ok("1.4.2", ">=1.2.0,<2.0.0"))
        self.assertFalse(pjpm.version_ok("2.0.0", ">=1.2.0,<2.0.0"))
        self.assertTrue(pjpm.version_ok("1.0", "1.0"))  # old Geometry version
        self.assertTrue(pjpm.version_ok("1.0.0-beta", ">=1.0.0-alpha,<1.0.0"))
        self.assertFalse(pjpm.version_ok("1.0.0-beta", "=1.0.0"))
        self.assertTrue(pjpm.version_ok("1.0.0", ">1.0.0-beta"))
        self.assertTrue(pjpm.version_ok("0.2.9", "^0.2.0"))
        self.assertFalse(pjpm.version_ok("0.3.0", "^0.2.0"))

    def test_transitive_resolution_order(self):
        catalog = {
            "alpha": [release("alpha", "1.0.0", {"beta": "^1.0.0"})],
            "beta": [release("beta", "1.2.0")],
        }
        with patch.object(pjpm, "registry_candidates", side_effect=lambda n: catalog[n]):
            self.assertEqual([x["name"] for x in pjpm.resolve_graph({"alpha": "*"})], ["beta", "alpha"])

    def test_dependency_cycle(self):
        catalog = {
            "alpha": [release("alpha", "1.0.0", {"beta": "*"})],
            "beta": [release("beta", "1.0.0", {"alpha": "*"})],
        }
        with patch.object(pjpm, "registry_candidates", side_effect=lambda n: catalog[n]):
            with self.assertRaisesRegex(pjpm.PjpmError, "cycle"):
                pjpm.resolve_graph({"alpha": "*"})

    def test_highest_matching_version(self):
        catalog = {"alpha": [release("alpha", "2.0.0"), release("alpha", "1.4.0"), release("alpha", "1.0.0")]}
        with patch.object(pjpm, "registry_candidates", side_effect=lambda n: catalog[n]):
            result = pjpm.resolve_graph({"alpha": "^1.0.0"})
        self.assertEqual(result[0]["version"], "1.4.0")

    def test_backtracking_when_newest_conflicts(self):
        catalog = {
            "alpha": [release("alpha", "1.2.0", {"beta": "^2.0.0"}),
                      release("alpha", "1.0.0", {"beta": "^1.0.0"})],
            "beta": [release("beta", "2.1.0"), release("beta", "1.1.0")],
        }
        with patch.object(pjpm, "registry_candidates", side_effect=lambda n: catalog[n]):
            result = pjpm.resolve_graph({"alpha": "*", "beta": "^1.0.0"})
        self.assertEqual({m["name"]: m["version"] for m in result}, {"alpha": "1.0.0", "beta": "1.1.0"})

    def test_pinned_lock_preferred_to_new_release(self):
        pinned = release("alpha", "1.0.0", commit="a" * 40)
        catalog = [release("alpha", "2.0.0", commit="b" * 40), pinned]
        old = {"alpha": {"name": "alpha", "version": "1.0.0", "commit": "a" * 40,
                         "path": "alpha", "registry": pjpm.REGISTRY}}
        with patch.object(pjpm, "registry_candidates", return_value=catalog), \
             patch.object(pjpm, "read_registry_manifest", return_value=pinned):
            result = pjpm.resolve_graph({"alpha": "*"}, locked=old)
        self.assertEqual(result[0]["version"], "1.0.0")
        self.assertEqual(result[0]["commit"], "a" * 40)

    def test_index_selects_committed_versions(self):
        index = {"alpha": {
            "1.0.0": {"commit": "a" * 40, "path": "releases/alpha/1.0.0"},
            "2.0.0": {"commit": "b" * 40, "path": "releases/alpha/2.0.0"},
        }}
        def fake_manifest(name, ref, path):
            version = path.rsplit("/", 1)[-1]
            return release(name, version, commit=ref, path=path)
        with patch.object(pjpm, "registry_index", return_value=index), \
             patch.object(pjpm, "read_registry_manifest", side_effect=fake_manifest):
            releases = pjpm.registry_candidates("alpha")
        self.assertEqual([item["version"] for item in releases], ["2.0.0", "1.0.0"])
        self.assertEqual(releases[0]["commit"], "b" * 40)

    def test_index_rejects_mutable_ref(self):
        pjpm.registry_index.cache_clear()
        with patch.object(pjpm, "http", return_value=json.dumps({
            "schema": 1, "packages": {"alpha": {"1.0.0": {"commit": "main", "path": "alpha"}}}
        })):
            with self.assertRaisesRegex(pjpm.PjpmError, "commit SHA"):
                pjpm.registry_index()

    def test_index_missing_means_legacy_root(self):
        with patch.object(pjpm, "registry_index", return_value={}), \
             patch.object(pjpm, "read_registry_manifest", return_value=release("Geometry", "1.0")) as get:
            result = pjpm.registry_candidates("Geometry")
        self.assertEqual(result[0]["version"], "1.0")
        get.assert_called_once_with("Geometry", "main", "Geometry")

    def test_manifest_version_must_match_index(self):
        with patch.object(pjpm, "registry_index", return_value={
            "alpha": {"1.0.0": {"commit": "a" * 40, "path": "alpha"}}}), \
             patch.object(pjpm, "read_registry_manifest", return_value=release("alpha", "2.0.0")):
            with self.assertRaisesRegex(pjpm.PjpmError, "differs"):
                pjpm.registry_candidates("alpha")

    def test_source_path_cannot_escape(self):
        for path in ("../secret", "/etc", "a//b", "a\\b"):
            with self.subTest(path=path), self.assertRaises(pjpm.PjpmError):
                pjpm.safe_registry_path(path)

    def test_install_nested_immutable_source_and_digest(self):
        source = release("alpha", "1.0.0", commit="a" * 40, path="releases/alpha/1.0.0")
        buf = io.BytesIO()
        with zipfile.ZipFile(buf, "w") as archive:
            archive.writestr("registry-snapshot/releases/alpha/1.0.0/pjpackage.toml",
                             '[package]\nschema=2\nname="alpha"\nversion="1.0.0"\nlanguage="python"\n')
            archive.writestr("registry-snapshot/releases/alpha/1.0.0/src/main.py", "print('hello')")
        binary = buf.getvalue()
        with tempfile.TemporaryDirectory() as temp, patch.object(pjpm, "http", return_value=binary):
            old_cwd = os.getcwd()
            try:
                os.chdir(temp)
                with self.assertRaisesRegex(pjpm.PjpmError, "integrity mismatch"):
                    pjpm.install_package(source, expected_sha256="0" * 64)
                record = pjpm.install_package(source, expected_sha256=hashlib.sha256(binary).hexdigest())
                installed = Path(".pjpm/packages/alpha/1.0.0/src/main.py")
                self.assertTrue(installed.is_file())
                self.assertEqual(record["commit"], "a" * 40)
                self.assertEqual(record["path"], "releases/alpha/1.0.0")
            finally:
                os.chdir(old_cwd)


    def test_cached_release_replaced_if_archive_changes(self):
        source = release("alpha", "1.0.0", commit="a" * 40)
        def archive_with(text):
            stream = io.BytesIO()
            with zipfile.ZipFile(stream, "w") as archive:
                archive.writestr("registry-snapshot/alpha/main.py", text)
            return stream.getvalue()
        first = archive_with("first")
        second = archive_with("second")
        with tempfile.TemporaryDirectory() as temp:
            cwd = os.getcwd()
            try:
                os.chdir(temp)
                with patch.object(pjpm, "http", return_value=first):
                    pjpm.install_package(source)
                with patch.object(pjpm, "http", return_value=second):
                    result = pjpm.install_package(source)
                self.assertEqual(Path(".pjpm/packages/alpha/1.0.0/main.py").read_text(), "second")
                marker = json.loads(Path(".pjpm/packages/alpha/1.0.0/.pjpm-source.json").read_text())
                self.assertEqual(marker["sha256"], result["sha256"])
            finally:
                os.chdir(cwd)


if __name__ == "__main__":
    unittest.main()
