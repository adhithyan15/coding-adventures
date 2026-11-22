use coding_adventures_hash_functions as hashes;
use coding_adventures_json_value::{parse, JsonNumber, JsonValue};

const FIXTURE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../../code/specs/fixtures/hash-functions-v1/cases.json"
));

fn field<'a>(value: &'a JsonValue, name: &str) -> &'a JsonValue {
    match value {
        JsonValue::Object(fields) => fields
            .iter()
            .find_map(|(key, value)| (key == name).then_some(value))
            .unwrap_or_else(|| panic!("missing field {name}")),
        _ => panic!("expected object while reading {name}"),
    }
}

fn string(value: &JsonValue) -> &str {
    match value {
        JsonValue::String(text) => text,
        _ => panic!("expected string, got {value:?}"),
    }
}

fn integer(value: &JsonValue) -> u64 {
    match value {
        JsonValue::Number(JsonNumber::Integer(number)) => *number as u64,
        JsonValue::String(text) => parse_u64(text),
        _ => panic!("expected unsigned integer/string, got {value:?}"),
    }
}

fn parse_u64(text: &str) -> u64 {
    if let Some(hex) = text.strip_prefix("0x") {
        u64::from_str_radix(hex, 16).unwrap()
    } else {
        text.parse().unwrap()
    }
}

fn bytes_hex(text: &str) -> Vec<u8> {
    assert_eq!(text.len() % 2, 0, "odd-length hex input");
    text.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

fn expected_for(op: &str, data: &[u8], parameters: Option<&JsonValue>) -> u64 {
    let param = |name: &str| {
        parameters.and_then(|params| match params {
            JsonValue::Object(fields) if fields.iter().any(|(key, _)| key == op) => {
                Some(field(field(params, op), name))
            }
            _ => match params {
                JsonValue::Object(fields) if fields.iter().any(|(key, _)| key == name) => {
                    Some(field(params, name))
                }
                _ => None,
            },
        })
    };
    let parsed_hex_key = |text: &str| -> [u8; 16] {
        bytes_hex(text)
            .try_into()
            .expect("SipHash key must be 16 bytes")
    };
    match op {
        "fnv1a_32" => hashes::fnv1a_32(data) as u64,
        "fnv1a_64" => hashes::fnv1a_64(data),
        "djb2" => hashes::djb2(data),
        "polynomial_rolling" => hashes::polynomial_rolling(data),
        "polynomial_rolling_with_params" => hashes::polynomial_rolling_with_params(
            data,
            param("base").map(integer).unwrap_or(31),
            param("modulus")
                .map(integer)
                .unwrap_or(hashes::POLYNOMIAL_ROLLING_DEFAULT_MODULUS),
        ),
        "murmur3_32" => hashes::murmur3_32(data) as u64,
        "murmur3_32_with_seed" => hashes::murmur3_32_with_seed(
            data,
            param("seed").map(integer).unwrap_or(0x9747_b28c) as u32,
        ) as u64,
        "siphash_2_4" => {
            let key = parsed_hex_key(
                param("key_hex")
                    .map(string)
                    .unwrap_or("000102030405060708090a0b0c0d0e0f"),
            );
            hashes::siphash_2_4(data, &key)
        }
        _ => panic!("unknown fixture operation {op}"),
    }
}

fn expected_text(op: &str, text: &str, parameters: Option<&JsonValue>) -> u64 {
    match op {
        "hash_str_fnv1a_32" => hashes::hash_str_fnv1a_32(text) as u64,
        "hash_str_siphash" => {
            let key: [u8; 16] = bytes_hex(
                parameters
                    .map(|params| string(field(params, "key_hex")))
                    .unwrap_or("000102030405060708090a0b0c0d0e0f"),
            )
            .try_into()
            .unwrap();
            hashes::hash_str_siphash(text, &key)
        }
        _ => panic!("unknown text operation {op}"),
    }
}

fn check_record(record: &JsonValue) {
    let id = string(field(record, "id"));
    let data = bytes_hex(string(field(record, "data_hex")));
    let parameters = match record {
        JsonValue::Object(fields) => fields
            .iter()
            .find_map(|(key, value)| (key == "parameters").then_some(value)),
        _ => None,
    };
    let expected = field(record, "expected_hex");
    if let JsonValue::String(answer) = expected {
        let op = string(field(record, "op"));
        let actual = expected_for(op, &data, parameters);
        let width = if matches!(op, "fnv1a_32" | "murmur3_32" | "murmur3_32_with_seed") {
            8
        } else {
            16
        };
        assert_eq!(format!("{actual:0width$x}"), answer.as_str(), "{id}/{op}");
        return;
    }
    let JsonValue::Object(outputs) = expected else {
        panic!("{id}: expected_hex must be an object")
    };
    for (op, answer) in outputs {
        let width = if matches!(
            op.as_str(),
            "fnv1a_32" | "murmur3_32" | "murmur3_32_with_seed"
        ) {
            8
        } else {
            16
        };
        let actual = expected_for(op, &data, parameters);
        assert_eq!(format!("{actual:0width$x}"), string(answer), "{id}/{op}");
    }
}

fn validate_stream(
    stream: &str,
    expected: &str,
    count: usize,
    width: usize,
) -> Result<(), &'static str> {
    if stream.len() != count * width {
        return Err("length");
    }
    if !stream
        .bytes()
        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err("encoding");
    }
    if stream != expected {
        return Err("answer");
    }
    Ok(())
}

