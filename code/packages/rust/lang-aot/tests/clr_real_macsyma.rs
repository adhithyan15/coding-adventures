//! # Macsyma on **real CoreCLR** (VM-049).
//!
//! `macsyma_conformance.rs`'s `run_clr` backend proves Macsyma's v0 integer
//! arithmetic against the **in-repo** `clr-simulator` — a fast, always-on
//! floor with no external dependency. It never asserts that the same emitted
//! CIL actually loads and runs on a **real** .NET host, the same gap VM-036
//! closed for native execution and the existing `clr_real_*` files close for
//! McCarthy.
//!
//! This file is the McCarthy `clr_real_scalar.rs` pattern retargeted at
//! Macsyma: `compile_source_to_cil_text(Language::Macsyma, …)` emits textual
//! CIL, the shared `clr_support` harness assembles it with real `ilasm` into a
//! loadable PE and runs it on real `dotnet`, asserting the printed result.
//! Gated on `dotnet` + `ilasm` (skips gracefully when either is absent, like
//! every other external-tool backend in this suite) — the in-repo simulator
//! backend in `macsyma_conformance.rs` remains the always-on conformance
//! floor; this file only adds a real-runtime proof on top of it.
//!
//! The program list below is `macsyma_conformance.rs::PROGRAMS` verbatim
//! (literals, all four binary ops, precedence/chains, exact division, unary,
//! assignment/reference and multi-statement chains) so this lane exercises
//! the identical corpus the in-repo-simulator floor already agrees on, not a
//! narrower one.

#[path = "clr_support/mod.rs"]
mod clr_support;
use clr_support::run_lang_on_real_clr;
use lang_aot::{compile_source_to_cil_text, Language};

const PROGRAMS: &[(&str, i64)] = &[
    // literals
    ("42$", 42),
    ("0$", 0),
    ("-7$", -7),
    // all 4 binary ops
    ("2 + 3$", 5),
    ("10 - 4$", 6),
    ("6 * 7$", 42),
    ("20 / 4$", 5),
    // precedence / chains
    ("2 + 3 * 4$", 14),
    ("1 + 2 + 3 + 4$", 10),
    ("(2 + 3) * 4$", 20),
    // exact division only (the `/` exactness rule — macsyma-iir-vm.md §3/§6)
    ("-4 / 2$", -2),
    ("100 / 25$", 4),
    // unary
    ("-5 + 3$", -2),
    ("-(5 + 3)$", -8),
    ("+5$", 5),
    ("-(-5)$", 5),
    // assignment + later reference
    ("x: 3$\nx + 1$", 4),
    ("x: 3$\nx: x + 1$\nx$", 4),
    ("a: 2$\nb: 3$\na * b$", 6),
    // multi-statement chains, mixed `;` and `$` terminators
    ("x: 5;\ny: 2$\nx - y$", 3),
    ("a: 1$\nb: 2$\nc: 3$\na + b + c$", 6),
];

/// Toolchain-independent floor: the exact `.il` text this lane hands to real
/// `ilasm` must be produced without error for the whole corpus, whether or not
/// `dotnet`/`ilasm` happen to be installed on this host. This is what actually
/// runs on a host missing the CLR toolchain (this sandbox, per VM-047c) — it
/// still exercises `compile_source_to_cil_text`'s real op-lowering path for
/// every program below, rather than the whole file silently reducing to zero
/// executed assertions when the external tools are absent.
#[test]
fn macsyma_emits_valid_cil_text_for_full_corpus() {
    for (src, _) in PROGRAMS {
        let il = compile_source_to_cil_text(Language::Macsyma, src, "Main")
            .unwrap_or_else(|e| panic!("emit .il for {src:?}: {e}"));
        assert!(il.contains(".assembly"), "{src:?} → .il missing .assembly header:\n{il}");
        assert!(il.contains(".method"), "{src:?} → .il missing a .method:\n{il}");
    }
}

#[test]
fn macsyma_runs_on_real_coreclr() {
    let Some(first) = run_lang_on_real_clr(Language::Macsyma, PROGRAMS[0].0, "m0") else {
        eprintln!("dotnet/ilasm absent — skipping real-CoreCLR Macsyma test");
        return;
    };
    assert_eq!(first, PROGRAMS[0].1, "Macsyma {:?} on real CoreCLR", PROGRAMS[0].0);

    // The toolchain is present (proven above): every remaining program must
    // also run and agree, not merely the first. A silently narrower real-CLR
    // corpus than the simulator floor would hide exactly the gap this lane
    // exists to catch.
    for (tag, (src, expected)) in PROGRAMS.iter().enumerate().skip(1) {
        let got = run_lang_on_real_clr(Language::Macsyma, src, &format!("m{tag}"))
            .unwrap_or_else(|| panic!("dotnet/ilasm vanished mid-run for {src:?}"));
        assert_eq!(got, *expected, "Macsyma {src:?} on real CoreCLR");
    }
}
