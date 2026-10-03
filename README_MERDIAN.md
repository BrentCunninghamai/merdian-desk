# Merdian-Desk

Merdian-Desk is a modified RustDesk 1.5.0 client for attended support on Windows 11 x64, build 22000 or later. It uses RustDesk's existing remote desktop engine and codecs. The companion rendezvous and relay server remains unmodified RustDesk Server.

This pilot starts only in a signed-in user's normal, non-elevated session. The host must approve each new connection with **Accept**. **Disconnect** ends the current session; **Exit Merdian-Desk** closes the host and its tracked children. Installation, Windows services, startup persistence, elevation, unattended password approval and automatic updates are disabled in this pilot. The backend pins `relay.meridianremote.site` and its public key, forces relay use, and rejects direct addresses or alternate servers.

The new app name, teal icon, host guide and Windows identity are part of this source. The upstream copyright and AGPL license are retained. See [modification notes](docs/MERDIAN-DESK-CHANGELOG.md), [source and license notice](docs/MERDIAN-DESK-LICENSE-MODIFICATIONS.md), and [LICENSE](LICENSE).

## Build and qualification status

The first Windows build is in progress. A source commit and passing focused tests alone do not establish a usable or customer-ready executable. Before distributing a branded binary, record its exact source revision, complete corresponding source and build information, executable hashes, signing status, protected security scans and an actual attended session showing Accept, relay use, Disconnect, fresh approval on reconnect and Exit. The original official-client pilot is kept separately while the fork qualifies.

Base source: RustDesk tag `1.5.0`, commit `fada664df7a294d1d1a9ca3e7cd3637069122f17`; `hbb_common` submodule `229b904508364c8997aad0fb5af57effac859f60`. The initial fork branch is `merdian-desk/pilot` in [BrentCunninghamai/merdian-desk](https://github.com/BrentCunninghamai/merdian-desk).

The Windows toolchain follows the upstream tag's workflow: Rust 1.75.0, Flutter 3.24.5 / Dart 3.5.4, LLVM 15.0.6, Visual Studio 2022 C++, and vcpkg commit `9e593bb18ea69cc5095e012465dcd675a822ed0d` with `x64-windows-static` native libraries. RustDesk's matching custom Flutter engine and dropdown patch are also required. Generated Rust/Flutter bridge files use `flutter_rust_bridge_codegen` 1.80.1. The build workflow records these dependencies and validates the actual output instead of trusting an inner build command's return code.

Do not run this pilot on older Windows builds, Windows Server, ARM64 or x86. Do not weaken antivirus protection, override a browser security block, restore quarantined files, or install an unattended service to complete a test.
