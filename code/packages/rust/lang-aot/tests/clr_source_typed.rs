use clr_simulator::{CLRSimulator, MethodCode, Value};
use iir_to_cil_bytecode::CILProgramArtifact;
use lang_aot::{compile_source_to_cil_artifact, compile_source_to_typed_cil_artifact, Language, LangAotError};

fn execute(a: &CILProgramArtifact) -> Value {
    let entry = a.methods.iter().position(|m| m.name == a.entry_label).unwrap();
    let mut sim = CLRSimulator::new();
    sim.load_program(a.methods.iter().map(|m| MethodCode {
        body: m.body.clone(), num_locals: m.local_types.len(), num_args: m.parameter_types.len(),
    }).collect(), entry);
    sim.run(10000);
    assert!(sim.halted);
    assert_eq!(sim.stack.len(), 1);
    sim.stack[0].unwrap()
}

#[test]
fn strict_source_preserves_wide_arithmetic() {
    for (src, expected) in [("(+ 2147483647 1)", 2147483648), ("(+ 10 20 12)", 42)] {
        let a = compile_source_to_typed_cil_artifact(Language::Twig, src, "WideSource").unwrap();
        assert!(a.methods.iter().all(|m| m.return_type == "int64"));
        assert!(a.methods.iter().all(|m| m.local_types.iter().all(|ty| *ty == "int64")));
        assert_eq!(execute(&a), Value::Int64(expected));
    }
}

#[test]
fn default_source_width_behavior_is_unchanged() {
    let err = compile_source_to_typed_cil_artifact(Language::Twig, "4294967296", "Strict")
        .err().expect("CLR01 literal gate must remain");
    assert!(matches!(err, LangAotError::ClrBackendError(ref message) if message.contains("CLR01 range")));
    assert!(compile_source_to_cil_artifact(Language::Twig, "4294967296", "Default").is_err());
    let a = compile_source_to_cil_artifact(Language::Twig, "(+ 2147483647 1)", "Default").unwrap();
    assert_eq!(execute(&a), Value::Int(i32::MIN));
}

#[test]
fn strict_source_refuses_dynamic_and_narrow_boundaries() {
    for (lang, src, hint) in [
        (Language::Twig, "(define (addone n) (+ n 1)) (addone 2147483647)", "any"),
        (Language::McCarthyLisp, "4294967296", "ref<any>"),
        (Language::Nib, "fn main() -> u8 { return 86 % 7; }", "u8"),
    ] {
        let err = compile_source_to_typed_cil_artifact(lang, src, "Refusal").err().expect("must refuse");
        match err {
            LangAotError::ClrBackendError(message) => {
                assert!(message.contains("UnsupportedType") && message.contains(hint), "{message}");
            },
            other => panic!("expected backend refusal: {other}"),
        }
    }
}

#[test]
fn strict_source_preserves_frontend_errors() {
    assert!(matches!(compile_source_to_typed_cil_artifact(Language::Nib, "fn", "Invalid"),
        Err(LangAotError::FrontendError { .. })));
}
