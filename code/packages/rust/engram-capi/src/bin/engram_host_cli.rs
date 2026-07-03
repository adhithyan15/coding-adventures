use std::env;
use std::fs;
use std::path::Path;
use std::process;

use engram_anki_package::write_legacy_apkg_from_engram_state;
use engram_core_wasm::EngramSession;
use serde_json::{json, Value};

fn main() {
    let result = run();
    match result {
        Ok(value) => {
            println!("{value}");
        }
        Err(error) => {
            println!("{}", json!({ "ok": false, "error": error }));
            process::exit(1);
        }
    }
}

fn run() -> Result<Value, String> {
    let args = env::args().collect::<Vec<_>>();
    match args.get(1).map(String::as_str) {
        Some("merge-apkg") => {
            let snapshot_path = required_arg(&args, 2, "snapshot path")?;
            let apkg_path = required_arg(&args, 3, "APKG path")?;
            merge_apkg(snapshot_path, apkg_path)
        }
        Some("export-apkg") => {
            let snapshot_path = required_arg(&args, 2, "snapshot path")?;
            let output_path = required_arg(&args, 3, "output APKG path")?;
            export_apkg(snapshot_path, output_path)
        }
        _ => Err("usage: engram-host-cli <merge-apkg|export-apkg> <snapshot-path> <path>".into()),
    }
}

fn merge_apkg(snapshot_path: &str, apkg_path: &str) -> Result<Value, String> {
    let mut session = load_session(snapshot_path)?;
    let bytes = fs::read(apkg_path).map_err(|err| format!("failed to read APKG file: {err}"))?;
    let merged = json_result(&session.merge_anki_apkg(&bytes))?;
    if merged["ok"] != true {
        return Err(merged["error"]
            .as_str()
            .unwrap_or("failed to merge APKG")
            .to_string());
    }
    write_snapshot(snapshot_path, &session)?;
    Ok(json!({
        "ok": true,
        "status": "imported",
        "path": apkg_path,
        "snapshotPath": snapshot_path
    }))
}

fn export_apkg(snapshot_path: &str, output_path: &str) -> Result<Value, String> {
    let session = load_session(snapshot_path)?;
    let apkg =
        write_legacy_apkg_from_engram_state(session.state(), &[]).map_err(|error| error.message)?;
    ensure_parent(output_path)?;
    fs::write(output_path, apkg).map_err(|err| format!("failed to write APKG file: {err}"))?;
    Ok(json!({
        "ok": true,
        "status": "exported",
        "path": output_path
    }))
}

fn load_session(snapshot_path: &str) -> Result<EngramSession, String> {
    let mut session = EngramSession::new_demo();
    if Path::new(snapshot_path).exists() {
        let snapshot = fs::read_to_string(snapshot_path)
            .map_err(|err| format!("failed to read Engram snapshot: {err}"))?;
        let loaded = json_result(&session.load_snapshot(&snapshot))?;
        if loaded["ok"] != true {
            return Err(loaded["error"]
                .as_str()
                .unwrap_or("failed to load Engram snapshot")
                .to_string());
        }
    }
    Ok(session)
}

fn write_snapshot(snapshot_path: &str, session: &EngramSession) -> Result<(), String> {
    ensure_parent(snapshot_path)?;
    let snapshot = serde_json::to_string(session.state())
        .map_err(|err| format!("failed to serialize Engram snapshot: {err}"))?;
    fs::write(snapshot_path, snapshot)
        .map_err(|err| format!("failed to write Engram snapshot: {err}"))
}

fn ensure_parent(path: &str) -> Result<(), String> {
    if let Some(parent) = Path::new(path)
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent).map_err(|err| format!("failed to create directory: {err}"))?;
    }
    Ok(())
}

fn json_result(raw: &str) -> Result<Value, String> {
    serde_json::from_str(raw).map_err(|err| format!("Engram returned invalid JSON: {err}"))
}

fn required_arg<'a>(args: &'a [String], index: usize, label: &str) -> Result<&'a str, String> {
    args.get(index)
        .map(String::as_str)
        .ok_or_else(|| format!("missing {label}"))
}
