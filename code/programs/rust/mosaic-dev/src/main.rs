//! # mosaic-dev — Storybook-like multi-backend dev environment
//!
//! Usage:
//!
//! ```text
//! mosaic-dev <PACKAGE_ROOT> --backend <react|swiftui|qt|webcomponent|html|xaml>
//!                           --component <NAME>
//!                           [--port 5173] [--no-open]
//! ```
//!
//! The flow per invocation:
//!
//! 1. Parse the chosen package's `mosaic-package.toml`.
//! 2. Build it for the selected backend via
//!    `mosaic_package_artifact_builder::build_package` into a temp dir.
//! 3. Generate a host wrapper (TSX, HTML, Swift, QML, …) into the temp dir.
//! 4. Spawn the backend's runtime against the temp dir.
//! 5. Watch `<package_root>/src/` and re-run step 2 on every change so
//!    the runtime picks up the new build.
//!
//! The runtime-spawning logic lives here because it's inherently
//! process-bound and not easily unit-testable; the *content* of the
//! generated wrappers lives in `lib.rs` where the tests can reach it.

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use clap::Parser;
use mosaic_dev::wrappers;
use mosaic_dev::{Backend, DummyProps};
use mosaic_package_artifact_builder::{
    build_package, Backend as BuilderBackend, BuildOptions,
};
use notify::{Event, EventKind, RecursiveMode, Watcher};

// ===========================================================================
// CLI surface
// ===========================================================================

#[derive(Parser, Debug)]
#[command(
    name = "mosaic-dev",
    version,
    about = "Storybook-like multi-backend dev environment for Mosaic components"
)]
struct Cli {
    /// Directory containing `mosaic-package.toml`.
    package_root: PathBuf,

    /// Which backend's runtime to spawn.
    #[arg(long, value_name = "BACKEND")]
    backend: String,

    /// Which exported component to preview.
    #[arg(long, value_name = "NAME")]
    component: String,

    /// HTTP port for web-based backends.
    #[arg(long, default_value_t = 5173)]
    port: u16,

    /// Don't auto-open the browser.
    #[arg(long, default_value_t = false)]
    no_open: bool,
}

// ===========================================================================
// Entry point
// ===========================================================================

fn main() {
    let cli = Cli::parse();

    let backend = Backend::from_cli(&cli.backend).unwrap_or_else(|| {
        eprintln!(
            "mosaic-dev: --backend must be one of \
             react|swiftui|qt|webcomponent|html|xaml, got '{}'",
            cli.backend
        );
        std::process::exit(1);
    });

    // ---- XAML: friendly bail-out -----------------------------------------
    //
    // The XAML emitter only meaningfully runs on Windows hosts (it depends
    // on `dotnet`/MSBuild for preview). Until we wire that up we return a
    // clear, non-cryptic error so users aren't left wondering what went
    // wrong.
    if matches!(backend, Backend::Xaml) {
        eprintln!(
            "mosaic-dev: XAML backend dev runtime requires Windows; \
             not yet supported by mosaic-dev. \
             Use `mosaic-compile pkg --backend xaml` for static \
             artifact generation."
        );
        std::process::exit(2);
    }

    // ---- Sanity-check the package root -----------------------------------
    let manifest_path = cli.package_root.join("mosaic-package.toml");
    if !manifest_path.exists() {
        eprintln!(
            "mosaic-dev: no mosaic-package.toml found in {}",
            cli.package_root.display()
        );
        std::process::exit(1);
    }

    // ---- Create a workspace temp dir for this session -------------------
    let work = tempfile::tempdir().unwrap_or_else(|e| {
        eprintln!("mosaic-dev: cannot create temp dir: {e}");
        std::process::exit(1);
    });
    eprintln!(
        "mosaic-dev: workspace {} (backend={}, component={})",
        work.path().display(),
        backend.label(),
        cli.component
    );

    // ---- Initial build + wrapper generation ------------------------------
    if let Err(e) = build_and_wrap(&cli.package_root, &cli.component, backend, work.path()) {
        eprintln!("mosaic-dev: initial build failed: {e}");
        std::process::exit(1);
    }

    // ---- Per-backend runtime dispatch -----------------------------------
    match backend {
        Backend::React => run_react(&cli, work.path()),
        Backend::Html => run_static_http(&cli, work.path(), "index.html"),
        Backend::WebComponent => run_static_http(&cli, work.path(), "index.html"),
        Backend::SwiftUI => run_swiftui(&cli, work.path()),
        Backend::Qt => run_qt(&cli, work.path()),
        Backend::Xaml => unreachable!("rejected above"),
    }
}

