//! LANG-FULL **E4-dyn** — runtime string helper unit tests.
//!
//! The E4-dyn helpers (`__twig_str_concat` / `__twig_str_slice` /
//! `__twig_str_index` / `__twig_str_len` / `__twig_str_cmp`) live in
//! `runtime/twig_runtime.c` and operate on the length-prefixed heap block
//! `[i64 len][bytes…]` that E5 arrays already use.  No backend calls them yet
//! (that lands in E4d-2…4); this test validates the helpers **directly** by
//! compiling a tiny C driver together with the runtime translation unit via the
//! system C compiler and asserting the driver's exit code.
//!
//! Gated to Unix (`cc` present on the linux/macOS CI runners, exactly as the
//! `*_smoke.rs` end-to-end tests assume).  On Windows the file is a no-op — the
//! helpers there are exercised by the Windows smoke path once E4d-4 wires them.

#![cfg(unix)]

use std::path::PathBuf;
use std::process::Command;

/// Absolute path to `runtime/twig_runtime.c` in this crate.
fn runtime_c() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("runtime/twig_runtime.c")
}

/// The C compiler to drive: honour `$CC`, else `cc` (present on every Unix CI
/// runner used by the smoke tests).
fn cc() -> String {
    std::env::var("CC").unwrap_or_else(|_| "cc".to_string())
}

/// Compile `driver_src` (a C `main`) together with the runtime, run it, and
/// return the process's `ExitStatus`.  Panics only on toolchain/compile
/// failure — the *exit status* is the assertion surface for the caller.
fn compile_and_run(driver_src: &str) -> std::process::ExitStatus {
    let dir = tempfile::tempdir().expect("tempdir");
    let src = dir.path().join("driver.c");
    let exe = dir.path().join("driver");
    std::fs::write(&src, driver_src).expect("write driver.c");

    let status = Command::new(cc())
        .arg(&src)
        .arg(runtime_c())
        .arg("-o")
        .arg(&exe)
        .status()
        .expect("failed to spawn C compiler");
    assert!(status.success(), "C driver failed to compile");

    Command::new(&exe).status().expect("failed to run driver")
}

/// Like [`compile_and_run`], but capture stdout and stderr, so a test can
/// check that a trap flushed earlier output and named its LANG83 kind.
fn compile_and_capture(driver_src: &str) -> std::process::Output {
    let dir = tempfile::tempdir().expect("tempdir");
    let src = dir.path().join("driver.c");
    let exe = dir.path().join("driver");
    std::fs::write(&src, driver_src).expect("write driver.c");
    let status = Command::new(cc())
        .arg(&src)
        .arg(runtime_c())
        .arg("-o")
        .arg(&exe)
        .status()
        .expect("failed to spawn C compiler");
    assert!(status.success(), "C driver failed to compile");
    Command::new(&exe).output().expect("failed to run driver")
}

/// LANG83 PR5a: the process failed, `stdout` is exactly what the driver
/// printed before the failing call (so `__twig_trap` flushed it), and stderr
/// names `kind`. Before PR5a the helpers called `abort()` directly: the
/// buffered `before` was lost and stderr was empty.
fn assert_trapped_after(out: &std::process::Output, before: &str, kind: &str) {
    assert!(!out.status.success(), "the call must trap, but the process exited 0");
    assert_eq!(String::from_utf8_lossy(&out.stdout), before, "stdout before the trap");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains(&format!("trap: {kind}")), "stderr must name {kind}: {stderr:?}");
}

/// Shared C prelude: the runtime externs + a `mk` block builder + a `CHECK`
/// macro that returns a distinct non-zero code for each failed assertion (so a
/// failing test names the exact check).
const PRELUDE: &str = r#"
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* The registry ceiling, STR_MAX_BYTES = 2^30 - 64 (LANG83 PR5a). */
#define MAX ((int64_t)1073741760)

