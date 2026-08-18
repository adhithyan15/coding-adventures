use coding_adventures_vault_pm_cli::{run, ExitCode, NativeCliHost};
use std::io::{self, Write};

// The shipped password manager must never contain VLT-PM41's crash injection,
// and *declaring no feature is not enough to guarantee that*: cargo's
// `--features <dep>/<feature>` syntax reaches a direct dependency's features
// even when the root package declares none of its own, so
// `cargo build --release --features coding_adventures_vault_pm_cli/crash-injection`
// would otherwise put an environment-variable kill switch at
// `target/release/vault-pm` — the path a packaging step copies from.
//
// This turns that into a compile error, which no invocation can talk its way
// past and which needs no test to have been run. The instrumented twin lives
// in `code/programs/rust/vault-pm-cli-drill` and is called `vault-pm-drill`.
const _: () = assert!(!coding_adventures_vault_pm_cli::CRASH_INJECTION_COMPILED);

fn main() {
    let output = run(std::env::args_os().skip(1), &NativeCliHost);
    if io::stdout().write_all(output.stdout().as_bytes()).is_err()
        || io::stderr().write_all(output.stderr().as_bytes()).is_err()
    {
        std::process::exit(ExitCode::Internal as i32);
    }
    std::process::exit(output.exit_code() as i32);
}
