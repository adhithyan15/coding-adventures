//! Dev helper: lower a Ruby (or Twig) snippet to SIR and print the emitted C.
//!
//! Usage:
//!   cargo run -p semantic-ir-to-c --example dump_c -- ruby 'puts 2 + 3 * 4'
//!   cargo run -p semantic-ir-to-c --example dump_c -- twig '(print (+ 2 3))'

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let lang = args.get(1).map(|s| s.as_str()).unwrap_or("ruby");
    let src = args
        .get(2)
        .cloned()
        .unwrap_or_else(|| "puts 2 + 3 * 4".to_string());

    let module = match lang {
        "twig" => twig_to_semantic_ir::compile_source(&src, "prog")
            .unwrap_or_else(|e| panic!("twig lowering failed: {e:?}")),
        _ => ruby_to_semantic_ir::compile_source(&src, "prog")
            .unwrap_or_else(|e| panic!("ruby lowering failed: {e:?}")),
    };

    match semantic_ir_to_c::compile(&module) {
        Ok(artifact) => print!("{}", artifact.source),
        Err(e) => {
            eprintln!("C backend rejected the module: {e}");
            std::process::exit(1);
        }
    }
}
