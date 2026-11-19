import tempfile, unittest
from pathlib import Path
from unittest.mock import patch
import pjpm

class ManifestTests(unittest.TestCase):
    def test_v2_manifest(self):
        with tempfile.TemporaryDirectory() as d:
            p=Path(d)/"pjpackage.toml"
            p.write_text('[package]\nschema=2\nname="demo"\nversion="1.2.3"\nlanguage="python"\n[dependencies]\n')
            m=pjpm.load_manifest(Path(d))
            self.assertEqual(m["name"],"demo")
            self.assertEqual(m["format"],"v2")
    def test_legacy_manifest(self):
        with tempfile.TemporaryDirectory() as d:
            root=Path(d); (root/"package.toml").write_text('description=""\nlanguage="payjar"\nbuild="PJC"\nversion="1.0"\n')
            self.assertEqual(pjpm.load_manifest(root)["format"],"legacy")
    def test_init_pj_fallback(self):
        with tempfile.TemporaryDirectory() as d:
            (Path(d)/"init.pj").write_text('print("ok")')
            self.assertEqual(pjpm.load_manifest(Path(d))["format"],"legacy-init")
    def test_version_constraints(self):
        self.assertTrue(pjpm.version_ok("1.4.0","^1.2.0"))
        self.assertFalse(pjpm.version_ok("2.0.0","^1.2.0"))
        self.assertTrue(pjpm.version_ok("1.4.2","~1.4.0"))
        self.assertTrue(pjpm.version_ok("1.4.2",">=1.2.0,<2.0.0"))
        self.assertFalse(pjpm.version_ok("2.0.0",">=1.2.0,<2.0.0"))
    def test_transitive_resolution_order(self):
        catalog={
            "alpha":{"version":"1.0.0","dependencies":{"beta":"^1.0.0"}},
            "beta":{"version":"1.2.0","dependencies":{}},
        }
        with patch.object(pjpm,"registry_manifest",side_effect=lambda n:catalog[n]):
            self.assertEqual(pjpm.resolve_graph({"alpha":"*"}),[("beta","^1.0.0"),("alpha","*")])
    def test_dependency_cycle(self):
        catalog={
            "alpha":{"version":"1.0.0","dependencies":{"beta":"*"}},
            "beta":{"version":"1.0.0","dependencies":{"alpha":"*"}},
        }
        with patch.object(pjpm,"registry_manifest",side_effect=lambda n:catalog[n]):
            with self.assertRaisesRegex(pjpm.PjpmError,"cycle"):
                pjpm.resolve_graph({"alpha":"*"})

if __name__=="__main__": unittest.main()
