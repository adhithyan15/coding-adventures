use chief_of_staff_host_control_protocol::{
    ChannelBindingAccess, CompletionCall, DataPlaneResponse, ModelToolChoice, ModelToolDefinition,
    PromptMessage, PromptRole, ToolCompletionCall,
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
        tools: vec![ModelToolDefinition {
            name: "smart_home.list_entities".to_string(),
            description: "List normalized entities".to_string(),
            input_schema: serde_json::json!({"type": "object"}),
        }],
        choice: ModelToolChoice::Required,
        results: Vec::new(),
    })?;
    if !matches!(tool_completed, DataPlaneResponse::Failed { .. }) {
        return Err("unexpected tool completion response".into());
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
