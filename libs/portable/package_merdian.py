#!/usr/bin/env python3
"""Validate/package a reviewed build bundle. Never launches the client."""
import argparse
import ctypes
import hashlib
import json
import os
from pathlib import Path
import shutil
import stat
import struct
import subprocess
import sys
from datetime import datetime, timezone

REPO = Path(__file__).resolve().parents[2]
DEFAULT_ARTIFACTS = (REPO.parent.parent if REPO.parent.name.casefold() == 'fork' else REPO) / 'artifacts'
ARTIFACTS = Path(os.environ.get('MERDIAN_ARTIFACTS_ROOT') or DEFAULT_ARTIFACTS).resolve()
PAYLOAD = 'Merdian-Desk.exe'
ENGINE_PATHS = ['src', 'flutter', 'Cargo.toml', 'build.rs', 'build.py',
                'libs/hbb_common', 'libs/base', 'libs/scrap', 'libs/enigo',
                'libs/clipboard', 'libs/virtual_display', 'libs/remote_printer']


def sha256(path):
    digest = hashlib.sha256()
    with Path(path).open('rb') as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b''):
            digest.update(chunk)
    return digest.hexdigest()


def git(*args):
    return subprocess.check_output(['git', '-C', str(REPO), *args], text=True).strip()


def inventory(folder):
    rows = []
    for path in sorted(folder.rglob('*')):
        metadata = path.lstat()
        if path.is_symlink() or getattr(metadata, 'st_file_attributes', 0) & stat.FILE_ATTRIBUTE_REPARSE_POINT:
            raise ValueError(f'Bundle cannot contain a reparse point: {path.name}')
        if path.is_file():
            rows.append({'path': path.relative_to(folder).as_posix(), 'size': metadata.st_size, 'sha256': sha256(path)})
    return rows


def normalized_inventory(rows):
    """Compare Windows CI/local inventories without hiding aliases or bad hashes."""
    result = {}
    aliases = set()
    for row in rows:
        path = row['path'].replace('\\', '/')
        parts = path.split('/')
        if any(part in ['', '.', '..'] for part in parts) or any(c in path for c in ':\x00'):
            raise ValueError('Build evidence contains an invalid relative file path')
        alias = path.casefold()
        if alias in aliases:
            raise ValueError('Build evidence contains a duplicated Windows file path')
        digest = row['sha256'].lower()
        if len(digest) != 64 or any(c not in '0123456789abcdef' for c in digest):
            raise ValueError('Build evidence contains an invalid SHA256')
        if type(row['size']) is not int or row['size'] < 0:
            raise ValueError('Build evidence contains an invalid file size')
        aliases.add(alias)
        result[path] = {'path': path, 'size': row['size'], 'sha256': digest}
    return result


def require_clean_sources(app_commit, code_files):
    for relative in code_files:
        git('ls-files', '--error-unmatch', relative)
    subprocess.run(['git', '-C', str(REPO), 'diff', '--exit-code', 'HEAD', '--', *code_files],
                   check=True, stdout=subprocess.DEVNULL)
    if git('status', '--porcelain', '--untracked-files=all', '--', *ENGINE_PATHS):
        raise ValueError('Commit and rebuild all engine/UI changes before packaging the matching application')
    subprocess.run(['git', '-C', str(REPO), 'diff', '--exit-code', app_commit, 'HEAD', '--', *ENGINE_PATHS],
                   check=True, stdout=subprocess.DEVNULL)


def require_x64_pe(path):
    with path.open('rb') as file:
        header = file.read(64)
        if len(header) < 64 or header[:2] != b'MZ':
            raise ValueError(f'{path.name} is not a Windows executable')
        file.seek(struct.unpack_from('<I', header, 60)[0])
        pe = file.read(6)
        if len(pe) != 6 or pe[:4] != b'PE\x00\x00' or struct.unpack_from('<H', pe, 4)[0] != 0x8664:
            raise ValueError(f'{path.name} must be a native x64 Windows PE')


