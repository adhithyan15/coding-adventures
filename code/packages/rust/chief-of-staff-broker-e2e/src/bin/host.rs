//! A scripted host for the broker end-to-end tests. It bootstraps exactly as
//! a real host does, then follows the markers in its package directory:
//!
//! ```text
//!   PUBLISH        publish "sunny" on its write channel, and write what
//!                  came back to the path the marker names
//!   EXIT_AT_ONCE   exit right after saying Ready
//! ```

use chief_of_staff_host_control_protocol::ChannelBindingAccess;
use chief_of_staff_host_runtime::{verify_agent_package, PackageKeyring};
use chief_of_staff_process_supervisor::ChildProcessControl;
use std::io;
use std::path::Path;

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut control = ChildProcessControl::bootstrap(io::stdin().lock(), io::stdout().lock())?;
    let mut keyring = PackageKeyring::new();
    keyring.trust(control.receive_package_trust()?)?;
    let package = verify_agent_package(Path::new("."), &keyring)?;
    let bindings = control.receive_launch_bindings()?;
    control.ready(package.digest())?;
    control.heartbeat()?;
    if Path::new("EXIT_AT_ONCE").is_file() {
        return Ok(());
    }
    if Path::new("PUBLISH").is_file() {
        let result_path = std::fs::read_to_string("PUBLISH")?;
        let channel = bindings
            .channels()
            .iter()
            .find(|channel| channel.access() == ChannelBindingAccess::Write)
            .ok_or("no write channel")?;
        let response = control.request_publish(
            channel.channel_id(),
            "text/plain".to_owned(),
            b"sunny".to_vec(),
        )?;
        std::fs::write(result_path.trim(), format!("{response:?}"))?;
    }
    control.receive_terminate()?;
    Ok(())
}

fn main() {
    if run().is_err() {
        std::process::exit(70);
    }
}
