//! Release-codegen regression gate for the fixed-memory Expand boundary.
//!
//! Source-level ownership tests cannot prove that an ABI return slot containing
//! `T(i)` was scrubbed after the tag was copied. This test asks the active Rust
//! compiler for optimized LLVM IR and verifies that both HMAC result payloads
//! and the previous-block owner reach the repository zeroize primitive, while
//! the public Expand function contains no allocator call.

use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn optimized_fixed_expand_scrubs_result_slots_without_allocating() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_dir = manifest_dir
        .parent()
        .expect("HKDF package must be inside the Rust workspace");
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock must follow the Unix epoch")
        .as_nanos();
    let target_dir = std::env::temp_dir().join(format!(
        "coding-adventures-hkdf-ir-{}-{unique}",
        std::process::id()
    ));
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());

    let compilation = Command::new(cargo)
        .current_dir(workspace_dir)
        .env("CARGO_TARGET_DIR", &target_dir)
        .args([
            "rustc",
            "-p",
            "coding_adventures_hkdf",
            "--release",
            "--lib",
            "--",
            "--emit=llvm-ir",
        ])
        .output()
        .expect("Cargo must be available to the test process");

    assert!(
        compilation.status.success(),
        "optimized IR compilation failed:\n{}",
        String::from_utf8_lossy(&compilation.stderr)
    );

    let deps_dir = target_dir.join("release/deps");
    let ir_path = fs::read_dir(&deps_dir)
        .expect("release dependency directory must exist")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| {
            path.extension().is_some_and(|extension| extension == "ll")
                && path.file_name().is_some_and(|name| {
                    name.to_string_lossy()
                        .starts_with("coding_adventures_hkdf-")
                })
        })
        .expect("Cargo must emit HKDF LLVM IR");
    let ir = fs::read_to_string(ir_path).expect("optimized LLVM IR must be readable");
    fs::remove_dir_all(&target_dir).expect("temporary codegen directory must be removable");

    let expand = rust_function_body(&ir, "coding_adventures_hkdf::hkdf_expand_fixed");
    for forbidden in [
        "__rust_alloc",
        "__rust_realloc",
        "alloc::alloc",
        "exchange_malloc",
        "RawVec",
    ] {
        assert!(
            !expand.contains(forbidden),
            "fixed Expand unexpectedly references allocator symbol {forbidden}:\n{expand}"
        );
    }

    assert_result_slot_wiped(expand, 33, 32);
    assert_result_slot_wiped(expand, 49, 48);
    let previous = alloca_name(expand, 48, 1);
    assert_wipe_precedes_lifetime_end(expand, &previous, &previous, 48, 48);
}

fn assert_result_slot_wiped(function: &str, result_size: usize, tag_size: usize) {
    let result = alloca_name(function, result_size, 1);
    let payload_marker = format!("ptr {result}, i64 1");
    let payload = function
        .lines()
        .find_map(|line| {
            let line = line.trim();
            (line.contains("getelementptr") && line.contains(&payload_marker)).then(|| {
                line.split(" = ")
                    .next()
                    .expect("GEP must define a value")
                    .to_owned()
            })
        })
        .unwrap_or_else(|| panic!("optimized Expand has no payload pointer for {result}"));

    assert_wipe_precedes_lifetime_end(function, &result, &payload, result_size, tag_size);
}

fn alloca_name(function: &str, size: usize, alignment: usize) -> String {
    let alloca_suffix = format!(" = alloca [{size} x i8], align {alignment}");
    let matches: Vec<String> = function
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            line.strip_suffix(&alloca_suffix).map(str::to_owned)
        })
        .collect();
    assert_eq!(
        matches.len(),
        1,
        "optimized Expand must have exactly one {size}-byte aligned-{alignment} slot, found {matches:?}"
    );
    matches.into_iter().next().expect("one slot was required")
}

fn assert_wipe_precedes_lifetime_end(
    function: &str,
    owner: &str,
    wipe_pointer: &str,
    owner_size: usize,
    wipe_size: usize,
) {
    let lines: Vec<&str> = function.lines().collect();
    let wipe_pointer_marker = format!("{wipe_pointer}, i64 noundef {wipe_size}");
    let wipe_index = lines
        .iter()
        .position(|line| line.contains("zeroize_bytes") && line.contains(&wipe_pointer_marker))
        .unwrap_or_else(|| {
            panic!("optimized Expand did not scrub {wipe_size} bytes through {wipe_pointer}")
        });
    let lifetime_marker = format!("lifetime.end.p0(i64 {owner_size}, ptr nonnull {owner})");
    let lifetime_end_index = lines
        .iter()
        .position(|line| line.contains(&lifetime_marker))
        .unwrap_or_else(|| panic!("optimized Expand did not end the lifetime of {owner}"));

    assert!(
        wipe_index < lifetime_end_index,
        "optimized Expand ended {owner} before scrubbing {wipe_pointer}:\n{function}"
    );
}

fn rust_function_body<'a>(ir: &'a str, rust_name: &str) -> &'a str {
    let marker = format!("; {rust_name}\n");
    let marker_start = ir
        .find(&marker)
        .unwrap_or_else(|| panic!("optimized IR did not identify {rust_name}"));
    let definition = marker_start
        + ir[marker_start..]
            .find("define ")
            .expect("optimized function must have a definition");
    let end = ir[definition..]
        .find("\n}")
        .map(|offset| definition + offset + 2)
        .expect("optimized function must have a closing brace");
    &ir[definition..end]
}
