//! Independent controls distinguish AST grouping and non-reproducing failures.
use coding_adventures_test_bootstrap::run::{Options, Suite};
use coding_adventures_test_bootstrap::tfv::{self, Record, Value};

#[allow(dead_code)]
mod support;
use support::{check, parse, witnesses, Fault, CORPUS};

fn arithmetic() -> Record {
    tfv::parse_file(CORPUS)
        .unwrap()
        .into_iter()
        .find(|r| r.id == "operators/multiply")
        .unwrap()
}

#[test]
fn wrong_precedence_still_parses_but_loses_the_authored_subtree() {
    let record = arithmetic();
    let source = record.field("source").unwrap().as_text().unwrap();
    let normal = parse(source, false, Fault::None).unwrap();
    let changed = parse(source, false, Fault::WrongPrecedence).unwrap();
    assert!(witnesses(&normal).contains("product=2 * 3"));
    assert!(!witnesses(&changed).contains("product=2 * 3"));
}

#[test]
fn subtree_matching_is_whole_entry_not_substring() {
    let mut record = arithmetic();
    record
        .fields
        .iter_mut()
        .find(|(name, _)| name == "witnesses")
        .unwrap()
        .1 = Value::List(vec![Value::Text("product=2 *".into())]);
    let mut suite = Suite::new();
    suite.add("short-witness", move |ctx| check(ctx, &record, Fault::None));
    let result = suite.run(&Options::default(), &mut Vec::new()).unwrap();
    assert_eq!(result.failed, 1);
}

#[test]
fn malformed_case_shapes_fail_without_exposing_source() {
    let mut records = vec![arithmetic(); 5];
    records[0].fields.push(("extra".into(), Value::Null));
    records[1]
        .fields
        .iter_mut()
        .find(|(n, _)| n == "accept")
        .unwrap()
        .1 = Value::from_u64(1);
    records[2]
        .fields
        .iter_mut()
        .find(|(n, _)| n == "source")
        .unwrap()
        .1 = Value::Text("PRIVATE_SOURCE".repeat(1000));
    records[3]
        .fields
        .iter_mut()
        .find(|(n, _)| n == "witnesses")
        .unwrap()
        .1 = Value::List(vec![]);
    records[4]
        .fields
        .iter_mut()
        .find(|(n, _)| n == "witnesses")
        .unwrap()
        .1 = Value::List(vec![Value::Bool(true)]);
    for record in records {
        let mut suite = Suite::new();
        suite.add("malformed", move |ctx| check(ctx, &record, Fault::None));
        let mut output = Vec::new();
        let result = suite.run(&Options::default(), &mut output).unwrap();
        assert_eq!(result.failed, 1);
        let text = String::from_utf8(output).unwrap();
        assert!(text.contains("corpus-malformed"));
        assert!(!text.contains("PRIVATE_SOURCE"));
    }
}
