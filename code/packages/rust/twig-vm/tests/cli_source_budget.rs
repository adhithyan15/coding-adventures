//! The `twig-vm` command line holds a source file to the compiler's source
//! budget *before* reading it into memory (#18474).
//!
//! Until then the CLI called `read_to_string`, so a file of any size was
//! read whole and only then refused by `compile_source`, and a file that
//! never ends (`/dev/zero`, a pipe) was read until memory ran out. It now
//! reads through `twig_module_driver::read_source`: the open file's size is
//! checked first, and the read itself stops after `MAX_SOURCE_BYTES + 1`
//! bytes.
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

/// `/dev/zero` reports a size of 0, so it passes the size check, and never
/// ends. The bounded read refuses it after `MAX_SOURCE_BYTES + 1` bytes; the
/// old `read_to_string` never returned.
#[cfg(unix)]
#[test]
fn a_file_longer_than_its_reported_size_is_refused_by_the_bounded_read() {
    let out = run_cli(std::path::Path::new("/dev/zero"));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(1), "stderr: {stderr}");
    assert!(stderr.contains("source bytes exceeds"), "stderr: {stderr}");
}
