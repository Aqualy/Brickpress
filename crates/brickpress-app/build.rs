//! Embeds the application icon and version information in the Windows
//! executable, so Explorer, the taskbar and the installer show Brickpress.

fn main() {
    println!("cargo:rerun-if-changed=packaging/icon.ico");
    #[cfg(windows)]
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("packaging/icon.ico")
            .set("ProductName", "Brickpress")
            .set("FileDescription", "Brickpress")
            .set("CompanyName", "Aqualy")
            .set("LegalCopyright", "Copyright 2026 Tiebo (Aqualy)")
            .set("OriginalFilename", "brickpress.exe");
        if let Err(e) = res.compile() {
            println!("cargo:warning=could not embed the Windows icon: {e}");
        }
    }
}
