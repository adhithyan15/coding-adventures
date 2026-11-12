//! Generate or check the canonical `ct-compare` Warrant corpus.
//!
//! ```text
//! cargo run -p coding_adventures_warrant_grammar --example gen_corpus -- [--check]
//! ```
//!
//! The supported profile is intentionally fixed to Warrant v0 `ct_compare`.
//! Unknown flags (including `--shard`) fail with usage instead of changing or
//! silently narrowing the corpus.

use std::{env, fs, path::PathBuf};

use coding_adventures_sha256::sha256_hex;
use coding_adventures_warrant_grammar::checker::check_warrant;
use coding_adventures_warrant_grammar::corpus::generate_corpus;
use coding_adventures_warrant_grammar::layout::{capture_layout, LayoutLimits};
use coding_adventures_warrant_grammar::parser::{parse_document, ParserLimits};
use coding_adventures_warrant_grammar::source::{SourceDocument, SourceLimits};
use coding_adventures_warrant_grammar::vectors::{ensure_no_symlinks, read_fixture};

const SPEC: &str = "code/specs/ct-compare.warrant";
const CORPUS: &str = "code/specs/fixtures/ct-compare-warrant-v1/corpus.json";
const TFV: &str = "code/specs/fixtures/ct-compare-warrant-v1/cases.tfv";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().skip(1).collect();
    let check = match args.as_slice() {
        [] => false,
        [flag] if flag == "--check" => true,
        _ => return Err("usage: gen_corpus [--check]".into()),
    };
    let package = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = package
        .ancestors()
        .nth(4)
        .ok_or("missing repository root")?;
    let spec = read_fixture(root, SPEC, 4 * 1024 * 1024)?;
    let document = SourceDocument::admit(&spec, SourceLimits::default())
        .map_err(|e| format!("source admission failed: {e:?}"))?;
    let layout = capture_layout(&document, LayoutLimits::default())
        .map_err(|e| format!("layout failed: {e:?}"))?;
    let file = parse_document(&layout, &document, ParserLimits::default())
        .map_err(|e| format!("parse failed: {e:?}"))?;
    check_warrant(&file).map_err(|e| format!("Warrant check failed: {e:?}"))?;
    let (corpus, tfv) = generate_corpus(&file, &spec)?;
    for (relative, contents) in [(CORPUS, corpus), (TFV, tfv)] {
        let target = root.join(relative);
        if check {
            let current = read_fixture(root, relative, 32 * 1024 * 1024)?;
            if current != contents.as_bytes() {
                return Err(format!("generated corpus drift: {relative} (run gen_corpus)").into());
            }
        } else {
            ensure_no_symlinks(root, relative)?;
            fs::create_dir_all(target.parent().ok_or("fixture has no parent")?)?;
            fs::write(&target, contents.as_bytes())?;
        }
    }
    println!(
        "warrant-corpus-v1 {} (spec sha256 {})",
        if check { "current" } else { "written" },
        sha256_hex(&spec)
    );
    Ok(())
}
