//! The `twig-vm` command line holds a source file to the compiler's source
//! budget *before* reading it into memory (#18474).
//!
//! Until then the CLI called `read_to_string`, so a file of any size was
//! read whole and only then refused by `compile_source`, and a file that
//! never ends (`/dev/zero`) was read until memory ran out, and a FIFO
//! blocked it forever. It now reads through `twig_module_driver::read_source`
//! (`bounded-source-file`): anything but a regular file is refused without
//! blocking, the open file's size is checked first, and the read itself
//! stops after `MAX_SOURCE_BYTES + 1` bytes.
//!
//! Not built under Miri (`code/scripts/miri-twig-vm.sh` runs every twig-vm
//! test target): each test spawns the CLI as a child process, which Miri's
//! isolation does not support, and the code under test is safe Rust I/O.

#![cfg(not(miri))]

use std::path::PathBuf;
use std::process::{Command, Output};

use twig_ir_compiler::MAX_SOURCE_BYTES;

/// A scratch directory unique to this test process.
fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("twig_vm_cli_{tag}_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn run_cli(path: &std::path::Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_twig-vm")).arg(path).output().expect("twig-vm runs")
}

/// A program of exactly `len` bytes: a comment filling the budget, then `1`.
fn program(len: usize) -> String {
    let src = format!(";{}\n1", "a".repeat(len - 3));
    assert_eq!(src.len(), len);
    src
}

#[test]
fn a_file_of_exactly_the_source_budget_runs() {
    let dir = scratch("at");
    let path = dir.join("at.tw");
    std::fs::write(&path, program(MAX_SOURCE_BYTES)).unwrap();
    let out = run_cli(&path);
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_file_one_byte_over_the_source_budget_is_refused() {
    let dir = scratch("over");
    let path = dir.join("over.tw");
    std::fs::write(&path, program(MAX_SOURCE_BYTES + 1)).unwrap();
    let out = run_cli(&path);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(1), "stderr: {stderr}");
    assert!(
        stderr.contains(&format!("source bytes exceeds the limit of {MAX_SOURCE_BYTES}")),
        "stderr: {stderr}"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

/// `/dev/zero` is not a regular file, so it is refused before it is
/// opened; `read_to_string` used to read it until memory ran out.
#[cfg(unix)]
#[test]
fn a_device_is_refused_as_not_a_regular_file() {
    let out = run_cli(std::path::Path::new("/dev/zero"));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(1), "stderr: {stderr}");
    assert!(stderr.contains("not a regular file"), "stderr: {stderr}");
}

/// A FIFO with no writer blocked `read_to_string`'s `open` forever. It is
/// refused, and the CLI exits; the test kills it after 10 s rather than
/// hang if it does not.
#[cfg(unix)]
#[test]
fn a_fifo_is_refused_without_blocking() {
    let dir = scratch("fifo");
    let fifo = dir.join("pipe.tw");
    let made = Command::new("mkfifo").arg(&fifo).status();
    if !made.map(|s| s.success()).unwrap_or(false) {
        eprintln!("mkfifo unavailable; skipping");
        return;
    }
    let mut child = Command::new(env!("CARGO_BIN_EXE_twig-vm"))
        .arg(&fifo)
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("twig-vm runs");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if std::time::Instant::now() > deadline {
            let _ = child.kill();
            panic!("twig-vm blocked on a FIFO");
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    };
    let out = child.wait_with_output().unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(status.code(), Some(1), "stderr: {stderr}");
    assert!(stderr.contains("not a regular file"), "stderr: {stderr}");
    std::fs::remove_dir_all(&dir).unwrap();
}

/// The #18474 security review's repro through the CLI: a record with a
/// 512 KB name and 1,000 fields (≈ 512 MB of accessor names before the
/// name budget) is a compile error naming the budget. A 256-byte name runs.
#[test]
fn a_long_record_name_is_refused_before_any_accessor_name_is_made() {
    let dir = scratch("long_name");
    let record = |name_len: usize| {
        let fields: String = (0..1_000).map(|i| format!(" (f{i} : int)")).collect();
        format!("(record {}{fields})\n1\n", "A".repeat(name_len))
    };
    let at = dir.join("at.tw");
    std::fs::write(&at, record(256)).unwrap();
    let out = run_cli(&at);
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));

    let over = dir.join("over.tw");
    std::fs::write(&over, record(1 << 19)).unwrap();
    let out = run_cli(&over);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(1), "stderr: {stderr}");
    assert!(stderr.contains("name bytes exceeds the limit of 256"), "stderr: {stderr}");
    std::fs::remove_dir_all(&dir).unwrap();
}
