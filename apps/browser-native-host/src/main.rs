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
    unsafe {
        libc::_setmode(0, libc::O_BINARY);
        libc::_setmode(1, libc::O_BINARY);
    }
}

#[cfg(not(windows))]
fn configure_binary_stdio() {}
