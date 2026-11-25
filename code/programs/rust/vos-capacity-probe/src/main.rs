//! The `vos-capacity-probe` command line (VOS30 section 9.2).
//!
//! ```text
//!   vos-capacity-probe list
//!   vos-capacity-probe run --tier quick|full [--only A1,B3] [--jobs N] --out results.json
//!   vos-capacity-probe check results.json [--jobs N]
//!   vos-capacity-probe validate results.json               (exit 2 structural, 1 row findings, 0 clean)
//!   vos-capacity-probe table [--check | --write] [--results FILE] [--spec FILE]
//!   vos-capacity-probe decide [--replan] [--results FILE]
//!   vos-capacity-probe merge --base A.json --fresh B.json --experiment E1 [--where field=value] [--ignore-outside] --out C.json
//!   vos-capacity-probe row --spec JSON --wall SECONDS      (the child process of `run`)
//!   vos-capacity-probe structure --spec JSON               (the child of `check`'s structural rebuild)
//! ```

use std::process::ExitCode;
use vos_capacity_probe::cli;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match cli::dispatch(&args, &mut std::io::stdout(), &mut std::io::stderr()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(cli::Failure::Usage(m)) => {
            eprintln!("vos-capacity-probe: {m}");
            ExitCode::from(2)
        }
        Err(cli::Failure::Violations(n)) => {
            eprintln!("vos-capacity-probe: {n} violation(s)");
            ExitCode::from(1)
        }
    }
}
