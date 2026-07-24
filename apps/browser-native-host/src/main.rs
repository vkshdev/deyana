mod bridge;
mod credential;
mod framing;
mod origin;

fn main() {
    configure_binary_stdio();
    if let Err(error) = run() {
        eprintln!("DEYANA browser host: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let extension_origin = origin::caller_origin()?;
    let credential = credential::load()?;
    bridge::run(credential, extension_origin)
}

#[cfg(windows)]
fn configure_binary_stdio() {
    extern "C" {
        fn _setmode(fd: std::os::raw::c_int, mode: std::os::raw::c_int) -> std::os::raw::c_int;
    }
    const O_BINARY: std::os::raw::c_int = 0x8000;
    unsafe {
        _setmode(0, O_BINARY);
        _setmode(1, O_BINARY);
    }
}

#[cfg(not(windows))]
fn configure_binary_stdio() {}
