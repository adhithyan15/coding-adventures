//! The `vos-link` command line (VOS02 section 10).
//!
//! ```text
//! vos-link gen-doc   <crate-dir> [--root <repo-root>]
//! vos-link check-doc <crate-dir> [--root <repo-root>]
//! ```
//!
//! Exit codes: 0 no findings, 1 findings, 2 no verdict (see the library docs).

use std::path::PathBuf;
use std::process::ExitCode;

use vos_link::{check_doc, gen_doc, NoVerdict, Outcome};

const USAGE: &str = "usage: vos-link (gen-doc | check-doc) <crate-dir> [--root <repo-root>]";

struct Args {
    command: String,
    crate_dir: PathBuf,
    root: Option<PathBuf>,
}

fn parse_args(args: &[String]) -> Result<Args, NoVerdict> {
    let mut it = args.iter();
    let command = it.next().ok_or_else(|| NoVerdict::new(USAGE))?.to_string();
    if command != "gen-doc" && command != "check-doc" {
        return Err(NoVerdict::new(format!(
            "unknown command {}\n{USAGE}",
            vos_link::fsio::esc(&command)
        )));
    }
    let mut crate_dir = None;
    let mut root = None;
    while let Some(a) = it.next() {
        if a == "--root" {
            let Some(r) = it.next() else {
                return Err(NoVerdict::new(format!("--root needs a path\n{USAGE}")));
            };
            if root.replace(PathBuf::from(r)).is_some() {
                return Err(NoVerdict::new(format!("--root given twice\n{USAGE}")));
            }
        } else if a.starts_with("--") {
            return Err(NoVerdict::new(format!(
                "unknown option {}\n{USAGE}",
                vos_link::fsio::esc(a)
            )));
        } else if crate_dir.replace(PathBuf::from(a)).is_some() {
            return Err(NoVerdict::new(format!(
                "more than one crate directory\n{USAGE}"
            )));
        }
    }
    let crate_dir =
        crate_dir.ok_or_else(|| NoVerdict::new(format!("no crate directory\n{USAGE}")))?;
    Ok(Args {
        command,
        crate_dir,
        root,
    })
}

fn print(command: &str, o: &Outcome) {
    if !o.header.is_empty() {
        println!("vos-link {command}: {}", o.header);
    }
    for f in &o.findings {
        println!("{f}");
    }
    if let Some(action) = &o.action {
        println!("{action}");
    }
    if !o.carries.is_empty() {
        println!(
            "carries: {}",
            o.carries
                .iter()
                .map(|t| t.name())
                .collect::<Vec<_>>()
                .join(",")
        );
    }
    if o.findings.is_empty() {
        for n in &o.notes {
            println!("note: {n}");
        }
        println!("result: PASS, 0 findings");
    } else {
        println!("result: FAIL, {} findings", o.findings.len());
    }
}

fn run(args: &[String]) -> Result<ExitCode, NoVerdict> {
    let a = parse_args(args)?;
    let outcome = if a.command == "gen-doc" {
        gen_doc(&a.crate_dir, a.root.as_deref())?
    } else {
        check_doc(&a.crate_dir, a.root.as_deref())?
    };
    print(&a.command, &outcome);
    Ok(if outcome.findings.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    })
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("vos-link: no verdict: {e}");
            ExitCode::from(2)
        }
    }
}
