use rsa_pss_data_block::{
    validate_rsa_pss_data_block, RsaPssDataBlockErrorKind, RsaPssDataBlockPolicy,
    RsaPssDataBlockPolicyErrorKind, RSA_PSS_DATA_BLOCK_DELIMITER,
};

fn canonical_data_block(padding_length: usize, salt: &[u8]) -> Vec<u8> {
    let mut block = vec![0; padding_length];
    block.push(RSA_PSS_DATA_BLOCK_DELIMITER);
    block.extend_from_slice(salt);
    block
}

#[test]
fn validates_typical_data_block_and_exact_borrows() {
    let salt: Vec<u8> = (0..32).collect();
    let block = canonical_data_block(190, &salt);
    let policy = RsaPssDataBlockPolicy::new(32, 223).unwrap();

    let validated = validate_rsa_pss_data_block(&block, policy).unwrap();
    assert_eq!(validated.data_block(), block);
    assert_eq!(validated.padding_string_length(), 190);
    assert_eq!(validated.salt(), salt);
    assert_eq!(format!("{validated:?}"), "ValidatedRsaPssDataBlock");
}

#[test]
fn supports_zero_length_padding_string() {
    let block = canonical_data_block(0, &[0xaa, 0xbb, 0xcc]);
    let policy = RsaPssDataBlockPolicy::new(3, 4).unwrap();
    let validated = validate_rsa_pss_data_block(&block, policy).unwrap();
    assert_eq!(validated.padding_string_length(), 0);
    assert_eq!(validated.salt(), &[0xaa, 0xbb, 0xcc]);
}

#[test]
fn supports_zero_length_salt() {
    let block = canonical_data_block(5, &[]);
    let policy = RsaPssDataBlockPolicy::new(0, 6).unwrap();
    let validated = validate_rsa_pss_data_block(&block, policy).unwrap();
    assert_eq!(validated.padding_string_length(), 5);
    assert!(validated.salt().is_empty());
}

#[test]
fn policy_reports_public_bounds() {
    let policy = RsaPssDataBlockPolicy::new(48, 512).unwrap();
    assert_eq!(policy.salt_length(), 48);
    assert_eq!(policy.maximum_data_block_bytes(), 512);
    assert_eq!(policy.minimum_data_block_bytes(), 49);
}

#[test]
fn policy_rejects_zero_ceiling_overflow_and_unusable_ceiling() {
    let zero = RsaPssDataBlockPolicy::new(0, 0).unwrap_err();
    assert_eq!(
        zero.kind(),
        RsaPssDataBlockPolicyErrorKind::ZeroMaximumDataBlockBytes
    );

    let overflow = RsaPssDataBlockPolicy::new(usize::MAX, usize::MAX).unwrap_err();
    assert_eq!(
        overflow.kind(),
        RsaPssDataBlockPolicyErrorKind::LengthOverflow
    );

    let too_small = RsaPssDataBlockPolicy::new(32, 32).unwrap_err();
    assert_eq!(
        too_small.kind(),
        RsaPssDataBlockPolicyErrorKind::MaximumDataBlockTooSmall
    );
}

#[test]
fn rejects_input_over_ceiling_before_structure_checks() {
    let policy = RsaPssDataBlockPolicy::new(2, 4).unwrap();
    let error = validate_rsa_pss_data_block(&[9, 9, 9, 9, 9], policy).unwrap_err();
    assert_eq!(
        error.kind(),
        RsaPssDataBlockErrorKind::DataBlockTooLongForPolicy
    );
}

#[test]
fn rejects_input_too_short_for_selected_salt() {
    let policy = RsaPssDataBlockPolicy::new(4, 8).unwrap();
    for block in [&[][..], &[1][..], &[0, 1, 2, 3][..]] {
        let error = validate_rsa_pss_data_block(block, policy).unwrap_err();
        assert_eq!(
            error.kind(),
            RsaPssDataBlockErrorKind::DataBlockTooShortForSalt
        );
    }
}

#[test]
fn rejects_nonzero_padding_at_each_position() {
    let policy = RsaPssDataBlockPolicy::new(2, 8).unwrap();
    for index in 0..5 {
        let mut block = canonical_data_block(5, &[0xaa, 0xbb]);
        block[index] = 1;
        let error = validate_rsa_pss_data_block(&block, policy).unwrap_err();
        assert_eq!(error.kind(), RsaPssDataBlockErrorKind::NonzeroPaddingString);
    }
}

#[test]
fn rejects_missing_or_wrong_delimiter() {
    let policy = RsaPssDataBlockPolicy::new(2, 8).unwrap();
    for delimiter in [0, 2, 0xff] {
        let mut block = canonical_data_block(5, &[0xaa, 0xbb]);
        block[5] = delimiter;
        let error = validate_rsa_pss_data_block(&block, policy).unwrap_err();
        assert_eq!(error.kind(), RsaPssDataBlockErrorKind::InvalidDelimiter);
    }
}

#[test]
fn errors_are_stable_and_redacted() {
    let policy_error = RsaPssDataBlockPolicy::new(125, 16).unwrap_err();
    assert_eq!(
        format!("{policy_error:?}"),
        "RsaPssDataBlockPolicyError { kind: MaximumDataBlockTooSmall }"
    );
    assert_eq!(
        policy_error.to_string(),
        "RSA-PSS data-block policy error MaximumDataBlockTooSmall"
    );

    let policy = RsaPssDataBlockPolicy::new(2, 4).unwrap();
    let error = validate_rsa_pss_data_block(&[0x7d, 1, 2, 3], policy).unwrap_err();
    assert_eq!(
        format!("{error:?}"),
        "RsaPssDataBlockError { kind: NonzeroPaddingString }"
    );
    assert_eq!(
        error.to_string(),
        "RSA-PSS data-block error NonzeroPaddingString"
    );
    for rendered in [
        format!("{policy_error:?}"),
        policy_error.to_string(),
        format!("{error:?}"),
        error.to_string(),
    ] {
        assert!(!rendered.contains("125"));
        assert!(!rendered.contains("0x7d"));
    }
}
