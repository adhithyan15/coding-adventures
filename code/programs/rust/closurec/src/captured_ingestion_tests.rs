//! Private D2 ownership/refusal controls, committed before production helpers.
//! The separately retained CLI semantic red is the runtime failure evidence.
use super::*;
use coding_adventures_correlation_vector::CVLog;
use source_inventory::stamp_ingestion_ids;

fn tokens(source: &str, edition: EsVersion) -> Vec<lexer::token::Token> {
    coding_adventures_javascript_lexer::tokenize_javascript_typed_with_ranges(
        source,
        edition,
        lexer::lexical::LexicalLimits::default(),
    )
    .unwrap()
    .into_tokens()
}
fn ids(count: usize) -> Vec<String> {
    (1..=count).map(|i| format!("cv1.{i:016x}")).collect()
}

#[test]
fn d2_id_transfer_preserves_token_vector_and_each_owned_string_buffer() {
    for &edition in EsVersion::ALL {
        for source in ["", "/*x*/\r\nreturn\n1;", "report(\"😀\\n\".length);"] {
            let mut input = tokens(source, edition);
            let original = input.clone();
            let vector_ptr = input.as_ptr();
            let capacity = input.capacity();
            let source_ids = ids(input.len());
            let expected: Vec<_> = source_ids
                .iter()
                .map(|id| (id.as_ptr(), id.capacity(), id.clone()))
                .collect();
            stamp_ingestion_ids(&mut input, source_ids).unwrap();
            assert_eq!(input.as_ptr(), vector_ptr);
            assert_eq!(input.capacity(), capacity);
            for ((actual, old), (ptr, cap, expected)) in
                input.iter_mut().zip(original).zip(expected)
            {
                let id = actual.cv.take().unwrap();
                assert_eq!(id.as_ptr(), ptr);
                assert_eq!(id.capacity(), cap);
                assert_eq!(id, expected);
                assert_eq!(
                    *actual, old,
                    "all other Token fields, including flags/type_name/processed value"
                );
                actual.cv = Some(id);
            }
        }
    }
}
#[test]
fn d2_invalid_private_stamp_invariants_leave_borrowed_tokens_unchanged() {
    for kind in ["short", "long", "stamped", "bad-id"] {
        let mut input = tokens("report(1+1);", EsVersion::Es2025);
        let mut source_ids = ids(input.len());
        match kind {
            "short" => {
                source_ids.pop();
            }
            "long" => source_ids.push("cv1.ffffffffffffffff".into()),
            "stamped" => input.last_mut().unwrap().cv = Some("existing".into()),
            "bad-id" => *source_ids.last_mut().unwrap() = "not-compact".into(),
            _ => unreachable!(),
        }
        let before = input.clone();
        let ptr = input.as_ptr();
        assert!(stamp_ingestion_ids(&mut input, source_ids).is_err());
        assert_eq!(input, before, "{kind}: refusal before any Token mutation");
        assert_eq!(input.as_ptr(), ptr);
    }
}
#[test]
fn d2_trusted_missing_and_lex_error_captures_never_enter_legacy_parser() {
    for level in [CompilationLevel::Simple, CompilationLevel::Advanced] {
        let mut config = CompilerConfig::default();
        config.compilation.level = level;
        let mut log = CVLog::new_compact(true);
        let root = log.try_create(None).unwrap();
        let before = log.to_json_string().unwrap();
        let mut array_ledger = super::array_source::ArrayLedger::new(super::array_source::ArrayLimits::default()).unwrap();
        let missing = transform_source_with_ingestion(
            "report(1+1);",
            &config,
            Some((&mut log, &root, &[], "a.js")),
            None,
            &mut array_ledger,
        );
        assert!(
            matches!(missing,Err(CompilerError::Provenance{stage,..}) if stage=="typed_ingestion")
        );
        assert_eq!(log.to_json_string().unwrap(), before);
        let lexical = transform_source_with_ingestion(
            "report(1+1);",
            &config,
            Some((&mut log, &root, &[], "a.js")),
            Some(Err("actual lexical error marker".into())),
            &mut array_ledger,
        );
        assert!(
            matches!(lexical,Err(CompilerError::TypedPipeline{stage:TypedPipelineStage::Parse,message,..}) if message=="actual lexical error marker")
        );
        assert_eq!(
            log.to_json_string().unwrap(),
            before,
            "no legacy relex or partial downstream graph"
        );
    }
}
#[test]
fn d2_public_detached_id_facade_retains_legacy_source_based_behavior() {
    let mut config = CompilerConfig::default();
    config.compilation.level = CompilationLevel::Simple;
    let mut log = CVLog::new_compact(true);
    let root = log.try_create(None).unwrap();
    let detached = vec!["arbitrary-unregistered-id".into()];
    let output = transform_source_with_cv(
        "report(1+1);",
        &config,
        Some((&mut log, &root, &detached, "legacy.js")),
    )
    .unwrap();
    assert_eq!(output, "report(2);");
    let wire: serde_json::Value = serde_json::from_str(&log.to_json_string().unwrap()).unwrap();
    assert!(!wire["entries"]
        .as_object()
        .unwrap()
        .contains_key(&detached[0]));
    assert!(wire["entries"]
        .as_object()
        .unwrap()
        .values()
        .any(|e| e["origin"]["source"] == "legacy.js"));
}
