use std::collections::BTreeSet;
use std::path::PathBuf;

use coding_adventures_bounded_json::{
    parse_with_depth_limit, JsonErrorKind, JsonNumber, JsonValue, DEFAULT_MAX_DEPTH,
};
use coding_adventures_test_bootstrap::fuzz::{
    crash_signature, fuzz_target, minimize_crashing_input, CoverageBitmap, FuzzOptions, FuzzOutcome,
};
use coding_adventures_test_bootstrap::pbt::{check_property, gen_bounded_int, PbtOptions};
use coding_adventures_test_bootstrap::prng::SplitMix64;
use coding_adventures_test_bootstrap::Verdict;
use dsp_filters::{sosfilt, Biquad, FilterError, MAX_ORDER, MAX_SAMPLES};

const PARTITION_COUNT: usize = 1 << 15;

#[derive(Debug)]
struct FixtureRecord {
    id: String,
    operation: String,
    sections: Vec<[f32; 6]>,
    expected: Vec<f32>,
    chunk_lengths: Option<Vec<usize>>,
}

#[derive(Debug)]
struct Fixture {
    input: Vec<f32>,
    biquad: FixtureRecord,
    sos: FixtureRecord,
}

fn parse_with_wide_integer_adapter(input: &str) -> Result<JsonValue, String> {
    let mut current = input.to_owned();
    let mut repairs = 0usize;
    loop {
        match parse_with_depth_limit(&current, DEFAULT_MAX_DEPTH) {
            Ok(value) => return Ok(value),
            Err(error) if error.kind() == JsonErrorKind::InvalidNumber && repairs < 70 => {
                let Some(offset) = wide_integer_insertion_offset(&current, error.offset()) else {
                    return Err(error.to_string());
                };
                current.insert_str(offset, "e0");
                repairs += 1;
            }
            Err(error) => return Err(error.to_string()),
        }
    }
}

fn wide_integer_insertion_offset(input: &str, offset: usize) -> Option<usize> {
    let bytes = input.as_bytes();
    if offset > bytes.len() {
        return None;
    }

    // The bounded parser reports its current byte position after consuming
    // a plain integer which failed only during i64 conversion.
    let end = offset;
    let mut start = end;
    while start > 0 && bytes[start - 1].is_ascii_digit() {
        start -= 1;
    }
    if start > 0 && bytes[start - 1] == b'-' {
        start -= 1;
    }
    let token = std::str::from_utf8(bytes.get(start..end)?).ok()?;
    if token.is_empty() || token == "-" {
        return None;
    }
    let digits = token.strip_prefix('-').unwrap_or(token);
    if digits.is_empty()
        || digits.starts_with('0')
        || !digits.bytes().all(|byte| byte.is_ascii_digit())
    {
        return None;
    }
    let overflow = match token.parse::<i64>() {
        Err(error) => matches!(
            error.kind(),
            std::num::IntErrorKind::PosOverflow | std::num::IntErrorKind::NegOverflow
        ),
        Ok(_) => false,
    };
    if !overflow {
        return None;
    }

    let mut before = start;
    while before > 0 && is_json_whitespace(bytes[before - 1]) {
        before -= 1;
    }
    if before > 0 && !matches!(bytes[before - 1], b'[' | b',' | b':') {
        return None;
    }
    let mut after = end;
    while after < bytes.len() && is_json_whitespace(bytes[after]) {
        after += 1;
    }
    if after < bytes.len() && !matches!(bytes[after], b',' | b']' | b'}') {
        return None;
    }

    let binary64 = token.parse::<f64>().ok()?;
    if !binary64.is_finite() {
        return None;
    }
    Some(end)
}

fn is_json_whitespace(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\r' | b'\n')
}

