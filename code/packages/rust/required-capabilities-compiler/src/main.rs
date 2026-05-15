#![forbid(unsafe_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process;

use cli_builder::types::ParserOutput;
use cli_builder::{load_spec_from_file, Parser};
use required_capabilities_compiler::compile_required_capabilities_json;

fn main() {
    let root = find_root();
    let spec_path = root.join("code/specs/required-capabilities-compiler.json");
    let spec = load_spec_from_file(
        spec_path
            .to_str()
            .unwrap_or("code/specs/required-capabilities-compiler.json"),
    )
    .unwrap_or_else(|error| {
        eprintln!("required-capabilities-compiler: failed to load CLI spec: {error}");
        process::exit(1);
    });

    let parser = Parser::new(spec);
    let argv: Vec<String> = std::env::args().collect();
    match parser.parse(&argv) {
        Ok(ParserOutput::Help(help)) => print!("{}", help.text),
        Ok(ParserOutput::Version(version)) => println!("{}", version.version),
        Ok(ParserOutput::Parse(result)) => {
            if let Err(error) = run(result) {
                eprintln!("required-capabilities-compiler: {error}");
                process::exit(1);
            }
        }
        Err(error) => {
            eprintln!("{error}");
            eprintln!("Run 'required-capabilities-compiler --help' for usage.");
            process::exit(1);
        }
    }
}

fn run(result: cli_builder::types::ParseResult) -> Result<(), String> {
    let input_path = result
        .flags
        .get("input")
        .and_then(|value| value.as_str())
        .ok_or_else(|| "--input is required".to_string())?;
    let output_path = result
        .flags
        .get("output")
        .and_then(|value| value.as_str())
        .ok_or_else(|| "--output is required".to_string())?;
    let check_only = result
        .flags
        .get("check")
        .and_then(|value| value.as_bool())
        .unwrap_or(false);

    let manifest_json = fs::read_to_string(input_path)
        .map_err(|error| format!("failed to read {input_path}: {error}"))?;
    let generated = compile_required_capabilities_json(&manifest_json)
        .map_err(|error| format!("failed to compile {input_path}: {error}"))?;

    if check_only {
        let existing = fs::read_to_string(output_path)
            .map_err(|error| format!("failed to read {output_path}: {error}"))?;
        if existing != generated.rust_source {
            return Err(format!(
                "{output_path} is stale; regenerate it from {input_path}"
            ));
        }
        println!("checked {output_path}: up to date");
        return Ok(());
    }

    let output = Path::new(output_path);
    if let Some(parent) = output.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
        }
    }
    fs::write(output, generated.rust_source)
        .map_err(|error| format!("failed to write {output_path}: {error}"))?;
    println!("compiled {input_path} -> {output_path}");
    Ok(())
}

fn find_root() -> PathBuf {
    const SENTINEL: &str = "code/specs/required-capabilities-compiler.json";

    let search_starts: Vec<PathBuf> = [
        std::env::current_dir().ok(),
        std::env::current_exe()
            .ok()
            .and_then(|path| path.parent().map(Path::to_path_buf)),
    ]
    .into_iter()
    .flatten()
    .collect();

    for start in search_starts {
        let mut current = start;
        for _ in 0..20 {
            if current.join(SENTINEL).exists() {
                return current;
            }
            match current.parent() {
                Some(parent) => current = parent.to_path_buf(),
                None => break,
            }
        }
    }

    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}
