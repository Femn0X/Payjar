import tempfile, unittest
from pathlib import Path
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

if __name__=="__main__": unittest.main()
