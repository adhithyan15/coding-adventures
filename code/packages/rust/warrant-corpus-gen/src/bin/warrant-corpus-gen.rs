//! The `warrant-corpus-gen` command line: see `coding_adventures_warrant_corpus_gen::cli`.
//!
//! The exit status is `cli::run`'s return value, unchanged:
//!
//! ```text
//!   0  success        1  `--check` found drift        2  refused, I/O error or misuse
//! ```
//!
//! This is the convention of the independent oracle (`warrant-oracle`: 1 = the two sides
//! disagree, 2 = the input was refused or unreadable), so a script can tell "the committed
//! fixture is stale, regenerate it" from "the generator would not run" without parsing stderr.
//!
//! The status needs `std::process::exit`: returning an `Err` from `main` always exits 1 and
//! cannot say 2.  (`std::process::ExitCode` would do too, but the capability analyzer reads it
//! as `proc:exec`, a claim that this tool spawns something; `exit` reads as `proc:signal`,
//! which is honest: the process ends itself and nothing else.  `required_capabilities.json`
//! declares it.)  `run` has returned, and written its own messages, before `exit` is called.

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let status = coding_adventures_warrant_corpus_gen::cli::run(
        &args,
        &mut std::io::stdout(),
        &mut std::io::stderr(),
    );
    if status != 0 {
        std::process::exit(status);
    }
}
