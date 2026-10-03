//! Windows launch keeps security dialogs and origin information intact.
use std::{ffi::OsStr, os::windows::ffi::OsStrExt, path::Path, ptr::null_mut};
use winapi::um::{
    handleapi::CloseHandle,
    jobapi2::{AssignProcessToJobObject, CreateJobObjectW, SetInformationJobObject},
    processthreadsapi::{
        GetCurrentProcess, GetCurrentProcessId, OpenProcessToken, ProcessIdToSessionId,
        TerminateProcess,
    },
    securitybaseapi::GetTokenInformation,
    shellapi::{
        ShellExecuteExW, SEE_MASK_NOASYNC, SEE_MASK_NOCLOSEPROCESS, SEE_MASK_UNICODE,
        SHELLEXECUTEINFOW,
    },
    synchapi::WaitForSingleObject,
    sysinfoapi::{GetNativeSystemInfo, SYSTEM_INFO},
    winbase::{INFINITE, WAIT_FAILED},
    winnt::{
        JobObjectExtendedLimitInformation, TokenElevation, HANDLE,
        JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE, OSVERSIONINFOEXW,
        TOKEN_ELEVATION, TOKEN_QUERY, VER_NT_WORKSTATION,
    },
    winuser::{MessageBoxW, MB_ICONERROR, MB_OK, SW_SHOWNORMAL},
};

#[link(name = "ntdll")]
extern "system" {
    fn RtlGetVersion(version: *mut OSVERSIONINFOEXW) -> i32;
}

struct OwnedHandle(HANDLE);
impl Drop for OwnedHandle {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}
fn wide(value: impl AsRef<OsStr>) -> Vec<u16> {
    value.as_ref().encode_wide().chain(Some(0)).collect()
}
fn os_error(context: &str) -> String {
    format!("{context}: {}", std::io::Error::last_os_error())
}

pub fn require_attended_platform() -> Result<(), String> {
    unsafe {
        let mut version: OSVERSIONINFOEXW = std::mem::zeroed();
        version.dwOSVersionInfoSize = std::mem::size_of::<OSVERSIONINFOEXW>() as u32;
        let mut system: SYSTEM_INFO = std::mem::zeroed();
        GetNativeSystemInfo(&mut system);
        if RtlGetVersion(&mut version) < 0
            || !crate::policy::client_policy::supported_windows(
                version.dwBuildNumber,
                version.wProductType == VER_NT_WORKSTATION,
                cfg!(target_arch = "x86_64") && system.u.s().wProcessorArchitecture == 9,
            )
        {
            return Err("Merdian-Desk requires Windows 11 x64 build 22000 or later.".into());
        }
        let mut session = 0;
        if ProcessIdToSessionId(GetCurrentProcessId(), &mut session) == 0 || session == 0 {
            return Err("Open Merdian-Desk in a signed-in user's interactive session.".into());
        }
        let mut token = null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
            return Err(os_error("Cannot verify normal-user token"));
        }
        let token = OwnedHandle(token);
        let mut elevation: TOKEN_ELEVATION = std::mem::zeroed();
        let mut length = 0;
        if GetTokenInformation(
            token.0,
            TokenElevation,
            (&mut elevation as *mut TOKEN_ELEVATION).cast(),
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut length,
        ) == 0
        {
            return Err(os_error("Cannot verify non-elevated token"));
        }
        if elevation.TokenIsElevated != 0 {
            return Err("Run Merdian-Desk normally. Installation, administrator and system use are disabled.".into());
        }
    }
    if std::env::var_os("SEE_MASK_NOZONECHECKS").is_some() {
        return Err(
            "Merdian-Desk cannot launch from an environment that disables Windows zone checks."
                .into(),
        );
    }
    Ok(())
}

pub fn download_origin() -> Result<Option<Vec<u8>>, String> {
    let exe = std::env::current_exe().map_err(|error| error.to_string())?;
    let stream = format!("{}:Zone.Identifier", exe.display());
    match std::fs::read(stream) {
        Ok(origin) if origin.len() <= 8192 => Ok(Some(origin)),
        Ok(_) => Err("Downloaded executable origin metadata is unexpectedly large.".into()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!(
            "Cannot retain downloaded executable origin: {error}"
        )),
    }
}
pub fn preserve_download_origin(path: &Path, origin: Option<&[u8]>) -> Result<(), String> {
    if let Some(origin) = origin {
        let stream = format!("{}:Zone.Identifier", path.display());
        std::fs::write(&stream, origin)
            .map_err(|error| format!("Cannot retain Windows download-origin metadata: {error}"))?;
        if std::fs::read(&stream).map_err(|error| error.to_string())? != origin {
            return Err("Windows download-origin metadata did not survive extraction.".into());
        }
    }
    Ok(())
}

pub fn execute_normal(exe: &Path, args: &[String]) -> Result<(), String> {
    require_attended_platform()?;
    crate::policy::launch_args(args).map_err(str::to_owned)?;
    let parameters = args
        .iter()
        .map(|arg| crate::policy::quote_windows_arg(arg))
        .collect::<Vec<_>>()
        .join(" ");
    let file = wide(exe);
    let directory = wide(exe.parent().ok_or("Missing extraction directory")?);
    let verb = wide("open");
    let parameters = wide(parameters);
    unsafe {
        let job = CreateJobObjectW(null_mut(), null_mut());
        if job.is_null() {
            return Err(os_error("Cannot create attended process scope"));
        }
        let job = OwnedHandle(job);
        let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
        limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        if SetInformationJobObject(
            job.0,
            JobObjectExtendedLimitInformation,
            (&mut limits as *mut JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
            std::mem::size_of_val(&limits) as u32,
        ) == 0
        {
            return Err(os_error("Cannot contain attended child processes"));
        }
        let mut launch: SHELLEXECUTEINFOW = std::mem::zeroed();
        launch.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
        // Never use runas, NOZONECHECKS, FLAG_NO_UI, hidden windows or cmd.exe.
        launch.fMask = SEE_MASK_NOCLOSEPROCESS | SEE_MASK_NOASYNC | SEE_MASK_UNICODE;
        launch.lpVerb = verb.as_ptr();
        launch.lpFile = file.as_ptr();
        launch.lpParameters = parameters.as_ptr();
        launch.lpDirectory = directory.as_ptr();
        launch.nShow = SW_SHOWNORMAL;
        std::env::set_var("RUSTDESK_APPNAME", crate::policy::PAYLOAD_EXE);
        if ShellExecuteExW(&mut launch) == 0 {
            return Err(os_error(
                "Windows did not open Merdian-Desk. Stop if security blocked the file",
            ));
        }
        if launch.hProcess.is_null() {
            return Err("Windows returned no handle for the attended process.".into());
        }
        let process = OwnedHandle(launch.hProcess);
        if AssignProcessToJobObject(job.0, process.0) == 0 {
            let error = os_error("Cannot contain the attended process; launch stopped");
            TerminateProcess(process.0, 1);
            return Err(error);
        }
        if WaitForSingleObject(process.0, INFINITE) == WAIT_FAILED {
            return Err(os_error("Cannot monitor attended process lifetime"));
        }
        // Closing this job also stops any connection-manager child left after Exit.
        drop(job);
    }
    Ok(())
}

pub fn show_error(error: &str) {
    let text = wide(format!(
        "{error}\n\nKeep Windows security enabled. Do not bypass a warning or install a service."
    ));
    let title = wide("Merdian-Desk could not open");
    unsafe {
        MessageBoxW(
            null_mut(),
            text.as_ptr(),
            title.as_ptr(),
            MB_OK | MB_ICONERROR,
        );
    }
}
