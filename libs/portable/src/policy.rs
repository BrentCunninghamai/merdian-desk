//! Portable packaging decisions. This wrapper never installs or elevates.
use std::{
    io,
    path::{Component, Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
#[path = "../../../src/merdian_policy_model.rs"]
pub mod client_policy;
pub const APP_PREFIX: &str = "Merdian-Desk";
pub const PAYLOAD_EXE: &str = "Merdian-Desk.exe";

pub fn validate_payload_exe(exe: &str) -> Result<(), &'static str> {
    if exe.replace('\\', "/").trim_start_matches("./") != PAYLOAD_EXE {
        return Err(
            "The portable payload must launch Merdian-Desk.exe in its own extraction directory.",
        );
    }
    Ok(())
}
pub fn launch_args(args: &[String]) -> Result<Vec<String>, &'static str> {
    client_policy::validate_args(args)?;
    Ok(args.to_vec())
}
pub fn resolve_payload_path(dir: &Path, relative: &str) -> Option<PathBuf> {
    let normalized = relative.replace('\\', "/");
    let mut result = dir.to_path_buf();
    let mut found = false;
    for part in Path::new(&normalized).components() {
        match part {
            Component::CurDir => {}
            Component::Normal(part) => {
                let name = part.to_str()?;
                let stem = name.split('.').next()?.to_ascii_uppercase();
                let reserved = ["CON", "PRN", "AUX", "NUL"].contains(&stem.as_str())
                    || (stem.len() == 4
                        && (stem.starts_with("COM") || stem.starts_with("LPT"))
                        && stem.as_bytes()[3].is_ascii_digit());
                if name.is_empty()
                    || name.ends_with(['.', ' '])
                    || reserved
                    || name
                        .chars()
                        .any(|c| c.is_control() || ":*?\"<>|".contains(c))
                {
                    return None;
                }
                result.push(part);
                found = true;
            }
            _ => return None,
        }
    }
    found.then_some(result)
}
fn plain_directory(path: &Path) -> io::Result<()> {
    let metadata = std::fs::symlink_metadata(path)?;
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if metadata.file_attributes() & 0x400 != 0 {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "Portable extraction cannot follow a reparse point",
            ));
        }
    }
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "Portable extraction requires a plain directory",
        ));
    }
    Ok(())
}
fn ensure_directory(path: &Path) -> io::Result<()> {
    match std::fs::create_dir(path) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
        Err(error) => return Err(error),
    }
    plain_directory(path)
}
pub fn fresh_extraction_dir(local_data: &Path) -> io::Result<PathBuf> {
    plain_directory(local_data)?;
    let app = local_data.join(APP_PREFIX);
    ensure_directory(&app)?;
    let portable = app.join("portable");
    ensure_directory(&portable)?;
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| {
            io::Error::new(
                io::ErrorKind::Other,
                "Cannot timestamp extraction directory",
            )
        })?
        .as_nanos();
    for attempt in 0..100 {
        let run = portable.join(format!("run-{}-{stamp}-{attempt}", std::process::id()));
        match std::fs::create_dir(&run) {
            Ok(()) => {
                plain_directory(&run)?;
                return Ok(run);
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "Cannot allocate a fresh portable directory",
    ))
}
pub fn prepare_parent(root: &Path, target: &Path) -> io::Result<()> {
    plain_directory(root)?;
    let relative = target.strip_prefix(root).map_err(|_| {
        io::Error::new(
            io::ErrorKind::PermissionDenied,
            "Package path escaped extraction root",
        )
    })?;
    let mut current = root.to_path_buf();
    if let Some(parent) = relative.parent() {
        for part in parent.components() {
            match part {
                Component::Normal(part) => {
                    current.push(part);
                    ensure_directory(&current)?;
                }
                _ => {
                    return Err(io::Error::new(
                        io::ErrorKind::PermissionDenied,
                        "Invalid package parent path",
                    ))
                }
            }
        }
    }
    Ok(())
}
/// Windows CRT escaping preserves argument boundaries without invoking cmd.exe.
pub fn quote_windows_arg(arg: &str) -> String {
    let mut result = String::from("\"");
    let mut slashes = 0;
    for c in arg.chars() {
        if c == '\\' {
            slashes += 1;
            continue;
        }
        if c == '"' {
            result.push_str(&"\\".repeat(slashes * 2 + 1));
        } else {
            result.push_str(&"\\".repeat(slashes));
        }
        slashes = 0;
        result.push(c);
    }
    result.push_str(&"\\".repeat(slashes * 2));
    result.push('"');
    result
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn forwards_normal_arguments_without_install_or_quick_support() {
        assert_eq!(launch_args(&[]).unwrap(), Vec::<String>::new());
        let args = vec![
            "--no-server".into(),
            "--connect".into(),
            "123456789".into(),
            "--relay".into(),
        ];
        assert_eq!(launch_args(&args).unwrap(), args);
        for flag in [
            "--silent-install",
            "--install",
            "--quick_support",
            "--service",
            "--elevate",
            "--run-as-system",
        ] {
            assert!(launch_args(&vec!["--connect".into(), "123".into(), flag.into()]).is_err());
        }
    }
    #[test]
    fn launch_path_cannot_select_stock_or_outside_executable() {
        assert!(validate_payload_exe("./Merdian-Desk.exe").is_ok());
        assert!(validate_payload_exe(".\\Merdian-Desk.exe").is_ok());
        for exe in [
            "rustdesk.exe",
            "../Merdian-Desk.exe",
            "C:\\Program Files\\RustDesk\\RustDesk.exe",
            "sub/Merdian-Desk.exe",
        ] {
            assert!(validate_payload_exe(exe).is_err());
        }
    }
    #[test]
    fn paths_cannot_traverse_use_ads_or_device_names() {
        let base = Path::new("base");
        assert_eq!(
            resolve_payload_path(base, ".\\data\\flutter_assets\\x.bin"),
            Some(base.join("data").join("flutter_assets").join("x.bin"))
        );
        for path in [
            "../x",
            "/x",
            "C:\\x",
            "C:x",
            "x:Zone.Identifier",
            "CON.txt",
            "data/AUX",
            "a./x",
            "data/../x",
            "//server/share/x",
        ] {
            assert!(
                resolve_payload_path(base, path).is_none(),
                "unsafe package path accepted: {path}"
            );
        }
    }
    #[test]
    fn windows_argument_quoting_preserves_boundaries() {
        assert_eq!(quote_windows_arg(""), "\"\"");
        assert_eq!(quote_windows_arg("123 456"), "\"123 456\"");
        assert_eq!(quote_windows_arg("a\"b"), "\"a\\\"b\"");
        assert_eq!(quote_windows_arg("C:\\dir\\"), "\"C:\\dir\\\\\"");
    }
    #[test]
    fn every_launch_is_fresh_and_stock_files_are_unchanged() {
        let test_root = std::env::temp_dir().join(format!(
            "merdian-portable-model-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&test_root).unwrap();
        let stock = test_root.join("rustdesk");
        std::fs::create_dir(&stock).unwrap();
        std::fs::write(stock.join("rustdesk.exe"), b"stock-sentinel").unwrap();
        let first = fresh_extraction_dir(&test_root).unwrap();
        let second = fresh_extraction_dir(&test_root).unwrap();
        assert_ne!(first, second);
        assert!(first.starts_with(test_root.join(APP_PREFIX).join("portable")));
        assert!(first.read_dir().unwrap().next().is_none());
        assert_eq!(
            std::fs::read(stock.join("rustdesk.exe")).unwrap(),
            b"stock-sentinel"
        );
        for run in [first, second] {
            std::fs::remove_dir(run).unwrap();
        }
        std::fs::remove_dir(test_root.join(APP_PREFIX).join("portable")).unwrap();
        std::fs::remove_dir(test_root.join(APP_PREFIX)).unwrap();
        std::fs::remove_file(stock.join("rustdesk.exe")).unwrap();
        std::fs::remove_dir(stock).unwrap();
        std::fs::remove_dir(test_root).unwrap();
    }
}
