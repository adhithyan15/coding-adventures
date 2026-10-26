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