fn check_stream_output(
    stream: &str,
    index: usize,
    width: usize,
    actual: u64,
) -> Result<(), &'static str> {
    let start = index.checked_mul(width).ok_or("length")?;
    let end = start.checked_add(width).ok_or("length")?;
    let expected = stream.get(start..end).ok_or("length")?;
    let encoded = format!("{actual:0width$x}");
    if expected == encoded {
        Ok(())
    } else {
        Err("answer")
    }
}

#[test]
fn public_rust_hashes_match_every_shared_v1_case() {
    let fixture = parse(FIXTURE).expect("fixture JSON must parse");
    for section in ["known_answers", "utf8_cases", "directed"] {
        if let JsonValue::Array(records) = field(&fixture, section) {
            for record in records {
                if section == "utf8_cases" {
                    let op = string(field(record, "op"));
                    let text = string(field(record, "text"));
                    let params = match record {
                        JsonValue::Object(fields) => fields
                            .iter()
                            .find_map(|(key, value)| (key == "parameters").then_some(value)),
                        _ => None,
                    };
                    let actual = expected_text(op, text, params);
                    let width = if op == "hash_str_fnv1a_32" { 8 } else { 16 };
                    assert_eq!(
                        format!("{actual:0width$x}"),
                        string(field(record, "expected_hex")),
                        "{}",
                        string(field(record, "id"))
                    );
                } else {
                    check_record(record);
                }
            }
        } else {
            panic!("{section} must be an array")
        }
    }
    for record in match field(&fixture, "generated").clone() {
        JsonValue::Object(fields) => match fields
            .iter()
            .find(|(key, _)| key == "cases")
            .unwrap()
            .1
            .clone()
        {
            JsonValue::Array(rows) => rows,
            _ => panic!("generated cases must be an array"),
        },
        _ => panic!("generated must be an object"),
    } {
        check_record(&record);
    }
    if let JsonValue::Object(mutations) = field(&fixture, "mutations") {
        if let JsonValue::Array(pairs) = field(&JsonValue::Object(mutations.clone()), "cases") {
            for pair in pairs {
                check_record(field(pair, "original"));
                check_record(field(pair, "mutated"));
            }
        }
    }

    check_exhaustive(&fixture);
    check_analysis_public_hash_callbacks(&fixture);
    check_rust_zero_modulus_panic(&fixture);
}

