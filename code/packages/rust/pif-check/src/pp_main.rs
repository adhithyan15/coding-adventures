//! `pif-pp`: the PIF pretty-printer. See [`pif_check::pp_cli`] and the README.

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let out = pif_check::pp_cli::run(&args);
    print!("{}", out.stdout);
    eprint!("{}", out.stderr);
    std::process::exit(out.code);
}
