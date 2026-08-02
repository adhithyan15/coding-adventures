#![cfg(target_os = "macos")]

use mosaic_package_artifact_builder::{build_package, Backend, BuildOptions};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use venture_browser_core::BrowserFetchResponse;
use venture_browser_macos::load_initial_session;

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
    std::env::temp_dir().join(format!(
        "venture-swiftui-launch-{}-{nonce}",
        std::process::id()
    ))
}

fn build_native_bridge(output: &Path) -> PathBuf {
    let target = output.join("native-target");
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let build = Command::new(cargo)
        .arg("build")
        .arg("-p")
        .arg("venture-browser-macos")
        .arg("--target-dir")
        .arg(&target)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("build Venture macOS bridge");
    assert!(
        build.status.success(),
        "Venture macOS bridge failed to build\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    target.join("debug/libventure_browser_macos.dylib")
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

fn wait_for_marker(child: &mut Child, marker: &Path, description: &str) {
    let deadline = Instant::now() + Duration::from_secs(30);
    while Instant::now() < deadline {
        if marker.exists() {
            return;
        }
        if let Some(status) = child.try_wait().expect("poll generated SwiftUI app") {
            panic!("generated SwiftUI app exited before {description}: {status}");
        }
        thread::sleep(Duration::from_millis(100));
    }
    let _ = child.kill();
    let _ = child.wait();
    panic!("generated SwiftUI app did not report {description} within 30 seconds");
}

#[test]
fn surface_link_acceptance_coordinate_matches_native_layout() {
    let fetcher = |url: &str| {
        Ok(BrowserFetchResponse::new(
            url,
            200,
            Some("text/html; charset=utf-8".to_string()),
            b"<!doctype html><title>Start</title><main><a href='next'>Open Venture link acceptance</a></main>"
                .to_vec(),
        ))
    };
    let session = load_initial_session("http://example.test/", 1024.0, 640.0, &fetcher)
        .expect("load native acceptance layout");
    let link = session
        .viewport()
        .and_then(|viewport| viewport.page().paint.links.first())
        .expect("acceptance page must expose its link");
    assert!(
        32.0 >= link.x
            && 32.0 < link.x + link.width
            && 26.0 >= link.y
            && 26.0 < link.y + link.height,
        "acceptance coordinate misses native link region {link:?}"
    );
}

#[test]
fn package_owned_swiftui_project_launches_renders_and_interacts() {
    let output = temporary_output();
    let result = build_package(&BuildOptions {
        package_root: venture_package_root(),
        output_root: output.clone(),
        backend: Backend::SwiftUI,
        emit_project: true,
        theme: Some("light".to_string()),
    })
    .expect("emit Venture SwiftUI package");
    let project = output.join("swiftui");
    let host = project.join("Sources/App/MosaicHost.swift");
    assert!(
        host.exists(),
        "package-owned SwiftUI host must be installed"
    );
    assert!(
        result.artifacts.iter().any(|artifact| artifact == &host),
        "installed host must be a package artifact"
    );

    let library = build_native_bridge(&output);
    assert!(
        library.exists(),
        "build venture-browser-macos before this launch gate: {}",
        library.display()
    );
    fs::copy(&library, project.join("libventure_browser_macos.dylib"))
        .expect("install Venture macOS bridge");

    let build = Command::new("swift")
        .arg("build")
        .current_dir(&project)
        .output()
        .expect("run swift build");
    assert!(
        build.status.success(),
        "generated Venture SwiftUI project failed to build\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let marker = output.join("swiftui-ready.json");
    let interaction_marker = output.join("swiftui-interaction.json");
    let link_url = serve_html_sequence(
        vec!["Venture link acceptance", "Venture link acceptance"],
        None,
    );
    let start_url = serve_html_sequence(
        vec![
            "Venture launch acceptance",
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
    let app_log = output.join("swiftui-app.log");
    let log = File::create(&app_log).expect("create SwiftUI app log");
    let mut child = Command::new(project.join(".build/debug/App"))
        .current_dir(&project)
        .env("VENTURE_START_URL", &start_url)
        .env("VENTURE_BROWSER_LIBRARY", &library)
        .env("VENTURE_BROWSER_ACCEPTANCE_PATH", &marker)
        .env("VENTURE_BROWSER_INTERACTION_URL", &target_url)
        .env("VENTURE_BROWSER_INTERACTION_LINK_URL", &link_url)
        .env(
            "VENTURE_BROWSER_INTERACTION_ACCEPTANCE_PATH",
            &interaction_marker,
        )
        .stdout(Stdio::from(log.try_clone().expect("clone SwiftUI app log")))
        .stderr(Stdio::from(log))
        .spawn()
        .expect("launch generated Venture SwiftUI app");
    wait_for_marker(&mut child, &marker, "a rendered Mosaic host surface");
    wait_for_marker(
        &mut child,
        &interaction_marker,
        "native chrome interaction acceptance",
    );
    let _ = child.kill();
    let _ = child.wait();

    let readiness = fs::read_to_string(&marker).expect("read SwiftUI readiness marker");
    assert!(readiness.contains("\"backend\":\"swiftui\""));
    assert!(readiness.contains("\"status\":\"ready\""));
    let interaction =
        fs::read_to_string(&interaction_marker).expect("read SwiftUI interaction marker");
    assert!(
        interaction.contains("\"backend\":\"swiftui\""),
        "unexpected SwiftUI interaction marker: {interaction}"
    );
    assert!(
        interaction.contains("\"status\":\"interacted\""),
        "SwiftUI interaction failed: {interaction}"
    );
    assert!(interaction.contains("\"controls\":\"back-forward-reload-home\""));
    assert!(interaction.contains("\"surfaceWheel\":\"scroll\""));
    assert!(interaction.contains("\"surfaceKeyboard\":\"document-end\""));
    assert!(interaction.contains("\"surfaceHistory\":\"back-forward\""));
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
    let diagnostics = fs::read_to_string(&app_log).expect("read SwiftUI app log");
    assert!(
        !diagnostics.contains("assertion failed") && !diagnostics.contains("Assertion failed"),
        "generated SwiftUI app reported a native assertion:\n{diagnostics}"
    );
    fs::remove_dir_all(&output).expect("remove clean acceptance output");
}
