//! End-to-end test for Newton's law of universal gravitation through the built
//! CLI binary: a consumer `import`s the SHIPPED `physics/gravitation.adj` formula
//! library, binds the three quantities from its own `observe`d facts, and applies
//! the cited law F = G·m₁·m₂/r². The CLI must compute the force on the CPU and
//! render the applied formula's NIST citation in the `derived` section — the
//! auditable answer, zero math by the model.
//!
//! The Newtonian constant of gravitation baked into the shipped law is grounded
//! VERBATIM to the NIST CODATA 2022 recommended value,
//! "6.674 30 x 10-11 m3 kg-1 s-2" (https://physics.nist.gov/cgi-bin/cuu/Value?bg),
//! at `authoritative` trust — so the magnitude, not just the inverse-square form,
//! carries primary-source provenance.
//!
//! Because the law multiplies an irrational-looking real constant through three
//! large real inputs, the result is a REAL: the assertion is an APPROXIMATE
//! (relative-epsilon) compare, the honest contract for floating-point physics.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Absolute path to the shipped gravitation library, resolved from this crate's
/// manifest dir so the test is location-independent.
fn shipped_gravitation_lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib/physics/gravitation.adj")
        .canonicalize()
        .expect("shipped physics/gravitation.adj must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_grav_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn run(program: &Path) -> (bool, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_adj-lang-cli"))
        .arg(program)
        .output()
        .expect("run adj-lang-cli");
    (out.status.success(), String::from_utf8(out.stdout).unwrap())
}

/// Pull the numeric `"value":<num>` immediately following the FIRST occurrence of
/// `marker` out of the CLI's JSON, parsed as `f64`. Tiny purpose-built extractor
/// (the crate ships no JSON dependency) so the test can do a real approximate
/// compare on the derived force rather than a brittle exact-string match.
fn derived_value_after(json: &str, marker: &str) -> f64 {
    let at = json.find(marker).expect("marker present in output");
    let vkey = "\"value\":";
    let vstart = json[at..].find(vkey).expect("value key after marker") + at + vkey.len();
    let rest = &json[vstart..];
    let end = rest
        .find(|c: char| c != '-' && c != '+' && c != '.' && c != 'e' && c != 'E' && !c.is_ascii_digit())
        .unwrap_or(rest.len());
    rest[..end].parse::<f64>().expect("value parses as f64")
}

/// Newton's law of universal gravitation — the Earth (m₁ = 5.972×10²⁴ kg) and the
/// Moon (m₂ = 7.348×10²² kg) separated by r = 3.844×10⁸ m. The model binds the
/// three quantities; the engine applies F = G·m₁·m₂/r² on the CPU → ≈1.982×10²⁰ N,
/// carrying NIST's CODATA value of G. Approximate (relative-epsilon) compare.
#[test]
fn universal_gravitation_binds_and_computes_with_nist_citation() {
    let dir = scratch("earthmoon");
    let lib = std::fs::read_to_string(shipped_gravitation_lib()).unwrap();
    std::fs::write(dir.join("gravitation.adj"), lib).unwrap();
    std::fs::write(
        dir.join("case.adj"),
        "import \"gravitation.adj\"\n\
         observe mass_one(5.972e24)\n\
         observe mass_two(7.348e22)\n\
         observe distance(3.844e8)\n\
         ? gravitational_force(mass_one, mass_two, distance)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    assert!(s.contains("\"derived\":["), "derived section present: {s}");
    assert!(
        s.contains("\"name\":\"gravitational_force\""),
        "applied law named in derived section: {s}"
    );

    // The Earth–Moon attraction is ≈1.982×10²⁰ N. Assert with a relative epsilon
    // (reals): F = 6.67430e-11 · 5.972e24 · 7.348e22 / (3.844e8)² = 1.9821…×10²⁰.
    let f = derived_value_after(&s, "\"name\":\"gravitational_force\"");
    let expected = 1.982110729079252e20;
    let rel_err = (f - expected).abs() / expected;
    assert!(
        rel_err < 1e-9,
        "gravitational force ≈ {expected:e} N (got {f:e}, rel_err {rel_err:e}): {s}"
    );
    // Sanity: the order of magnitude alone is the headline answer (~10²⁰ N).
    assert!(
        (1.0e20..1.0e21).contains(&f),
        "force is on the order of 10²⁰ N: {f:e}"
    );

    // The applied law carries its cited NIST provenance — verbatim G value,
    // authoritative trust, and the NIST value-page locator — so it is auditable.
    assert!(
        s.contains("\"trust\":\"authoritative\""),
        "applied formula is authoritative-tier: {s}"
    );
    assert!(
        s.contains("physics.nist.gov"),
        "applied formula cites the NIST locator: {s}"
    );
    assert!(
        s.contains("6.674 30 x 10-11"),
        "source quotes NIST's verbatim value of G: {s}"
    );
}
