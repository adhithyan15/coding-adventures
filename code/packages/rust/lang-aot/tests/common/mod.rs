//! Shared test helpers for the lang-aot integration tests.
//!
//! Files under `tests/common/` are **not** compiled as their own test target
//! (only top-level `tests/*.rs` are), so this module can be `mod common;`-included
//! by any integration test without emitting a spurious empty test binary.

use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::OnceLock;

/// Build gc-core-capi's release staticlib and return the path to
/// the static archive reported by Cargo, including custom target directories.
///
/// #118b-2b retired `twig-aot/runtime/twig_gc.c`; the garbage collector now lives
/// in the `gc-core-capi` crate. The LLVM/WASM emit tests used to hand `twig_gc.c`
/// to `clang` alongside `dynval_runtime.c` + `twig_runtime.c`; they now link this
/// Rust `staticlib` instead, which exports the same `__twig_gc_*` ABI (plus the
/// generic `__gc_*` names) that the emitted code and `dynval_runtime.c` reference.
///
/// # Why the build runs once per test process
///
/// Cargo's target-dir lock serialises concurrent *builds*, but it does not
/// protect a *reader* once a build has returned. Cargo re-creates the archive
/// it copies up to `target/release/libgc_core_capi.a` on EVERY build, even a
/// fully cached one: it deletes the old file and hard-links a new one in its
/// place (the file's inode changes on each no-op `cargo build`). So when
/// parallel test threads each ran their own nested build, this race could
/// happen:
///
/// ```text
///   thread A                         thread B
///   cargo build ... returns
///   (lock released)                  cargo build ... takes the lock
///                                      deletes libgc_core_capi.a   <-- gap
///   archive.is_file() -> false  !!     hard-links the new one
/// ```
///
/// Thread A then panicked with "Cargo-reported archive does not exist", and a
/// `clang` reading the archive in that same gap could fail too. It hit
/// `lang_matrix`'s `pr5a_string_failures_are_registered_traps_on_every_column`
/// on a macOS CI runner. Locally, 40 back-to-back cached builds with a reader
/// polling the path saw it missing 263 times in about 2.3 million checks.
///
/// The fix: run the nested build exactly once per test process and hand every
/// thread the same path. No nested Cargo runs after that first build in this
/// process, so nothing can delete the file under a reader. (If the first build
/// panics, `OnceLock` stays empty and the next caller retries, which reports
/// the same failure.) `cargo_archive_path`'s child test uses its own
/// `CARGO_TARGET_DIR` in a separate process, so it never shares this file.
pub fn gc_core_capi_archive() -> PathBuf {
    static ARCHIVE: OnceLock<PathBuf> = OnceLock::new();
    ARCHIVE.get_or_init(build_gc_core_capi_archive).clone()
}

/// The single nested `cargo build` behind [`gc_core_capi_archive`]. Call that
/// function instead; calling this directly from parallel threads brings back
/// the race described there.
fn build_gc_core_capi_archive() -> PathBuf {
    // `CARGO_MANIFEST_DIR` for a lang-aot test is `.../code/packages/rust/lang-aot`;
    // its parent is the workspace root that owns the shared `target/` dir.
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest.parent().expect("lang-aot has a parent dir");

    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let output = Command::new(&cargo)
        .args([
            "build",
            "--release",
            "-p",
            "gc-core-capi",
            "--message-format=json-render-diagnostics",
        ])
        .current_dir(workspace_root)
        // Cargo keeps readable diagnostics on stderr; stdout is the artifact protocol.
        .stderr(Stdio::inherit())
        .output()
        .expect("spawn cargo build for gc-core-capi staticlib");
    assert!(
        output.status.success(),
        "gc-core-capi staticlib build failed: {}",
        output.status
    );
    let messages = std::str::from_utf8(&output.stdout).expect("Cargo JSON must be UTF-8");
    let archive = gc_staticlib_from_messages(messages).expect("locate Cargo's GC staticlib");
    assert!(
        archive.is_file(),
        "Cargo-reported archive does not exist: {}",
        archive.display()
    );
    archive
}

/// Select the staticlib from Cargo's artifact stream, never from a guessed path.
/// A crate may also emit an rlib, DLL and DLL import library in the same record.
/// Exact staticlib basenames distinguish those outputs on Unix and MSVC hosts.
pub fn gc_staticlib_from_messages(messages: &str) -> Result<PathBuf, String> {
    let mut archives = std::collections::BTreeSet::new();
    for (index, line) in messages.lines().enumerate() {
        let message: serde_json::Value = serde_json::from_str(line)
            .map_err(|error| format!("invalid Cargo JSON on line {}: {error}", index + 1))?;
        if message["reason"] != "compiler-artifact"
            || message["target"]["name"] != "gc_core_capi"
            || !message["target"]["crate_types"]
                .as_array()
                .is_some_and(|types| types.iter().any(|kind| kind == "staticlib"))
        {
            continue;
        }
        if let Some(filenames) = message["filenames"].as_array() {
            for filename in filenames.iter().filter_map(|value| value.as_str()) {
                let path = PathBuf::from(filename);
                if matches!(
                    path.file_name().and_then(|name| name.to_str()),
                    Some("gc_core_capi.lib" | "libgc_core_capi.a")
                ) {
                    archives.insert(path);
                }
            }
        }
    }
    if archives.len() != 1 {
        return Err(format!(
            "expected one gc_core_capi staticlib from Cargo, found {}: {archives:?}",
            archives.len()
        ));
    }
    Ok(archives.into_iter().next().unwrap())
}

/// The `clang`/`cc` arguments that replace the retired `twig_gc.c` object: the
/// gc-core-capi staticlib, followed by the system libraries its bundled Rust std
/// needs. On Linux that is `-lpthread -ldl`; macOS provides both through
/// `libSystem`, so no extra flags (and `-ldl` would error — there is no libdl).
/// On Windows, `cc`/rustc default to the DYNAMIC CRT on this target, so the
/// staticlib carries `__imp_`-style dllimport references (malloc/memcpy/
/// abort/...) that only `ucrt`/`vcruntime`/`msvcrt` satisfy, plus the Win32
/// API surface gc-core-capi's Rust std pulls in — the same mismatch (and the
/// same fix) as `twig-aot::link_windows_x86_64_executable`'s `libcmt.lib` bug.
///
/// Ordering: the archive comes first so the linker can satisfy
/// `dynval_runtime.o`'s undefined `__twig_gc_alloc` from it, then the system libs
/// resolve the archive's own std references.
pub fn gc_link_args() -> Vec<String> {
    let mut args = vec![gc_core_capi_archive().to_string_lossy().into_owned()];
    if cfg!(target_os = "linux") {
        args.push("-lpthread".into());
        args.push("-ldl".into());
    }
    if cfg!(target_os = "windows") {
        for lib in [
            "ucrt",
            "vcruntime",
            "msvcrt",
            "kernel32",
            "ws2_32",
            "userenv",
            "advapi32",
            "bcrypt",
            "ntdll",
            // Rust 1.100's std uses WaitOnAddress/WakeByAddress* for its futex
            // Mutex/Condvar, which live in synchronization.lib.
            "synchronization",
        ] {
            args.push(format!("-l{lib}"));
        }
    }
    args
}
