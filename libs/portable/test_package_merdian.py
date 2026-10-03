import importlib.util
import io
from pathlib import Path
import struct
import tempfile
import unittest
from unittest import mock

spec = importlib.util.spec_from_file_location('merdian_package', Path(__file__).with_name('package_merdian.py'))
packager = importlib.util.module_from_spec(spec)
spec.loader.exec_module(packager)


class PackageEvidenceTests(unittest.TestCase):
    def test_new_proof_path_preserves_history_and_rejects_outside_destination(self):
        with tempfile.TemporaryDirectory() as directory:
            artifacts = Path(directory) / 'artifacts'
            artifacts.mkdir()
            history = artifacts / 'first-build.json'
            history.write_text('original evidence', encoding='utf-8')
            with mock.patch.object(packager, 'ARTIFACTS', artifacts.resolve()):
                self.assertEqual(packager.new_proof_path(artifacts / 'second-build.json'), (artifacts / 'second-build.json').resolve())
                with self.assertRaisesRegex(ValueError, 'fresh'):
                    packager.new_proof_path(history)
                with self.assertRaisesRegex(ValueError, 'inside'):
                    packager.new_proof_path(Path(directory) / 'outside.json')
            self.assertEqual(history.read_text(encoding='utf-8'), 'original evidence')

    def test_windows_inventory_normalizes_values_and_catches_changed_bytes(self):
        expected = [{'path': 'data\\assets\\file.txt', 'size': 3, 'sha256': 'A' * 64}]
        actual = [{'path': 'data/assets/file.txt', 'size': 3, 'sha256': 'a' * 64}]
        self.assertEqual(packager.normalized_inventory(expected), packager.normalized_inventory(actual))
        actual[0]['sha256'] = 'b' * 64
        self.assertNotEqual(packager.normalized_inventory(expected), packager.normalized_inventory(actual))

    def test_evidence_cannot_alias_windows_paths_or_hide_escape_paths(self):
        for paths in [('data/x', 'data\\x'), ('Data/X', 'data/x'), ('../x',), ('C:/x',), ('x:stream',)]:
            rows = [{'path': path, 'size': 1, 'sha256': 'a' * 64} for path in paths]
            with self.subTest(paths=paths), self.assertRaises(ValueError):
                packager.normalized_inventory(rows)

    def test_pending_ui_change_stops_package_before_bundle_can_be_used(self):
        def git_result(*args):
            return ' M flutter/lib/desktop/merdian_lifecycle.dart' if args[0] == 'status' else ''
        with mock.patch.object(packager, 'git', side_effect=git_result), mock.patch('subprocess.run') as run:
            with self.assertRaisesRegex(ValueError, 'Commit and rebuild'):
                packager.require_clean_sources('a' * 40, ['libs/portable/main.rs'])
            self.assertEqual(run.call_count, 1)

    def test_launcher_and_dll_checks_reject_x86_without_loading_or_executing(self):
        for machine, accepted in [(0x8664, True), (0x14c, False)]:
            data = bytearray(70)
            data[:2] = b'MZ'
            struct.pack_into('<I', data, 60, 64)
            data[64:68] = b'PE\x00\x00'
            struct.pack_into('<H', data, 68, machine)
            with self.subTest(machine=machine), mock.patch.object(Path, 'open', return_value=io.BytesIO(data)):
                if accepted:
                    packager.require_x64_pe(Path('not-executed.exe'))
                else:
                    with self.assertRaises(ValueError):
                        packager.require_x64_pe(Path('not-executed.exe'))


if __name__ == '__main__':
    unittest.main()
