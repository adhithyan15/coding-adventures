use chief_of_staff_host_control_protocol::{
    CompletionCall, DataPlaneResponse, PromptMessage, PromptRole,
};
use chief_of_staff_host_runtime::{
    verify_agent_package, AgentPackageRuntime, PackageKeyType, PackageKeyring, TrustedPackageKey,
};
use chief_of_staff_process_supervisor::ChildProcessControl;
use chief_of_staff_tool_api::PrivilegeTier;
use std::collections::BTreeMap;
use std::env;
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

fn uuid_v7(last: u8) -> [u8; 16] {
    let mut bytes = [0u8; 16];
    bytes[6] = 0x70;
    bytes[8] = 0x80;
    bytes[15] = last;
    bytes
}

fn exercise_data_plane(
    control: &mut ChildProcessControl<impl io::Read, impl Write>,
) -> Result<(), Box<dyn std::error::Error>> {
    let received = control.request_receive(uuid_v7(1), 1)?;
    if !matches!(received, DataPlaneResponse::Received { messages, .. } if messages.is_empty()) {
        return Err("unexpected receive response".into());
    }
    let published =
        control.request_publish(uuid_v7(2), "text/plain".to_string(), b"weather".to_vec())?;
    if !matches!(published, DataPlaneResponse::Published { sequence: 1, .. }) {
        return Err("unexpected publish response".into());
    }
    let acknowledged = control.request_acknowledge(uuid_v7(1), uuid_v7(3))?;
    if !matches!(
        acknowledged,
        DataPlaneResponse::Acknowledged { sequence: 2, .. }
    ) {
        return Err("unexpected acknowledge response".into());
    }
    let completed = control.request_completion(CompletionCall {
        model: "test-model".to_string(),
        system: Some("be concise".to_string()),
        messages: vec![PromptMessage {
            role: PromptRole::User,
            text: "weather".to_string(),
        }],
        temperature: 0.0,
        max_tokens: Some(32),
        stop_sequences: Vec::new(),
        seed: Some(0),
        metadata: BTreeMap::new(),
    })?;
    if !matches!(completed, DataPlaneResponse::Failed { .. }) {
        return Err("unexpected completion response".into());
    }
    Ok(())
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
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    let expected_runtime = match package.runtime() {
        AgentPackageRuntime::Deno => "deno",
        AgentPackageRuntime::Skill => "skill",
    };
    if arguments != ["--package-runtime", expected_runtime] {
        return Err("package runtime launch argument mismatch".into());
    }
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
    if has_marker("DATA_PLANE") {
        exercise_data_plane(&mut control)?;
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
