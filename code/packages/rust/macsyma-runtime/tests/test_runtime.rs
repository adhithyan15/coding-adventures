use cas_solve::SOLVE;
use coding_adventures_macsyma_runtime::{
    extend_macsyma_name_table, macsyma_name_table, MacsymaSession, EV, KILL,
};
use std::collections::HashMap;
use symbolic_ir::{
    apply, int, rat, sym, ADD, AND, ASIN, DIV, EQUAL, EXP, GREATER, GREATER_EQUAL, LESS,
    LESS_EQUAL, LIST, LOG, MUL, POW, RULE, SIN, SUB,
};

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

#[test]
fn evaluates_linsolve_linear_systems_through_cas_solve() {
    let x = sym("x");
    let y = sym("y");
    let mut session = MacsymaSession::new();
    let results = session.eval_statements(vec![apply(
        sym("linsolve"),
        vec![
            apply(
                sym(LIST),
                vec![
                    apply(
                        sym(EQUAL),
                        vec![apply(sym(ADD), vec![x.clone(), y.clone()]), int(3)],
                    ),
                    apply(
                        sym(EQUAL),
                        vec![apply(sym(SUB), vec![x.clone(), y.clone()]), int(1)],
                    ),
                ],
            ),
            apply(sym(LIST), vec![x.clone(), y.clone()]),
        ],
    )]);

    assert_eq!(
        results[0].input,
        apply(
            sym(SOLVE),
            vec![
                apply(
                    sym(LIST),
                    vec![
                        apply(
                            sym(EQUAL),
                            vec![apply(sym(ADD), vec![x.clone(), y.clone()]), int(3)]
                        ),
                        apply(
                            sym(EQUAL),
                            vec![apply(sym(SUB), vec![x.clone(), y.clone()]), int(1)]
                        ),
                    ],
                ),
                apply(sym(LIST), vec![x.clone(), y.clone()]),
            ],
        )
    );
    assert_eq!(
        results[0].output,
        apply(
            sym(LIST),
            vec![
                apply(sym(RULE), vec![x.clone(), int(2)]),
                apply(sym(RULE), vec![y.clone(), int(1)]),
            ],
        )
    );
}

#[test]
fn keeps_non_linear_solve_calls_unevaluated() {
    let x = sym("x");
    let expr = apply(
        sym(SOLVE),
        vec![
            apply(
                sym(LIST),
                vec![apply(
                    sym(EQUAL),
                    vec![apply(sym(POW), vec![x.clone(), int(2)]), int(4)],
                )],
            ),
            apply(sym(LIST), vec![x]),
        ],
    );

    let mut session = MacsymaSession::new();
    let results = session.eval_statements(vec![expr.clone()]);
    assert_eq!(results[0].output, expr);
}

#[test]
fn solves_polynomial_inequalities_through_cas_solve() {
    let x = sym("x");
    let mut session = MacsymaSession::new();
    let results = session.eval_statements(vec![
        apply(
            sym(SOLVE),
            vec![
                apply(
                    sym(GREATER),
                    vec![apply(sym(SUB), vec![x.clone(), int(1)]), int(0)],
                ),
                x.clone(),
            ],
        ),
        apply(
            sym(SOLVE),
            vec![
                apply(
                    sym(GREATER),
                    vec![
                        apply(
                            sym(SUB),
                            vec![apply(sym(POW), vec![x.clone(), int(2)]), int(1)],
                        ),
                        int(0),
                    ],
                ),
                x.clone(),
            ],
        ),
        apply(
            sym(SOLVE),
            vec![
                apply(
                    sym(LESS_EQUAL),
                    vec![
                        apply(
                            sym(SUB),
                            vec![apply(sym(POW), vec![x.clone(), int(2)]), int(1)],
                        ),
                        int(0),
                    ],
                ),
                x.clone(),
            ],
        ),
        apply(
            sym(SOLVE),
            vec![
                apply(
                    sym(GREATER_EQUAL),
                    vec![apply(sym(POW), vec![x.clone(), int(2)]), int(0)],
                ),
                x.clone(),
            ],
        ),
    ]);

    assert_eq!(
        results[0].output,
        apply(
            sym(LIST),
            vec![apply(sym(GREATER), vec![x.clone(), int(1)])]
        )
    );
    assert_eq!(
        results[1].output,
        apply(
            sym(LIST),
            vec![
                apply(sym(LESS), vec![x.clone(), int(-1)]),
                apply(sym(GREATER), vec![x.clone(), int(1)]),
            ],
        )
    );
    assert_eq!(
        results[2].output,
        apply(
            sym(LIST),
            vec![apply(
                sym(AND),
                vec![
                    apply(sym(GREATER_EQUAL), vec![x.clone(), int(-1)]),
                    apply(sym(LESS_EQUAL), vec![x.clone(), int(1)]),
                ],
            )],
        )
    );
    assert_eq!(
        results[3].output,
        apply(
            sym(LIST),
            vec![apply(sym(GREATER_EQUAL), vec![int(0), int(0)])],
        )
    );
}

