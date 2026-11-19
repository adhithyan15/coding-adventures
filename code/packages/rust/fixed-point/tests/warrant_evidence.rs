#![allow(dead_code, unused_variables, unused_mut, clippy::all, unused_parens)]
mod harness {
    include!("generated/warrant_evidence.rs");
}
fn main() {
    let status = harness::run_main(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src"));
    if status != 0 {
        std::process::exit(status);
    }
}
