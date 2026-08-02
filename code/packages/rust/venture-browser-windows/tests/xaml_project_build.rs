#![cfg(target_os = "windows")]

use mosaic_package_artifact_builder::{build_package, Backend, BuildOptions};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

fn venture_package_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .and_then(|path| path.parent())
        .map(|code| code.join("programs").join("mosaic").join("venture-browser"))
        .expect("derive Venture package root")
}

fn temporary_output() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("venture-xaml-{}-{nonce}", std::process::id()))
}

fn build_native_bridge(output: &Path) -> PathBuf {
    let target = output.join("native-target");
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let build = Command::new(cargo)
        .arg("build")
        .arg("-p")
        .arg("venture-browser-windows")
        .arg("--target-dir")
        .arg(&target)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("build Venture Windows bridge");
    assert!(
        build.status.success(),
        "Venture Windows bridge failed to build\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    target.join("debug/venture_browser_windows.dll")
}

fn serve_html_sequence(titles: Vec<&'static str>, link_url: Option<String>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind acceptance server");
    let address = listener.local_addr().expect("read acceptance address");
    thread::spawn(move || {
        for title in titles {
            let (mut stream, _) = listener.accept().expect("accept Venture request");
            let mut request = [0_u8; 1024];
            let _ = stream.read(&mut request);
            let link = link_url
                .as_deref()
                .map(|url| format!("<a href='{url}'>Open Venture link acceptance</a>"))
                .unwrap_or_default();
            let body = format!(
                "<!doctype html><title>{title}</title><main>{link}{}</main>",
                "<p>Scrollable Venture acceptance content</p>".repeat(120)
            );
            write!(
                stream,
                "HTTP/1.0 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            )
            .expect("write acceptance response headers");
            stream
                .write_all(body.as_bytes())
                .expect("write acceptance response body");
        }
    });
    format!("http://{address}/")
}

fn find_executable(root: &Path) -> Option<PathBuf> {
    for entry in fs::read_dir(root).ok()? {
        let path = entry.ok()?.path();
        if path.is_dir() {
            if let Some(found) = find_executable(&path) {
                return Some(found);
            }
        } else if path.file_name().and_then(|name| name.to_str()) == Some("VentureChrome.exe") {
            return Some(path);
        }
    }
    None
}

fn application_failure_diagnostics(executable: &Path) -> String {
    let executable_name = executable
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("VentureChrome.exe");
    let script = r#"
$events = Get-WinEvent -FilterHashtable @{
    LogName = 'Application'
    StartTime = (Get-Date).AddMinutes(-5)
} -ErrorAction SilentlyContinue |
    Where-Object { $_.Message -like "*$env:VENTURE_ACCEPTANCE_EXE_NAME*" } |
    Select-Object -First 5 TimeCreated, ProviderName, Id, LevelDisplayName, Message
if ($events) {
    $events | Format-List | Out-String -Width 4096
} else {
    'No matching Windows Application event was recorded.'
}
"#;
    let output = Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .env("VENTURE_ACCEPTANCE_EXE_NAME", executable_name)
        .output();
    match output {
        Ok(output) => {
            let mut diagnostics = String::from_utf8_lossy(&output.stdout).into_owned();
            diagnostics.push_str(&String::from_utf8_lossy(&output.stderr));
            diagnostics
        }
        Err(error) => format!("failed to query Windows Application events: {error}"),
    }
}

fn wait_for_marker(
    child: &mut Child,
    executable: &Path,
    marker: &Path,
    phase_log: &Path,
    description: &str,
) {
    let deadline = Instant::now() + Duration::from_secs(30);
    while Instant::now() < deadline {
        if marker.exists() {
            return;
        }
        if let Some(status) = child.try_wait().expect("poll generated WinUI app") {
            let phase = fs::read_to_string(phase_log)
                .unwrap_or_else(|_| "no package-host phase was reported".to_string());
            panic!(
                "generated WinUI app exited before {description}: {status}\nlast host phase: {phase}\n{}",
                application_failure_diagnostics(executable)
            );
        }
        thread::sleep(Duration::from_millis(100));
    }
    let _ = child.kill();
    let _ = child.wait();
    panic!("generated WinUI app did not report {description} within 30 seconds");
}

#[test]
fn package_owned_xaml_project_builds_launches_and_interacts() {
    let output = temporary_output();
    let result = build_package(&BuildOptions {
        package_root: venture_package_root(),
        output_root: output.clone(),
        backend: Backend::Xaml,
        emit_project: true,
        theme: Some("light".to_string()),
    })
    .expect("emit Venture XAML package");
    let project = output.join("xaml");
    let host = project.join("MosaicHost.cs");
    assert!(host.exists(), "package-owned XAML host must be installed");
    assert!(
        result.artifacts.iter().any(|artifact| artifact == &host),
        "installed host must be a package artifact"
    );

    let library = build_native_bridge(&output);
    assert!(
        library.exists(),
        "build venture-browser-windows before this launch gate: {}",
        library.display()
    );
    fs::copy(&library, project.join("venture_browser_windows.dll"))
        .expect("install Venture Windows bridge");

    let build = Command::new("dotnet")
        .arg("build")
        .arg("VentureChrome.csproj")
        .arg("-p:Platform=x64")
        .current_dir(&project)
        .output()
        .expect("run dotnet build for generated WinUI project");
    if !build.status.success() {
        panic!(
            "generated Venture WinUI project failed to build\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&build.stdout),
            String::from_utf8_lossy(&build.stderr)
        );
    }

    let executable = find_executable(&project.join("bin"))
        .expect("generated WinUI build must produce VentureChrome.exe");
    let marker = output.join("xaml-ready.json");
    let interaction_marker = output.join("xaml-interaction.json");
    let phase_log = output.join("xaml-phase.json");
    let link_url = serve_html_sequence(vec!["Venture link acceptance"], None);
    let start_url = serve_html_sequence(
        vec![
            "Venture launch acceptance",
            "Venture launch acceptance",
            "Venture launch acceptance",
        ],
        Some(link_url.clone()),
    );
    let target_url = serve_html_sequence(
        vec![
            "Venture interaction acceptance",
            "Venture interaction acceptance",
            "Venture reload acceptance",
        ],
        None,
    );
    let app_log = output.join("xaml-app.log");
    let log = File::create(&app_log).expect("create WinUI app log");
    let mut child = Command::new(&executable)
        .current_dir(executable.parent().expect("WinUI executable directory"))
        .env("VENTURE_START_URL", &start_url)
        .env("VENTURE_BROWSER_ACCEPTANCE_PATH", &marker)
        .env("VENTURE_BROWSER_ACCEPTANCE_DIAGNOSTIC_PATH", &phase_log)
        .env("VENTURE_BROWSER_INTERACTION_URL", &target_url)
        .env("VENTURE_BROWSER_INTERACTION_LINK_URL", &link_url)
        .env(
            "VENTURE_BROWSER_INTERACTION_ACCEPTANCE_PATH",
            &interaction_marker,
        )
        .stdout(Stdio::from(log.try_clone().expect("clone WinUI app log")))
        .stderr(Stdio::from(log))
        .spawn()
        .expect("launch generated Venture WinUI app");
    wait_for_marker(
        &mut child,
        &executable,
        &marker,
        &phase_log,
        "a rendered Mosaic host surface",
    );
    wait_for_marker(
        &mut child,
        &executable,
        &interaction_marker,
        &phase_log,
        "native chrome interaction acceptance",
    );
    let _ = child.kill();
    let _ = child.wait();

    let readiness = fs::read_to_string(&marker).expect("read WinUI readiness marker");
    assert!(readiness.contains("\"backend\":\"xaml\""));
    assert!(readiness.contains("\"status\":\"ready\""));
    let interaction =
        fs::read_to_string(&interaction_marker).expect("read WinUI interaction marker");
    assert!(
        interaction.contains("\"backend\":\"xaml\""),
        "unexpected WinUI interaction marker: {interaction}"
    );
    assert!(
        interaction.contains("\"status\":\"interacted\""),
        "WinUI interaction failed: {interaction}"
    );
    assert!(interaction.contains("\"controls\":\"back-forward-reload-home\""));
    assert!(interaction.contains("\"surfaceKeyboard\":\"document-end\""));
    assert!(interaction.contains("\"surfacePointer\":\"link\""));
    assert!(interaction.contains("\"surfaceResize\":\"native-reflow\""));
    assert!(interaction.contains("\"reloadTitle\":\"Venture reload acceptance\""));
    assert!(
        interaction.contains(&start_url) || interaction.contains(&start_url.replace('/', "\\/"))
    );
    assert!(
        interaction.contains(&target_url) || interaction.contains(&target_url.replace('/', "\\/"))
    );
    assert!(interaction.contains(&link_url) || interaction.contains(&link_url.replace('/', "\\/")));
    assert!(interaction.contains("Venture link acceptance"));
    let diagnostics = fs::read_to_string(&app_log).expect("read WinUI app log");
    assert!(
        !diagnostics.contains("assertion failed") && !diagnostics.contains("Assertion failed"),
        "generated WinUI app reported a native assertion:\n{diagnostics}"
    );

    fs::remove_dir_all(&output).expect("remove clean acceptance output");
}
