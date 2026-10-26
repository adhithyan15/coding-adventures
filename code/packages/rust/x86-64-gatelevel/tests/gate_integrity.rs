#[test]
fn execution_uses_gate_qualification_instead_of_host_wide_comparisons() {
    let execution = include_str!("../src/execution.rs");
    for forbidden in [
        "product != i128::from",
        "product >> 64 != 0",
        "q < i64::MIN",
        "q > i64::MAX",
        "q > u64::MAX",
        "if d == 0",
    ] {
        assert!(
            !execution.contains(forbidden),
            "host result qualification returned: {forbidden}"
        );
    }
    assert_eq!(execution.matches("product.overflow != 0").count(), 3);
    assert_eq!(
        execution.matches("division.quotient.overflow != 0").count(),
        2
    );
}

#[test]
fn qualifier_networks_reduce_every_discarded_wire_with_gates() {
    let source = include_str!("../src/gate_networks.rs");
    let production = source
        .split_once("#[cfg(test)]")
        .map_or(source, |(production, _)| production);
    let unsigned_qualifier = production
        .split_once("fn qualify_unsigned")
        .and_then(|(_, rest)| rest.split_once("fn qualify_signed"))
        .map(|(body, _)| body)
        .expect("unsigned qualifier remains a distinct production function");
    let signed_qualifier = production
        .split_once("fn qualify_signed")
        .and_then(|(_, rest)| rest.split_once("fn u128_to_bits"))
        .map(|(body, _)| body)
        .expect("signed qualifier remains a distinct production function");

    for forbidden in [
        "high != 0",
        "high == 0",
        "high > 0",
        "high < 0",
        "product >> 64",
        "quotient >> 64",
        "q < i64::MIN",
        "q > i64::MAX",
        "q > u64::MAX",
        "!= i128::from",
        "== i128::from",
    ] {
        assert!(
            !production.contains(forbidden),
            "host-wide qualifier returned to gate_networks.rs: {forbidden}"
        );
    }
    for qualifier in [unsigned_qualifier, signed_qualifier] {
        for forbidden in [" == ", " != ", " < ", " > ", ".any(", ".all(", ".contains("] {
            assert!(
                !qualifier.contains(forbidden),
                "qualifier bypassed its gate reduction with {forbidden:?}"
            );
        }
    }

    assert!(production.contains("let any_high = bits[64..].iter().copied().fold(0, or_gate);"));
    assert!(production.contains(".map(|bit| xor_gate(bit, sign))\n        .fold(0, or_gate);"));
    assert!(unsigned_qualifier.contains("overflow: any_high"));
    assert!(signed_qualifier.contains("overflow: differs_from_sign"));
    assert_eq!(
        production
            .matches("qualify_unsigned(&u128_to_bits(")
            .count(),
        2
    );
    assert_eq!(
        production.matches("qualify_signed(&u128_to_bits(").count(),
        2
    );
    assert_eq!(production.matches("zero_64(divisor) != 0").count(), 2);
}
