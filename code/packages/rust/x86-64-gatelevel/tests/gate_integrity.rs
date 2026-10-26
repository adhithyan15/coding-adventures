const UNSIGNED_ENTRY: &str = r#"
pub(crate) fn multiply_unsigned(a: u64, b: u64) -> QualifiedWideResult {
    qualify_unsigned(&u128_to_bits(multiply_unsigned_wide(a, b)))
}
"#;

const SIGNED_ENTRY: &str = r#"
pub(crate) fn multiply_signed(a: u64, b: u64) -> QualifiedWideResult {
    let a_negative = u64_to_bits(a)[63];
    let b_negative = u64_to_bits(b)[63];
    let a_magnitude = magnitude64(a, a_negative);
    let b_magnitude = magnitude64(b, b_negative);
    let product = multiply_unsigned_wide(a_magnitude, b_magnitude);
    let product = if xor_gate(a_negative, b_negative) == 0 {
        product as i128
    } else {
        negate128(product) as i128
    };
    qualify_signed(&u128_to_bits(product as u128))
}
"#;

const UNSIGNED_DIVISION_ENTRY: &str = r#"
pub(crate) fn divide_unsigned(dividend: u128, divisor: u64) -> Option<QualifiedDivision> {
    let (quotient, remainder) = divide_unsigned_wide(dividend, divisor)?;
    Some(QualifiedDivision {
        quotient: qualify_unsigned(&u128_to_bits(quotient)),
        remainder,
    })
}
"#;

const SIGNED_DIVISION_ENTRY: &str = r#"
pub(crate) fn divide_signed(dividend: u128, divisor: u64) -> Option<QualifiedDivision> {
    if zero_64(divisor) != 0 {
        return None;
    }
    let dividend_negative = u128_to_bits(dividend)[127];
    let divisor_negative = u64_to_bits(divisor)[63];
    let dividend_magnitude = if dividend_negative == 0 {
        dividend
    } else {
        negate128(dividend)
    };
    let divisor_magnitude = magnitude64(divisor, divisor_negative);
    let (quotient, remainder) = divide_unsigned_wide(dividend_magnitude, divisor_magnitude)?;
    let quotient = if xor_gate(dividend_negative, divisor_negative) == 0 {
        quotient
    } else {
        negate128(quotient)
    };
    let remainder = if dividend_negative == 0 {
        remainder
    } else {
        add_64(invert_64(remainder), 0, 1).0
    };
    Some(QualifiedDivision {
        quotient: qualify_signed(&u128_to_bits(quotient)),
        remainder,
    })
}
"#;

const UNSIGNED_QUALIFIER: &str = r#"
fn qualify_unsigned(bits: &[u8; 128]) -> QualifiedWideResult {
    let low = bits_to_u64(&bits[..64]);
    let high = bits_to_u64(&bits[64..]);
    let any_high = bits[64..]
        .iter()
        .copied()
        .fold(0, ::logic_gates::gates::or_gate);
    QualifiedWideResult {
        low,
        high,
        overflow: any_high,
    }
}
"#;

const SIGNED_QUALIFIER: &str = r#"
fn qualify_signed(bits: &[u8; 128]) -> QualifiedWideResult {
    let low = bits_to_u64(&bits[..64]);
    let high = bits_to_u64(&bits[64..]);
    let sign = bits[63];
    let differs_from_sign = bits[64..]
        .iter()
        .copied()
        .map(|bit| ::logic_gates::gates::xor_gate(bit, sign))
        .fold(0, ::logic_gates::gates::or_gate);
    QualifiedWideResult {
        low,
        high,
        overflow: differs_from_sign,
    }
}
"#;

const MULTIPLY_CONSUMERS: &str = r#"
Instr::Imul { dst, src } => {
    let product = crate::gate_networks::multiply_signed(
        st.get(*dst),
        read_op(st, mem, src, next_ip, 8)?,
    );
    st.set(*dst, product.low);
    let truncated = product.overflow != 0;
    st.flags.cf = truncated;
    st.flags.of = truncated;
    Ok(Flow::Next)
}
Instr::ImulImm { dst, src, imm } => {
    let product = crate::gate_networks::multiply_signed(
        read_op(st, mem, src, next_ip, 8)?,
        *imm as u64,
    );
    st.set(*dst, product.low);
    let truncated = product.overflow != 0;
    st.flags.cf = truncated;
    st.flags.of = truncated;
    Ok(Flow::Next)
}
Instr::Mul { src } => {
    let source = read_op(st, mem, src, next_ip, 8)?;
    let product = crate::gate_networks::multiply_unsigned(st.get(Reg::Rax), source);
    st.set(Reg::Rax, product.low);
    st.set(Reg::Rdx, product.high);
    st.flags.cf = product.overflow != 0;
    st.flags.of = st.flags.cf;
    Ok(Flow::Next)
}
"#;

