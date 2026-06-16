fn main() {
    let mut attributes = tauri_build::Attributes::new();

    #[cfg(windows)]
    {
        let icon_path = prepare_windows_icon();
        let windows = tauri_build::WindowsAttributes::new().window_icon_path(icon_path);
        attributes = attributes.windows_attributes(windows);
    }

    tauri_build::try_build(attributes).expect("failed to run Tauri build script");
}

#[cfg(windows)]
fn prepare_windows_icon() -> std::path::PathBuf {
    let source = std::path::Path::new(&std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR missing"))
        .join("icons")
        .join("icon.ico");
    let target = std::env::temp_dir().join("deyana-tauri-icon.ico");

    std::fs::copy(&source, &target).expect("failed to stage Windows icon");
    target
}