fn parse_fixture(input: &str) -> Result<Fixture, String> {
    let root_value = parse_with_wide_integer_adapter(input)?;
    let root = object(&root_value, "fixture")?;
    exact_keys(
        root,
        &["schema_version", "input", "comparison", "cases"],
        "fixture",
    )?;
    if finite_f64(field(root, "schema_version")?, "schema_version")? != 1.0 {
        return Err("schema_version must equal numeric value 1".into());
    }
    if string(field(root, "comparison")?, "comparison")?
        != "finite_exact_numeric_equal_zero_sign_ignored"
    {
        return Err("unsupported comparison rule".into());
    }

    let input_values = array(field(root, "input")?, "input")?;
    if input_values.len() != 16 {
        return Err("input must contain exactly 16 samples".into());
    }
    let input = numeric_vector(input_values, "input")?;
    if input
        != [
            1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
        ]
    {
        return Err("input must be the 16-sample unit impulse".into());
    }

    let cases = array(field(root, "cases")?, "cases")?;
    if cases.len() != 2 {
        return Err("fixture must contain exactly two operation records".into());
    }
    let mut biquad = None;
    let mut sos = None;
    let mut seen_ids = BTreeSet::new();
    for (index, value) in cases.iter().enumerate() {
        let record = object(value, "case record")?;
        let operation = string(field(record, "operation")?, "operation")?;
        let (expected_id, expected_operation, section_count, has_chunks) = match operation {
            "biquad.process" => ("biquad.process.impulse16", operation, 1, true),
            "sosfilt" => ("sosfilt.impulse16.two_sections", operation, 2, false),
            _ => return Err(format!("cases[{index}] has unknown operation")),
        };
        if has_chunks {
            exact_keys(
                record,
                &["id", "operation", "sections", "expected", "chunk_lengths"],
                "biquad record",
            )?;
        } else {
            exact_keys(
                record,
                &["id", "operation", "sections", "expected"],
                "sos record",
            )?;
        }
        let id = string(field(record, "id")?, "id")?;
        if id != expected_id || !seen_ids.insert(id.to_owned()) {
            return Err(format!("unexpected or duplicate operation ID {id}"));
        }
        let raw_sections = array(field(record, "sections")?, "sections")?;
        if raw_sections.len() != section_count {
            return Err(format!("{id}: wrong section count"));
        }
        let mut sections = Vec::with_capacity(section_count);
        for section_value in raw_sections {
            let coefficients = array(section_value, "section")?;
            if coefficients.len() != 6 {
                return Err(format!("{id}: section must have six coefficients"));
            }
            let values = numeric_vector(coefficients, "section")?;
            let mut section = [0.0f32; 6];
            for (slot, value) in values.into_iter().enumerate() {
                section[slot] = exact_f32(value, "section coefficient")?;
            }
            sections.push(section);
        }
        let expected_values = array(field(record, "expected")?, "expected")?;
        if expected_values.len() != 16 {
            return Err(format!("{id}: expected must contain 16 samples"));
        }
        let mut expected = Vec::with_capacity(16);
        for value in numeric_vector(expected_values, "expected")? {
            expected.push(exact_f32(value, "expected sample")?);
        }
        let chunk_lengths = if has_chunks {
            let values = array(field(record, "chunk_lengths")?, "chunk_lengths")?;
            let mut chunks = Vec::with_capacity(values.len());
            for value in values {
                let numeric = finite_f64(value, "chunk length")?;
                if numeric <= 0.0 || !numeric.is_finite() || numeric.fract() != 0.0 {
                    return Err("chunk lengths must decode to positive integers".into());
                }
                chunks.push(numeric as usize);
            }
            if chunks != [3, 1, 12] {
                return Err("chunk lengths must equal [3, 1, 12]".into());
            }
            Some(chunks)
        } else {
            None
        };
        let parsed = FixtureRecord {
            id: id.to_owned(),
            operation: expected_operation.to_owned(),
            sections,
            expected,
            chunk_lengths,
        };
        if has_chunks {
            biquad = Some(parsed);
        } else {
            sos = Some(parsed);
        }
    }
    if seen_ids.len() != 2 {
        return Err("fixture is missing a required operation ID".into());
    }
    let mut narrowed_input = Vec::with_capacity(input.len());
    for value in input {
        narrowed_input.push(exact_f32(value, "input sample")?);
    }
    Ok(Fixture {
        input: narrowed_input,
        biquad: biquad.ok_or("missing Biquad record")?,
        sos: sos.ok_or("missing SOS record")?,
    })
}

fn object<'a>(value: &'a JsonValue, where_: &str) -> Result<&'a [(String, JsonValue)], String> {
    match value {
        JsonValue::Object(members) => {
            let mut seen = BTreeSet::new();
            if members.iter().any(|(key, _)| !seen.insert(key.as_str())) {
                return Err(format!("{where_} has duplicate decoded field names"));
            }
            Ok(members)
        }
        _ => Err(format!("{where_} must be an object")),
    }
}

fn exact_keys(
    members: &[(String, JsonValue)],
    expected: &[&str],
    where_: &str,
) -> Result<(), String> {
    let actual = members
        .iter()
        .map(|(key, _)| key.as_str())
        .collect::<BTreeSet<_>>();
    let expected = expected.iter().copied().collect::<BTreeSet<_>>();
    if actual != expected {
        return Err(format!("{where_} has unexpected or missing fields"));
    }
    Ok(())
}

fn field<'a>(members: &'a [(String, JsonValue)], name: &str) -> Result<&'a JsonValue, String> {
    members
        .iter()
        .find_map(|(key, value)| (key == name).then_some(value))
        .ok_or_else(|| format!("missing field {name}"))
}

fn array<'a>(value: &'a JsonValue, where_: &str) -> Result<&'a [JsonValue], String> {
    match value {
        JsonValue::Array(values) => Ok(values),
        _ => Err(format!("{where_} must be an array")),
    }
}

fn string<'a>(value: &'a JsonValue, where_: &str) -> Result<&'a str, String> {
    match value {
        JsonValue::String(value) => Ok(value),
        _ => Err(format!("{where_} must be a string")),
    }
}

fn finite_f64(value: &JsonValue, where_: &str) -> Result<f64, String> {
    let number = match value {
        JsonValue::Number(JsonNumber::Integer(number)) => *number as f64,
        JsonValue::Number(JsonNumber::Float(number)) => *number,
        _ => return Err(format!("{where_} must be a JSON number")),
    };
    if !number.is_finite() {
        return Err(format!("{where_} must be finite"));
    }
    Ok(number)
}

fn numeric_vector(values: &[JsonValue], where_: &str) -> Result<Vec<f64>, String> {
    values
        .iter()
        .enumerate()
        .map(|(index, value)| finite_f64(value, &format!("{where_}[{index}]")))
        .collect()
}

fn exact_f32(value: f64, where_: &str) -> Result<f32, String> {
    let narrowed = value as f32;
    if !narrowed.is_finite() || narrowed as f64 != value {
        return Err(format!(
            "{where_} is not exactly representable as finite f32"
        ));
    }
    Ok(narrowed)
}

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../../code/specs/fixtures/biquad-sos-v1/cases.json")
}

fn assert_finite_exact(actual: &[f32], expected: &[f32]) {
    assert_eq!(actual.len(), expected.len(), "output length mismatch");
    for (index, (&got, &want)) in actual.iter().zip(expected).enumerate() {
        assert!(got.is_finite(), "output[{index}] is non-finite: {got}");
        assert_eq!(got, want, "output[{index}] mismatch");
    }
}

fn finite_exact_match(actual: &[f32], expected: &[f32]) -> bool {
    actual.len() == expected.len()
        && actual
            .iter()
            .zip(expected)
            .all(|(got, want)| got.is_finite() && got == want)
}

