//! The `warrant-corpus-gen` command line: see `coding_adventures_warrant_corpus_gen::cli`.

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = coding_adventures_warrant_corpus_gen::cli::run(
        &args,
        &mut std::io::stdout(),
        &mut std::io::stderr(),
    );
    std::process::exit(code);
}
