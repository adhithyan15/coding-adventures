#![cfg(any(target_os = "linux", target_os = "macos"))]

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
    std::env::temp_dir().join(format!("venture-qt-launch-{}-{nonce}", std::process::id()))
}

fn command_available(command: &str, argument: &str) -> bool {
    Command::new(command)
        .arg(argument)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

fn qt_build_available() -> bool {
    command_available("cmake", "--version")
        && (command_available("qmake", "--version")
            || command_available("qtpaths6", "--version")
            || Command::new("pkg-config")
                .args(["--exists", "Qt6Quick", "Qt6Qml", "Qt6Widgets"])
                .status()
                .is_ok_and(|status| status.success()))
}

fn build_native_bridge(output: &Path) -> PathBuf {
    let target = output.join("native-target");
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let build = Command::new(cargo)
        .arg("build")
        .arg("-p")
        .arg("venture-browser-qt")
        .arg("--target-dir")
        .arg(&target)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("build Venture Qt bridge");
    assert!(
        build.status.success(),
        "Venture Qt bridge failed to build\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    #[cfg(target_os = "macos")]
    let library = target.join("debug/libventure_browser_qt.dylib");
    #[cfg(target_os = "linux")]
    let library = target.join("debug/libventure_browser_qt.so");
    library
}

fn serve_html() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind Qt acceptance server");
    let address = listener.local_addr().expect("read Qt acceptance address");
    thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept Venture Qt request");
        let mut request = [0_u8; 1024];
        let _ = stream.read(&mut request);
        let body = "<!doctype html><title>Venture Qt launch acceptance</title><main><h1>Venture Qt live page</h1><p>Rendered by the shared browser pipeline.</p></main>";
        write!(
            stream,
            "HTTP/1.0 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        )
        .expect("write Qt acceptance response headers");
        stream
            .write_all(body.as_bytes())
            .expect("write Qt acceptance response body");
    });
    format!("http://{address}/")
}

fn generated_executable(project: &Path) -> PathBuf {
    #[cfg(target_os = "macos")]
    return project.join("build/VentureChrome.app/Contents/MacOS/VentureChrome");
    #[cfg(target_os = "linux")]
    return project.join("build/VentureChrome");
}

fn wait_for_marker(child: &mut Child, marker: &Path, log: &Path) {
    let deadline = Instant::now() + Duration::from_secs(30);
    while Instant::now() < deadline {
        if marker.exists() {
            return;
        }
        if let Some(status) = child.try_wait().expect("poll generated Qt app") {
            let output = fs::read_to_string(log).unwrap_or_default();
            panic!("generated Qt app exited before acceptance marker: {status}\n{output}");
        }
        thread::sleep(Duration::from_millis(100));
    }
    let _ = child.kill();
    let _ = child.wait();
    let output = fs::read_to_string(log).unwrap_or_default();
    panic!("generated Qt app did not report acceptance within 30 seconds\n{output}");
}

#[test]
fn package_owned_qt_project_launches_and_renders_live_page() {
    let required = std::env::var_os("VENTURE_QT_ACCEPTANCE_REQUIRED").is_some();
    if !qt_build_available() {
        assert!(
            !required,
            "Qt direct-launch acceptance requires CMake and Qt6 Quick/QML/Widgets"
        );
        eprintln!("skipping Qt direct-launch acceptance: CMake or Qt6 unavailable");
        return;
    }

    let output = temporary_output();
    let result = build_package(&BuildOptions {
        package_root: venture_package_root(),
        output_root: output.clone(),
        backend: Backend::Qt,
        emit_project: true,
        theme: Some("light".to_string()),
    })
    .expect("emit Venture Qt package");
    let project = output.join("qt");
    let host = project.join("MosaicHost.cpp");
    assert!(host.exists(), "package-owned Qt host must be installed");
    assert!(
        result.artifacts.iter().any(|artifact| artifact == &host),
        "installed Qt host must be a package artifact"
    );

    let library = build_native_bridge(&output);
    assert!(library.exists(), "Venture Qt bridge must exist");
    let library_name = library.file_name().expect("Qt bridge file name");
    fs::copy(&library, project.join(library_name)).expect("install Venture Qt bridge");

    let configure = Command::new("cmake")
        .args(["-S", ".", "-B", "build"])
        .current_dir(&project)
        .output()
        .expect("configure generated Venture Qt project");
    assert!(
        configure.status.success(),
        "generated Venture Qt project failed to configure\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&configure.stdout),
        String::from_utf8_lossy(&configure.stderr)
    );
    let build = Command::new("cmake")
        .args(["--build", "build", "--config", "Debug"])
        .current_dir(&project)
        .output()
        .expect("build generated Venture Qt project");
    assert!(
        build.status.success(),
        "generated Venture Qt project failed to build\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let executable = generated_executable(&project);
    assert!(
        executable.exists(),
        "generated Venture Qt executable must exist: {}",
        executable.display()
    );
    let marker = output.join("qt-ready.json");
    let app_log = output.join("qt-app.log");
    let log = File::create(&app_log).expect("create Qt app log");
    let mut child = Command::new(&executable)
        .current_dir(&project)
        .env("QT_QPA_PLATFORM", "offscreen")
        .env("VENTURE_START_URL", serve_html())
        .env("VENTURE_BROWSER_LIBRARY", &library)
        .env("VENTURE_BROWSER_ACCEPTANCE_PATH", &marker)
        .stdout(Stdio::from(log.try_clone().expect("clone Qt app log")))
        .stderr(Stdio::from(log))
        .spawn()
        .expect("launch generated Venture Qt app");
    wait_for_marker(&mut child, &marker, &app_log);
    let status = child.wait().expect("wait for generated Qt app");
    assert!(
        status.success(),
        "generated Venture Qt app failed: {status}"
    );

    let report = fs::read_to_string(&marker).expect("read Qt acceptance marker");
    assert!(
        report.contains("\"ok\":true"),
        "unexpected marker: {report}"
    );
    assert!(report.contains("Venture Qt launch acceptance"));
    assert!(report.contains("\"rendered\":true"));
    assert!(report.contains("\"componentReady\":true"));
    assert!(report.contains("\"surfaceMounted\":true"));

    fs::remove_dir_all(&output).expect("remove clean Qt acceptance output");
}