const DIVIDE_CONSUMER: &str = r#"
Instr::Div { divisor, signed } => {
    let d = read_op(st, mem, divisor, next_ip, 8)?;
    let lo = st.get(Reg::Rax);
    let hi = st.get(Reg::Rdx);
    let dividend = ((hi as u128) << 64) | (lo as u128);
    let (quot, rem) = if *signed {
        let division = crate::gate_networks::divide_signed(dividend, d)
            .ok_or(Trap::DivideError(instr_off as u64))?;
        if division.quotient.overflow != 0 {
            return Err(Trap::DivideError(instr_off as u64));
        }
        (division.quotient.low, division.remainder)
    } else {
        let division = crate::gate_networks::divide_unsigned(dividend, d)
            .ok_or(Trap::DivideError(instr_off as u64))?;
        if division.quotient.overflow != 0 {
            return Err(Trap::DivideError(instr_off as u64));
        }
        (division.quotient.low, division.remainder)
    };
    st.set(Reg::Rax, quot);
    st.set(Reg::Rdx, rem);
    Ok(Flow::Next)
}
"#;

fn normalize(source: &str) -> String {
    source
        .lines()
        .map(|line| line.split_once("//").map_or(line, |(code, _)| code))
        .flat_map(str::chars)
        .filter(|character| !character.is_whitespace())
        .collect()
}

fn region<'a>(source: &'a str, start: &str, end: &str) -> &'a str {
    let start_index = source
        .find(start)
        .unwrap_or_else(|| panic!("missing structural-contract start marker: {start}"));
    let tail = &source[start_index..];
    let end_index = tail
        .find(end)
        .unwrap_or_else(|| panic!("missing structural-contract end marker: {end}"));
    &tail[..end_index]
}

fn matches_contract(actual: &str, expected: &str) -> bool {
    normalize(actual) == normalize(expected)
}

fn assert_region(source: &str, start: &str, end: &str, expected: &str) {
    let actual = region(source, start, end);
    assert_eq!(
        normalize(actual),
        normalize(expected),
        "gate-integrity contract changed between {start:?} and {end:?}"
    );
}

#[test]
fn production_qualification_call_paths_match_the_reviewed_contract() {
    let gates = include_str!("../src/gate_networks.rs");
    assert_region(
        gates,
        "pub(crate) fn multiply_unsigned",
        "fn multiply_unsigned_wide",
        UNSIGNED_ENTRY,
    );
    assert_region(
        gates,
        "pub(crate) fn multiply_signed",
        "pub(crate) fn divide_unsigned",
        SIGNED_ENTRY,
    );
    assert_region(
        gates,
        "pub(crate) fn divide_unsigned",
        "fn divide_unsigned_wide",
        UNSIGNED_DIVISION_ENTRY,
    );
    assert_region(
        gates,
        "pub(crate) fn divide_signed",
        "pub(crate) fn address",
        SIGNED_DIVISION_ENTRY,
    );
    assert_region(
        gates,
        "fn qualify_unsigned",
        "fn qualify_signed",
        UNSIGNED_QUALIFIER,
    );
    assert_region(
        gates,
        "fn qualify_signed",
        "fn u128_to_bits",
        SIGNED_QUALIFIER,
    );

    let execution = include_str!("../src/execution.rs");
    assert_region(
        execution,
        "Instr::Imul { dst, src }",
        "Instr::IncDec { dst, decrement }",
        MULTIPLY_CONSUMERS,
    );
    assert_region(
        execution,
        "Instr::Div { divisor, signed }",
        "Instr::MovsdLoad { xmm, src }",
        DIVIDE_CONSUMER,
    );
}

#[test]
fn structural_contract_rejects_helper_shadow_and_iterator_bypasses() {
    let unsigned_shadow = UNSIGNED_QUALIFIER.replace(
        "QualifiedWideResult {",
        "let any_high = host_nonzero(bits); QualifiedWideResult {",
    );
    let unsigned_sum = UNSIGNED_QUALIFIER.replace(
        ".iter()\n        .copied()\n        .fold(0, ::logic_gates::gates::or_gate)",
        ".iter().copied().sum::<u8>().min(1)",
    );
    let signed_shadow = SIGNED_QUALIFIER.replace(
        "QualifiedWideResult {",
        "let differs_from_sign = bits[64..].iter().filter(|bit| **bit != sign).count().min(1) as u8; QualifiedWideResult {",
    );
    assert!(!matches_contract(&unsigned_shadow, UNSIGNED_QUALIFIER));
    assert!(!matches_contract(&unsigned_sum, UNSIGNED_QUALIFIER));
    assert!(!matches_contract(&signed_shadow, SIGNED_QUALIFIER));
}

#[test]
fn structural_contract_rejects_dead_anchors_and_host_wide_consumers() {
    let multiply_bypass = MULTIPLY_CONSUMERS.replacen(
        "let truncated = product.overflow != 0;",
        "let truncated = product.high.count_ones() != 0; let _anchor = product.overflow != 0;",
        1,
    );
    let divide_bypass = DIVIDE_CONSUMER.replacen(
        "if division.quotient.overflow != 0 {",
        "let _anchor = division.quotient.overflow != 0; if division.quotient.high > 0 {",
        1,
    );
    assert!(!matches_contract(&multiply_bypass, MULTIPLY_CONSUMERS));
    assert!(!matches_contract(&divide_bypass, DIVIDE_CONSUMER));
}
