//! Intègre l'icône et les informations de version dans l'exécutable Windows.

fn main() {
    println!("cargo:rerun-if-changed=assets/ScreenCapture.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let mut res = winresource::WindowsResource::new();
    if std::path::Path::new("assets/ScreenCapture.ico").is_file() {
        res.set_icon("assets/ScreenCapture.ico");
    }
    res.set("ProductName", "ScreenCapture")
        .set("FileDescription", "ScreenCapture — capture d’écran et annotation")
        .set("LegalCopyright", "© 2026 Stéphane Salou")
        .set_language(0x040c);
    if let Err(e) = res.compile() {
        println!("cargo:warning=ressources Windows non intégrées : {e}");
    }
}
