use chief_of_staff_host_control_protocol::{
    ChannelBindingAccess, CompletionCall, DataPlaneResponse, ModelToolChoice, PromptMessage,
    PromptRole, ToolCompletionCall, ToolCompletionOutput,
};
use chief_of_staff_host_runtime::{verify_agent_package, AgentPackageRuntime, PackageKeyring};
use chief_of_staff_process_supervisor::ChildProcessControl;
use std::collections::BTreeMap;
use std::env;
use std::io::{self, Write};
use std::path::Path;
use std::thread;
use std::time::Duration;

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

/// Send 20 receive requests back to back and record how many were served
/// and how many the supervisor's request budget refused (D18S S-K5).
fn flood(
    control: &mut ChildProcessControl<impl io::Read, impl Write>,
) -> Result<(), Box<dyn std::error::Error>> {
    let (mut served, mut refused) = (0, 0);
    for _ in 0..20 {
        match control.request_receive(uuid_v7(1), 1)? {
            DataPlaneResponse::Received { .. } => served += 1,
            DataPlaneResponse::Failed {
                failure: chief_of_staff_host_control_protocol::DataPlaneFailure::Unavailable,
                ..
            } => refused += 1,
            _ => return Err("unexpected flood response".into()),
        }
    }
    std::fs::write("FLOOD_RESULT", format!("served={served} refused={refused}"))?;
    Ok(())
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
    let listed = control.request_model_tools()?;
    let DataPlaneResponse::ModelToolsListed { tools, .. } = listed else {
        return Err("unexpected model tool catalog response".into());
    };
    let tool_completed = control.request_tool_completion(ToolCompletionCall {
        completion: CompletionCall {
            model: "test-model".to_string(),
            system: Some("use the offered tool".to_string()),
            messages: vec![PromptMessage {
                role: PromptRole::User,
                text: "list entities".to_string(),
            }],
            temperature: 0.0,
            max_tokens: Some(32),
            stop_sequences: Vec::new(),
            seed: Some(0),
            metadata: BTreeMap::new(),
        },
        tools,
        choice: ModelToolChoice::Required,
        results: Vec::new(),
    })?;
    let DataPlaneResponse::ToolCompleted { result, .. } = tool_completed else {
        return Err("unexpected tool completion response".into());
    };
    let ToolCompletionOutput::ToolCall(call) = result.output else {
        return Err("expected model-returned tool call".into());
    };
    let executed = control.request_tool_execution(call.clone())?;
    if !matches!(
        executed,
        DataPlaneResponse::ToolExecuted { result, .. } if result.call == call
    ) {
        return Err("unexpected tool execution response".into());
    }
    Ok(())
}

/// Report what this process was handed (D18S S-I2, S-I3). The marker holds
/// the number of a descriptor the parent leaked on purpose, then the path to
/// write the report to.
#[cfg(unix)]
fn report_descriptors() -> Result<(), Box<dyn std::error::Error>> {
    use std::os::unix::fs::MetadataExt;
    let marker = std::fs::read_to_string("REPORT_DESCRIPTORS")?;
    let (fd, path) = marker.split_once('\n').ok_or("bad marker")?;
    // `lstat` of the entry itself: it exists exactly while the fd is open.
    // fd 0 is the positive control: if /dev/fd cannot be read at all, the
    // report says so instead of reading as "not leaked".
    let visible = std::fs::symlink_metadata("/dev/fd/0").is_ok();
    let leaked = std::fs::symlink_metadata(format!("/dev/fd/{fd}")).is_ok();
    let stderr_null =
        std::fs::metadata("/dev/fd/2")?.rdev() == std::fs::metadata("/dev/null")?.rdev();
    std::fs::write(
        path,
        format!("visible={visible}\nleaked={leaked}\nstderr_null={stderr_null}\n"),
    )?;
    Ok(())
}

#[cfg(not(unix))]
fn report_descriptors() -> Result<(), Box<dyn std::error::Error>> {
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

    let arguments = env::args().skip(1).collect::<Vec<_>>();
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut control = ChildProcessControl::bootstrap(stdin.lock(), stdout.lock())?;
    let mut keyring = PackageKeyring::new();
    keyring.trust(control.receive_package_trust()?)?;
    let package = verify_agent_package(Path::new("."), &keyring)?;
    let launch_bindings = control.receive_launch_bindings()?;
    if launch_bindings.channels().len() != 2
        || launch_bindings.channels()[0].name() != "weather-reports"
        || launch_bindings.channels()[0].access() != ChannelBindingAccess::Write
        || launch_bindings.channels()[1].name() != "weather-requests"
        || launch_bindings.channels()[1].access() != ChannelBindingAccess::Read
    {
        return Err("unexpected authorized channel bindings".into());
    }
    let expected_runtime = match package.runtime() {
        AgentPackageRuntime::Deno if launch_bindings.level_one_model().is_none() => "deno",
        AgentPackageRuntime::Skill if launch_bindings.level_one_model().is_some() => "skill",
        _ => return Err("launch model settings do not match package runtime".into()),
    };
    if arguments != ["--package-runtime", expected_runtime] {
        return Err("package runtime launch argument mismatch".into());
    }
    if has_marker("REPORT_DESCRIPTORS") {
        report_descriptors()?;
    }
    if has_marker("EXIT_BEFORE_READY") {
        return Ok(());
    }
    if has_marker("NEVER_READY") {
        // Complete the bootstrap, then never say Ready.
        thread::sleep(Duration::from_secs(30));
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
    if has_marker("FLOOD") {
        flood(&mut control)?;
    }
    if has_marker("ORPHAN") {
        // Leave a process behind that holds this host's stdout, and record
        // its pid. The host then runs on and stops normally.
        let orphan = std::process::Command::new("sleep").arg("30").spawn()?;
        std::fs::write("ORPHAN_PID", orphan.id().to_string())?;
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
