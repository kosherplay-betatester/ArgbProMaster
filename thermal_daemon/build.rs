fn main() {
    println!("cargo:rerun-if-changed=../assets/icon.ico");
    // Embed the app icon into the .exe so Explorer, the Start Menu, the
    // taskbar and shortcuts all show the flame (regenerate the .ico with
    // `cargo run -p argb_core --example make_icon`).
    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("../assets/icon.ico");
        if let Err(e) = res.compile() {
            println!("cargo:warning=failed to embed icon: {e}");
        }
    }
}