#[test]
fn keeps_unsupported_inequality_solve_calls_unevaluated() {
    let x = sym("x");
    let expr = apply(
        sym(SOLVE),
        vec![
            apply(
                sym(GREATER),
                vec![apply(sym("Sin"), vec![x.clone()]), int(0)],
            ),
            x,
        ],
    );

    let mut session = MacsymaSession::new();
    let results = session.eval_statements(vec![expr.clone()]);
    assert_eq!(results[0].output, expr);
}

#[test]
fn solves_direct_transcendental_equations_through_cas_solve() {
    let x = sym("x");
    let mut session = MacsymaSession::new();
    let results = session.eval_statements(vec![
        apply(
            sym(SOLVE),
            vec![
                apply(sym(EQUAL), vec![apply(sym(EXP), vec![x.clone()]), int(2)]),
                x.clone(),
            ],
        ),
        apply(
            sym(SOLVE),
            vec![
                apply(sym(EQUAL), vec![apply(sym(SIN), vec![x.clone()]), int(0)]),
                x,
            ],
        ),
    ]);

    assert_eq!(
        results[0].output,
        apply(sym(LIST), vec![apply(sym(LOG), vec![int(2)])])
    );
    assert_eq!(
        results[1].output,
        apply(
            sym(LIST),
            vec![
                apply(
                    sym(ADD),
                    vec![
                        apply(sym(ASIN), vec![int(0)]),
                        apply(
                            sym(MUL),
                            vec![
                                int(2),
                                apply(sym(MUL), vec![sym("%pi"), sym("FreeInteger")])
                            ],
                        ),
                    ],
                ),
                apply(
                    sym(ADD),
                    vec![
                        apply(sym(SUB), vec![sym("%pi"), apply(sym(ASIN), vec![int(0)])]),
                        apply(
                            sym(MUL),
                            vec![
                                int(2),
                                apply(sym(MUL), vec![sym("%pi"), sym("FreeInteger")])
                            ],
                        ),
                    ],
                ),
            ],
        )
    );
}

#[test]
fn keeps_unsupported_transcendental_solve_calls_unevaluated() {
    let x = sym("x");
    let expr = apply(
        sym(SOLVE),
        vec![
            apply(
                sym(EQUAL),
                vec![
                    apply(sym(SIN), vec![apply(sym(SIN), vec![x.clone()])]),
                    int(0),
                ],
            ),
            x,
        ],
    );

    let mut session = MacsymaSession::new();
    let results = session.eval_statements(vec![expr.clone()]);
    assert_eq!(results[0].output, expr);
}

