use coding_adventures_macsyma_runtime::MacsymaSession;
use symbolic_ir::{apply, int, sym, MUL, POW};

#[test]
fn evaluates_arithmetic_program() {
    let mut session = MacsymaSession::new();
    let results = session.eval_source("1 + 2 * 3;").unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].output, int(7));
    assert!(results[0].display);
}

#[test]
fn preserves_suppressed_statement_metadata() {
    let mut session = MacsymaSession::new();
    let results = session.eval_source("x : 5$ x + 1;").unwrap();
    assert_eq!(results.len(), 2);
    assert!(!results[0].display);
    assert!(results[1].display);
    assert_eq!(results[1].output, int(6));
}

#[test]
fn records_input_and_output_history() {
    let mut session = MacsymaSession::new();
    let results = session.eval_source("x : 5$ x + 2;").unwrap();
    assert_eq!(results[0].input_index, 1);
    assert_eq!(results[1].input_index, 2);
    assert_eq!(session.history().get_output(1), Some(&int(5)));
    assert_eq!(session.history().last_output(), Some(&int(7)));
}

#[test]
fn evaluates_function_definitions_across_statements() {
    let mut session = MacsymaSession::new();
    let results = session.eval_source("f(x) := x^2; f(4);").unwrap();
    assert_eq!(results[0].output, sym("f"));
    assert_eq!(results[1].output, int(16));
}

#[test]
fn leaves_symbolic_results_unevaluated_when_needed() {
    let mut session = MacsymaSession::new();
    let results = session.eval_source("(x + 0) * (y^2);").unwrap();
    assert_eq!(
        results[0].output,
        apply(
            sym(MUL),
            vec![sym("x"), apply(sym(POW), vec![sym("y"), int(2)])]
        )
    );
}

#[test]
fn prebinds_macsyma_numeric_constants() {
    let mut session = MacsymaSession::new();
    let results = session.eval_source("%pi; %e;").unwrap();
    assert_eq!(results.len(), 2);
    assert!(results[0].output.to_string().starts_with("3.14159"));
    assert!(results[1].output.to_string().starts_with("2.71828"));
}

#[test]
fn history_can_be_reset() {
    let mut session = MacsymaSession::new();
    session.eval_source("1; 2;").unwrap();
    session.history_mut().reset();
    assert_eq!(session.history().next_input_index(), 1);
    assert!(session.history().last_output().is_none());
}
