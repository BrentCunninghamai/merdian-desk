# Merdian-Desk pilot modifications

2026-10-03 — first attended Windows 11 pilot fork of RustDesk 1.5.0.

The user approved a separately built branded client while retaining the unmodified official RustDesk pilot. This fork's source is on branch `merdian-desk/pilot`. These notes describe source changes; a successful build, protected download, and real attended relay session must be recorded separately before calling a binary ready.

- Added the Merdian-Desk app name, teal monogram, light and dark action colors, and Windows product metadata and icon. The native window class is separate from an installed RustDesk app; the existing Rust/Flutter ABI remains compatible.
- Added a host ID, Accept, Disconnect and Exit onboarding guide using existing Flutter widgets. Host approval and session controls retain the upstream connection-manager implementation.
- Main-window Exit and X await owned-session cleanup and terminate the Merdian-Desk process. Closing a remote session subwindow retains its existing session-close behavior; the attended host no longer hides in the background when its main window is closed.
- Removed password, install, update, service, elevation, camera, remote restart, input-blocking, privacy, terminal, tunnel, RDP, peer shortcut and wake-on-LAN affordances from this attended pilot's interface. The Rust policy separately enforces the attended-only behavior and relay configuration; hiding a control alone does not enforce policy.
- Kept the RustDesk copyright, powered-by disclosure, AGPL-3.0 notice and upstream links. The modified client must ship with corresponding modified source and build information under AGPL-3.0.

The additional onboarding copy is English for this Windows-only pilot. Existing upstream translated controls keep their translations. This change adds no new codec, protocol, service or dependency.

The existing official pilot executables remain unmodified RustDesk 1.5.0. Their provenance, Microsoft Defender investigation and password-gated pilot qualification are recorded in the parent project; they are not artifacts of this fork.
