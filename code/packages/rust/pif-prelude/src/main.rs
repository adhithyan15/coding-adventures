//! Print the PIF prelude on standard output. Regenerate the committed copy with
//! `cargo run -q -p pif-prelude > code/specs/data/pif/prelude.pif`.

fn main() {
    print!("{}", pif_prelude::generate());
}