// ===========================================================================
// Build + wrapper generation (used by initial run *and* every watch tick)
// ===========================================================================

/// Run one build pass: invoke the package-artifact builder, then write a
/// fresh host wrapper into the workspace root.
///
/// Each call overwrites the previous build — Vite/HMR (React), the static
/// HTTP server (HTML/WebComponent), or the next process-restart cycle
/// (SwiftUI/Qt) will see the new files on its next pickup.
fn build_and_wrap(
    package_root: &Path,
    component: &str,
    backend: Backend,
    work: &Path,
) -> Result<(), String> {
    // ---- 1. Drive build_package for backends it supports ----------------
    //
    // The artifact builder rejects WebComponent/Html today; we work
    // around this for those backends by emitting our own minimal output
    // from the .mil + .mll instead of going through `build_package`. For
    // *every other* backend we delegate.
    let builder_backend = match backend {
        Backend::React => Some(BuilderBackend::React),
        Backend::SwiftUI => Some(BuilderBackend::SwiftUI),
        Backend::Qt => Some(BuilderBackend::Qt),
        // The artifact builder doesn't have WebComponent/Html wired yet.
        // We fall back to a "stub" path below — see emit_stub_artifact.
        Backend::WebComponent | Backend::Html => None,
        Backend::Xaml => unreachable!("xaml handled earlier"),
    };

    if let Some(b) = builder_backend {
        let opts = BuildOptions {
            package_root: package_root.to_path_buf(),
            output_root: work.to_path_buf(),
            backend: b,
            // mosaic-dev only needs the per-component artifacts for
            // its dummy-props preview; it doesn't care about the
            // UI32-M project shell.
            emit_project: false,
            theme: None,
        };
        build_package(&opts).map_err(|e| e.to_string())?;
    } else {
        emit_stub_artifact(package_root, component, backend, work)?;
    }

    // ---- 2. Parse the component's interface for dummy-prop synthesis ----
    let cmp = mosaic_dev::parse_component_interface(package_root, component)?;
    let dummy = DummyProps::from_component(&cmp);

    // ---- 3. Generate the host wrapper -----------------------------------
    match backend {
        Backend::React => {
            std::fs::write(work.join("index.html"), wrappers::react_index_html(component))
                .map_err(|e| format!("write index.html: {e}"))?;
            std::fs::write(work.join("main.tsx"), wrappers::react_main_tsx(component, &dummy))
                .map_err(|e| format!("write main.tsx: {e}"))?;
        }
        Backend::Html => {
            let body = read_artifact_or(work.join("html").join(format!("{component}.html")))
                .unwrap_or_else(|| format!("<!-- no html artifact for {component} -->"));
            std::fs::write(work.join("index.html"), wrappers::html_index(component, &body))
                .map_err(|e| format!("write index.html: {e}"))?;
        }
        Backend::WebComponent => {
            std::fs::write(
                work.join("index.html"),
                wrappers::webcomponent_index(component, &dummy),
            )
            .map_err(|e| format!("write index.html: {e}"))?;
        }
        Backend::SwiftUI => {
            let host = work.join("Host");
            std::fs::create_dir_all(host.join("Sources").join("Host"))
                .map_err(|e| format!("mkdir Host/Sources: {e}"))?;
            std::fs::write(
                host.join("Package.swift"),
                wrappers::swiftui_package_swift("Host", "../swiftui"),
            )
            .map_err(|e| format!("write Package.swift: {e}"))?;
            std::fs::write(
                host.join("Sources").join("Host").join("main.swift"),
                wrappers::swiftui_main_swift(component, &dummy),
            )
            .map_err(|e| format!("write main.swift: {e}"))?;
        }
        Backend::Qt => {
            std::fs::write(
                work.join("qt").join("main.qml"),
                wrappers::qt_main_qml(component, &dummy),
            )
            .map_err(|e| format!("write main.qml: {e}"))?;
        }
        Backend::Xaml => unreachable!(),
    }

    eprintln!("mosaic-dev: build complete ({})", backend.label());
    Ok(())
}