def version_info(path):
    library = ctypes.WinDLL('version', use_last_error=True)
    library.GetFileVersionInfoSizeW.argtypes = [ctypes.c_wchar_p, ctypes.POINTER(ctypes.c_ulong)]
    library.GetFileVersionInfoSizeW.restype = ctypes.c_ulong
    library.GetFileVersionInfoW.argtypes = [ctypes.c_wchar_p, ctypes.c_ulong, ctypes.c_ulong, ctypes.c_void_p]
    library.VerQueryValueW.argtypes = [ctypes.c_void_p, ctypes.c_wchar_p, ctypes.POINTER(ctypes.c_void_p), ctypes.POINTER(ctypes.c_uint)]
    size = library.GetFileVersionInfoSizeW(str(path), ctypes.byref(ctypes.c_ulong()))
    if not size:
        raise ValueError('Wrapper is missing Windows version metadata')
    buffer = ctypes.create_string_buffer(size)
    if not library.GetFileVersionInfoW(str(path), 0, size, buffer):
        raise ctypes.WinError(ctypes.get_last_error())
    pointer, length = ctypes.c_void_p(), ctypes.c_uint()
    if not library.VerQueryValueW(buffer, '\\VarFileInfo\\Translation', ctypes.byref(pointer), ctypes.byref(length)) or length.value < 4:
        raise ValueError('Wrapper version translation metadata is missing')
    translation = (ctypes.c_ushort * 2).from_address(pointer.value)
    prefix = f'\\StringFileInfo\\{translation[0]:04x}{translation[1]:04x}\\'
    result = {}
    for key in ['CompanyName', 'ProductName', 'OriginalFilename', 'FileDescription', 'LegalCopyright']:
        if not library.VerQueryValueW(buffer, prefix + key, ctypes.byref(pointer), ctypes.byref(length)):
            raise ValueError(f'Wrapper version metadata is missing {key}')
        result[key] = ctypes.wstring_at(pointer.value)
    if result['ProductName'] != 'Merdian-Desk' or result['OriginalFilename'] != PAYLOAD or result['CompanyName'] != 'Merdian-Desk':
        raise ValueError('Wrapper does not have owned Merdian-Desk branding')
    return result


def manifest_info(path):
    # Read as a resource data file: this does not run DllMain or client code.
    library = ctypes.WinDLL('kernel32', use_last_error=True)
    library.LoadLibraryExW.argtypes = [ctypes.c_wchar_p, ctypes.c_void_p, ctypes.c_ulong]
    library.LoadLibraryExW.restype = ctypes.c_void_p
    library.FindResourceW.argtypes = [ctypes.c_void_p, ctypes.c_void_p, ctypes.c_void_p]
    library.FindResourceW.restype = ctypes.c_void_p
    library.SizeofResource.argtypes = [ctypes.c_void_p, ctypes.c_void_p]
    library.SizeofResource.restype = ctypes.c_ulong
    library.LoadResource.argtypes = [ctypes.c_void_p, ctypes.c_void_p]
    library.LoadResource.restype = ctypes.c_void_p
    library.LockResource.argtypes = [ctypes.c_void_p]
    library.LockResource.restype = ctypes.c_void_p
    library.FreeLibrary.argtypes = [ctypes.c_void_p]
    handle = library.LoadLibraryExW(str(path), None, 0x02 | 0x20)
    if not handle:
        raise ctypes.WinError(ctypes.get_last_error())
    try:
        resource = library.FindResourceW(handle, ctypes.c_void_p(1), ctypes.c_void_p(24))
        if not resource:
            raise ValueError('Wrapper application manifest is missing')
        size = library.SizeofResource(handle, resource)
        data = library.LockResource(library.LoadResource(handle, resource))
        text = ctypes.string_at(data, size).decode('utf-8-sig')
        if 'level="asInvoker"' not in text or 'uiAccess="false"' not in text or 'requireAdministrator' in text:
            raise ValueError('Wrapper manifest does not enforce normal-user launch')
        return {'as_invoker': True, 'ui_access': False, 'sha256': hashlib.sha256(text.encode()).hexdigest()}
    finally:
        library.FreeLibrary(handle)


