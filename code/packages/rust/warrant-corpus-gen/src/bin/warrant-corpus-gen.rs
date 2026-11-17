//! The `warrant-corpus-gen` command line: see `coding_adventures_warrant_corpus_gen::cli`.
//!
//! `main` returns a `Result` so the process needs nothing from `std::process` (the capability
//! analyzer treats that module as process control, which this tool never uses): success exits
//! 0 and every failure exits 1 (the runtime's status for an `Err`), after `run` has written
//! its own message to stderr.  `cli::run` itself tells usage errors (2) from refusals (1).

/// The failure `main` returns: its `Debug` form is the last line the runtime prints.
struct Failed(i32);

impl std::fmt::Debug for Failed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "warrant-corpus-gen failed (status {})", self.0)
    }
}

fn main() -> Result<(), Failed> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match coding_adventures_warrant_corpus_gen::cli::run(
        &args,
        &mut std::io::stdout(),
        &mut std::io::stderr(),
    ) {
        0 => Ok(()),
        n => Err(Failed(n)),
    }
}