/// Emit a "stub" backend artifact for backends the package-artifact
/// builder doesn't yet wire.
///
/// We don't have an end-to-end IR pipeline for HTML / WebComponent yet,
/// so for the dev preview we just create the expected output directory
/// and a placeholder file.  This lets the wrapper-generation step still
/// produce a valid preview page; the user sees the placeholder text plus
/// the dummy slot values.
fn emit_stub_artifact(
    _package_root: &Path,
    component: &str,
    backend: Backend,
    work: &Path,
) -> Result<(), String> {
    match backend {
        Backend::Html => {
            let dir = work.join("html");
            std::fs::create_dir_all(&dir).map_err(|e| format!("mkdir html: {e}"))?;
            std::fs::write(
                dir.join(format!("{component}.html")),
                format!(
                    "<div class=\"mosaic-stub\" style=\"padding:24px;border:1px dashed #888\">\
                     <h2>{component}</h2>\
                     <p>HTML backend artifact stub — the kernel HTML pipeline is not yet \
                     wired in mosaic-package-artifact-builder; this stub is enough to \
                     exercise the dev runtime.</p>\
                     </div>"
                ),
            )
            .map_err(|e| format!("write stub html: {e}"))?;
        }
        Backend::WebComponent => {
            let dir = work.join("webcomponent");
            std::fs::create_dir_all(&dir).map_err(|e| format!("mkdir webcomponent: {e}"))?;
            // A minimum-viable self-registering custom element.  Until the
            // real pipeline lands, this lets the preview verify the
            // wrapper's attribute wiring end-to-end.
            let tag = pascal_to_kebab_with_dash(component);
            let js = format!(
                "// Auto-generated stub — replaced when the WebComponent kernel pipeline lands.\n\
                 class {component}Element extends HTMLElement {{\n\
                 \u{20}\u{20}connectedCallback() {{\n\
                 \u{20}\u{20}\u{20}\u{20}const attrs = Array.from(this.attributes)\n\
                 \u{20}\u{20}\u{20}\u{20}\u{20}\u{20}.map(a => `<li><code>${{a.name}}</code>: \
                 ${{a.value}}</li>`).join('');\n\
                 \u{20}\u{20}\u{20}\u{20}this.innerHTML = `<div style=\"padding:16px;\
                 border:1px dashed #888\"><h2>{component}</h2><ul>${{attrs}}</ul></div>`;\n\
                 \u{20}\u{20}}}\n\
                 }}\n\
                 customElements.define('{tag}', {component}Element);\n"
            );
            std::fs::write(dir.join(format!("{component}.js")), js)
                .map_err(|e| format!("write stub js: {e}"))?;
        }
        _ => {}
    }
    Ok(())
}

/// `Card` → `card-mosaic`, `FormulaBar` → `formula-bar`. The mosaic-dev
/// stub WC tag must contain a dash; single-word names get `-mosaic`
/// suffixed, matching `wrappers::webcomponent_index`.
fn pascal_to_kebab_with_dash(pascal: &str) -> String {
    let mut out = String::new();
    for (i, c) in pascal.chars().enumerate() {
        if c.is_ascii_uppercase() && i > 0 {
            out.push('-');
        }
        out.push(c.to_ascii_lowercase());
    }
    if !out.contains('-') {
        out.push_str("-mosaic");
    }
    out
}

