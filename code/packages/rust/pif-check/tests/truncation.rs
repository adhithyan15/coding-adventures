//! Every prefix of every line, in both the PIF text and the printed text, is handled without a
//! panic, and a truncated declaration is never accepted as the whole one.

use pif_check::filemodel::{decode, DeclModel};
use pif_check::filetext::{parse_file, print_file};

#[test]
fn truncated_lines_are_errors_never_panics() {
    let fixture = include_str!("../../lean-kernel/tests/fixtures/worked-example.pif");
    let model = decode(fixture).unwrap();
    // The printable part of the model: definitions and the goal.
    let mut printable = model.clone();
    printable
        .decls
        .retain(|d| !matches!(d, DeclModel::Theorem { .. }));
    let printed = print_file(&printable).unwrap();
    for (text, is_pif) in [(fixture, true), (printed.as_str(), false)] {
        let lines: Vec<&str> = text.trim_end_matches('\n').split('\n').collect();
        for (i, line) in lines.iter().enumerate() {
            let fields: Vec<&str> = line.split(' ').collect();
            for keep in 0..fields.len() {
                let cut = fields[..keep].join(" ");
                let mut v = lines.clone();
                v[i] = &cut;
                let t = format!("{}\n", v.join("\n"));
                // Never a panic; and a cut line is never accepted as the original file.
                let same = if is_pif {
                    decode(&t).map(|m| m == model)
                } else {
                    parse_file(&t).map(|m| m == printable)
                };
                assert_ne!(same, Ok(true), "line {i} cut to {cut:?}");
            }
        }
    }
}
