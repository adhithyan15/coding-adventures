//! `vos-model-sat cross-check <contract-crate-dir> [--root <repo-root>]`
//!
//! Runs the three cross-checks of VOS31 section 6 on a contract crate.  Exit 0: every check
//! clean.  Exit 1: a check found a disagreement.  Exit 2: no verdict (bad arguments, an
//! unreadable crate).  The solving commands (`gen`, `check`) are VOS-040b.

use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut dir: Option<PathBuf> = None;
    let mut root: Option<PathBuf> = None;
    let mut it = args.iter();
    match it.next().map(String::as_str) {
        Some("cross-check") => {}
        _ => return usage(),
    }
    while let Some(a) = it.next() {
        if a == "--root" {
            match it.next() {
                Some(r) => root = Some(PathBuf::from(r)),
                None => return usage(),
            }
        } else if dir.is_none() {
            dir = Some(PathBuf::from(a));
        } else {
            return usage();
        }
    }
    let Some(dir) = dir else { return usage() };
    let files = match vos_model_sat::oracle::load(&dir, root.as_deref()) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("no verdict: {e}");
            return ExitCode::from(2);
        }
    };
    match vos_model_sat::run_cross_checks(&files) {
        Ok((lines, ok)) => {
            for l in lines {
                println!("{l}");
            }
            if ok {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            }
        }
        Err(e) => {
            eprintln!("no verdict: {e}");
            ExitCode::from(2)
        }
    }
}

fn usage() -> ExitCode {
    eprintln!("usage: vos-model-sat cross-check <contract-crate-dir> [--root <repo-root>]");
    ExitCode::from(2)
}
