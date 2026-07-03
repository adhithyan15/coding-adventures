//! # L6 — Historical-arch round-trip: McCarthy 42 == Twig 42 on every byte.
//!
//! **The IIR convergence proof on the historical lanes.**
//!
//! After the W1–W16 cascade and L7, McCarthy Lisp runs uniformly on the
//! 8 modern backends.  The historical-arch lane — GE-225 (1959),
//! Intel 4004 (1971), Intel 8008 (1972), ARMv7, RV32I, and now
//! IBM 704 (1954) — is structurally independent: each has its own
//! byte-pinned per-language e2e smoke test pinning the exact
//! instruction stream the Twig `42` program produces.
//!
//! L6 closes the loop.  One test, six rows, one assertion per row:
//! the bytes emitted for **McCarthy `42`** on each historical-arch
//! backend equal — byte-for-byte — the bytes emitted for **Twig
//! `42`** through the same backend.
//!
//! Because both languages lower `42` to `const_i64 v=42; ret_i64 v`
//! CIR, that equality follows from the IIR/CIR layer being one
//! shared artifact — which is the architectural promise the
//! historical-arch migration (Phases 1–7) and L1–L5 of the McCarthy
//! Lisp arc together delivered.  This test pins the promise as a
//! single explicit invariant.
//!
//! ## Why this matters
//!
//! - **Twig** is a typed Lisp-family language with full closures and
//!   tagged-word values.
//! - **McCarthy 1960 Lisp** is the dynamic-typed ancestor with the
//!   seven original primitives.
//! - They share **nothing at the surface level** — different lexers,
//!   parsers, type rules, lowering passes.
//! - Yet for the canonical "self-evaluating integer literal" program,
//!   they produce **bit-identical** machine code on six independent
//!   historical-arch backends spanning 1954 (vacuum-tube IBM 704)
//!   through 2025 (ARMv7 / RV32I).
//!
//! That convergence is what proves the IIR / CIR / Backend-trait
//! layer is genuinely a *shared abstraction*, not just two parallel
//! pipelines that happen to coexist.
//!
//! ## Scope
//!
//! v0.1.0 scope decision for every historical-arch backend
//! (confirmed in `MCCARTHY-LISP-PLAN.md`): no CONS at runtime.  We
//! therefore exercise only a self-evaluating integer literal — the
//! simplest possible no-CONS program.
//!
//! **Why `7` and not the canonical `42`?**  The Intel 4004 (1971) is
//! a *4-bit* microprocessor; its immediate-load instruction (`LDM`)
//! holds a value in `[0, 15]`.  The canonical `42` literal that the
//! IBM 704 / GE-225 / Intel 8008 / ARMv7 / RV32I per-arch e2e tests
//! use overflows that 4-bit window, so the Intel 4004 backend
//! rejects it on **both** languages — which is consistent (Twig
//! and McCarthy agree to refuse), but trivially so.  Choosing `7`
//! — a small prime that fits every historical-arch backend's
//! widest narrow-immediate window — lets us exercise all six
//! backends with a single non-trivial byte sequence, strengthening
//! the convergence claim to "all 6 historical-arch lanes agree
//! byte-for-byte" rather than "5 agree, 1 jointly refuses."
//!
//! CONS-using programs (and integers above 15) on Intel 4004 are
//! out of scope for v0.1.0 — a future increment can extend the
//! table when each backend grows additional ops.

use std::path::Path;

use lang_aot::{
    compile_file_to_armv7_bin, compile_file_to_ge225_bin, compile_file_to_ibm704_bin,
    compile_file_to_intel4004_bin, compile_file_to_intel8008_bin, compile_file_to_riscv32_bin,
    LangAotError, Language,
};

/// One compile function per historical-arch backend.  Each writes a
/// flat `.bin` of machine code bytes for the given source file —
/// no linker, no host gating, no OS-specific behaviour.
type CompileFn = fn(&Path, &Path, Language) -> Result<(), LangAotError>;

