//! # BEAM10 — `array_len` on BEAM, verified by RUNNING it.
//!
//! `iir-to-beam` had arrays but not their length. Closing that unblocked 31
//! ALGOL corpus programs, but "it compiles now" is the weakest possible claim
//! here, because the whole difficulty of BEAM10 is that the obvious
//! implementation emits *perfectly valid* code that returns a *wrong number*:
//!
//!   - BEAM keeps `array<i64>` on `:atomics` and `array<f64>`/`array<str>` on
//!     `:ets` (BEAM04/BEAM06). `:atomics` is fixed-size and knows its extent.
//!     An ets table does not have one — `ets:info(Tab, size)` counts INSERTED
//!     ENTRIES, so a ten-element array with three cells written reports `3`.
//!   - `array_len`'s `type_hint` is `"i64"`, its own result type, so it cannot
//!     dispatch between the two the way `array_get`/`array_set` do (whose hint
//!     is the element type). And both substrates are references at runtime, so
//!     `is_reference/1` cannot tell them apart either.
//!
//! A test that only asserted "compiles without error" would pass against an
//! implementation that reported `3` for a 10-element array. So every case here
//! **runs the emitted `.beam` on a real `erl`** and checks a hand-computed
//! value (skipped if `erl` is absent, as the other BEAM tests do).
//!
//! See `code/specs/BEAM10-array-length.md`.

use lang_aot::{compile_source_to_beam, Language};

fn erl_available() -> bool {
    std::process::Command::new("erl")
        .arg("-version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Compile ALGOL to `.beam`, run `module:main()` on a real `erl`, return its
/// printout.
fn run(src: &str, module: &str) -> String {
    let bytes = compile_source_to_beam(Language::Algol60, src, module)
        .unwrap_or_else(|e| panic!("compile {src:?} to BEAM: {e}"));
    let tmp = std::env::temp_dir().join(format!("beam10_{}", std::process::id()));
    std::fs::create_dir_all(&tmp).expect("temp dir");
    std::fs::write(tmp.join(format!("{module}.beam")), &bytes).expect("write .beam");
    let out = std::process::Command::new("erl")
        .arg("-noshell")
        .arg("-pa")
        .arg(&tmp)
        .arg("-eval")
        .arg(format!("io:format(\"~w~n\",[{module}:main()]),halt(0)."))
        .output()
        .expect("spawn erl");
    assert!(
        out.status.success(),
        "erl non-zero for {module}; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// `array<i64>` lives on `:atomics`, which knows its own size.
#[test]
fn integer_array_runs_on_beam() {
    if !erl_available() {
        eprintln!("erl absent — skipping BEAM array_len test");
        return;
    }
    assert_eq!(
        run(
            "begin integer array B[1:4]; integer result; B[1] := 10; B[4] := 32; \
             result := B[1] + B[4] end",
            "b10_int"
        ),
        "42",
        "integer array indexed at both ends (atomics substrate)"
    );
}

/// `array<f64>` lives on `:ets`, which does NOT know its own size — the length
/// has to have been recorded at `alloc_array` under the reserved key. If it
/// were read back with `ets:info/2` instead, this array would report a length
/// of 2 (two cells written) rather than 3, and the bounds check on `A[3]`
/// would reject a perfectly valid index.
#[test]
fn real_array_runs_on_beam() {
    if !erl_available() {
        eprintln!("erl absent — skipping BEAM array_len test");
        return;
    }
    assert_eq!(
        run(
            "begin real array A[1:3]; integer result; A[1] := 40.0; A[3] := 2.0; \
             result := entier(A[1] + A[3]) end",
            "b10_real"
        ),
        "42",
        "sparsely-written real array, highest index still reachable (ets substrate)"
    );
}

/// Both substrates in one module. A substrate map that leaked one handle's
/// classification onto another would call `atomics:info/1` on an ets table and
/// raise `badarg`, so this fails loudly rather than subtly.
#[test]
fn both_substrates_in_one_module_run_on_beam() {
    if !erl_available() {
        eprintln!("erl absent — skipping BEAM array_len test");
        return;
    }
    assert_eq!(
        run(
            "begin real array R[1:2]; integer array I[1:2]; integer result; \
             R[1] := 2.5; I[2] := 40; result := entier(R[1]) + I[2] end",
            "b10_mixed"
        ),
        "42",
        "an ets-backed and an atomics-backed array side by side"
    );
}

/// Exercises `array_len` on the bounds-check path repeatedly rather than once,
/// and reads back every cell it wrote.
#[test]
fn array_loop_runs_on_beam() {
    if !erl_available() {
        eprintln!("erl absent — skipping BEAM array_len test");
        return;
    }
    assert_eq!(
        run(
            "begin integer array C[1:5]; integer i; integer result; \
             for i := 1 step 1 until 5 do C[i] := i * 2; \
             result := 0; for i := 1 step 1 until 5 do result := result + C[i] end",
            "b10_loop"
        ),
        "30",
        "2+4+6+8+10 over a fully-written integer array"
    );
}

/// **The sensitivity test.** Everywhere else `array_len` only feeds a bounds
/// check, and a bounds check is one-sided: a length that is too LARGE passes
/// it silently. Call-by-value is the one construct where the length is
/// *observed* — `emit_array_value_copy` uses it as the copy count, so a
/// too-small length drops elements and a too-large one reads past the end and
/// traps.
///
/// Without this case the suite would pass against an implementation that
/// over-reported the length.
#[test]
fn call_by_value_array_copy_runs_on_beam() {
    if !erl_available() {
        eprintln!("erl absent — skipping BEAM array_len test");
        return;
    }
    assert_eq!(
        run(
            "begin integer array values[4:5]; integer result; \
             integer procedure fill(a); value a; integer array a; \
             begin a[4] := 40; a[5] := 2; fill := a[4] + a[5] end; \
             procedure invoke; result := fill(values); invoke end",
            "b10_byval"
        ),
        "42",
        "array passed by value: array_len is the element COUNT, not a bound"
    );
}
