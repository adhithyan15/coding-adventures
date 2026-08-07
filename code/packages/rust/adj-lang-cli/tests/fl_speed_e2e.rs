//! End-to-end tests for the `speed.adj` distance–speed–time rates library —
//! driven through the built CLI binary against the SHIPPED stdlib. Each proves
//! the composition invariant: the library COMPOSES the cited `arithmetic.adj`
//! primitives (`quotient`, `product`) — it re-derives no arithmetic — computes
//! the exact value on the CPU, and carries BOTH its own citation and the
//! primitive's as a corroboration. The three formulas are the one relation
//! v = d / t and its rearrangements d = v·t and t = d / v.

use std::path::{Path, PathBuf};
use std::process::Command;

fn stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-formula-stdlib")
        .canonicalize()
        .expect("shipped adj-formula-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_flspeed_{tag}_{}", std::process::id()));
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

fn place(dir: &Path, rel: &str) {
    let src = stdlib().join(rel);
    let name = Path::new(rel).file_name().unwrap();
    std::fs::copy(&src, dir.join(name)).unwrap_or_else(|e| panic!("copy {rel}: {e}"));
}

fn with_lib(dir: &Path) {
    place(dir, "arithmetic/arithmetic.adj");
    place(dir, "arithmetic/speed.adj");
}

// ---------------------------------------------------------------------------
// average_speed — distance over time, composing quotient.
// ---------------------------------------------------------------------------

#[test]
fn average_speed_composes_quotient_and_carries_both_citations() {
    let dir = scratch("avg");
    with_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"speed.adj\"\n\
         observe distance(150)\n\
         observe time(3)\n\
         ? average_speed(distance, time)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 150 / 3 = 50, via the quotient primitive on the CPU.
    assert!(
        s.contains("\"name\":\"average_speed\"") && s.contains("\"value\":50"),
        "average_speed(150, 3) = 50: {s}"
    );
    // BOTH citations: the speed definition (primary) AND the quotient primitive
    // it composed (corroboration).
    assert!(
        s.contains("physics.info"),
        "primary cites the speed definition: {s}"
    );
    assert!(
        s.contains("mathworld.wolfram.com/Quotient.html"),
        "corroboration cites the quotient primitive it composed: {s}"
    );
}

// ---------------------------------------------------------------------------
// distance_travelled — speed times time (d = v·t), composing product.
// ---------------------------------------------------------------------------

#[test]
fn distance_travelled_composes_product() {
    let dir = scratch("dist");
    with_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"speed.adj\"\n\
         observe speed(50)\n\
         observe time(3)\n\
         ? distance_travelled(speed, time)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 50 * 3 = 150, via the product primitive.
    assert!(
        s.contains("\"name\":\"distance_travelled\"") && s.contains("\"value\":150"),
        "distance_travelled(50, 3) = 150: {s}"
    );
    assert!(
        s.contains("mathworld.wolfram.com/Product.html"),
        "corroboration cites the product primitive it composed: {s}"
    );
}

// ---------------------------------------------------------------------------
// travel_time — distance over speed (t = d/v), composing quotient. The trace
// bottoms out at the observed slots — no model arithmetic.
// ---------------------------------------------------------------------------

#[test]
fn travel_time_composes_quotient_and_names_its_leaves() {
    let dir = scratch("time");
    with_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"speed.adj\"\n\
         observe distance(150)\n\
         observe speed(50)\n\
         ? travel_time(distance, speed)\n",
    )
    .unwrap();

    let (ok, s) = run(&dir.join("case.adj"));
    assert!(ok, "CLI exited non-zero: {s}");
    assert!(!s.contains("\"error\""), "no compile error: {s}");
    // 150 / 50 = 3, and the trace shows the division over the two observed
    // leaves — reconstructable operand by operand, not asserted.
    assert!(
        s.contains("\"name\":\"travel_time\"") && s.contains("\"value\":3"),
        "travel_time(150, 50) = 3: {s}"
    );
    assert!(
        s.contains("\"node\":\"op\"") && s.contains("\"op\":\"/\""),
        "the derivation names the quotient op: {s}"
    );
    assert!(
        s.contains("\"slot\":\"distance\"") && s.contains("\"slot\":\"speed\""),
        "the leaves name the observed slots the value was built from: {s}"
    );
}
