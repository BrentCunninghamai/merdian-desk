import importlib.util
from pathlib import Path
import struct
import unittest
from unittest import mock
import brotli

spec = importlib.util.spec_from_file_location('merdian_generate', Path(__file__).with_name('generate.py'))
generator = importlib.util.module_from_spec(spec)
spec.loader.exec_module(generator)


class GenerationTests(unittest.TestCase):
    def test_wrong_launch_name_and_missing_payload_fail_before_file_creation(self):
        with mock.patch('builtins.open') as opened:
            with self.assertRaises(ValueError):
                generator.write_blob({}, 'never-written.bin', 'rustdesk.exe')
            with self.assertRaises(ValueError):
                generator.write_blob({}, 'never-written.bin', './Merdian-Desk.exe')
            opened.assert_not_called()

    def test_builtin_brotli_blob_retains_original_payload_bytes_and_launch_name(self):
        import io
        from hashlib import md5
        original = b'Owned packaging test data; no executable code.'
        payload = brotli.compress(original, quality=5)
        stream = io.BytesIO()
        table = {'./Merdian-Desk.exe': (payload, md5(original).hexdigest().encode())}
        with mock.patch('builtins.open', return_value=mock.MagicMock(__enter__=lambda _: stream, __exit__=lambda *_: None)):
            generator.write_blob(table, 'not-on-disk.bin', './Merdian-Desk.exe')
        raw = stream.getvalue()
        self.assertEqual(raw[:8], b'rustdesk')
        path_length = struct.unpack_from('>I', raw, 8)[0]
        self.assertEqual(raw[12:12 + path_length], b'./Merdian-Desk.exe')
        at = 12 + path_length
        data_length = struct.unpack_from('>I', raw, at)[0]
        self.assertEqual(brotli.decompress(raw[at + 4:at + 4 + data_length]), original)
        self.assertTrue(raw.endswith(b'rustdesk./Merdian-Desk.exe'))

    def test_build_uses_explicit_owned_binary_without_running_it(self):
        with mock.patch('os.getcwd', return_value='original'), mock.patch('os.chdir'), mock.patch('subprocess.run') as run:
            generator.build_portable('libs/portable', 'x86_64-pc-windows-msvc')
        run.assert_called_once_with(['cargo', 'build', '--locked', '--release', '--package', 'rustdesk-portable-packer', '--bin', 'merdian-desk-portable', '--target', 'x86_64-pc-windows-msvc'], check=True)


if __name__ == '__main__':
    unittest.main()
