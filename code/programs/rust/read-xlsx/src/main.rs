//! `read-xlsx` — CLI: open a real `.xlsx` and print its evaluated cell grid.
//!
//! Usage:
//!   read-xlsx <file.xlsx>   Open a spreadsheet file and print its sheets.
//!   read-xlsx --demo        Run the two built-in fixtures (no file needed).
//!   read-xlsx --help        Show this help.

use std::process::ExitCode;

use read_xlsx::{fixtures, format_report, render_xlsx};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        None | Some("-h") | Some("--help") => {
            eprintln!(
                "read-xlsx — print a spreadsheet's evaluated cell grid\n\n\
                 USAGE:\n  read-xlsx <file.xlsx>   open and print a .xlsx file\n  \
                 read-xlsx --demo        run the built-in fixtures\n  \
                 read-xlsx --help        show this help"
            );
            // No args is a usage error; explicit --help is success.
            if args.is_empty() { ExitCode::FAILURE } else { ExitCode::SUCCESS }
        }
        Some("--demo") => {
            for (label, bytes) in [
                ("minimal.xlsx (formulas)", fixtures::MINIMAL_XLSX),
                ("styled.xlsx (number formats)", fixtures::STYLED_XLSX),
            ] {
                println!("=== demo: {label} — {} bytes ===", bytes.len());
                match render_xlsx(bytes) {
                    Ok(sheets) => print!("{}", format_report(&sheets)),
                    Err(e) => eprintln!("error: {e}"),
                }
            }
            ExitCode::SUCCESS
        }
        Some(path) => {
            let bytes = match std::fs::read(path) {
                Ok(b) => b,
                Err(e) => {
                    eprintln!("read-xlsx: cannot read '{path}': {e}");
                    return ExitCode::FAILURE;
                }
            };
            println!("=== {path} — {} bytes ===", bytes.len());
            match render_xlsx(&bytes) {
                Ok(sheets) => {
                    print!("{}", format_report(&sheets));
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("read-xlsx: {e}");
                    ExitCode::FAILURE
                }
            }
        }
    }
}