fn read_artifact_or(path: PathBuf) -> Option<String> {
    std::fs::read_to_string(path).ok()
}

// ===========================================================================
// React: Vite dev server
// ===========================================================================

/// Spawn `npx vite` against the workspace and watch the source dir for
/// changes.  Vite watches the workspace itself for HMR; our job is just
/// to re-run `build_package` whenever the *source* changes so Vite sees
/// fresh artifacts.
fn run_react(cli: &Cli, work: &Path) {
    let port = cli.port.to_string();
    let mut cmd = Command::new("npx");
    cmd.arg("--yes")
        .arg("vite")
        .arg("--port")
        .arg(&port)
        .arg("--host")
        .arg("127.0.0.1")
        .arg(work);

    eprintln!("mosaic-dev: launching vite on http://127.0.0.1:{port}");
    let child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            eprintln!(
                "mosaic-dev: failed to launch vite ({e}). \
                 Install vite (npm i -g vite) or ensure npx is available."
            );
            return;
        }
    };

    maybe_open_browser(&format!("http://127.0.0.1:{port}"), cli.no_open);
    watch_and_rebuild(cli, work, Some(child), /* restart_on_change = */ false);
}

// ===========================================================================
// HTML / WebComponent: tiny_http
// ===========================================================================

/// Serve the workspace over an in-process tiny HTTP server.
///
/// We deliberately use a *very* small server (no SSE, no auto-refresh)
/// for the v0.1.0 cut — the user can manually refresh to see updates.
/// Auto-refresh via SSE is a clearly-bounded follow-up PR.
fn run_static_http(cli: &Cli, work: &Path, index_filename: &str) {
    let addr = format!("127.0.0.1:{}", cli.port);
    let server = match tiny_http::Server::http(&addr) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("mosaic-dev: cannot bind {addr}: {e}");
            return;
        }
    };
    eprintln!("mosaic-dev: serving http://{addr}");

    maybe_open_browser(&format!("http://{addr}"), cli.no_open);

    // Spawn the watcher on a side thread so the main thread can serve
    // requests synchronously.
    let watcher_root = cli.package_root.clone();
    let watcher_work = work.to_path_buf();
    let watcher_component = cli.component.clone();
    let watcher_backend = Backend::from_cli(&cli.backend).expect("validated earlier");
    std::thread::spawn(move || {
        if let Err(e) = run_watcher(
            &watcher_root,
            &watcher_component,
            watcher_backend,
            &watcher_work,
            /* on_change = */ |_| {},
        ) {
            eprintln!("mosaic-dev: watcher error: {e}");
        }
    });

    for req in server.incoming_requests() {
        let url = req.url();
        let path = if url == "/" {
            work.join(index_filename)
        } else {
            // Strip leading slash, refuse `..` traversal.
            let rel = url.trim_start_matches('/');
            if rel.contains("..") {
                let _ = req.respond(tiny_http::Response::from_string("forbidden").with_status_code(403));
                continue;
            }
            work.join(rel)
        };

        match std::fs::read(&path) {
            Ok(bytes) => {
                let mime = mime_from_path(&path);
                let resp = tiny_http::Response::from_data(bytes)
                    .with_header(tiny_http::Header::from_bytes("Content-Type", mime).unwrap());
                let _ = req.respond(resp);
            }
            Err(_) => {
                let _ = req.respond(
                    tiny_http::Response::from_string("not found").with_status_code(404),
                );
            }
        }
    }
}

/// Map a file extension to a tiny MIME table — enough for the artifacts
/// the kernel produces today.
fn mime_from_path(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("js") => "application/javascript; charset=utf-8",
        Some("mjs") => "application/javascript; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("json") => "application/json; charset=utf-8",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        _ => "application/octet-stream",
    }
}

