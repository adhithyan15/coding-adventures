//! The type checker refuses a `Program` whose records would make it invent
//! more than `twig_parser::MAX_GENERATED_NAME_BYTES` of accessor names,
//! before Pass 1 formats one (#18474 security review).
//!
//! A `Program` from `twig_parser::extract_program` was already checked
//! there, so this builds one by hand: a parsed one-field record, grown to a
//! 256-byte name and 5,000 fields (≈ 1.3 MB of `name-field` globals).

use std::collections::HashMap;

use twig_parser::{parse, Form, MAX_GENERATED_NAME_BYTES};
use twig_type_checker::{check_program, check_program_with_globals};

fn hand_built(fields: usize) -> twig_parser::Program {
    let mut program = parse("(module m (typed lenient)) (record P (f : int)) 1").unwrap();
    for form in &mut program.forms {
        if let Form::RecordDef(record) = form {
            record.name = "P".repeat(256);
            let template = record.fields[0].clone();
            record.fields = (0..fields)
                .map(|i| {
                    let mut field = template.clone();
                    field.name = format!("f{i}");
                    field
                })
                .collect();
        }
    }
    program
}

#[test]
fn a_hand_built_program_over_the_generated_name_budget_is_refused_in_every_mode() {
    let program = hand_built(5_000);
    let expected = format!("generated name bytes exceeds the limit of {MAX_GENERATED_NAME_BYTES}");
    for result in [
        check_program(&program, None),
        check_program_with_globals(&program, None, &HashMap::new()),
    ] {
        // `(typed lenient)` would report ok: true for an ordinary error;
        // this one says the checker never looked, so ok is false.
        assert!(!result.ok);
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0].message, expected);
        let accessor = format!("{}-f0", "p".repeat(256));
        assert!(!result.typed_ast.env.globals.contains_key(&accessor), "no accessor was registered");
    }
}

#[test]
fn a_hand_built_program_under_the_budget_is_checked_as_before() {
    let result = check_program(&hand_built(100), None);
    assert!(result.ok, "{:?}", result.errors);
    assert!(result.typed_ast.env.globals.contains_key(&format!("{}-f99", "p".repeat(256))));
}
