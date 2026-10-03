# Merdian-Desk owned portable package

This is the modified RustDesk 1.5.0 portable wrapper for the Windows 11 x64 attended pilot. It embeds a complete, matching application bundle and launches `Merdian-Desk.exe`. The RustDesk remote engine, codecs and upstream container format remain in use. The wrapper and client modifications are available under AGPL-3.0 with the corresponding source at <https://github.com/BrentCunninghamai/merdian-desk>.

Before extracting files, the wrapper requires a native x64 Windows client edition at build 22000 or later, an interactive signed-in user, and a non-elevated token. Unsupported operating systems, architecture, administrator contexts and rejected command arguments receive a visible error. Installation, services, startup, update and elevation commands are rejected using the application's shared policy. Normal accepted arguments are forwarded unchanged; the executable filename never injects install or quick-support commands.

Every launch extracts into a new plain directory below `%LOCALAPPDATA%\Merdian-Desk\portable\run-...`. It rejects reparse points, traversal, alternate data streams in package paths, duplicated paths and corrupt data, and never reuses the installed RustDesk executable or its extraction directory. The owned product metadata, icon and `asInvoker` manifest identify Merdian-Desk. The package is not code-signed by this helper.

If Windows attached download-origin metadata to the wrapper, the wrapper preserves that metadata on every extracted file. `ShellExecuteExW` uses the normal `open` verb with visible Windows security dialogs. It does not request elevation, suppress zone checks, suppress security dialogs or invoke a shell command. The wrapper waits for the main client; its kill-on-close job also contains children assigned to the attended process. The client separately closes its tracked session children on Exit. Actual lifecycle behavior still requires a runtime test. Extracted files remain on disk after closing; they do not register startup, a service or an installation.

## Input evidence

Use a complete, successfully compiled application bundle containing the native x64 launcher (`rustdesk.exe` or `Merdian-Desk.exe`), `librustdesk.dll`, `flutter_windows.dll`, the complete assets and other required DLLs. Its `merdian-build-evidence.json` must contain:

```json
{
  "source_commit": "exact 40-character application source commit",
  "compiled": true,
  "packaged": false,
  "installed": false,
  "executed": false,
  "published": false,
  "files": [
    {"path": "relative/path", "size": 123, "sha256": "64-character SHA256"}
  ]
}
```

The file inventory includes every bundle file except the evidence file itself. Paths may use Windows separators; comparison normalizes separators and SHA256 case while rejecting duplicate Windows path aliases. The helper reads PE headers without executing the files.

Commit the reviewed wrapper, core and UI changes before validating. The helper requires its source inputs to match HEAD, rejects pending engine/UI changes, and compares the application's source commit with current engine/UI sources. A bundle compiled before the Exit/X fix must be rebuilt. Wrapper-only commits may follow a matching application commit.

## Build on this workspace

Use a developer terminal with the reviewed MSVC toolchain. The environment below is process-scoped. Brotli 1.2.0 is already available in this workspace's Python dependency folder; `requirements.txt` records that version.

```powershell
$taskRoot = 'F:\remote desktop app'
$env:CARGO_HOME = "$taskRoot\.tools\cargo"
$env:RUSTUP_HOME = "$taskRoot\.tools\rustup"
$env:RUSTC = "$taskRoot\.tools\rustup\toolchains\1.75.0-x86_64-pc-windows-msvc\bin\rustc.exe"
$env:PATH = "$(Split-Path $env:RUSTC);$taskRoot\.tools\build-bin;$env:PATH"
$env:PYTHONPATH = "$taskRoot\.tools\portable-python"
$env:CARGO_TARGET_DIR = "$taskRoot\.tools\portable-package"
$env:MERDIAN_ARTIFACTS_ROOT = "$taskRoot\artifacts"
Set-Location "$taskRoot\fork\rustdesk"
# Replace this path with the complete matching bundle produced by the build.
$taskBundle = '<complete matching application bundle>'
$taskOutput = "$taskRoot\artifacts\fp1"
C:\Python313\python.exe libs/portable/package_merdian.py --phase validate --bundle $taskBundle --output $taskOutput
C:\Python313\python.exe libs/portable/package_merdian.py --phase package --bundle $taskBundle --output $taskOutput
```

Use a fresh short output folder on each retry. The helper preserves failed attempts and never deletes or overwrites their folders. Packaging copies the complete bundle, changes only the launch filename when necessary, retains its exact executable bytes, and adds the license and modification notes. It runs:

```text
python libs/portable/generate.py -f <copied payload> -o libs/portable -e Merdian-Desk.exe --target x86_64-pc-windows-msvc --level 9
cargo build --locked --release --package rustdesk-portable-packer --bin merdian-desk-portable --target x86_64-pc-windows-msvc
```

The final review artifact is `<fresh output>\Merdian-Desk.exe`. The helper reads its x64 PE header, product metadata and manifest without launching it. It records the exact source commits, input inventory, source hashes, embedded inventory, output size/SHA256 and `installed`, `executed`, `published`, `signed`, `defender_qualified`, and `attended_session_verified` as false in `artifacts/fork-package-result.json`. Validation alone reports `packaged=false` and no output hash.

For a normal clone or Windows CI runner, install the standard x64 MSVC Rust toolchain matching this repository's `rust-version`, Visual Studio C++ build tools with the Windows SDK/resource compiler, and Python with `python -m pip install -r libs/portable/requirements.txt`. Run the helper from the clone with the same complete build evidence. Set `MERDIAN_ARTIFACTS_ROOT` to the intended absolute artifact directory and `CARGO_TARGET_DIR` to the intended Cargo output directory; both are process environment settings. For example:

```powershell
$env:MERDIAN_ARTIFACTS_ROOT = Join-Path $PWD 'artifacts'
$env:CARGO_TARGET_DIR = Join-Path $PWD 'target'
python libs/portable/package_merdian.py --phase validate --bundle '<matching bundle>' --output (Join-Path $env:MERDIAN_ARTIFACTS_ROOT 'fp1')
python libs/portable/package_merdian.py --phase package --bundle '<matching bundle>' --output (Join-Path $env:MERDIAN_ARTIFACTS_ROOT 'fp1')
```

Without `MERDIAN_ARTIFACTS_ROOT`, a repository located directly below a folder named `fork` uses that folder's parent `artifacts` directory; other clones use their own `artifacts` directory. The helper accepts only a fresh descendant of that exact selected root, never the root itself or a filesystem root. The proof is written at the selected root's `fork-package-result.json`.

Packaging is not distribution approval. Before publishing, obtain fresh protected Defender and normal browser-download results for the exact owned executable, then prove local Accept, relayed session and Disconnect/Exit on two distinct endpoints. Provide the complete corresponding source for the exact build and its dependency/submodule revisions. Keep Windows security enabled and stop on a warning.

## Focused verification

```text
python -m unittest discover -s libs/portable -p "test_*.py"
cargo check --locked --offline --package rustdesk-portable-packer --tests
cargo test --locked --offline --package rustdesk-portable-packer --tests
```

These commands exercise test runners and ordinary test data. They do not launch or install the remote-support application.
