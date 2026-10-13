//! #16930: the Compose pipeline cannot render story fixtures or write a
//! project shell yet (#14704). It used to ignore both silently, so a story
//! passed `--strict-fixtures` on Compose while showing nothing of its fixture.
//! Now strict mode refuses, and otherwise each flag warns.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(4)
        .expect("mosaic-compile lives at code/packages/rust/mosaic-compile")
        .to_path_buf()
}

fn scratch() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock")
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "mosaic-compile-compose-fixtures-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir_all(&path).expect("scratch directory");
    path
}

fn compile(dir: &Path, extra: &[&str]) -> Output {
    let src = repository_root().join("code/packages/mosaic/mosaic-pkg-toolkit/src");
    let fixtures = dir.join("fixtures.json");
    fs::write(&fixtures, r#"{ "label": "Save" }"#).expect("write fixtures");
    let mut args: Vec<String> = [
        "--interface",
        src.join("Button.mil").to_str().unwrap(),
        "--layout",
        src.join("Button.mll").to_str().unwrap(),
        "--backend",
        "compose",
        "--output",
        dir.join("Button.kt").to_str().unwrap(),
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    for flag in extra {
        if *flag == "--fixtures" {
            args.push("--fixtures".into());
            args.push(fixtures.to_str().unwrap().into());
        } else {
            args.push(flag.to_string());
        }
    }
    Command::new(env!("CARGO_BIN_EXE_mosaic-compile"))
        .args(&args)
        .output()
        .expect("run mosaic-compile")
}

#[test]
fn compose_refuses_fixtures_under_strict_mode_and_warns_otherwise() {
    let dir = scratch();

    let strict = compile(&dir, &["--fixtures", "--strict-fixtures"]);
    assert!(!strict.status.success(), "strict mode must refuse on compose");
    assert!(
        String::from_utf8_lossy(&strict.stderr).contains("cannot render --fixtures yet (#14704)"),
        "{}",
        String::from_utf8_lossy(&strict.stderr)
    );

    let lenient = compile(&dir, &["--fixtures", "--emit-project"]);
    let stderr = String::from_utf8_lossy(&lenient.stderr);
    assert!(lenient.status.success(), "{stderr}");
    assert!(stderr.contains("compose pipeline ignores --fixtures"), "{stderr}");
    assert!(stderr.contains("no --emit-project shell"), "{stderr}");

    let plain = compile(&dir, &[]);
    let stderr = String::from_utf8_lossy(&plain.stderr);
    assert!(plain.status.success(), "{stderr}");
    assert!(!stderr.contains("warning"), "no flag, no warning: {stderr}");

    fs::remove_dir_all(&dir).ok();
}