#[test]
fn bounded_json_overflow_offsets_are_after_complete_plain_integer_tokens() {
    let cases = [
        ("[-18446744073709551616,0]", "-18446744073709551616"),
        ("[18446744073709551616]", "18446744073709551616"),
        ("[\"λ\",18446744073709551616]", "18446744073709551616"),
    ];
    for (input, token) in cases {
        let error = parse_with_depth_limit(input, DEFAULT_MAX_DEPTH).unwrap_err();
        assert_eq!(error.kind(), JsonErrorKind::InvalidNumber, "{input}");
        let expected = input.find(token).unwrap() + token.len();
        assert_eq!(error.offset(), expected, "{input}");
    }
}

#[test]
fn wide_integer_adapter_repairs_only_complete_overflow_tokens() {
    for input in [
        "[18446744073709551616]",
        "[-18446744073709551616,0]",
        "[0,18446744073709551616,0,1,0,0]",
        "[\"λ\",18446744073709551616]",
    ] {
        assert!(parse_with_wide_integer_adapter(input).is_ok(), "{input}");
    }
    let rounded_wide = parse_with_wide_integer_adapter("[18446744073709551617]").unwrap();
    let rounded_value = finite_f64(&array(&rounded_wide, "root").unwrap()[0], "wide[0]").unwrap();
    assert_eq!(rounded_value, 18_446_744_073_709_551_616.0);
    let negative_wide = parse_with_wide_integer_adapter("[-18446744073709551617]").unwrap();
    let negative_value = finite_f64(&array(&negative_wide, "root").unwrap()[0], "wide[0]").unwrap();
    assert_eq!(negative_value, -18_446_744_073_709_551_616.0);
    for input in [
        "[1e18446744073709551616]",
        "[01]",
        "[+1]",
        "[1.]",
        "[1e]",
        "[1e9999]",
        "[18446744073709551616x]",
    ] {
        assert!(parse_with_wide_integer_adapter(input).is_err(), "{input}");
    }
    let fractional = parse_with_wide_integer_adapter("[0.18446744073709551616]").unwrap();
    assert!(matches!(
        array(&fractional, "root").unwrap()[0],
        JsonValue::Number(JsonNumber::Float(_))
    ));
    let quoted = parse_with_wide_integer_adapter("[\"18446744073709551616\"]").unwrap();
    assert!(matches!(
        &array(&quoted, "root").unwrap()[0],
        JsonValue::String(value) if value == "18446744073709551616"
    ));
    let many_wide = format!("[{}]", vec!["18446744073709551616"; 71].join(","));
    assert!(parse_with_wide_integer_adapter(&many_wide).is_err());
    let duplicate_after_repair =
        parse_with_wide_integer_adapter("{\"wide\":18446744073709551616,\"wide\":1}").unwrap();
    assert!(object(&duplicate_after_repair, "root").is_err());
}

#[test]
fn shared_fixture_runs_public_biquad_and_sos_exports() {
    let raw = std::fs::read_to_string(fixture_path()).unwrap();
    let fixture = parse_fixture(&raw).unwrap();
    assert_eq!(fixture.biquad.id, "biquad.process.impulse16");
    assert_eq!(fixture.biquad.operation, "biquad.process");
    assert_eq!(fixture.sos.id, "sosfilt.impulse16.two_sections");
    assert_eq!(fixture.sos.operation, "sosfilt");

    let section = fixture.biquad.sections[0];
    assert_finite_exact(
        &Biquad::new(section)
            .unwrap()
            .process(&fixture.input)
            .unwrap(),
        &fixture.biquad.expected,
    );
    let chunks = fixture.biquad.chunk_lengths.as_ref().unwrap();
    let mut chunked = Biquad::new(section).unwrap();
    let mut output = Vec::new();
    let mut start = 0;
    for &length in chunks {
        output.extend(
            chunked
                .process(&fixture.input[start..start + length])
                .unwrap(),
        );
        start += length;
    }
    assert_eq!(start, fixture.input.len());
    assert_finite_exact(&output, &fixture.biquad.expected);

    let mut samplewise = Biquad::new(section).unwrap();
    let sample_output = fixture
        .input
        .iter()
        .map(|&sample| samplewise.process_sample(sample).unwrap())
        .collect::<Vec<_>>();
    assert_finite_exact(&sample_output, &fixture.biquad.expected);

    let mut reset = Biquad::new(section).unwrap();
    reset.process(&[7.0, -3.0, 2.0]).unwrap();
    reset.reset();
    assert_finite_exact(
        &reset.process(&fixture.input).unwrap(),
        &fixture.biquad.expected,
    );

    let mut partition_count = 0usize;
    let mut sample_count = 0usize;
    for mask in 0..PARTITION_COUNT {
        let mut filter = Biquad::new(section).unwrap();
        let mut result = Vec::with_capacity(fixture.input.len());
        let mut start = 0usize;
        for boundary in 0..15 {
            if mask & (1 << boundary) != 0 {
                let end = boundary + 1;
                result.extend(filter.process(&fixture.input[start..end]).unwrap());
                start = end;
            }
        }
        result.extend(filter.process(&fixture.input[start..]).unwrap());
        assert_finite_exact(&result, &fixture.biquad.expected);
        partition_count += 1;
        sample_count += result.len();
    }
    assert_eq!(partition_count, 32_768);
    assert_eq!(sample_count, 524_288);

    for _ in 0..2 {
        assert_finite_exact(
            &sosfilt(&fixture.sos.sections, &fixture.input).unwrap(),
            &fixture.sos.expected,
        );
    }
    let identity = sosfilt(&[], &[0.0, -0.0, 1.0]).unwrap();
    assert_finite_exact(&identity, &[0.0, -0.0, 1.0]);
    assert!(matches!(sosfilt(&[], &[]), Err(FilterError::EmptySignal)));
}