// ===========================================================================
// SwiftUI: swift run + restart on change
// ===========================================================================

/// Spawn `swift run` against the generated host SwiftPM project. We
/// don't have HMR for SwiftUI, so file changes kill the running process
/// and restart it from scratch.
fn run_swiftui(cli: &Cli, work: &Path) {
    let host = work.join("Host");
    let child = spawn_swift_run(&host);
    watch_and_rebuild(cli, work, child, /* restart_on_change = */ true);
}

fn spawn_swift_run(host: &Path) -> Option<Child> {
    let mut cmd = Command::new("swift");
    cmd.arg("run").current_dir(host).stdout(Stdio::inherit()).stderr(Stdio::inherit());
    match cmd.spawn() {
        Ok(c) => Some(c),
        Err(e) => {
            eprintln!(
                "mosaic-dev: failed to launch `swift run` in {} ({e}). \
                 Install Swift 5.9+ from https://swift.org",
                host.display()
            );
            None
        }
    }
}

// ===========================================================================
// Qt: qmlscene + restart on change
// ===========================================================================

fn run_qt(cli: &Cli, work: &Path) {
    let qml = work.join("qt").join("main.qml");
    let child = spawn_qmlscene(&qml);
    watch_and_rebuild(cli, work, child, /* restart_on_change = */ true);
}

fn spawn_qmlscene(qml_path: &Path) -> Option<Child> {
    let mut cmd = Command::new("qmlscene");
    cmd.arg(qml_path).stdout(Stdio::inherit()).stderr(Stdio::inherit());
    match cmd.spawn() {
        Ok(c) => Some(c),
        Err(e) => {
            eprintln!(
                "mosaic-dev: failed to launch qmlscene ({e}). \
                 Install Qt 5 / 6 dev tools and ensure `qmlscene` is on PATH."
            );
            None
        }
    }
}

// ===========================================================================
// Shared: file watcher + rebuild loop
// ===========================================================================

/// Watch `<package_root>/src/` for `.mil`/`.mll`/`.msl` changes and re-
/// run `build_and_wrap` on each batch.
///
/// `restart_on_change` controls process supervision: native backends
/// (SwiftUI, Qt) get a SIGTERM-and-respawn loop; web backends rely on
/// Vite/HTTP picking up file changes automatically.
fn watch_and_rebuild(cli: &Cli, work: &Path, mut child: Option<Child>, restart_on_change: bool) {
    let backend = Backend::from_cli(&cli.backend).expect("validated earlier");
    let package_root = cli.package_root.clone();
    let component = cli.component.clone();
    let work = work.to_path_buf();

    let result = run_watcher(&package_root, &component, backend, &work, |_| {
        if restart_on_change {
            if let Some(c) = child.as_mut() {
                let _ = c.kill();
                let _ = c.wait();
            }
            child = match backend {
                Backend::SwiftUI => spawn_swift_run(&work.join("Host")),
                Backend::Qt => spawn_qmlscene(&work.join("qt").join("main.qml")),
                _ => None,
            };
        }
    });

    if let Err(e) = result {
        eprintln!("mosaic-dev: watcher error: {e}");
    }

    if let Some(mut c) = child {
        let _ = c.wait();
    }
}

