//! `vos-model-sat`: the host model-SAT lane (VOS31 section 9).
//!
//! ```text
//! vos-model-sat cross-check <contract-crate-dir> [--root <repo-root>]
//! vos-model-sat gen         <contract-crate-dir> [--root <repo-root>]
//! vos-model-sat check       <contract-crate-dir> [--root <repo-root>] [--emit-drat <dir>]
//! ```
//!
//! Exit 0: every gate passed.  Exit 1: findings (`MS01` to `MS10`).  Exit 2: no verdict (bad
//! arguments, the evaluator could not run).  `gen` writes `warrant/lemmas.sat` only when there
//! is no finding at all.  `check` regenerates, runs every gate and compares the table byte for
//! byte; it never trusts the committed table.

use std::process::ExitCode;

use vos_model_sat::cli;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(cmd) = cli::parse(&args) else {
        eprintln!(
            "usage: vos-model-sat <cross-check|gen|check> <contract-crate-dir> [--root <repo-root>] [--emit-drat <dir>] [--allow-missing-sat-key]"
        );
        return ExitCode::from(2);
    };
    let (code, out, err) = cli::run(&cmd);
    for l in out {
        println!("{l}");
    }
    for l in err {
        eprintln!("{l}");
    }
    ExitCode::from(code)
}
