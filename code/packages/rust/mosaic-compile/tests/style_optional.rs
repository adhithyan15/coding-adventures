use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(4)
        .expect("mosaic-compile lives at code/packages/rust/mosaic-compile")
        .to_path_buf()
}

fn temporary_output_directory() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock")
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "mosaic-compile-style-optional-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir_all(&path).expect("temporary output directory");
    path
}

#[test]
fn repository_column_compiles_without_a_stylesheet_on_browser_backends() {
    let column_root = repository_root().join("code/packages/mosaic/mosaic-pkg-grid/src");
    let interface = column_root.join("Column.mil");
    let layout = column_root.join("Column.mll");
    assert!(interface.is_file(), "missing {}", interface.display());
    assert!(layout.is_file(), "missing {}", layout.display());

    let output_directory = temporary_output_directory();
    for (backend, extension) in [("html", "html"), ("webcomponent", "js"), ("react", "tsx")] {
        let output = output_directory.join(format!("Column.{extension}"));
        let result = Command::new(env!("CARGO_BIN_EXE_mosaic-compile"))
            .args([
                "--backend",
                backend,
                "--interface",
                interface.to_str().unwrap(),
                "--layout",
                layout.to_str().unwrap(),
                "--output",
                output.to_str().unwrap(),
            ])
            .output()
            .expect("run mosaic-compile");

        assert!(
            result.status.success(),
            "mosaic-compile --backend {backend} failed without --style:\n{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(
            fs::metadata(&output).is_ok_and(|metadata| metadata.len() > 0),
            "missing non-empty {backend} output at {}",
            output.display()
        );
    }

    fs::remove_dir_all(output_directory).ok();
}
