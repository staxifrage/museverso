import importlib.util
import json
import subprocess
import tempfile
import unittest
from pathlib import Path
ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('prepare', ROOT / 'scripts/prepare.py')
prepare = importlib.util.module_from_spec(spec)
spec.loader.exec_module(prepare)

class PatchTests(unittest.TestCase):
    def test_patch_refuses_changed_upstream_anchor(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'file').write_text('different code')
            with self.assertRaises(RuntimeError): prepare.replace_once(root, 'file', 'expected', 'new')
            self.assertEqual((root / 'file').read_text(), 'different code')

    def test_patch_refuses_ambiguous_anchor(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'file').write_text('repeat repeat')
            with self.assertRaises(RuntimeError): prepare.replace_once(root, 'file', 'repeat', 'new')

    def test_revision_is_immutable_and_playback_limit_is_explicit(self):
        lock = json.loads((ROOT / 'servo.lock.json').read_text())
        self.assertRegex(lock['revision'], r'^[a-f0-9]{40}$')
        self.assertFalse(lock['media_source_extensions'])

    def test_python_scripts_parse(self):
        for path in (ROOT / 'scripts').glob('*.py'):
            compile(path.read_text(), str(path), 'exec')

if __name__ == '__main__': unittest.main()
