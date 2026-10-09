//! `chief-of-staff-seatbelt-shim`: installs a Seatbelt profile in a fresh
//! process image and execs the agent under it (D18S step 7). The supervisor
//! runs it through `MacosConfinement`, which pins it by SHA-256; it is not
//! meant to be run by hand. The body is `chief_of_staff_macos_sandbox::shim`.

#[cfg(target_os = "macos")]
fn main() {
    chief_of_staff_macos_sandbox::shim::main()
}

/// Elsewhere there is no Seatbelt: refuse, as the shim refuses anything it
/// cannot do.
#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("chief-of-staff-seatbelt-shim: macOS only");
    std::process::exit(127)
}