int64_t __twig_str_len(int64_t);
int64_t __twig_str_concat(int64_t, int64_t, int64_t);
int64_t __twig_str_slice(int64_t, int64_t, int64_t);
int64_t __twig_str_index(int64_t, int64_t);
int64_t __twig_str_cmp(int64_t, int64_t);
int64_t __twig_str_eq(int64_t, int64_t);

/* `__twig_alloc_bytes` now allocates string blocks through gc-core
 * (`__gc_register_kind` + `__gc_alloc_kind`).  This unit test compiles
 * `twig_runtime.c` standalone to exercise string *logic*, not the collector, so
 * we back those two symbols with a trivial `calloc` stub — the `[len][bytes]`
 * behaviour is identical.  (End-to-end GC reclamation is proven by the
 * `*_smoke.rs` tests, which link the real `libgc_core_capi.a`.)
 *
 * Array reference-tracing fix (Twig GC completion round): `twig_runtime.c`
 * also declares `__gc_register_ref_array_kind` now (for `alloc_array`'s new
 * `__twig_alloc_ref_array_bytes` allocator) — none of these string tests
 * exercise it, but standalone-compiling `twig_runtime.c` still needs every
 * `extern` it references to resolve at link time, so it gets the same
 * trivial stub treatment. */
int64_t __gc_register_kind(const int64_t *offsets, int64_t count) {
    (void)offsets; (void)count; return 1;
}
int64_t __gc_alloc_kind(int64_t n, uint16_t kind) {
    (void)kind; return (int64_t)(intptr_t)calloc(1, (size_t)n);
}
int64_t __gc_register_ref_array_kind(const int64_t *fixed, int64_t fixed_count, int64_t tail_from) {
    (void)fixed; (void)fixed_count; (void)tail_from; return 2;
}

/* Build a [i64 len][bytes] heap block, the E5/E4-dyn string layout. */
static int64_t mk(const char *s, int64_t n) {
    char *p = (char *)malloc(8 + (size_t)n);
    *(int64_t *)p = n;
    if (n > 0) memcpy(p + 8, s, (size_t)n);
    return (int64_t)(intptr_t)p;
}
#define CHECK(cond, code) do { if (!(cond)) return (code); } while (0)
"#;