#[test]
fn shared_fixture_schema_decodes_equivalent_numeric_spellings_and_rejects_mutations() {
    let raw = std::fs::read_to_string(fixture_path()).unwrap();
    let version_float = raw.replacen("\"schema_version\": 1", "\"schema_version\": 1.0", 1);
    assert!(parse_fixture(&version_float).is_ok());
    let version_exponent = raw.replacen("\"schema_version\": 1", "\"schema_version\": 1e0", 1);
    assert!(parse_fixture(&version_exponent).is_ok());
    let chunks_alt = raw.replace(
        "\"chunk_lengths\": [3, 1, 12]",
        "\"chunk_lengths\": [3.0, 1e0, 12]",
    );
    assert!(parse_fixture(&chunks_alt).is_ok());

    for malformed in [
        raw.replacen("\"schema_version\": 1", "\"schema_version\": true", 1),
        raw.replacen("\"schema_version\": 1", "\"schema_version\": 1.5", 1),
        raw.replace(
            "\"chunk_lengths\": [3, 1, 12]",
            "\"chunk_lengths\": [3.5, 1, 12]",
        ),
        raw.replace(
            "\"chunk_lengths\": [3, 1, 12]",
            "\"chunk_lengths\": [true, 1, 12]",
        ),
        raw.replace(
            "\"chunk_lengths\": [3, 1, 12]",
            "\"chunk_lengths\": [\"3\", 1, 12]",
        ),
        raw.replacen(
            "\"schema_version\": 1,",
            "\"schema_version\": 1, \"schema_version\": 1,",
            1,
        ),
        raw.replacen(
            "\"schema_version\": 1,",
            "\"schema_version\": 1, \"schema\\u005fversion\": 1,",
            1,
        ),
        raw.replace(
            "\"comparison\": \"finite_exact_numeric_equal_zero_sign_ignored\",",
            "\"comparison\": \"finite_exact_numeric_equal_zero_sign_ignored\", \"extra\": 0,",
        ),
        raw.replace(
            "\"id\": \"sosfilt.impulse16.two_sections\"",
            "\"id\": \"biquad.process.impulse16\"",
        ),
        raw.replace("[0.5, 0.25, 0.5, 2, -2, 0.5]", "[0.5, 0.25]"),
        raw.replace("0.5, 0.25, 0.5, 2, -2, 0.5", "0.1, 0.25, 0.5, 2, -2, 0.5"),
    ] {
        assert!(parse_fixture(&malformed).is_err());
    }

    let changed_expected_raw = raw.replacen("0.46875", "0.4687509536743164", 1);
    let changed_sample = parse_fixture(&changed_expected_raw).unwrap();
    let actual = Biquad::new(changed_sample.biquad.sections[0])
        .unwrap()
        .process(&changed_sample.input)
        .unwrap();
    assert!(!finite_exact_match(
        &actual,
        &changed_sample.biquad.expected
    ));

    let changed_section_raw = raw.replace(
        "[0.5, 0.25, 0.5, 2, -2, 0.5]",
        "[0.125, 0.25, 0.5, 2, -2, 0.5]",
    );
    let changed_section = parse_fixture(&changed_section_raw).unwrap();
    let changed_actual = Biquad::new(changed_section.biquad.sections[0])
        .unwrap()
        .process(&changed_section.input)
        .unwrap();
    assert!(!finite_exact_match(
        &changed_actual,
        &changed_section.biquad.expected
    ));

    let f32_overflow_coefficient = raw.replace(
        "[0.5, 0.25, 0.5, 2, -2, 0.5]",
        "[3.402823669209385e38, 0.25, 0.5, 2, -2, 0.5]",
    );
    assert!(parse_fixture(&f32_overflow_coefficient).is_err());
}

#[test]
fn checked_biquad_rejects_bad_coefficients_and_is_failure_atomic() {
    for section in [
        [f32::NAN, 0.0, 0.0, 1.0, 0.0, 0.0],
        [1.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0, -0.0, 0.0, 0.0],
        [f32::MAX, 0.0, 0.0, f32::MIN_POSITIVE, 0.0, 0.0],
    ] {
        assert!(matches!(
            Biquad::new(section),
            Err(FilterError::InvalidCoefficient(_))
        ));
    }
    let raw_precedence = Biquad::new([f32::NAN, 0.0, 0.0, 0.0, 0.0, 0.0]).unwrap_err();
    assert!(raw_precedence.to_string().contains("coefficient[0]"));
    assert!(matches!(
        Biquad::new([1.0, 0.0, 0.0, 1.0, 0.0, 0.0])
            .unwrap()
            .process(&[]),
        Err(FilterError::EmptySignal)
    ));
    let minsub = f32::from_bits(1);
    assert_eq!(minsub / minsub, 1.0);
    assert_eq!(
        Biquad::new([minsub, 0.0, 0.0, minsub, 0.0, 0.0])
            .unwrap()
            .process(&[1.0])
            .unwrap(),
        [1.0]
    );

    let mut delay = Biquad::new([0.0, 0.0, 1.0, 1.0, 0.0, 0.0]).unwrap();
    assert_finite_exact(&delay.process(&[1.0, 2.0]).unwrap(), &[0.0, 0.0]);
    assert!(matches!(
        delay.process(&[4.0, f32::NAN, 8.0]),
        Err(FilterError::InvalidCoefficient(_))
    ));
    assert_finite_exact(&delay.process(&[0.0, 0.0]).unwrap(), &[1.0, 2.0]);

    let mut output_overflow = Biquad::new([f32::MAX, 0.0, 0.0, 1.0, 1.0, 0.0]).unwrap();
    assert_eq!(output_overflow.process_sample(1.0).unwrap(), f32::MAX);
    assert!(matches!(
        output_overflow.process_sample(2.0),
        Err(FilterError::InvalidCoefficient(_))
    ));
    assert_eq!(output_overflow.process_sample(0.0).unwrap(), -f32::MAX);

    let mut hidden_delay = Biquad::new([0.0, 0.0, f32::MAX, 1.0, 0.0, 0.0]).unwrap();
    assert!(matches!(
        hidden_delay.process_sample(2.0),
        Err(FilterError::InvalidCoefficient(_))
    ));
    assert_eq!(hidden_delay.process_sample(0.0).unwrap(), 0.0);
    hidden_delay.reset();
    assert_eq!(hidden_delay.process_sample(0.0).unwrap(), 0.0);
}

