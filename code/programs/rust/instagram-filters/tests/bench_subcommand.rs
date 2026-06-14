//! Integration test for the `bench-specialisation` subcommand.
//!
//! Spawns the compiled `instagram-filters` binary, points it at a
//! tiny PPM, and asserts the two output files exist with sensible
//! content.  The unit tests in `src/bench.rs` cover the library
//! surface; this test exists specifically to lock down the binary's
//! argv handling and exit codes — the contract the CLI presents to
//! shell users.

use std::process::Command;

fn binary_path() -> std::path::PathBuf {
    // CARGO_BIN_EXE_<name> is set by Cargo when running integration
    // tests, so we don't have to guess at target/debug paths.
    std::path::PathBuf::from(env!("CARGO_BIN_EXE_instagram-filters"))
}

fn write_tiny_ppm(path: &std::path::Path) {
    // Hand-build a P6 PPM with 2×2 pixels.  No alpha; image-codec-ppm
    // synthesises alpha=255 on decode.
    let mut bytes: Vec<u8> = b"P6\n2 2\n255\n".to_vec();
    bytes.extend_from_slice(&[200, 100, 50]); // (0,0)
    bytes.extend_from_slice(&[50, 100, 200]); // (1,0)
    bytes.extend_from_slice(&[120, 220, 90]); // (0,1)
    bytes.extend_from_slice(&[240, 30, 180]); // (1,1)
    std::fs::write(path, &bytes).unwrap();
}

fn workdir(name: &str) -> std::path::PathBuf {
    let mut p = std::env::temp_dir();
    p.push(format!(
        "instagram-filters-int-{}-{}",
        name,
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    p
}

#[test]
fn bench_subcommand_happy_path_writes_outputs() {
    let dir = workdir("happy");
    let input = dir.join("in.ppm");
    write_tiny_ppm(&input);
    let out_dir = dir.join("out");

    let status = Command::new(binary_path())
        .arg("bench-specialisation")
        .arg(&input)
        .arg(&out_dir)
        .arg("--iterations")
        .arg("4")
        .arg("--batch")
        .arg("2")
        .status()
        .expect("spawn binary");
    assert!(status.success(), "binary exited with {:?}", status.code());

    // Both output files must exist.
    let result = out_dir.join("result.ppm");
    let summary = out_dir.join("summary.json");
    assert!(std::fs::metadata(&result).unwrap().len() > 0);
    let summary_text = std::fs::read_to_string(&summary).unwrap();

    // Spot-check the JSON shape — full schema is covered by the lib
    // tests, here we only assert the binary actually wrote it.
    assert!(summary_text.contains("\"iterations\": 4"));
    assert!(summary_text.contains("\"batch\": 2"));
    assert!(summary_text.contains("\"snapshots\":"));
    assert!(summary_text.contains("\"spec_cache_len\""));
    assert!(summary_text.contains("\"specialised_install_count\""));
    assert!(summary_text.contains("\"specialised_dispatch_count\""));
    assert!(summary_text.contains("\"deoptimisation_count\""));
}

#[test]
fn bench_subcommand_missing_positionals_exits_nonzero() {
    let status = Command::new(binary_path())
        .arg("bench-specialisation")
        .status()
        .expect("spawn binary");
    assert!(!status.success());
    assert_eq!(status.code(), Some(2));
}

#[test]
fn bench_subcommand_unknown_flag_exits_nonzero() {
    let dir = workdir("unknown-flag");
    let input = dir.join("in.ppm");
    write_tiny_ppm(&input);

    let status = Command::new(binary_path())
        .arg("bench-specialisation")
        .arg(&input)
        .arg(dir.join("out"))
        .arg("--frob")
        .arg("1")
        .status()
        .expect("spawn binary");
    assert_eq!(status.code(), Some(2));
}

#[test]
fn bench_subcommand_missing_input_file_exits_runtime_error() {
    let dir = workdir("missing-file");
    let status = Command::new(binary_path())
        .arg("bench-specialisation")
        .arg(dir.join("does-not-exist.ppm"))
        .arg(dir.join("out"))
        .arg("--iterations")
        .arg("1")
        .arg("--batch")
        .arg("1")
        .status()
        .expect("spawn binary");
    // Runtime error path returns 6 from main.rs.
    assert_eq!(status.code(), Some(6));
}

#[test]
fn help_still_works_alongside_subcommand() {
    let out = Command::new(binary_path())
        .arg("--help")
        .output()
        .expect("spawn binary");
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("bench-specialisation"));
}
