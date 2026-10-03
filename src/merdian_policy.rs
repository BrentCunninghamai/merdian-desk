//! Owned attended-only Windows pilot policy. RustDesk's engine/codecs remain.
pub use crate::merdian_policy_model::{APP_NAME, PUBLIC_KEY, RELAY};
use hbb_common::config;

pub fn active() -> bool {
    true
}

pub fn apply() {
    // Set namespace before any lazy configuration/IPC/installation lookup.
    *config::APP_NAME.write().unwrap() = APP_NAME.to_owned();
    *config::EXE_RENDEZVOUS_SERVER.write().unwrap() = RELAY.to_owned();
    let mut options = config::OVERWRITE_SETTINGS.write().unwrap();
    for (key, value) in [
        ("custom-rendezvous-server", RELAY),
        ("relay-server", RELAY),
        ("key", PUBLIC_KEY),
        ("api-server", ""),
        ("approve-mode", "click"),
        ("verification-method", "use-temporary-password"),
        ("allow-remote-config-modification", "N"),
        ("allow-auto-update", "N"),
        ("allow-hide-cm", "N"),
        ("enable-trusted-devices", "N"),
        ("hide-elevate-button-in-accept-window", "Y"),
        ("enable-remote-restart", "N"),
        ("enable-remote-printer", "N"),
        ("enable-privacy-mode", "N"),
        ("enable-block-input", "N"),
        ("enable-terminal", "N"),
        ("enable-tunnel", "N"),
        ("force-always-relay", "Y"),
        ("direct-server", "N"),
    ] {
        options.insert(key.to_owned(), value.to_owned());
    }
    drop(options);
    let mut local = config::OVERWRITE_LOCAL_SETTINGS.write().unwrap();
    local.insert("pre-elevate-service".into(), "N".into());
    drop(local);
    let mut hard = config::HARD_SETTINGS.write().unwrap();
    for key in ["disable-installation", "disable-ab", "disable-account"] {
        hard.insert(key.into(), "Y".into());
    }
    drop(hard);
    let mut builtin = config::BUILTIN_SETTINGS.write().unwrap();
    for (key, value) in [
        ("allow-logon-screen-password", "N"),
        ("hide-security-setting", "Y"),
        ("hide-network-setting", "Y"),
        ("hide-remote-printer-setting", "Y"),
        ("hide-cm-remote-printer", "Y"),
        ("hide-stop-service", "Y"),
    ] {
        builtin.insert(key.into(), value.into());
    }
}

pub fn require_startup() -> Result<(), String> {
    apply();
    crate::merdian_policy_model::validate_args(&std::env::args().skip(1).collect::<Vec<_>>())
        .map_err(str::to_owned)?;
    #[cfg(windows)]
    {
        use winreg::{enums::HKEY_LOCAL_MACHINE, RegKey};
        let version = RegKey::predef(HKEY_LOCAL_MACHINE)
            .open_subkey("SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion")
            .map_err(|_| "Cannot verify Windows 11 version".to_owned())?;
        let build: String = version
            .get_value("CurrentBuildNumber")
            .map_err(|_| "Cannot verify Windows build".to_owned())?;
        let edition: String = version
            .get_value("InstallationType")
            .map_err(|_| "Cannot verify Windows client edition".to_owned())?;
        let build = build
            .parse::<u32>()
            .map_err(|_| "Invalid Windows build".to_owned())?;
        if !crate::merdian_policy_model::supported_windows(
            build,
            edition == "Client",
            cfg!(target_arch = "x86_64") && crate::platform::is_x64(),
        ) {
            return Err("Merdian-Desk requires Windows 11 x64 build 22000 or later.".into());
        }
        if crate::platform::is_elevated(None)
            .map_err(|_| "Cannot verify normal-user token".to_owned())?
            || crate::platform::is_prelogin()
        {
            return Err("Run Merdian-Desk in a signed-in user's normal non-elevated session; elevation and system use are disabled.".into());
        }
        Ok(())
    }
    #[cfg(not(windows))]
    Err("Merdian-Desk pilot supports Windows 11 x64 only.".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use hbb_common::config::Config;
    #[test]
    fn local_config_and_repeated_external_initialization_cannot_downgrade_click_or_pin() {
        apply();
        for (key, value) in [
            ("approve-mode", "password"),
            ("verification-method", "use-permanent-password"),
            ("custom-rendezvous-server", "evil.invalid"),
            ("key", "wrong"),
            ("allow-remote-config-modification", "Y"),
        ] {
            Config::set_option(key.into(), value.into());
        }
        crate::read_custom_client("malformed external configuration");
        assert_eq!(Config::get_option("approve-mode"), "click");
        assert_eq!(
            Config::get_option("verification-method"),
            "use-temporary-password"
        );
        assert_eq!(Config::get_option("custom-rendezvous-server"), RELAY);
        assert_eq!(Config::get_option("key"), PUBLIC_KEY);
        assert_eq!(Config::get_option("allow-remote-config-modification"), "N");
        assert!(config::is_disable_installation());
        assert_eq!(&*config::APP_NAME.read().unwrap(), APP_NAME);
    }
}
