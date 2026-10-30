use rsa_pss_hash_match::{
    validate_rsa_pss_hash_match, RsaPssHashMatchErrorKind, RsaPssHashMatchPolicy,
    RsaPssHashMatchPolicyErrorKind,
};

#[test]
fn validates_exact_sha256_hashes_and_borrows() {
    let computed: Vec<u8> = (0..32).collect();
    let encoded = computed.clone();
    let policy = RsaPssHashMatchPolicy::new(32, 64).unwrap();

    let matched = validate_rsa_pss_hash_match(&computed, &encoded, policy).unwrap();
    assert_eq!(matched.computed_hash(), computed);
    assert_eq!(matched.encoded_hash(), encoded);
    assert_eq!(matched.hash_length(), 32);
    assert_eq!(format!("{matched:?}"), "ValidatedRsaPssHashMatch");
}

#[test]
fn supports_data_driven_digest_lengths() {
    for hash_length in [1, 28, 32, 48, 64] {
        let hash = vec![0x5a; hash_length];
        let policy = RsaPssHashMatchPolicy::new(hash_length, 128).unwrap();
        let matched = validate_rsa_pss_hash_match(&hash, &hash, policy).unwrap();
        assert_eq!(matched.hash_length(), hash_length);
    }
}

#[test]
fn policy_reports_public_bounds() {
    let policy = RsaPssHashMatchPolicy::new(48, 128).unwrap();
    assert_eq!(policy.hash_length(), 48);
    assert_eq!(policy.maximum_hash_bytes(), 128);
}

#[test]
fn policy_rejects_zero_values_and_hash_over_ceiling() {
    let zero_hash = RsaPssHashMatchPolicy::new(0, 1).unwrap_err();
    assert_eq!(
        zero_hash.kind(),
        RsaPssHashMatchPolicyErrorKind::ZeroHashLength
    );

    let zero_maximum = RsaPssHashMatchPolicy::new(1, 0).unwrap_err();
    assert_eq!(
        zero_maximum.kind(),
        RsaPssHashMatchPolicyErrorKind::ZeroMaximumHashBytes
    );

    let too_long = RsaPssHashMatchPolicy::new(65, 64).unwrap_err();
    assert_eq!(
        too_long.kind(),
        RsaPssHashMatchPolicyErrorKind::HashLengthExceedsMaximum
    );
}

#[test]
fn rejects_computed_length_before_encoded_length() {
    let policy = RsaPssHashMatchPolicy::new(4, 8).unwrap();
    let error = validate_rsa_pss_hash_match(&[1, 2, 3], &[1, 2, 3], policy).unwrap_err();
    assert_eq!(
        error.kind(),
        RsaPssHashMatchErrorKind::ComputedHashLengthMismatch
    );
}

#[test]
fn rejects_encoded_length_after_computed_length_passes() {
    let policy = RsaPssHashMatchPolicy::new(4, 8).unwrap();
    for encoded in [&[][..], &[1, 2, 3][..], &[1, 2, 3, 4, 5][..]] {
        let error = validate_rsa_pss_hash_match(&[1, 2, 3, 4], encoded, policy).unwrap_err();
        assert_eq!(
            error.kind(),
            RsaPssHashMatchErrorKind::EncodedHashLengthMismatch
        );
    }
}

#[test]
fn rejects_mismatch_at_every_position_without_reporting_position() {
    let policy = RsaPssHashMatchPolicy::new(4, 8).unwrap();
    for index in 0..4 {
        let computed = [1, 2, 3, 4];
        let mut encoded = computed;
        encoded[index] ^= 0xff;
        let error = validate_rsa_pss_hash_match(&computed, &encoded, policy).unwrap_err();
        assert_eq!(error.kind(), RsaPssHashMatchErrorKind::HashMismatch);
    }
}

#[test]
fn errors_are_stable_and_redacted() {
    let policy_error = RsaPssHashMatchPolicy::new(125, 64).unwrap_err();
    assert_eq!(
        format!("{policy_error:?}"),
        "RsaPssHashMatchPolicyError { kind: HashLengthExceedsMaximum }"
    );
    assert_eq!(
        policy_error.to_string(),
        "RSA-PSS hash-match policy error HashLengthExceedsMaximum"
    );

    let policy = RsaPssHashMatchPolicy::new(4, 8).unwrap();
    let error = validate_rsa_pss_hash_match(&[0x7d; 4], &[0x7e; 4], policy).unwrap_err();
    assert_eq!(
        format!("{error:?}"),
        "RsaPssHashMatchError { kind: HashMismatch }"
    );
    assert_eq!(error.to_string(), "RSA-PSS hash-match error HashMismatch");
    for rendered in [
        format!("{policy_error:?}"),
        policy_error.to_string(),
        format!("{error:?}"),
        error.to_string(),
    ] {
        assert!(!rendered.contains("125"));
        assert!(!rendered.contains("0x7d"));
        assert!(!rendered.contains("0x7e"));
    }
}