fn check_analysis_public_hash_callbacks(fixture: &JsonValue) {
    let JsonValue::Array(cases) = field(fixture, "analysis") else {
        panic!("analysis must be an array")
    };
    for case in cases {
        if string(field(field(case, "callback"), "kind")) != "hash" {
            // Synthetic callback vectors are profile-only and checked by the
            // independent oracle plus the Dart public consumer.
            continue;
        }
        assert_eq!(string(field(field(case, "callback"), "op")), "fnv1a_32");
        let expected = field(case, "expected");
        match string(field(case, "kind")) {
            "avalanche" => {
                let bits = integer(field(case, "output_bits")) as u32;
                let JsonValue::Array(samples) = field(case, "samples_hex") else {
                    panic!("samples_hex must be an array")
                };
                let mask = if bits == 32 {
                    u32::MAX
                } else {
                    (1u32 << bits) - 1
                };
                let mut flipped = 0u64;
                let mut compared = 0u64;
                for sample in samples {
                    let bytes = bytes_hex(string(sample));
                    assert!(!bytes.is_empty());
                    let original = hashes::fnv1a_32(&bytes);
                    for bit in 0..bytes.len() * 8 {
                        let mut changed = bytes.clone();
                        changed[bit / 8] ^= 1 << (bit % 8);
                        flipped +=
                            ((original ^ hashes::fnv1a_32(&changed)) & mask).count_ones() as u64;
                        compared += bits as u64;
                    }
                }
                assert_eq!(flipped.to_string(), string(field(expected, "flipped_bits")));
                assert_eq!(
                    compared.to_string(),
                    string(field(expected, "compared_bits"))
                );
            }
            "distribution" => {
                let bucket_count = integer(field(case, "bucket_count")) as usize;
                let JsonValue::Array(inputs) = field(case, "inputs_hex") else {
                    panic!("inputs_hex must be an array")
                };
                let mut buckets = vec![0u64; bucket_count];
                for input in inputs {
                    let output = hashes::fnv1a_32(&bytes_hex(string(input))) as usize;
                    buckets[output % bucket_count] += 1;
                }
                let expected_buckets = match field(expected, "bucket_counts") {
                    JsonValue::Array(values) => values.iter().map(integer).collect::<Vec<_>>(),
                    _ => panic!("bucket_counts must be an array"),
                };
                assert_eq!(
                    buckets,
                    expected_buckets,
                    "{} bucket counts",
                    string(field(case, "id"))
                );
                let total = inputs.len() as u64;
                let mut numerator: u64 = buckets
                    .iter()
                    .map(|count| (bucket_count as i64 * *count as i64 - total as i64).pow(2) as u64)
                    .sum();
                let mut denominator = bucket_count as u64 * total;
                let divisor = gcd(numerator, denominator);
                numerator /= divisor;
                denominator /= divisor;
                assert_eq!(
                    numerator.to_string(),
                    string(field(expected, "chi_squared_numerator"))
                );
                assert_eq!(
                    denominator.to_string(),
                    string(field(expected, "chi_squared_denominator"))
                );
            }
            kind => panic!("unknown analysis kind {kind}"),
        }
    }
}

fn gcd(mut left: u64, mut right: u64) -> u64 {
    while right != 0 {
        (left, right) = (right, left % right);
    }
    left.max(1)
}

