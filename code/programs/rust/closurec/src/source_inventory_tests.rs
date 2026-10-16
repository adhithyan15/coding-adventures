//! Private budget tests are committed before the producer. CLI tests already
//! establish the semantic red; this module becomes executable with that producer.
use super::*;
use coding_adventures_correlation_vector::CVLog;

fn values(value: &Value) -> usize {
    1 + match value {
        Value::Array(items) => items.iter().map(values).sum(),
        Value::Object(items) => items.values().map(values).sum(),
        _ => 0,
    }
}
fn serialized_cost(log: &CVLog) -> (usize, usize) {
    let wire: Value = serde_json::from_str(&log.to_json_string().unwrap()).unwrap();
    let mut count = 0;
    let mut bytes = 0;
    for entry in wire["entries"].as_object().unwrap().values() {
        for meta in std::iter::once(&entry["origin"]["meta"]).chain(
            entry["contributions"]
                .as_array()
                .unwrap()
                .iter()
                .map(|c| &c["meta"]),
        ) {
            if let Some(descriptor) = meta.get(NS) {
                let wrapper = serde_json::json!({NS:descriptor});
                count += values(&wrapper);
                bytes += serde_json::to_vec(&wrapper).unwrap().len();
            }
        }
    }
    (count, bytes)
}
#[test]
fn arithmetic_matches_actual_closed_metadata_for_every_edition_and_piece_kind() {
    for &edition in EsVersion::ALL {
        for source in ["", "/*x*/", "1;/*x*/", "\"😀\\n\";", "1;\r\n2;"] {
            let mut budget = SourceInventoryBudget::new(SourceInventoryLimits::default()).unwrap();
            let mut log = CVLog::new_compact(true);
            let recorded =
                record_file(&mut log, Path::new("a.js"), source, edition, &mut budget).unwrap();
            let tokens = recorded.tokens.unwrap();
            assert_eq!(
                tokens,
                coding_adventures_javascript_lexer::tokenize_javascript_typed(source, edition)
                    .unwrap()
            );
            let (actual_values, actual_bytes) = serialized_cost(&log);
            assert_eq!(
                (budget.used.values, budget.used.metadata_bytes),
                (actual_values, actual_bytes)
            );
            if source.is_empty() {
                assert_eq!(actual_values, 42);
            }
            let root = log.get(&recorded.id).unwrap();
            let inventory = &root.contributions.last().unwrap().meta[NS];
            let n = inventory["piece_ids"].as_array().unwrap().len();
            let t = tokens.len();
            assert_eq!(
                budget.used.reference_bytes,
                n * (std::mem::size_of::<Value>() + 40)
                    + t * (std::mem::size_of::<String>() + 20)
                    + 64
            );
            assert_eq!(
                budget.used.work,
                source.len() + n + actual_values + actual_bytes + 2 * n + t
            );
            assert_eq!(recorded.token_ids.len(), t);
        }
    }
}
#[test]
fn exact_one_less_and_cumulative_limits_refuse_before_recording() {
    let mut baseline = SourceInventoryBudget::new(SourceInventoryLimits::default()).unwrap();
    let mut log = CVLog::new_compact(true);
    record_file(
        &mut log,
        Path::new("a.js"),
        "1;",
        EsVersion::Es2025,
        &mut baseline,
    )
    .unwrap();
    let cost = baseline.used;
    let exact = SourceInventoryLimits {
        values: cost.values,
        metadata_bytes: cost.metadata_bytes,
        reference_bytes: cost.reference_bytes,
        work: cost.work,
    };
    for axis in 0..4 {
        let mut less = exact;
        match axis {
            0 => less.values -= 1,
            1 => less.metadata_bytes -= 1,
            2 => less.reference_bytes -= 1,
            _ => less.work -= 1,
        }
        let mut budget = SourceInventoryBudget::new(less).unwrap();
        let mut log = CVLog::new_compact(true);
        let before = log.to_json_string().unwrap();
        assert!(record_file(
            &mut log,
            Path::new("a.js"),
            "1;",
            EsVersion::Es2025,
            &mut budget
        )
        .is_err());
        assert_eq!(
            log.to_json_string().unwrap(),
            before,
            "axis {axis} must reject before any graph mutation"
        );
    }
    let mut budget = SourceInventoryBudget::new(exact).unwrap();
    let mut log = CVLog::new_compact(true);
    record_file(
        &mut log,
        Path::new("a.js"),
        "1;",
        EsVersion::Es2025,
        &mut budget,
    )
    .unwrap();
    assert_eq!(budget.used, cost);
    let before = log.to_json_string().unwrap();
    assert!(record_file(
        &mut log,
        Path::new("b.js"),
        "1;",
        EsVersion::Es2025,
        &mut budget
    )
    .is_err());
    assert_eq!(log.to_json_string().unwrap(), before);
}
#[test]
fn unavailable_is_only_the_ordinary_lexical_error_class() {
    for error in [
        LexicalError::Unsupported("custom"),
        LexicalError::Limit("pieces"),
        LexicalError::Invalid("partition"),
        LexicalError::Allocation,
    ] {
        let mut budget = SourceInventoryBudget::new(SourceInventoryLimits::default()).unwrap();
        let mut log = CVLog::new_compact(true);
        let before = log.to_json_string().unwrap();
        assert!(record_capture(
            &mut log,
            Path::new("a.js"),
            "",
            EsVersion::Es2025,
            &mut budget,
            Err(error)
        )
        .is_err());
        assert_eq!(log.to_json_string().unwrap(), before);
        assert_eq!(budget.used, Cost::default());
    }
    let mut budget = SourceInventoryBudget::new(SourceInventoryLimits::default()).unwrap();
    let mut log = CVLog::new_compact(true);
    let recorded = record_file(
        &mut log,
        Path::new("a.js"),
        "var x=\0",
        EsVersion::Es2025,
        &mut budget,
    )
    .unwrap();
    assert!(recorded.tokens.is_err());
    assert!(recorded.token_ids.is_empty());
    assert_eq!(budget.used.values, 19);
    assert_eq!(budget.used.reference_bytes, 0);
    assert_eq!(
        serialized_cost(&log),
        (budget.used.values, budget.used.metadata_bytes)
    );
    let wire: Value = serde_json::from_str(&log.to_json_string().unwrap()).unwrap();
    assert_eq!(wire["entries"].as_object().unwrap().len(), 1);
    assert_eq!(
        log.get(&recorded.id)
            .unwrap()
            .contributions
            .last()
            .unwrap()
            .meta[NS]["reason"],
        "lexical_error"
    );
}
#[test]
fn configuration_arithmetic_and_backing_capacity_are_checked() {
    let limits = SourceInventoryLimits::default();
    for bad in [
        SourceInventoryLimits {
            values: 4_000_001,
            ..limits
        },
        SourceInventoryLimits {
            metadata_bytes: 128 * 1024 * 1024 + 1,
            ..limits
        },
        SourceInventoryLimits {
            reference_bytes: 64 * 1024 * 1024 + 1,
            ..limits
        },
        SourceInventoryLimits {
            work: 512_000_001,
            ..limits
        },
    ] {
        assert!(SourceInventoryBudget::new(bad).is_err());
    }
    let zero = SourceInventoryLimits {
        values: 0,
        metadata_bytes: 0,
        reference_bytes: 0,
        work: 0,
    };
    let mut budget = SourceInventoryBudget::new(zero).unwrap();
    let mut log = CVLog::new_compact(true);
    let before = log.to_json_string().unwrap();
    assert!(record_file(
        &mut log,
        Path::new("a.js"),
        "",
        EsVersion::Es2025,
        &mut budget
    )
    .is_err());
    assert_eq!(log.to_json_string().unwrap(), before);
    assert!(add(usize::MAX, 1).is_err());
    assert!(multiply(usize::MAX, 2).is_err());
    assert!(check_capacity(21, 20).is_err());
    assert!(check_capacity(20, 20).is_ok());
    assert!(compact_id("cv1.0000000000000000").is_ok());
    for bad in [
        "cv1.000000000000000A",
        "cv1.000000000000000",
        "cv1.00000000000000000",
        "a.1",
    ] {
        assert!(compact_id(bad).is_err());
    }
    for s in ["", "a", "\"\\\n\r\t", "\0\u{0001}", "😀"] {
        assert_eq!(
            json_string_bytes(s).unwrap(),
            serde_json::to_vec(s).unwrap().len()
        );
    }
}
#[test]
fn every_token_variant_has_a_frozen_wire_name() {
    use TokenType::*;
    let variants = [
        Name,
        Number,
        String,
        Keyword,
        Plus,
        Minus,
        Star,
        Slash,
        Equals,
        EqualsEquals,
        LParen,
        RParen,
        Comma,
        Colon,
        Semicolon,
        LBrace,
        RBrace,
        LBracket,
        RBracket,
        Dot,
        Bang,
        Newline,
        Indent,
        Dedent,
        Eof,
    ];
    let expected = [
        "Name",
        "Number",
        "String",
        "Keyword",
        "Plus",
        "Minus",
        "Star",
        "Slash",
        "Equals",
        "EqualsEquals",
        "LParen",
        "RParen",
        "Comma",
        "Colon",
        "Semicolon",
        "LBrace",
        "RBrace",
        "LBracket",
        "RBracket",
        "Dot",
        "Bang",
        "Newline",
        "Indent",
        "Dedent",
        "EOF",
    ];
    for (kind, name) in variants.into_iter().zip(expected) {
        assert_eq!(token_name(kind), name);
    }
}
