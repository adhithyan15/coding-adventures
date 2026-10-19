use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// A scratch-name nonce that is unique for every call in this process.
///
/// The clock alone is not: macOS reports `SystemTime` in whole microseconds,
/// so two tests running in parallel threads could read the same "nanos" and
/// be handed the same directory, each seeing the other's files (this made
/// `compose_fixtures` fail about one run in three). The counter makes every
/// call distinct; the clock keeps names from a previous run apart.
fn unique_nonce() -> String {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock after epoch")
        .as_nanos();
    format!("{nanos}-{n}")
}

fn temporary_workspace() -> PathBuf {
    let nonce = unique_nonce();
    let path = std::env::temp_dir().join(format!(
        "mosaic-compile-shared-composition-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir_all(&path).expect("temporary workspace");
    path
}

fn write_dependency(root: &Path) {
    let package = root.join("mosaic-pkg-accent");
    let source = package.join("src");
    fs::create_dir_all(&source).unwrap();
    fs::write(
        package.join("mosaic-package.toml"),
        r#"[package]
name = "mosaic-pkg-accent"
version = "0.1.0"
description = "standalone composition fixture"
license = "MIT"

[components]
exports = ["Accent"]

[dependencies]

[kernel]
version = "1"
"#,
    )
    .unwrap();
    fs::write(
        source.join("Accent.mil"),
        "component Accent { slot label : text ; }",
    )
    .unwrap();
    fs::write(
        source.join("Accent.mll"),
        r#"layout Accent {
  Box [ accent-panel ] {
    Text [ accent-label ] ( content : slot: label )
  }
}"#,
    )
    .unwrap();
    fs::write(
        source.join("Accent.msl"),
        r##"style Accent {
  part accent-panel { background : "#123456" ; }
  part accent-label { color : "#abcdef" ; }
}"##,
    )
    .unwrap();
}

#[test]
fn standalone_pipeline_preserves_dependency_styles() {
    let workspace = temporary_workspace();
    write_dependency(&workspace);
    let consumer = workspace.join("consumer");
    fs::create_dir_all(&consumer).unwrap();
    let interface = consumer.join("Shell.mil");
    let layout = consumer.join("Shell.mll");
    let style = consumer.join("Shell.msl");
    let output = consumer.join("Shell.html");
    fs::write(&interface, "component Shell { slot label : text ; }").unwrap();
    fs::write(
        &layout,
        r#"layout Shell {
  Column [ shell-root ] {
    pkg::mosaic-pkg-accent::Accent ( label : slot: label )
  }
}"#,
    )
    .unwrap();
    fs::write(&style, "style Shell { part shell-root { padding : 8 ; } }").unwrap();

    let result = Command::new(env!("CARGO_BIN_EXE_mosaic-compile"))
        .args([
            "--backend",
            "html",
            "--interface",
            interface.to_str().unwrap(),
            "--layout",
            layout.to_str().unwrap(),
            "--style",
            style.to_str().unwrap(),
            "--package-search-path",
            workspace.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
        ])
        .output()
        .expect("run mosaic-compile");

    assert!(
        result.status.success(),
        "mosaic-compile failed:\n{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let html = fs::read_to_string(&output).expect("standalone HTML artifact");
    assert!(
        html.contains("#123456"),
        "missing dependency panel style:\n{html}"
    );
    assert!(
        html.contains("#abcdef"),
        "missing dependency label style:\n{html}"
    );

    fs::remove_dir_all(workspace).ok();
}
