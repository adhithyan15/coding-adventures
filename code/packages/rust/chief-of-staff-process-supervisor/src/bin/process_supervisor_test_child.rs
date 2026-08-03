use chief_of_staff_host_runtime::{
    verify_agent_package, PackageKeyType, PackageKeyring, TrustedPackageKey,
};
use chief_of_staff_process_supervisor::ChildProcessControl;
use chief_of_staff_tool_api::PrivilegeTier;
use std::io::{self, Write};
use std::path::Path;
use std::thread;
use std::time::Duration;

const TEST_PUBLIC_KEY: [u8; 32] = [
    25, 127, 107, 35, 225, 108, 133, 50, 198, 171, 200, 56, 250, 205, 94, 167, 137, 190, 12, 118,
    178, 146, 3, 52, 3, 155, 250, 139, 61, 54, 141, 97,
];

fn has_marker(name: &str) -> bool {
    Path::new(name).is_file()
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    if has_marker("SILENT_BOOTSTRAP") {
        thread::sleep(Duration::from_secs(10));
        return Ok(());
    }
    if has_marker("OVERSIZED_BOOTSTRAP") {
        let mut stdout = io::stdout().lock();
        stdout.write_all(&((1024_u32 * 1024) + 1).to_be_bytes())?;
        stdout.flush()?;
        thread::sleep(Duration::from_secs(10));
        return Ok(());
    }

    let mut keyring = PackageKeyring::new();
    keyring.trust(TrustedPackageKey::new(
        "prod-test",
        PackageKeyType::Production,
        TEST_PUBLIC_KEY,
        PrivilegeTier::Tier3,
    )?)?;
    let package = verify_agent_package(Path::new("."), &keyring)?;
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut control = ChildProcessControl::bootstrap(stdin.lock(), stdout.lock())?;

    if has_marker("EXIT_BEFORE_READY") {
        return Ok(());
    }
    let mut digest = package.digest();
    if has_marker("WRONG_READY") {
        digest[0] ^= 0xff;
    }
    control.ready(digest)?;
    if !has_marker("NO_HEARTBEAT") {
        control.heartbeat()?;
    }
    control.receive_terminate()?;
    if has_marker("IGNORE_TERMINATE") {
        thread::sleep(Duration::from_secs(10));
    }
    Ok(())
}

fn main() {
    if run().is_err() {
        std::process::exit(70);
    }
}