#[test]
fn checked_caps_and_fma_distinguishing_recurrence_are_pinned() {
    let oversized_signal = vec![0.0; MAX_SAMPLES + 1];
    let process_cap = Biquad::new([1.0, 0.0, 0.0, 1.0, 0.0, 0.0])
        .unwrap()
        .process(&oversized_signal)
        .unwrap_err();
    assert!(matches!(process_cap, FilterError::InvalidCoefficient(_)));
    let sos_sample_cap =
        sosfilt(&[[f32::NAN, 0.0, 0.0, 1.0, 0.0, 0.0]], &oversized_signal).unwrap_err();
    assert!(matches!(sos_sample_cap, FilterError::InvalidCoefficient(_)));
    assert!(sos_sample_cap.to_string().contains("MAX_SAMPLES"));
    let too_many_sections = vec![[1.0, 0.0, 0.0, 1.0, 0.0, 0.0]; MAX_ORDER + 1];
    let section_cap = sosfilt(&too_many_sections, &[1.0]).unwrap_err();
    assert!(matches!(section_cap, FilterError::InvalidCoefficient(_)));
    assert!(section_cap.to_string().contains("section count"));
    let maximum_sections = vec![[1.0, 0.0, 0.0, 1.0, 0.0, 0.0]; MAX_ORDER];
    assert_eq!(sosfilt(&maximum_sections, &[1.0]).unwrap(), [1.0]);
    let maximum_signal = vec![0.0; MAX_SAMPLES];
    let maximum_output = Biquad::new([1.0, 0.0, 0.0, 1.0, 0.0, 0.0])
        .unwrap()
        .process(&maximum_signal)
        .unwrap();
    assert_eq!(maximum_output.len(), MAX_SAMPLES);
    assert!(maximum_output.iter().all(|sample| *sample == 0.0));
    assert!(matches!(
        sosfilt(&too_many_sections, &[]),
        Err(FilterError::EmptySignal)
    ));
    assert!(matches!(
        sosfilt(&[[1.0, 0.0, 0.0, 1.0, 0.0, 0.0]; 1], &[]),
        Err(FilterError::EmptySignal)
    ));
    assert!(matches!(
        sosfilt(&[], &[f32::NAN]),
        Err(FilterError::InvalidCoefficient(_))
    ));

    let half = [2.0, 0.0, 0.0, 1.0, 0.0, 0.0];
    let identity = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
    assert!(matches!(
        sosfilt(&[half], &[f32::MAX]),
        Err(FilterError::InvalidCoefficient(_))
    ));
    assert!(matches!(
        sosfilt(&[half, identity], &[f32::MAX]),
        Err(FilterError::InvalidCoefficient(_))
    ));
    let half_gain = [0.5, 0.0, 0.0, 1.0, 0.0, 0.0];
    assert_eq!(
        sosfilt(&[half_gain, half], &[f32::MAX]).unwrap(),
        [f32::MAX]
    );
    assert!(matches!(
        sosfilt(&[half, half_gain], &[f32::MAX]),
        Err(FilterError::InvalidCoefficient(_))
    ));

    let mut fma_control = Biquad::new([f32::MAX, 0.0, 0.0, 1.0, 1.0, 0.0]).unwrap();
    assert_eq!(fma_control.process_sample(1.0).unwrap(), f32::MAX);
    assert!(matches!(
        fma_control.process_sample(2.0),
        Err(FilterError::InvalidCoefficient(_))
    ));
}

#[derive(Clone, Debug)]
struct DyadicCase {
    coefficients: [i32; 3],
    samples: Vec<i32>,
}

fn sample_dyadic_case(rng: &mut SplitMix64, max_length: usize) -> DyadicCase {
    let coefficients = [
        gen_bounded_int(rng, -4, 4) as i32,
        gen_bounded_int(rng, -4, 4) as i32,
        gen_bounded_int(rng, -4, 4) as i32,
    ];
    let length = gen_bounded_int(rng, 1, max_length as i128) as usize;
    let samples = (0..length)
        .map(|_| gen_bounded_int(rng, -8, 8) as i32)
        .collect();
    DyadicCase {
        coefficients,
        samples,
    }
}

fn direct_integer_oracle(case: &DyadicCase) -> Vec<f32> {
    (0..case.samples.len())
        .map(|n| {
            let at = |index: isize| -> i128 {
                if index < 0 {
                    0
                } else {
                    case.samples[index as usize] as i128
                }
            };
            let d = case.coefficients[0] as i128 * at(n as isize)
                + case.coefficients[1] as i128 * at(n as isize - 1)
                + case.coefficients[2] as i128 * at(n as isize - 2);
            (d as f32) / 128.0
        })
        .collect()
}

fn direct_case_inputs(case: &DyadicCase) -> ([f32; 6], Vec<f32>) {
    let coefficients = [
        case.coefficients[0] as f32 / 4.0,
        case.coefficients[1] as f32 / 4.0,
        case.coefficients[2] as f32 / 4.0,
        2.0,
        0.0,
        0.0,
    ];
    let samples = case
        .samples
        .iter()
        .map(|&sample| sample as f32 / 16.0)
        .collect();
    (coefficients, samples)
}

