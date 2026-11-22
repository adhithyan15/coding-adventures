//! The `vos-shape-check` command line (VOS03 section 3).
//!
//! ```text
//! vos-shape-check <crate-dir> [--evidence-features a,b,c]
//! ```
//!
//! Exit codes: 0 PASS, 1 FAIL (findings), 2 no verdict (usage error, or the
//! filesystem could not be read safely).  The report goes to standard output,
//! diagnostics for exit 2 to standard error.  The only I/O in the program is
//! in [`vos_shape_check::loader`].

use std::io::Write;
use std::path::Path;
use std::process::ExitCode;

use vos_shape_check::{check_crate, loader, Policy};

const USAGE: &str = "usage: vos-shape-check <crate-dir> [--evidence-features a,b,c]\n\
  Checks one component crate against the VOS01/VOS02 shape rules (VOS03).\n\
  exit 0: PASS, 1: findings, 2: no verdict (usage error or unsafe/unreadable input)\n";

/// Parse the arguments, run the check, return the exit code.
fn run(args: &[String], out: &mut dyn Write, err: &mut dyn Write) -> u8 {
    let mut dir: Option<&str> = None;
    let mut policy = Policy::default();
    let mut i = 0;
    while i < args.len() {
        vos_shape_check::steps::tick();
        match args[i].as_str() {
            "-h" | "--help" => {
                let _ = out.write_all(USAGE.as_bytes());
                return 0;
            }
            "--evidence-features" => {
                let Some(v) = args.get(i + 1) else {
                    let _ = writeln!(
                        err,
                        "vos-shape-check: --evidence-features needs a value\n{USAGE}"
                    );
                    return 2;
                };
                let names: Vec<String> = v
                    .split(',')
                    .filter(|s| !s.is_empty())
                    .map(str::to_string)
                    .collect();
                if names.len() > 64 || names.iter().any(|n| n.len() > 32) {
                    let _ = writeln!(
                        err,
                        "vos-shape-check: more than 64 names or a name over 32 bytes"
                    );
                    return 2;
                }
                policy.evidence_features = names;
                i += 1;
            }
            a if a.starts_with('-') => {
                let _ = writeln!(err, "vos-shape-check: unknown option `{a}`\n{USAGE}");
                return 2;
            }
            a => {
                if dir.replace(a).is_some() {
                    let _ = writeln!(err, "vos-shape-check: exactly one crate directory\n{USAGE}");
                    return 2;
                }
            }
        }
        i += 1;
    }
    let Some(dir) = dir else {
        let _ = writeln!(err, "vos-shape-check: missing crate directory\n{USAGE}");
        return 2;
    };
    let input = match loader::load_crate(Path::new(dir)) {
        Ok(i) => i,
        Err(e) => {
            let _ = writeln!(err, "vos-shape-check: no verdict: {e}");
            return 2;
        }
    };
    let report = check_crate(&input, &policy);
    let _ = out.write_all(report.render().as_bytes());
    u8::try_from(report.exit_code()).unwrap_or(1)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let stdout = std::io::stdout();
    let stderr = std::io::stderr();
    ExitCode::from(run(&args, &mut stdout.lock(), &mut stderr.lock()))
}