#[test]
fn evaluates_deterministic_list_operations_through_cas_list_operations() {
    let xs = apply(sym(LIST), vec![int(1), int(2), int(3)]);
    let nested = apply(
        sym(LIST),
        vec![
            int(1),
            apply(sym(LIST), vec![int(2), apply(sym(LIST), vec![int(3)])]),
        ],
    );
    let mut session = MacsymaSession::new();
    let results = session.eval_statements(vec![
        apply(sym("Length"), vec![xs.clone()]),
        apply(sym("First"), vec![xs.clone()]),
        apply(sym("Rest"), vec![xs.clone()]),
        apply(sym("Last"), vec![xs.clone()]),
        apply(sym("Reverse"), vec![xs.clone()]),
        apply(
            sym("Append"),
            vec![
                apply(sym(LIST), vec![int(1)]),
                apply(sym(LIST), vec![int(2), int(3)]),
            ],
        ),
        apply(
            sym("Join"),
            vec![
                apply(sym(LIST), vec![int(1)]),
                apply(sym(LIST), vec![int(2)]),
            ],
        ),
        apply(sym("Range"), vec![int(1), int(5), int(2)]),
        apply(sym("Part"), vec![xs.clone(), int(-1)]),
        apply(
            sym("Map"),
            vec![sym("f"), apply(sym(LIST), vec![sym("x"), sym("y")])],
        ),
        apply(
            sym("Apply"),
            vec![sym(ADD), apply(sym(LIST), vec![sym("x"), sym("y")])],
        ),
        apply(
            sym("Sort"),
            vec![apply(sym(LIST), vec![sym("b"), sym("a")])],
        ),
        apply(sym("Flatten"), vec![nested, int(-1)]),
    ]);

    assert_eq!(results[0].output, int(3));
    assert_eq!(results[1].output, int(1));
    assert_eq!(results[2].output, apply(sym(LIST), vec![int(2), int(3)]));
    assert_eq!(results[3].output, int(3));
    assert_eq!(
        results[4].output,
        apply(sym(LIST), vec![int(3), int(2), int(1)])
    );
    assert_eq!(
        results[5].output,
        apply(sym(LIST), vec![int(1), int(2), int(3)])
    );
    assert_eq!(results[6].output, apply(sym(LIST), vec![int(1), int(2)]));
    assert_eq!(
        results[7].output,
        apply(sym(LIST), vec![int(1), int(3), int(5)])
    );
    assert_eq!(results[8].output, int(3));
    assert_eq!(
        results[9].output,
        apply(
            sym(LIST),
            vec![
                apply(sym("f"), vec![sym("x")]),
                apply(sym("f"), vec![sym("y")])
            ]
        )
    );
    assert_eq!(
        results[10].output,
        apply(sym(ADD), vec![sym("x"), sym("y")])
    );
    assert_eq!(
        results[11].output,
        apply(sym(LIST), vec![sym("a"), sym("b")])
    );
    assert_eq!(
        results[12].output,
        apply(sym(LIST), vec![int(1), int(2), int(3)])
    );
}

#[test]
fn keeps_invalid_list_operation_calls_unevaluated() {
    let bad_part = apply(sym("Part"), vec![apply(sym(LIST), vec![int(1)]), int(0)]);
    let bad_length = apply(sym("Length"), vec![sym("x")]);
    let mut session = MacsymaSession::new();
    let results = session.eval_statements(vec![bad_part.clone(), bad_length.clone()]);

    assert_eq!(results[0].output, bad_part);
    assert_eq!(results[1].output, bad_length);
}

#[test]
fn returns_rational_linsolve_results() {
    let x = sym("x");
    let y = sym("y");
    let mut session = MacsymaSession::new();
    let results = session.eval_statements(vec![apply(
        sym(SOLVE),
        vec![
            apply(
                sym(LIST),
                vec![
                    apply(
                        sym(EQUAL),
                        vec![
                            apply(
                                sym(ADD),
                                vec![
                                    apply(sym(MUL), vec![int(2), x.clone()]),
                                    apply(sym(MUL), vec![int(3), y.clone()]),
                                ],
                            ),
                            int(7),
                        ],
                    ),
                    apply(
                        sym(EQUAL),
                        vec![
                            apply(
                                sym(SUB),
                                vec![apply(sym(MUL), vec![int(4), x.clone()]), y.clone()],
                            ),
                            int(1),
                        ],
                    ),
                ],
            ),
            apply(sym(LIST), vec![x.clone(), y.clone()]),
        ],
    )]);

    assert_eq!(
        results[0].output,
        apply(
            sym(LIST),
            vec![
                apply(sym(RULE), vec![x.clone(), rat(5, 7)]),
                apply(sym(RULE), vec![y.clone(), rat(13, 7)]),
            ],
        )
    );
}