/// Block on the file-system watcher; rebuild on each debounced batch.
///
/// `on_change` is called *after* a successful rebuild, giving the caller
/// a chance to bounce its supervised process.
fn run_watcher<F: FnMut(&[Event])>(
    package_root: &Path,
    component: &str,
    backend: Backend,
    work: &Path,
    mut on_change: F,
) -> Result<(), String> {
    let (tx, rx) = mpsc::channel::<notify::Result<Event>>();
    let mut watcher =
        notify::recommended_watcher(tx).map_err(|e| format!("watcher init: {e}"))?;
    let src = package_root.join("src");
    watcher
        .watch(&src, RecursiveMode::Recursive)
        .map_err(|e| format!("watch {}: {e}", src.display()))?;

    eprintln!("mosaic-dev: watching {} for changes", src.display());

    // Debounce: collect events until 100ms of quiet, then rebuild once.
    let debounce = Duration::from_millis(100);
    let mut pending: Vec<Event> = Vec::new();
    let mut last_event: Option<Instant> = None;

    loop {
        let wait = match last_event {
            Some(t) => debounce.saturating_sub(t.elapsed()),
            None => Duration::from_secs(60 * 60), // arbitrary long wait
        };

        match rx.recv_timeout(wait) {
            Ok(Ok(ev)) => {
                if is_interesting(&ev) {
                    pending.push(ev);
                    last_event = Some(Instant::now());
                }
            }
            Ok(Err(e)) => eprintln!("mosaic-dev: watcher event error: {e}"),
            Err(mpsc::RecvTimeoutError::Timeout) => {
                if !pending.is_empty()
                    && last_event.is_some_and(|t| t.elapsed() >= debounce)
                {
                    let batch = std::mem::take(&mut pending);
                    last_event = None;
                    eprintln!(
                        "mosaic-dev: {} change(s) detected, rebuilding…",
                        batch.len()
                    );
                    if let Err(e) = build_and_wrap(package_root, component, backend, work) {
                        eprintln!("mosaic-dev: rebuild failed: {e}");
                    } else {
                        on_change(&batch);
                    }
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Err("watcher channel disconnected".to_string());
            }
        }
    }
}

/// Decide whether an event should trigger a rebuild.
///
/// We filter on event kind (modify/create/remove) and on the file
/// extension being one of the three Mosaic source extensions.  Bare
/// access events from inotify/FSEvents shouldn't trigger rebuilds.
fn is_interesting(ev: &Event) -> bool {
    matches!(
        ev.kind,
        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
    ) && ev.paths.iter().any(|p| {
        matches!(
            p.extension().and_then(|e| e.to_str()),
            Some("mil") | Some("mll") | Some("msl")
        )
    })
}

// ===========================================================================
// Browser opening
// ===========================================================================

/// Open the given URL in the user's default browser, unless `--no-open`
/// was passed.  We use platform-native openers and ignore errors —
/// failure to open a browser is never fatal to the dev session.
fn maybe_open_browser(url: &str, no_open: bool) {
    if no_open {
        return;
    }
    // Tiny delay so the server has a chance to bind before the browser
    // pings it.
    std::thread::sleep(Duration::from_millis(250));

    #[cfg(target_os = "macos")]
    let _ = Command::new("open").arg(url).status();
    #[cfg(target_os = "linux")]
    let _ = Command::new("xdg-open").arg(url).status();
    #[cfg(target_os = "windows")]
    let _ = Command::new("cmd").args(["/C", "start", url]).status();
}

// ===========================================================================
// Tests — CLI parsing
//
// All the wrapper-generation tests live in `lib.rs`.  Here we only test
// the clap shape so misspelt flag names get caught at build time.
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn cli_parses_minimum_args() {
        let cli = Cli::parse_from([
            "mosaic-dev",
            "/tmp/pkg",
            "--backend",
            "react",
            "--component",
            "Card",
        ]);
        assert_eq!(cli.backend, "react");
        assert_eq!(cli.component, "Card");
        assert_eq!(cli.port, 5173);
        assert!(!cli.no_open);
    }

    #[test]
    fn cli_parses_all_args() {
        let cli = Cli::parse_from([
            "mosaic-dev",
            "/tmp/pkg",
            "--backend",
            "html",
            "--component",
            "Grid",
            "--port",
            "8080",
            "--no-open",
        ]);
        assert_eq!(cli.port, 8080);
        assert!(cli.no_open);
    }
}
