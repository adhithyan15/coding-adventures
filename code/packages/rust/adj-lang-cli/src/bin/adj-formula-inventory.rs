//! Emit a canonical, parser-backed inventory of formula source bytes.

use std::env;
use std::fs;
use std::io::{Read, Write};
use std::process::ExitCode;

use coding_adventures_sha256::sha256_hex;
use serde::Serialize;

const MAX_SOURCE_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Serialize)]
struct Span {
    end: usize,
    sha256: String,
    start: usize,
}

#[derive(Serialize)]
struct Formula<'a> {
    body: Span,
    declaration: Span,
    formula: &'a str,
    formulabook: &'a str,
    parameters: &'a [String],
    step_count: usize,
}

#[derive(Serialize)]
struct Inventory<'a> {
    formulas: Vec<Formula<'a>>,
    kind: &'static str,
    parser_contract: &'static str,
    schema_version: u8,
    scope: &'static str,
    source_sha256: String,
    source_size: usize,
}

enum Failure {
    Input(String),
    Usage(String),
}

struct LimitedWriter<W> {
    inner: W,
    remaining: usize,
}

impl<W: Write> Write for LimitedWriter<W> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if bytes.len() > self.remaining {
            return Err(std::io::Error::other(
                "formula inventory output exceeds byte limit",
            ));
        }
        let written = self.inner.write(bytes)?;
        self.remaining -= written;
        Ok(written)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

fn span(source: &[u8], value: adj_lang::SourceSpan) -> Span {
    Span {
        end: value.end,
        sha256: sha256_hex(&source[value.start..value.end]),
        start: value.start,
    }
}

fn run() -> Result<(), Failure> {
    let mut args = env::args_os();
    let program = args
        .next()
        .and_then(|value| value.into_string().ok())
        .unwrap_or_else(|| "adj-formula-inventory".to_string());
    let path = args
        .next()
        .ok_or_else(|| Failure::Usage(format!("usage: {program} PROGRAM.adj")))?;
    if args.next().is_some() {
        return Err(Failure::Usage(format!("usage: {program} PROGRAM.adj")));
    }

    let file = fs::File::open(&path)
        .map_err(|error| Failure::Usage(format!("cannot open {:?}: {error}", path)))?;
    let metadata = file
        .metadata()
        .map_err(|error| Failure::Usage(format!("cannot inspect {:?}: {error}", path)))?;
    if !metadata.is_file() {
        return Err(Failure::Usage(format!(
            "source is not a regular file: {:?}",
            path
        )));
    }
    if metadata.len() > MAX_SOURCE_BYTES {
        return Err(Failure::Usage(format!(
            "source exceeds {MAX_SOURCE_BYTES} byte limit"
        )));
    }
    let mut source = Vec::with_capacity(metadata.len() as usize);
    file.take(MAX_SOURCE_BYTES + 1)
        .read_to_end(&mut source)
        .map_err(|error| Failure::Usage(format!("cannot read {:?}: {error}", path)))?;
    if source.len() as u64 > MAX_SOURCE_BYTES {
        return Err(Failure::Usage(format!(
            "source exceeds {MAX_SOURCE_BYTES} byte limit"
        )));
    }
    let text = std::str::from_utf8(&source)
        .map_err(|error| Failure::Input(format!("source is not UTF-8: {error}")))?;
    let inventory = adj_lang::formula_source_map(text)
        .map_err(|error| Failure::Input(format!("formula source map failed: {error:?}")))?;

    let formulas = inventory
        .iter()
        .map(|item| Formula {
            body: span(&source, item.body_span),
            declaration: span(&source, item.declaration_span),
            formula: &item.formula.name,
            formulabook: &item.formulabook,
            parameters: &item.formula.params,
            step_count: item.formula.steps.len(),
        })
        .collect();
    let output = Inventory {
        formulas,
        kind: "formula_parser_inventory",
        parser_contract: "adj-lang/formula_source_map/v1",
        schema_version: 1,
        scope: "source_file",
        source_sha256: sha256_hex(&source),
        source_size: source.len(),
    };
    let stdout = std::io::stdout();
    let mut writer = LimitedWriter {
        inner: stdout.lock(),
        remaining: MAX_SOURCE_BYTES as usize,
    };
    serde_json::to_writer_pretty(&mut writer, &output)
        .map_err(|error| Failure::Usage(format!("cannot serialize inventory: {error}")))?;
    writer
        .write_all(b"\n")
        .and_then(|()| writer.flush())
        .map_err(|error| Failure::Usage(format!("cannot write inventory: {error}")))?;
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(Failure::Input(error)) => {
            eprintln!("{error}");
            ExitCode::from(1)
        }
        Err(Failure::Usage(error)) => {
            eprintln!("{error}");
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_writer_rejects_a_chunk_past_its_limit() {
        let mut writer = LimitedWriter {
            inner: Vec::new(),
            remaining: 4,
        };

        assert!(writer.write_all(b"12345").is_err());
        assert!(writer.inner.is_empty());
    }
}
