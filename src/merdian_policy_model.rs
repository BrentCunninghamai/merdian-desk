//! Dependency-free policy decisions shared by the Windows attended fork and its
//! focused tests. Network credentials never count as local session consent.
pub const APP_NAME: &str = "Merdian-Desk";
pub const RELAY: &str = "relay.meridianremote.site";
pub const PUBLIC_KEY: &str = "wXOaIJn3BpQ4ss1bpTSEwDkbntNljbnNzGzUUmpEIug=";

pub fn supported_windows(build: u32, client_edition: bool, x64: bool) -> bool {
    build >= 22_000 && client_edition && x64
}

/// A peer ID may request the relay suffix, but cannot redirect this client to a
/// direct IP, another rendezvous server, or a URI carrying another key.
pub fn validate_peer_id(peer: &str) -> Result<(), &'static str> {
    let id = peer.strip_suffix("/r").unwrap_or(peer);
    if id.is_empty()
        || id.len() > 64
        || !id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
    {
        return Err(
            "Enter the host's Merdian-Desk ID. Direct addresses and other servers are disabled.",
        );
    }
    Ok(())
}

pub fn validate_args(args: &[String]) -> Result<(), &'static str> {
    for arg in args {
        let flag = arg.split('=').next().unwrap_or(arg).to_ascii_lowercase();
        if !flag.starts_with('-') {
            continue;
        }
        match flag.as_str() {
            "--noinstall" | "--no-server" | "--connect" | "--relay" | "--cm"
            | "--whiteboard" | "--play" | "--file-transfer" | "--view-camera"
            | "--version" | "--build-date" | "--check-hwcodec-config" => {}
            _ => return Err("Merdian-Desk supports only interactive attended commands; install, service, startup, update and elevation commands are disabled."),
        }
    }
    Ok(())
}

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub enum SessionConsent {
    #[default]
    Pending,
    Accepted,
    Closed,
}
impl SessionConsent {
    pub fn accept_local(&mut self) -> bool {
        if *self == Self::Closed {
            return false;
        }
        *self = Self::Accepted;
        true
    }
    pub fn can_authorize(self) -> bool {
        self == Self::Accepted
    }
    pub fn close(&mut self) {
        *self = Self::Closed;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn win11_boundary_excludes_server_and_x86() {
        assert!(!supported_windows(21_999, true, true));
        assert!(supported_windows(22_000, true, true));
        assert!(supported_windows(26_100, true, true));
        assert!(!supported_windows(26_100, false, true));
        assert!(!supported_windows(26_100, true, false));
    }
    #[test]
    fn unsafe_commands_rejected_even_when_nested_after_connect() {
        for unsafe_arg in [
            "--install",
            "--silent-install",
            "--install-service",
            "--service",
            "--server",
            "--tray",
            "--update",
            "--elevate",
            "--run-as-system",
            "--quick_support",
            "--portable-service-shmem-name=x",
            "--config",
            "--option",
            "--password",
            "--deploy",
            "--INSTALL",
        ] {
            let args = vec![
                "--no-server".into(),
                "--connect".into(),
                "123456789".into(),
                unsafe_arg.into(),
            ];
            assert!(
                validate_args(&args).is_err(),
                "unsafe nested argument accepted: {unsafe_arg}"
            );
        }
        assert!(validate_args(&vec![
            "--no-server".into(),
            "--connect".into(),
            "123456789".into(),
            "--relay".into()
        ])
        .is_ok());
        assert!(validate_args(&vec!["--cm".into()]).is_ok());
        assert!(validate_args(&vec!["--noinstall".into()]).is_ok());
    }
    #[test]
    fn fresh_connection_and_reconnect_each_need_local_accept() {
        let mut first = SessionConsent::default();
        assert!(!first.can_authorize());
        assert!(first.accept_local());
        assert!(first.can_authorize());
        first.close();
        assert!(!first.can_authorize());
        assert!(!first.accept_local());
        let reconnect = SessionConsent::default();
        assert!(!reconnect.can_authorize());
    }
    #[test]
    fn peer_ids_cannot_redirect_pinned_relay() {
        for id in ["123456789", "123456789/r", "my-host_1"] {
            assert!(validate_peer_id(id).is_ok());
        }
        for id in [
            "",
            "192.168.1.2",
            "[::1]:21118",
            "localhost:21118",
            "123@public",
            "123@evil.invalid?key=wrong",
            "rustdesk://connection/123",
            "123/r/r",
        ] {
            assert!(validate_peer_id(id).is_err(), "redirect accepted: {id}");
        }
    }
}