fn shrink_dyadic_case(
    initial: DyadicCase,
    property: &dyn Fn(&DyadicCase) -> bool,
) -> (DyadicCase, usize) {
    let mut current = initial;
    let mut steps = 0;
    for index in 0..3 {
        if current.coefficients[index] != 0 {
            let mut candidate = current.clone();
            candidate.coefficients[index] = 0;
            if !property(&candidate) {
                current = candidate;
                steps += 1;
            }
        }
    }
    for index in 0..current.samples.len() {
        if current.samples[index] != 0 {
            let mut candidate = current.clone();
            candidate.samples[index] = 0;
            if !property(&candidate) {
                current = candidate;
                steps += 1;
            }
        }
    }
    while current.samples.len() > 1 {
        let mut candidate = current.clone();
        candidate.samples.truncate(candidate.samples.len() / 2);
        if !property(&candidate) {
            current = candidate;
            steps += 1;
        } else {
            break;
        }
    }
    (current, steps)
}

#[test]
fn ver05_exact_direct_form_dyadic_property_has_nonzero_budget() {
    let options = PbtOptions {
        num_tests: 512,
        max_retries: 64,
        max_shrink_steps: 128,
        is_ci: false,
    };
    let property = |case: &DyadicCase| {
        let (coefficients, input) = direct_case_inputs(case);
        let expected = direct_integer_oracle(case);
        match Biquad::new(coefficients).and_then(|mut filter| filter.process(&input)) {
            Ok(actual) => {
                actual.len() == expected.len()
                    && actual
                        .iter()
                        .zip(expected.iter())
                        .all(|(a, b)| a.is_finite() && a == b)
            }
            Err(_) => false,
        }
    };
    let verdict = check_property(
        0xAF03_0001,
        &options,
        |rng| Ok(sample_dyadic_case(rng, 32)),
        property,
        |case, prop| shrink_dyadic_case(case, prop),
    );
    assert_eq!(verdict, Verdict::Pass);
}

fn feedback_integer_oracle(case: &DyadicCase, s1: i32, s2: i32) -> Vec<f32> {
    let mut y = Vec::<i128>::with_capacity(case.samples.len());
    let mut output = Vec::with_capacity(case.samples.len());
    for n in 0..case.samples.len() {
        let at = |index: isize| -> i128 {
            if index < 0 {
                0
            } else {
                case.samples[index as usize] as i128
            }
        };
        let d = case.coefficients[0] as i128 * at(n as isize)
            + case.coefficients[1] as i128 * at(n as isize - 1)
            + case.coefficients[2] as i128 * at(n as isize - 2);
        let previous1 = if n >= 1 { y[n - 1] } else { 0 };
        let previous2 = if n >= 2 { y[n - 2] } else { 0 };
        let numerator = (1i128 << n) * d - s1 as i128 * previous1 - s2 as i128 * previous2;
        y.push(numerator);
        output.push(numerator as f32 / (1u32 << (7 + n)) as f32);
    }
    output
}

fn feedback_coefficients(case: &DyadicCase, s1: i32, s2: i32) -> [f32; 6] {
    [
        case.coefficients[0] as f32 / 4.0,
        case.coefficients[1] as f32 / 4.0,
        case.coefficients[2] as f32 / 4.0,
        2.0,
        s1 as f32,
        s2 as f32 / 2.0,
    ]
}

fn shrink_feedback_case(
    initial: (DyadicCase, i32, i32),
    property: &dyn Fn(&(DyadicCase, i32, i32)) -> bool,
) -> ((DyadicCase, i32, i32), usize) {
    let (mut case, s1, s2) = initial;
    let mut steps = 0;
    for index in 0..3 {
        if case.coefficients[index] != 0 {
            let mut candidate = case.clone();
            candidate.coefficients[index] = 0;
            if !property(&(candidate.clone(), s1, s2)) {
                case = candidate;
                steps += 1;
            }
        }
    }
    for index in 0..case.samples.len() {
        if case.samples[index] != 0 {
            let mut candidate = case.clone();
            candidate.samples[index] = 0;
            if !property(&(candidate.clone(), s1, s2)) {
                case = candidate;
                steps += 1;
            }
        }
    }
    while case.samples.len() > 1 {
        let mut candidate = case.clone();
        candidate.samples.truncate(candidate.samples.len() / 2);
        if !property(&(candidate.clone(), s1, s2)) {
            case = candidate;
            steps += 1;
        } else {
            break;
        }
    }
    // Feedback signs are deliberately retained as {-1,+1}; shrinking them
    // to zero would leave the specified property domain.
    ((case, s1, s2), steps)
}

#[test]
fn ver05_exact_two_feedback_dyadic_property_has_nonzero_budget() {
    let options = PbtOptions {
        num_tests: 512,
        max_retries: 64,
        max_shrink_steps: 128,
        is_ci: false,
    };
    let property = |candidate: &(DyadicCase, i32, i32)| {
        let (case, s1, s2) = candidate;
        let coefficients = feedback_coefficients(case, *s1, *s2);
        let input = case
            .samples
            .iter()
            .map(|&sample| sample as f32 / 16.0)
            .collect::<Vec<_>>();
        let expected = feedback_integer_oracle(case, *s1, *s2);
        match Biquad::new(coefficients).and_then(|mut filter| filter.process(&input)) {
            Ok(actual) => {
                actual.len() == expected.len()
                    && actual
                        .iter()
                        .zip(expected.iter())
                        .all(|(a, b)| a.is_finite() && a == b)
            }
            Err(_) => false,
        }
    };
    let sanity = DyadicCase {
        coefficients: [2, -3, 1],
        samples: [4, -8, 2, 6].to_vec(),
    };
    let sanity_input = sanity
        .samples
        .iter()
        .map(|&sample| sample as f32 / 16.0)
        .collect::<Vec<_>>();
    let sanity_expected = feedback_integer_oracle(&sanity, 1, -1);
    let sanity_actual = Biquad::new(feedback_coefficients(&sanity, 1, -1))
        .unwrap()
        .process(&sanity_input)
        .unwrap();
    assert!(sanity_expected.iter().any(|value| *value != 0.0));
    assert_eq!(sanity_actual, sanity_expected);

    let verdict = check_property(
        0xAF03_0002,
        &options,
        |rng| {
            let case = sample_dyadic_case(rng, 12);
            let s1 = if rng.next_u64() & 1 == 0 { -1 } else { 1 };
            let s2 = if rng.next_u64() & 1 == 0 { -1 } else { 1 };
            Ok((case, s1, s2))
        },
        property,
        |candidate, prop| shrink_feedback_case(candidate, prop),
    );
    assert_eq!(verdict, Verdict::Pass);
}