def validate(bundle, output):
    if os.name != 'nt':
        raise ValueError('Packaging is supported on Windows x64 only')
    if not bundle.is_dir():
        raise ValueError('A successfully compiled complete build bundle is required')
    if ARTIFACTS == Path(ARTIFACTS.anchor):
        raise ValueError('The selected artifacts root cannot be a drive or filesystem root')
    if not output.is_relative_to(ARTIFACTS) or output == ARTIFACTS:
        raise ValueError('The fresh package output must be inside the selected artifacts root')
    if output.exists():
        raise ValueError('Output already exists; use a fresh short folder name, preserving previous evidence')
    source_commit = git('rev-parse', 'HEAD')
    code_files = ['libs/portable/Cargo.toml', 'libs/portable/build.rs', 'libs/portable/generate.py', 'libs/portable/package_merdian.py',
                  'libs/portable/src/main.rs', 'libs/portable/src/policy.rs', 'libs/portable/src/win.rs', 'libs/portable/src/ui.rs',
                  'libs/portable/src/bin_reader.rs', 'libs/portable/res/merdian.manifest.xml',
                  'src/merdian_policy_model.rs', 'flutter/windows/runner/resources/merdian_desk.ico', 'Cargo.lock',
                  'libs/portable/requirements.txt', 'libs/portable/README-MERDIAN.md', 'LICENCE',
                  'docs/MERDIAN-DESK-CHANGELOG.md', 'docs/MERDIAN-DESK-LICENSE-MODIFICATIONS.md']
    evidence_path = bundle / 'merdian-build-evidence.json'
    evidence = json.loads(evidence_path.read_text(encoding='utf-8-sig'))
    if (evidence.get('compiled') is not True or evidence.get('installed') is not False
            or evidence.get('executed') is not False or evidence.get('published') is not False
            or evidence.get('packaged') is not False):
        raise ValueError('Build evidence must describe a compiled, unpackaged, uninstalled, unexecuted, unpublished review bundle')
    app_commit = evidence['source_commit']
    if len(app_commit) != 40 or any(c not in '0123456789abcdef' for c in app_commit.lower()):
        raise ValueError('Build source commit is invalid')
    require_clean_sources(app_commit, code_files)
    actual = inventory(bundle)
    expected = normalized_inventory(evidence['files'])
    measured = normalized_inventory(row for row in actual if row['path'] != evidence_path.name)
    if measured != expected:
        raise ValueError('Complete input bundle no longer matches the build evidence hashes')
    exe = bundle / ('Merdian-Desk.exe' if (bundle / PAYLOAD).exists() else 'rustdesk.exe')
    require_x64_pe(exe)
    for name in ['librustdesk.dll', 'flutter_windows.dll']:
        require_x64_pe(bundle / name)
    if (bundle / 'rustdesk.exe').exists() and (bundle / PAYLOAD).exists():
        raise ValueError('Input contains two competing executable launch names')
    return {
        'purpose': 'merdian-desk-owned-portable-review-package', 'source_commit': source_commit,
        'artifacts_root': str(ARTIFACTS),
        'app_source_commit': app_commit, 'input_bundle_path': str(bundle), 'input_bundle_hashes': actual,
        'input_build_evidence_sha256': sha256(evidence_path), 'input_executable': str(exe),
        'source_hashes': {relative: sha256(REPO / relative) for relative in code_files},
        'output_path': str(output / PAYLOAD), 'installed': False, 'executed': False,
        'published': False, 'signed': False, 'defender_qualified': False, 'attended_session_verified': False,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--phase', choices=['validate', 'package'], default='validate')
    parser.add_argument('--bundle', required=True)
    parser.add_argument('--output', required=True)
    parser.add_argument('--quality', type=int, choices=range(12), default=9)
    options = parser.parse_args()
    bundle, output = Path(options.bundle).resolve(), Path(options.output).resolve()
    result = validate(bundle, output)
    result['checked_utc'] = datetime.now(timezone.utc).isoformat()
    result['packaged'] = False
    result['state'] = 'Validated'
    if options.phase == 'package':
        output.mkdir(parents=True, exist_ok=False)
        payload = output / 'payload'
        shutil.copytree(bundle, payload)
        original_name = Path(result['input_executable']).name
        if original_name != PAYLOAD:
            (payload / original_name).rename(payload / PAYLOAD)
        shutil.copy2(REPO / 'LICENCE', payload / 'LICENSE-AGPL-3.0.txt')
        for name in ['MERDIAN-DESK-CHANGELOG.md', 'MERDIAN-DESK-LICENSE-MODIFICATIONS.md']:
            shutil.copy2(REPO / 'docs' / name, payload / name)
        result['embedded_payload_hashes'] = inventory(payload)
        if sha256(payload / PAYLOAD) != sha256(Path(result['input_executable'])):
            raise ValueError('Branded launch rename changed executable bytes')
        command = [sys.executable, str(REPO / 'libs/portable/generate.py'), '-f', str(payload), '-o', str(REPO / 'libs/portable'),
                   '-e', PAYLOAD, '--target', 'x86_64-pc-windows-msvc', '--level', str(options.quality)]
        subprocess.run(command, cwd=REPO, check=True)
        target = Path(os.environ.get('CARGO_TARGET_DIR', REPO / 'target')).resolve()
        built = target / 'x86_64-pc-windows-msvc/release/merdian-desk-portable.exe'
        destination = output / PAYLOAD
        shutil.copy2(built, destination)
        require_x64_pe(destination)
        result['version_info'] = version_info(destination)
        result['manifest_info'] = manifest_info(destination)
        result['output_size'] = destination.stat().st_size
        result['output_sha256'] = sha256(destination)
        result['embedded_blob_sha256'] = sha256(REPO / 'libs/portable/data.bin')
        result['packaged'] = True
        result['state'] = 'PackagedForReview'
        result['package_invocation_argv'] = command
    ARTIFACTS.mkdir(parents=True, exist_ok=True)
    proof = ARTIFACTS / 'fork-package-result.json'
    proof.write_text(json.dumps(result, indent=2) + '\n', encoding='utf-8')
    print(json.dumps({'state': result['state'], 'proof_path': str(proof), 'source_commit': result['source_commit'],
                      'output_path': result['output_path'], 'output_sha256': result.get('output_sha256'),
                      'installed': False, 'executed': False, 'published': False}))


if __name__ == '__main__':
    main()
