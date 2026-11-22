//! SplitMix64 property-based tests for ca-capability-analyzer (VER05a / VER08a).
#![forbid(unsafe_code)]

mod common;

use coding_adventures_ca_capability_analyzer::{analyze_package, scan_source};
use common::{pure_manifest, Pkg};

struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn bounded(&mut self, bound: usize) -> usize {
        (self.next_u64() % (bound as u64)) as usize
    }
}

#[test]
fn pbt_pure_arithmetic_and_decoy_var_names_stay_pure_and_deterministic() {
    let mut rng = SplitMix64::new(0xCA08_A112_BEEF_F00D);
    let var_names = [
        "alpha", "beta", "windows", "rand", "hyper", "nix", "smol", "mio", "fs", "time", "io",
    ];
    let ops = ["+", "-", "*"];

    for case_idx in 0..48 {
        let v1 = var_names[rng.bounded(var_names.len())];
        let mut v2 = var_names[rng.bounded(var_names.len())];
        if v1 == v2 {
            v2 = "sample";
        }
        let a = (rng.bounded(500) as i64) - 250;
        let b = (rng.bounded(500) as i64) - 250;
        let op = ops[rng.bounded(ops.len())];

        let src = format!(
            "pub fn compute_{case_idx}() -> i64 {{\n    let {v1}: i64 = {a};\n    let {v2}: i64 = {b};\n    {v1} {op} {v2}\n}}\n"
        );

        let r1 = scan_source("src/lib.rs", &src).unwrap();
        let r2 = scan_source("src/lib.rs", &src).unwrap();
        assert_eq!(r1.detections.len(), 0, "unexpected detection in:\n{src}");
        assert_eq!(r1.banned.len(), 0, "unexpected banned in:\n{src}");
        assert_eq!(r1.unanalyzable.len(), 0, "unexpected CAP003 in:\n{src}");
        assert_eq!(r1.detections.len(), r2.detections.len());
    }
}

#[test]
fn pbt_qualified_third_party_capability_calls_are_always_detected() {
    let mut rng = SplitMix64::new(0x51BA_DA55_1234_5678);
    let probes = [
        ("windows::Win32::foo()", "ffi:call:*"),
        ("libc::getpid()", "ffi:call:*"),
        ("rand::random::<u64>()", "rand:read:*"),
        ("getrandom::getrandom(&mut [])", "rand:read:*"),
        ("tokio::net::TcpStream::connect(\"127.0.0.1:80\")", "net:connect:*"),
        ("reqwest::blocking::get(\"https://example.com\")", "net:connect:*"),
    ];

    for i in 0..30 {
        let (call_expr, expected_cap) = probes[rng.bounded(probes.len())];
        let fn_name = format!("probe_{i}_{}", rng.bounded(1000));
        let src = format!("pub fn {fn_name}() {{ let _ = {call_expr}; }}\n");
        let pkg = Pkg::with(
            "pbt_probe",
            &[
                ("required_capabilities.json", &pure_manifest("pbt_probe")),
                ("src/lib.rs", &src),
            ],
        );
        let rep = analyze_package(pkg.dir(), false).unwrap();
        assert!(!rep.is_clean());
        assert!(
            rep.detected_capabilities.contains(expected_cap),
            "expected {expected_cap} for `{call_expr}`, got {:?}",
            rep.detected_capabilities
        );
    }
}

#[test]
fn pbt_arbitrary_token_streams_never_panic_scanner() {
    let mut rng = SplitMix64::new(0xDEADBEEF_CAFEBABE);
    let tokens = [
        "fn", "pub", "use", "std", "::", "fs", "windows", "unsafe", "macro_rules!",
        "(", ")", "{", "}", "[", "]", ";", ",", ".", "\"x\"", "// c\n", "/* b */",
        "r#\"raw\"#", "$t", "<", ">", "extern", "\"C\"", "Self",
    ];
    for _ in 0..80 {
        let count = 1 + rng.bounded(40);
        let mut src = String::new();
        for _ in 0..count {
            src.push_str(tokens[rng.bounded(tokens.len())]);
            src.push(' ');
        }
        let _ = scan_source("src/fuzz.rs", &src);
    }
}