#[derive(Clone, Debug)]
struct RecoveryCase {
    warm: Vec<f32>,
    bad_prefix: Vec<f32>,
}

#[test]
fn ver05_general_f32_block_error_property_preserves_entry_state() {
    let options = PbtOptions {
        num_tests: 128,
        max_retries: 64,
        max_shrink_steps: 64,
        is_ci: false,
    };
    let verdict = check_property(
        0xAF03_0003,
        &options,
        |rng| {
            let warm_len = gen_bounded_int(rng, 1, 8) as usize;
            let bad_len = gen_bounded_int(rng, 1, 8) as usize;
            let samples = |rng: &mut SplitMix64, length| {
                (0..length)
                    .map(|_| {
                        let bits = (rng.next_u64() as u32) & 0x7f7f_ffff;
                        let sign = if rng.next_u64() & 1 == 0 {
                            0
                        } else {
                            0x8000_0000
                        };
                        f32::from_bits(bits | sign)
                    })
                    .collect::<Vec<_>>()
            };
            let mut warm = samples(rng, warm_len);
            warm[0] = 1.0;
            let mut bad_prefix = samples(rng, bad_len);
            bad_prefix[0] = f32::MAX;
            Ok(RecoveryCase { warm, bad_prefix })
        },
        |case| {
            let section = [0.0, 0.0, 1.0, 1.0, 0.0, 0.0];
            let mut after_failure = match Biquad::new(section) {
                Ok(filter) => filter,
                Err(_) => return false,
            };
            if after_failure.process(&case.warm).is_err() {
                return false;
            }
            let mut baseline = after_failure.clone();
            let mut bad = case.bad_prefix.clone();
            bad.push(f32::NAN);
            bad.push(9.0);
            if after_failure.process(&bad).is_ok() {
                return false;
            }
            match (
                after_failure.process(&[0.0, 0.0]),
                baseline.process(&[0.0, 0.0]),
            ) {
                (Ok(a), Ok(b)) => a == b,
                _ => false,
            }
        },
        |case, _| (case, 0),
    );
    assert_eq!(verdict, Verdict::Pass);
}

fn fuzz_seed(op: u8, section: [f32; 6], samples: [f32; 3]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(37);
    bytes.push(op);
    for value in section.into_iter().chain(samples) {
        bytes.extend_from_slice(&value.to_bits().to_le_bytes());
    }
    bytes
}

fn fuzz_word(bytes: &[u8], offset: usize) -> u32 {
    let mut word = [0u8; 4];
    for (index, byte) in word.iter_mut().enumerate() {
        *byte = bytes.get(offset + index).copied().unwrap_or(0);
    }
    u32::from_le_bytes(word)
}

fn fuzz_crash(kind: &str) -> FuzzOutcome {
    FuzzOutcome::Crash {
        kind: kind.into(),
        frames: vec!["biquad_sos::public_semantic_oracle".into()],
    }
}

fn run_structured_fuzz_case_with_reset_mutant(
    bytes: &[u8],
    coverage: &mut CoverageBitmap,
    skip_reset_for_negative_control: bool,
) -> FuzzOutcome {
    if bytes.len() < 33 {
        coverage.record_edge(0);
        return FuzzOutcome::Pass;
    }
    let op = bytes[0] % 4;
    let mut section = [0.0f32; 6];
    for (index, coefficient) in section.iter_mut().enumerate() {
        *coefficient = f32::from_bits(fuzz_word(bytes, 1 + index * 4));
    }
    let samples = [
        f32::from_bits(fuzz_word(bytes, 25)),
        f32::from_bits(fuzz_word(bytes, 29)),
        f32::from_bits(fuzz_word(bytes, 33)),
    ];
    let class: usize;
    let verdict = match op {
        0 => match Biquad::new(section) {
            Err(FilterError::InvalidCoefficient(_)) => {
                class = 6;
                true
            }
            Err(_) => return fuzz_crash("unexpected-constructor-error"),
            Ok(mut filter) => match filter.process_sample(samples[0]) {
                Ok(value) if value.is_finite() => {
                    class = 1;
                    true
                }
                Ok(_) => return fuzz_crash("nonfinite-sample-output"),
                Err(FilterError::InvalidCoefficient(_)) => {
                    class = 4;
                    true
                }
                Err(_) => return fuzz_crash("unexpected-sample-error"),
            },
        },
        1 => match Biquad::new(section) {
            Err(FilterError::InvalidCoefficient(_)) => {
                class = 6;
                true
            }
            Err(_) => return fuzz_crash("unexpected-constructor-error"),
            Ok(mut filter) => match filter.process(&samples) {
                Ok(values)
                    if values.len() == samples.len() && values.iter().all(|v| v.is_finite()) =>
                {
                    class = 1;
                    true
                }
                Ok(_) => return fuzz_crash("invalid-block-result"),
                Err(FilterError::InvalidCoefficient(_)) => {
                    class = 4;
                    true
                }
                Err(_) => return fuzz_crash("unexpected-block-error"),
            },
        },
        2 => match sosfilt(&[section], &samples) {
            Ok(values) if values.len() == samples.len() && values.iter().all(|v| v.is_finite()) => {
                class = 1;
                true
            }
            Ok(_) => return fuzz_crash("invalid-sos-result"),
            Err(FilterError::InvalidCoefficient(_)) => {
                class = 4;
                true
            }
            Err(_) => return fuzz_crash("unexpected-sos-error"),
        },
        _ => match Biquad::new(section) {
            Err(FilterError::InvalidCoefficient(_)) => {
                class = 6;
                true
            }
            Err(_) => return fuzz_crash("unexpected-constructor-error"),
            Ok(mut filter) => match filter.process_sample(samples[0]) {
                Err(FilterError::InvalidCoefficient(_)) => {
                    class = 4;
                    true
                }
                Err(_) => return fuzz_crash("unexpected-reset-prefix-error"),
                Ok(first) if !first.is_finite() => return fuzz_crash("nonfinite-reset-prefix"),
                Ok(_) => {
                    if !skip_reset_for_negative_control {
                        filter.reset();
                    }
                    let actual = filter.process_sample(samples[1]);
                    let expected =
                        Biquad::new(section).and_then(|mut fresh| fresh.process_sample(samples[1]));
                    match (actual, expected) {
                        (Ok(actual), Ok(expected))
                            if actual.is_finite() && expected.is_finite() && actual == expected =>
                        {
                            class = 1;
                        }
                        (Ok(_), Ok(_)) => return fuzz_crash("reset-result-mismatch"),
                        (
                            Err(FilterError::InvalidCoefficient(_)),
                            Err(FilterError::InvalidCoefficient(_)),
                        ) => {
                            class = 4;
                        }
                        _ => return fuzz_crash("reset-result-class-mismatch"),
                    }
                    true
                }
            },
        },
    };
    if !verdict {
        return fuzz_crash("semantic-invariant-failed");
    }
    // These manually assigned public-outcome bins guide admission; they are
    // not production branch instrumentation or source-proof evidence.
    coverage.record_edge((op as usize) * 16 + class);
    FuzzOutcome::Pass
}