/// All the valid-path semantics in one driver; exit 0 means every CHECK passed,
/// a non-zero exit names the first failing check.
#[test]
fn runtime_str_helpers_valid_paths() {
    let driver = format!(
        "{PRELUDE}
int main(void) {{
    int64_t he  = mk(\"HE\", 2);
    int64_t llo = mk(\"LLO\", 3);

    /* concat → \"HELLO\" */
    int64_t hello = __twig_str_concat(he, llo, MAX);
    CHECK(__twig_str_len(hello) == 5, 1);
    CHECK(__twig_str_index(hello, 0) == 'H', 2);
    CHECK(__twig_str_index(hello, 4) == 'O', 3);

    /* slice [1,4) = \"ELL\" */
    int64_t ell = __twig_str_slice(hello, 1, 4);
    CHECK(__twig_str_len(ell) == 3, 4);
    CHECK(__twig_str_index(ell, 0) == 'E', 5);
    CHECK(__twig_str_index(ell, 2) == 'L', 6);

    /* concat with an empty operand is identity (by bytes) */
    int64_t empty = mk(\"\", 0);
    int64_t he2 = __twig_str_concat(he, empty, MAX);
    CHECK(__twig_str_eq(he2, he) == 1, 7);
    int64_t he3 = __twig_str_concat(empty, he, MAX);
    CHECK(__twig_str_eq(he3, he) == 1, 8);

    /* cmp: byte order, then length breaks ties (shorter is less) */
    CHECK(__twig_str_cmp(mk(\"AB\", 2), mk(\"AC\", 2)) == -1, 9);
    CHECK(__twig_str_cmp(mk(\"AB\", 2), mk(\"AB\", 2)) ==  0, 10);
    CHECK(__twig_str_cmp(mk(\"ABC\", 3), mk(\"AB\", 2)) ==  1, 11);
    CHECK(__twig_str_cmp(mk(\"AB\", 2), mk(\"ABC\", 3)) == -1, 12);

    /* immutability: concat/slice never mutate their operands */
    CHECK(__twig_str_len(he)  == 2, 13);
    CHECK(__twig_str_len(llo) == 3, 14);

    /* boundary slices: whole string and empty range */
    CHECK(__twig_str_len(__twig_str_slice(hello, 0, 5)) == 5, 15);
    CHECK(__twig_str_len(__twig_str_slice(hello, 2, 2)) == 0, 16);

    /* index returns an unsigned byte (0..255) */
    int64_t hi = mk(\"\\x80\", 1);
    CHECK(__twig_str_index(hi, 0) == 128, 17);

    return 0;
}}
"
    );
    let status = compile_and_run(&driver);
    assert_eq!(
        status.code(),
        Some(0),
        "a CHECK failed; exit code names the check (see runtime_str_helpers_valid_paths)"
    );
}

/// An out-of-range `__twig_str_index` is the registered `Trap.Bounds`
/// (LANG83 PR5a): the helper flushes stdout, names the kind and stops.
#[test]
fn runtime_str_index_out_of_range_traps() {
    for i in ["5", "2", "-1", "INT64_MAX", "INT64_MIN", "4294967297LL"] {
        let driver = format!(
            "{PRELUDE}
int main(void) {{
    int64_t s = mk(\"AB\", 2);
    printf(\"before\");          /* buffered: only the trap's flush delivers it */
    __twig_str_index(s, {i});   /* out of range → Trap.Bounds */
    return 0;                    /* not reached */
}}
"
        );
        assert_trapped_after(&compile_and_capture(&driver), "before", "Trap.Bounds");
    }
}

/// An out-of-range `__twig_str_slice` (`end > len`) is `Trap.Bounds`.
#[test]
fn runtime_str_slice_out_of_range_traps() {
    for (a, b) in [("0", "99"), ("-1", "1"), ("4294967296LL", "4294967298LL"), ("0", "INT64_MAX")] {
        let driver = format!(
            "{PRELUDE}
int main(void) {{
    int64_t s = mk(\"AB\", 2);
    printf(\"before\");
    __twig_str_slice(s, {a}, {b});   /* out of range → Trap.Bounds */
    return 0;                        /* not reached */
}}
"
        );
        assert_trapped_after(&compile_and_capture(&driver), "before", "Trap.Bounds");
    }
}

/// A backwards range (`start > end`) is `Trap.Bounds` too.
#[test]
fn runtime_str_slice_backwards_range_traps() {
    let driver = format!(
        "{PRELUDE}
int main(void) {{
    int64_t s = mk(\"ABCD\", 4);
    printf(\"before\");
    __twig_str_slice(s, 3, 1);   /* start > end → Trap.Bounds */
    return 0;
}}
"
    );
    assert_trapped_after(&compile_and_capture(&driver), "before", "Trap.Bounds");
}

/// LANG83 PR5a: `__twig_str_concat` takes the module's string ceiling and
/// raises `Trap.Overflow` past it. Exactly at the ceiling it succeeds.
#[test]
fn runtime_str_concat_past_the_ceiling_traps_overflow() {
    let driver = format!(
        "{PRELUDE}
int main(void) {{
    int64_t a = mk(\"01234567\", 8);
    int64_t b = __twig_str_concat(a, a, 16);       /* exactly 16: fits */
    printf(\"%lld\", (long long)__twig_str_len(b));
    __twig_str_concat(b, mk(\"x\", 1), 16);        /* 17 > 16 → Trap.Overflow */
    return 0;                                      /* not reached */
}}
"
    );
    assert_trapped_after(&compile_and_capture(&driver), "16", "Trap.Overflow");
}
