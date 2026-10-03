fn main() {
    #[cfg(windows)]
    {
        use std::io::Write;
        let mut res = winres::WindowsResource::new();
        println!("cargo:rerun-if-changed=../../flutter/windows/runner/resources/merdian_desk.ico");
        println!("cargo:rerun-if-changed=res/merdian.manifest.xml");
        println!("cargo:rerun-if-changed=data.bin");
        res.set_icon("../../flutter/windows/runner/resources/merdian_desk.ico")
            .set_language(winapi::um::winnt::MAKELANGID(
                winapi::um::winnt::LANG_ENGLISH,
                winapi::um::winnt::SUBLANG_ENGLISH_US,
            ))
            .set_manifest_file("res/merdian.manifest.xml");
        match res.compile() {
            Err(e) => {
                write!(std::io::stderr(), "{}", e).unwrap();
                std::process::exit(1);
            }
            Ok(_) => {}
        }
    }
}
