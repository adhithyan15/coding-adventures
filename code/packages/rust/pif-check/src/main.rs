//! `pif-check`: the PIF release gate. See the library documentation and the README.

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let out = pif_check::cli::run(&args);
    print!("{}", out.stdout);
    eprint!("{}", out.stderr);
    std::process::exit(out.code);
}
