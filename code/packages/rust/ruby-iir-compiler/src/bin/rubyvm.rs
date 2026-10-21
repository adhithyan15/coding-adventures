//! `rubyvm [--release <id>] <file.rb>` — compile a Ruby file for one release
//! and run it on the repository's own VM (`vm-core`).
//!
//! Output goes to stdout. A LANG83 trap (a zero divisor) behaves like an
//! uncaught Ruby exception: the output written so far is printed, then
//! `trap: <kind>` goes to stderr and the exit status is 1. A compile error
//! also exits 1, with the typed diagnostic on stderr.

use std::io::Read;

fn main() {
    let mut args = std::env::args().skip(1);
    let mut release = None;
    let mut path = None;
    while let Some(arg) = args.next() {
        match (arg.as_str(), &path) {
            ("--release", _) => release = args.next(),
            (_, None) => path = Some(arg),
            _ => usage(),
        }
    }
    let Some(path) = path else { usage() };
    let limit = ruby_iir_compiler::MAX_SOURCE_BYTES;
    let source = match std::fs::File::open(&path).and_then(|file| {
        let mut source = String::new();
        file.take((limit + 1) as u64).read_to_string(&mut source)?;
        Ok(source)
    }) {
        Ok(source) if source.len() <= limit => source,
        Ok(_) => fail(&format!("{path:?}: Ruby source exceeds {limit} bytes")),
        Err(error) => fail(&format!("{path:?}: {error}")),
    };
    match ruby_iir_compiler::run_source_at(&source, release.as_deref()) {
        Ok(output) => {
            print!("{}", output.stdout);
            if let Some(kind) = output.trap {
                fail(&format!("trap: {kind}"));
            }
        }
        Err(error) => fail(&format!("{path:?}: {error}")),
    }
}

fn usage() -> ! {
    fail("usage: rubyvm [--release <id>] <file.rb>")
}

fn fail(message: &str) -> ! {
    use std::io::Write;
    let _ = std::io::stdout().flush();
    eprintln!("{message}");
    std::process::exit(1)
}