fn run_structured_fuzz_case(bytes: &[u8], coverage: &mut CoverageBitmap) -> FuzzOutcome {
    run_structured_fuzz_case_with_reset_mutant(bytes, coverage, false)
}

#[test]
fn ver06_structured_raw_bit_fuzz_replays_and_exercises_minimization() {
    let initial = vec![
        fuzz_seed(0, [1.0, 0.0, 0.0, 1.0, 0.0, 0.0], [1.0, 0.0, 0.0]),
        fuzz_seed(1, [0.0, 0.0, 1.0, 1.0, 0.0, 0.0], [1.0, 2.0, 3.0]),
        fuzz_seed(2, [2.0, 0.0, 0.0, 1.0, 0.0, 0.0], [f32::MAX, 0.0, 0.0]),
        fuzz_seed(
            3,
            [0.0, f32::MIN_POSITIVE, 0.0, 1.0, 0.0, 0.0],
            [0.0, -0.0, f32::from_bits(1)],
        ),
        fuzz_seed(
            0,
            [0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            [f32::NAN, f32::INFINITY, f32::NEG_INFINITY],
        ),
    ];
    let options = FuzzOptions {
        max_iterations: 1024,
        seed: 0xAF03_F022,
        max_input_len: 40,
        base_energy: 8,
    };
    let mut triage_coverage = CoverageBitmap::new();
    let mut triage_admitted = 0usize;
    for seed in &initial {
        let mut trace = CoverageBitmap::new();
        assert!(matches!(
            run_structured_fuzz_case(seed, &mut trace),
            FuzzOutcome::Pass
        ));
        let (has_new_edges, _) = triage_coverage.admit_trace(&trace.covered_edges());
        if has_new_edges || triage_admitted == 0 {
            triage_admitted += 1;
        }
    }
    let first = fuzz_target(&initial, &options, run_structured_fuzz_case);
    let replay = fuzz_target(&initial, &options, run_structured_fuzz_case);
    let post_triage_admitted = first.admitted_seeds - triage_admitted;
    println!(
        "AF-03 VER06 seed=0xAF03F022 iterations={} admitted={} initial={} post-triage={} edges={} crashes={} timeouts={} ooms={}",
        first.iterations,
        first.admitted_seeds,
        triage_admitted,
        post_triage_admitted,
        first.total_edges,
        first.crashes.len(),
        first.timeouts,
        first.ooms
    );
    assert!(first.is_clean());
    assert_eq!(first.iterations, 1024);
    assert!(first.admitted_seeds > initial.len());
    assert!(first.total_edges > 0);
    assert_eq!(first.iterations, replay.iterations);
    assert_eq!(first.admitted_seeds, replay.admitted_seeds);
    assert_eq!(first.total_edges, replay.total_edges);
    assert!(first.crashes.is_empty());
    assert_eq!(first.timeouts, 0);
    assert_eq!(first.ooms, 0);

    let reset_fault = fuzz_seed(3, [0.0, 1.0, 0.0, 1.0, 0.0, 0.0], [1.0, 2.0, 0.0]);
    let crash_kind = "reset-result-mismatch";
    let signature = crash_signature(crash_kind, &["biquad_sos::public_semantic_oracle"]);
    let mutant_outcome = |input: &[u8]| {
        run_structured_fuzz_case_with_reset_mutant(input, &mut CoverageBitmap::new(), true)
    };
    assert!(matches!(
        mutant_outcome(&reset_fault),
        FuzzOutcome::Crash { .. }
    ));
    let minimized = minimize_crashing_input(&reset_fault, &signature, mutant_outcome);
    assert!(matches!(
        run_structured_fuzz_case_with_reset_mutant(
            &minimized,
            &mut CoverageBitmap::new(),
            true
        ),
        FuzzOutcome::Crash { ref kind, .. } if kind == crash_kind
    ));
    assert!(matches!(
        run_structured_fuzz_case(&reset_fault, &mut CoverageBitmap::new()),
        FuzzOutcome::Pass
    ));
    assert!(minimized.len() < reset_fault.len());
    println!(
        "AF-03 VER06 reset semantic mutant minimized {} bytes to {} bytes and replayed",
        reset_fault.len(),
        minimized.len()
    );
}
