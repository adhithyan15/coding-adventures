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
