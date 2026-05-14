use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_dir(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "required-capabilities-compiler-{name}-{}-{nanos}",
        std::process::id()
    ))
}

fn weather_manifest_json() -> &'static str {
    r#"{
      "version": 1,
      "package": "rust/weather-agent-e2e",
      "capabilities": [
        {
          "category": "net",
          "action": "dns",
          "target": "api.weather.gov",
          "justification": "Resolve Weather.gov."
        },
        {
          "category": "net",
          "action": "connect",
          "target": "api.weather.gov:443",
          "justification": "Fetch Weather.gov over TLS."
        }
      ]
    }"#
}

#[test]
fn cli_writes_and_checks_generated_operation_source() {
    let dir = temp_dir("write-check");
    fs::create_dir_all(&dir).expect("temp dir should be created");
    let input = dir.join("required_capabilities.json");
    let output = dir.join("generated_operations.rs");
    fs::write(&input, weather_manifest_json()).expect("manifest should be written");

    let package_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let write_status = Command::new(env!("CARGO_BIN_EXE_required-capabilities-compiler"))
        .current_dir(&package_dir)
        .arg("--input")
        .arg(&input)
        .arg("--output")
        .arg(&output)
        .status()
        .expect("compiler command should run");
    assert!(write_status.success());

    let generated = fs::read_to_string(&output).expect("generated source should be readable");
    assert!(generated.contains("GeneratedOperationHttpClient"));
    assert!(generated.contains("\"api.weather.gov\""));
    assert!(!generated.contains("required_capabilities.json"));

    let check_status = Command::new(env!("CARGO_BIN_EXE_required-capabilities-compiler"))
        .current_dir(&package_dir)
        .arg("--input")
        .arg(&input)
        .arg("--output")
        .arg(&output)
        .arg("--check")
        .status()
        .expect("compiler check command should run");
    assert!(check_status.success());

    fs::write(&output, "// stale\n").expect("stale generated source should be written");
    let stale_status = Command::new(env!("CARGO_BIN_EXE_required-capabilities-compiler"))
        .current_dir(&package_dir)
        .arg("--input")
        .arg(&input)
        .arg("--output")
        .arg(&output)
        .arg("--check")
        .status()
        .expect("compiler stale check should run");
    assert!(!stale_status.success());

    let _ = fs::remove_dir_all(&dir);
}
