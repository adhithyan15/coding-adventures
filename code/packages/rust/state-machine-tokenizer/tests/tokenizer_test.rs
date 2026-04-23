use std::collections::HashSet;

use state_machine::{EffectfulMatcher, EffectfulStateMachine, EffectfulTransition};
use state_machine_tokenizer::{Tokenizer, TokenizerError};

#[test]
fn tokenizer_rejects_unknown_actions() {
    let mut tokenizer = Tokenizer::new(
        EffectfulStateMachine::new(
            set(&["data"]),
            HashSet::new(),
            vec![
                EffectfulTransition::new("data", EffectfulMatcher::Any, "data")
                    .with_effects(&["host_callback()"]),
            ],
            "data".to_string(),
            HashSet::new(),
        )
        .unwrap(),
    );

    let error = tokenizer.push("x").unwrap_err();

    assert_eq!(
        error,
        TokenizerError::UnknownAction("host_callback()".to_string())
    );
}

#[test]
fn tokenizer_bounds_non_consuming_transition_loops() {
    let mut tokenizer = Tokenizer::new(
        EffectfulStateMachine::new(
            set(&["data"]),
            set(&["x"]),
            vec![EffectfulTransition::new("data", EffectfulMatcher::Any, "data").consuming(false)],
            "data".to_string(),
            HashSet::new(),
        )
        .unwrap(),
    )
    .with_max_steps_per_input(3);

    let error = tokenizer.push("x").unwrap_err();

    assert!(matches!(
        error,
        TokenizerError::StepLimitExceeded { limit: 3, .. }
    ));
}

fn set(values: &[&str]) -> HashSet<String> {
    values.iter().map(|value| value.to_string()).collect()
}
