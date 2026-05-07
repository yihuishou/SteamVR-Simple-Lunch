#[cfg(windows)]
fn main() {
    use winres::WindowsResource;
    WindowsResource::new()
        .set_icon("assets/SteamVRIcon.ico")
        .compile()
        .expect("Failed to compile Windows resources");
}

#[cfg(not(windows))]
fn main() {}