fn check_exhaustive(fixture: &JsonValue) {
    let exhaustive = field(fixture, "exhaustive");
    let domain = field(exhaustive, "domain");
    let count = integer(field(domain, "count")) as usize;
    assert_eq!(count, 65_793);
    let streams = field(exhaustive, "expected_hex_streams");
    let fixed = field(exhaustive, "fixed_parameters");
    let seed = parse_u64(string(field(fixed, "murmur3_32_with_seed"))) as u32;
    let key: [u8; 16] = bytes_hex(string(field(fixed, "siphash_2_4_key_hex")))
        .try_into()
        .unwrap();
    let polynomial_base = integer(field(fixed, "polynomial_base"));
    let polynomial_modulus = integer(field(fixed, "polynomial_modulus"));
    let operations: [(&str, usize); 8] = [
        ("fnv1a_32", 8),
        ("fnv1a_64", 16),
        ("djb2", 16),
        ("polynomial_rolling", 16),
        ("polynomial_rolling_with_params", 16),
        ("murmur3_32", 8),
        ("murmur3_32_with_seed", 8),
        ("siphash_2_4", 16),
    ];
    for (op, width) in operations {
        let stream = string(field(streams, op));
        assert_eq!(stream.len(), count * width, "{op} stream length");
        assert!(
            stream
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
            "{op} lowercase hex"
        );
        let mut index = 0usize;
        for len in 0..=2 {
            let limit = 256usize.pow(len as u32);
            for value in 0..limit {
                let data = match len {
                    0 => Vec::new(),
                    1 => vec![value as u8],
                    _ => vec![(value >> 8) as u8, value as u8],
                };
                let boundary = match index {
                    0 => Some(&b""[..]),
                    1 => Some(&b"\x00"[..]),
                    256 => Some(&b"\xff"[..]),
                    257 => Some(&b"\x00\x00"[..]),
                    65_792 => Some(&b"\xff\xff"[..]),
                    _ => None,
                };
                if let Some(expected_input) = boundary {
                    assert_eq!(data, expected_input, "{op} fixture ordering at {index}");
                }
                let actual = match op {
                    "fnv1a_32" => hashes::fnv1a_32(&data) as u64,
                    "fnv1a_64" => hashes::fnv1a_64(&data),
                    "djb2" => hashes::djb2(&data),
                    "polynomial_rolling" => hashes::polynomial_rolling(&data),
                    "polynomial_rolling_with_params" => hashes::polynomial_rolling_with_params(
                        &data,
                        polynomial_base,
                        polynomial_modulus,
                    ),
                    "murmur3_32" => hashes::murmur3_32(&data) as u64,
                    "murmur3_32_with_seed" => hashes::murmur3_32_with_seed(&data, seed) as u64,
                    "siphash_2_4" => hashes::siphash_2_4(&data, &key),
                    _ => unreachable!(),
                };
                assert_eq!(
                    check_stream_output(stream, index, width, actual),
                    Ok(()),
                    "{op} exhaustive index {index}"
                );
                index += 1;
            }
        }
        assert_eq!(index, count, "{op} must consume every entry");
        assert_eq!(
            &stream[..width],
            format!(
                "{:0width$x}",
                match op {
                    "fnv1a_32" => hashes::fnv1a_32(&[]) as u64,
                    "fnv1a_64" => hashes::fnv1a_64(&[]),
                    "djb2" => hashes::djb2(&[]),
                    "polynomial_rolling" => hashes::polynomial_rolling(&[]),
                    "polynomial_rolling_with_params" => hashes::polynomial_rolling_with_params(
                        &[],
                        polynomial_base,
                        polynomial_modulus,
                    ),
                    "murmur3_32" => hashes::murmur3_32(&[]) as u64,
                    "murmur3_32_with_seed" => {
                        hashes::murmur3_32_with_seed(&[], seed) as u64
                    }
                    "siphash_2_4" => hashes::siphash_2_4(&[], &key),
                    _ => unreachable!(),
                }
            ),
            "{op} index 0 (empty input)"
        );
        assert_eq!(index, 65_793);
        validate_stream(stream, stream, count, width).unwrap();
        if op == "fnv1a_32" {
            assert_eq!(
                validate_stream(&stream[..stream.len() - 1], stream, count, width),
                Err("length")
            );
            let mut malformed = stream.to_owned();
            malformed.replace_range(..1, "A");
            assert_eq!(
                validate_stream(&malformed, stream, count, width),
                Err("encoding")
            );
            let mut mutated = stream.to_owned();
            let replacement = if &mutated[..1] == "0" { "1" } else { "0" };
            mutated.replace_range(..1, replacement);
            assert_eq!(
                check_stream_output(&mutated, 0, width, hashes::fnv1a_32(&[]) as u64,),
                Err("answer")
            );
        }
        if op == "polynomial_rolling_with_params" {
            // A default-dispatch regression must disagree on this two-byte
            // input because the fixture base is deliberately nondefault 257.
            let data = [0x01, 0x00];
            let index = 513; // empty + 256 single bytes + 256 preceding pairs
            assert_eq!(
                check_stream_output(stream, index, width, hashes::polynomial_rolling(&data),),
                Err("answer"),
                "dispatching this lane to the default API must be detected"
            );
        }
    }
}

fn check_rust_zero_modulus_panic(fixture: &JsonValue) {
    let cases = field(fixture, "rust_panics");
    let JsonValue::Array(cases) = cases else {
        panic!("rust_panics must be array")
    };
    let case = cases
        .iter()
        .find(|case| string(field(case, "id")) == "poly-modulus-zero-rust-reference")
        .unwrap();
    let data = bytes_hex(string(field(case, "data_hex")));
    let base = integer(field(case, "base"));
    let modulus = integer(field(case, "modulus"));
    let result =
        std::panic::catch_unwind(|| hashes::polynomial_rolling_with_params(&data, base, modulus));
    assert!(
        result.is_err(),
        "native Rust zero-modulus behavior is panic, not an error value"
    );
}
