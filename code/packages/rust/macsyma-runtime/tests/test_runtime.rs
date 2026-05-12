use coding_adventures_macsyma_runtime::{
    extend_macsyma_name_table, macsyma_name_table, MacsymaSession, EV, KILL,
};
use std::collections::HashMap;
use symbolic_ir::{apply, int, rat, sym, ADD, DIV, MUL, POW};

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
fn resolves_history_symbols_from_history_table() {
    let mut session = MacsymaSession::new();
    session.eval_source("x; 7;").unwrap();

    assert_eq!(session.history().resolve_history_symbol("%"), Some(&int(7)));
    assert_eq!(
        session.history().resolve_history_symbol("%i1"),
        Some(&sym("x"))
    );
    assert_eq!(
        session.history().resolve_history_symbol("%o2"),
        Some(&int(7))
    );
    assert_eq!(session.history().resolve_history_symbol("%foo"), None);
    assert_eq!(session.history().resolve_history_symbol("%i999"), None);
}

#[test]
fn evaluates_percent_as_previous_output() {
    let mut session = MacsymaSession::new();
    let results = session.eval_source("2 + 3; % * 2;").unwrap();

    assert_eq!(results[0].output, int(5));
    assert_eq!(results[1].input, apply(sym(MUL), vec![sym("%"), int(2)]));
    assert_eq!(results[1].output, int(10));
}

#[test]
fn evaluates_numbered_input_and_output_history_references() {
    let mut session = MacsymaSession::new();
    let results = session.eval_source("2 + 3; % * 2; %i1; %o2;").unwrap();

    let original_input = apply(sym(ADD), vec![int(2), int(3)]);
    assert_eq!(results[0].output, int(5));
    assert_eq!(results[1].output, int(10));
    assert_eq!(results[2].input, sym("%i1"));
    assert_eq!(results[2].output, int(5));
    assert_eq!(results[3].input, sym("%o2"));
    assert_eq!(results[3].output, int(10));
    assert_eq!(session.history().get_input(1), Some(&original_input));
    assert_eq!(session.history().get_input(3), Some(&sym("%i1")));
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

#[test]
fn exports_and_extends_runtime_name_table_idempotently() {
    let table = macsyma_name_table();
    assert_eq!(table.get("kill").map(String::as_str), Some(KILL));
    assert_eq!(table.get("ev").map(String::as_str), Some(EV));
    assert_eq!(
        table.get("ratsimp").map(String::as_str),
        Some("RatSimplify")
    );
    assert_eq!(
        table.get("trigsimp").map(String::as_str),
        Some("TrigSimplify")
    );

    let mut target = HashMap::from([("custom".to_string(), "CustomHead".to_string())]);
    extend_macsyma_name_table(&mut target);
    let once = target.clone();
    extend_macsyma_name_table(&mut target);
    assert_eq!(target, once);
    assert_eq!(target.get("expand").map(String::as_str), Some("Expand"));
    assert_eq!(target.get("custom").map(String::as_str), Some("CustomHead"));
}

#[test]
fn kill_clears_single_and_multiple_bindings() {
    let mut session = MacsymaSession::new();
    let results = session.eval_source("x : 5$ y : 7$ kill(x); x; y;").unwrap();
    assert_eq!(results[2].output, sym("done"));
    assert_eq!(results[3].output, sym("x"));
    assert_eq!(results[4].output, int(7));

    let results = session.eval_source("kill(y, missing); y;").unwrap();
    assert_eq!(results[0].output, sym("done"));
    assert_eq!(results[1].output, sym("y"));
}

#[test]
fn kill_all_clears_bindings_and_history() {
    let mut session = MacsymaSession::new();
    let results = session.eval_source("x : 5$ 42; kill(all);").unwrap();
    assert_eq!(results[2].output, sym("done"));
    assert_eq!(session.history().next_input_index(), 1);
    assert!(session.history().last_output().is_none());

    let results = session.eval_source("x; %;").unwrap();
    assert_eq!(results[0].output, sym("x"));
    assert_eq!(results[1].output, sym("x"));
}

#[test]
fn ev_numer_and_float_coerce_exact_numbers() {
    let mut session = MacsymaSession::new();
    let results = session
        .eval_source("ev(1 / 2, numer); ev(x^2 + 1, float);")
        .unwrap();
    assert_eq!(
        results[0].input,
        apply(
            sym(EV),
            vec![apply(sym(DIV), vec![int(1), int(2)]), sym("numer")]
        )
    );
    assert_eq!(results[0].output, symbolic_ir::flt(0.5));
    assert_eq!(
        results[1].output,
        apply(
            sym(ADD),
            vec![
                apply(sym(POW), vec![sym("x"), int(2)]),
                symbolic_ir::flt(1.0)
            ]
        )
    );
}

#[test]
fn ev_routes_supported_flags_and_preserves_unsupported_heads() {
    let mut session = MacsymaSession::new();
    let results = session
        .eval_source("ev((x + 0) * 1, ratsimp); ev(sin(0) + cos(0), trigsimp); ev(x + 1, expand);")
        .unwrap();

    assert_eq!(results[0].output, sym("x"));
    assert_eq!(results[1].output, int(1));
    assert_eq!(
        results[2].output,
        apply(sym("Expand"), vec![apply(sym(ADD), vec![sym("x"), int(1)])])
    );
}

#[test]
fn manual_kill_and_ev_heads_are_first_class() {
    let mut session = MacsymaSession::new();
    let results = session.eval_statements(vec![
        apply(sym(KILL), vec![sym("all")]),
        apply(sym(EV), vec![rat(3, 2), sym("numer")]),
    ]);
    assert_eq!(results[0].output, sym("done"));
    assert_eq!(results[1].output, symbolic_ir::flt(1.5));
}
