//! `vault-pm-drill` — the instrumented twin of `vault-pm`, for VLT-PM41.
//!
//! Byte for byte the same composition as `code/programs/rust/vault-pm-cli`'s
//! `main.rs`. The only difference is in the manifest next to this file, which
//! enables `coding_adventures_vault_pm_cli`'s `crash-injection` feature. That
//! feature makes the process removable at a chosen durable write so the drill
//! can prove what an interrupted vault looks like to the *next* process.
//!
//! The duplication is deliberate. Sharing one crate between the shipped
//! binary and the instrumented one would mean sharing one feature resolution,
//! and `cargo build --all-targets` would then hand a packaging step an
//! instrumented `vault-pm`. Twelve copied lines buy the guarantee that the
//! product executable cannot be built with a kill switch in it.

use coding_adventures_vault_pm_cli::{run, ExitCode, NativeCliHost};
use std::io::{self, Write};

fn main() {
    let output = run(std::env::args_os().skip(1), &NativeCliHost);
    if io::stdout().write_all(output.stdout().as_bytes()).is_err()
        || io::stderr().write_all(output.stderr().as_bytes()).is_err()
    {
        std::process::exit(ExitCode::Internal as i32);
    }
    std::process::exit(output.exit_code() as i32);
}
