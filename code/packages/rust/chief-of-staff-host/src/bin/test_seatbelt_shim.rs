//! A test fixture: the Seatbelt shim, built inside this crate so its
//! end-to-end tests can pin it next to the test broker (D18S P2.7b). Its
//! body is the production shim's, `chief_of_staff_macos_sandbox::shim::main`.

#[cfg(target_os = "macos")]
fn main() {
    chief_of_staff_macos_sandbox::shim::main()
}

/// Elsewhere there is no Seatbelt, and no test runs this.
#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("chief-of-staff-host-test-seatbelt-shim: macOS only");
    std::process::exit(127)
}
