#![windows_subsystem = "windows"]
//! Merdian-Desk's attended, normal-user portable packaging of RustDesk.
use std::path::PathBuf;

mod bin_reader;
mod policy;
#[cfg(windows)]
mod ui;
#[cfg(windows)]
mod win;

fn setup(reader: bin_reader::BinaryReader) -> Result<PathBuf, String> {
    policy::validate_payload_exe(&reader.exe).map_err(str::to_owned)?;
    let local = dirs::data_local_dir().ok_or("Cannot locate the user's LocalAppData directory")?;
    let root = policy::fresh_extraction_dir(&local).map_err(|error| error.to_string())?;
    #[cfg(windows)]
    let origin = win::download_origin()?;
    for file in &reader.files {
        let path = file.write_to_file(&root)?;
        #[cfg(windows)]
        win::preserve_download_origin(&path, origin.as_deref())?;
    }
    let exe = policy::resolve_payload_path(&root, &reader.exe)
        .ok_or("Invalid embedded executable path")?;
    if !exe.is_file() {
        return Err(
            "The extracted client is unavailable. Stop if Windows security blocked it.".into(),
        );
    }
    Ok(exe)
}

fn run() -> Result<(), String> {
    #[cfg(windows)]
    win::require_attended_platform()?;
    #[cfg(not(windows))]
    return Err("Merdian-Desk requires Windows 11 x64 build 22000 or later.".into());

    let args = policy::launch_args(&std::env::args().skip(1).collect::<Vec<_>>())
        .map_err(str::to_owned)?;
    let reader = bin_reader::BinaryReader::new()?;
    policy::validate_payload_exe(&reader.exe).map_err(str::to_owned)?;
    #[cfg(windows)]
    let splash = ui::setup();
    let prepared = setup(reader);
    #[cfg(windows)]
    if splash.close() {
        return Ok(());
    }
    let exe = prepared?;
    #[cfg(windows)]
    win::execute_normal(&exe, &args)?;
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        #[cfg(windows)]
        win::show_error(&error);
        #[cfg(not(windows))]
        eprintln!("{}", error);
        std::process::exit(1);
    }
}