/// Compile the source string `src` as language `lang` to a fresh
/// `.bin` in `dir`, then read it back as bytes.  Returns `None` if
/// the backend refused the program (e.g. an unsupported op slipped
/// through).
fn compile_and_read(
    dir: &Path,
    src: &str,
    ext: &str,
    lang: Language,
    compile: CompileFn,
    tag: &str,
) -> Option<Vec<u8>> {
    let src_path = dir.join(format!("{tag}.{ext}"));
    let bin_path = dir.join(format!("{tag}.bin"));
    std::fs::write(&src_path, src).expect("write source");
    compile(&src_path, &bin_path, lang).ok()?;
    Some(std::fs::read(&bin_path).expect("read .bin"))
}

/// The closing test of the McCarthy Lisp arc — and of the entire
/// historical-arch story.
///
/// For each of the six historical-arch backends, compile the
/// integer-literal program `7` from **both** Twig source and
/// McCarthy Lisp source.  Assert: the emitted bytes are byte-for-
/// byte identical.
///
/// One IIR / CIR / Backend layer; two surface languages; six
/// machines spanning 71 years of computing history (1954 IBM 704
/// → 2025 ARMv7).  Six byte-identical outputs.
#[test]
fn mccarthy_byte_identical_to_twig_on_every_historical_arch() {
    let backends: &[(&str, CompileFn)] = &[
        ("ge225", compile_file_to_ge225_bin),
        ("intel4004", compile_file_to_intel4004_bin),
        ("intel8008", compile_file_to_intel8008_bin),
        ("armv7", compile_file_to_armv7_bin),
        ("rv32i", compile_file_to_riscv32_bin),
        ("ibm704", compile_file_to_ibm704_bin),
    ];

    // `7` fits every historical-arch backend's narrowest immediate
    // window (Intel 4004 is 4-bit, so `[0, 15]`).  See the module
    // docstring for the "Why `7` and not `42`?" rationale.
    let program = "7\n";

    let dir = tempfile::tempdir().expect("tempdir");
    let mut exercised: Vec<&str> = Vec::new();
    let mut byte_lens: Vec<(String, usize)> = Vec::new();

    for (name, compile) in backends {
        let twig_bytes = compile_and_read(
            dir.path(),
            program,
            "twig",
            Language::Twig,
            *compile,
            &format!("twig_{name}"),
        );
        let mccarthy_bytes = compile_and_read(
            dir.path(),
            program,
            "mcl",
            Language::McCarthyLisp,
            *compile,
            &format!("mcl_{name}"),
        );

        match (twig_bytes, mccarthy_bytes) {
            (Some(t), Some(m)) => {
                assert_eq!(
                    t, m,
                    "BACKEND DISAGREEMENT on `7` between Twig and McCarthy on {name}: \
                     Twig produced {} bytes, McCarthy produced {} bytes.  \
                     IIR convergence is broken — both should lower to const_i64 + ret_i64.",
                    t.len(),
                    m.len(),
                );
                byte_lens.push((name.to_string(), t.len()));
                exercised.push(name);
            }
            (None, None) => {
                // Both refused — that's a documented v0.1.0 gap, not a
                // convergence failure.  No assertion fires; the backend
                // is simply not exercised this round.
                eprintln!("skipping {name}: both Twig and McCarthy backends refused the program");
            }
            (None, Some(_)) => panic!(
                "Twig backend refused `7` on {name} but McCarthy compiled it — \
                 the languages disagree on a program both should support"
            ),
            (Some(_), None) => panic!(
                "McCarthy backend refused `7` on {name} but Twig compiled it — \
                 the languages disagree on a program both should support"
            ),
        }
    }

    // Every historical-arch backend in v0.1.0 supports the `7`
    // program.  None of them are tool-gated (no linker, no external
    // process) and they're all cross-platform.  We require every
    // row to be exercised — anything less is a regression.
    assert_eq!(
        exercised.len(),
        backends.len(),
        "expected every historical-arch backend to exercise `7`; exercised: {exercised:?}"
    );

    eprintln!(
        "L6 historical-arch round-trip: {} backends agree on `7` byte-for-byte across Twig and McCarthy",
        exercised.len()
    );
    for (name, len) in &byte_lens {
        eprintln!("  {name}: {len} bytes");
    }
}
