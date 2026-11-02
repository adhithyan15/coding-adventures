use rsa_pss_mgf1_sha256::{
    generate_mgf1_sha256_mask, Mgf1Sha256ErrorKind, Mgf1Sha256Limits, Mgf1Sha256LimitsErrorKind,
    MAXIMUM_MGF1_MASK_LENGTH, MGF1_COUNTER_LENGTH, SHA256_DIGEST_LENGTH,
};

const SEED: [u8; SHA256_DIGEST_LENGTH] = [
    0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f,
    0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f,
];

// Independent RFC 8017 MGF1 evaluation for seed 00..1f and maskLen 95.
const EXPECTED_MASK: [u8; 95] = [
    0x70, 0xf4, 0x00, 0x3d, 0x52, 0xb6, 0xeb, 0x03, 0xda, 0x85, 0x2e, 0x93, 0x25, 0x6b, 0x59, 0x86,
    0xb5, 0xd4, 0x88, 0x30, 0x98, 0xbb, 0x79, 0x73, 0xbc, 0x53, 0x18, 0xcc, 0x66, 0x63, 0x7a, 0x84,
    0x04, 0xa6, 0x95, 0x0a, 0x06, 0xd3, 0xe3, 0x30, 0x8a, 0xd7, 0xd3, 0x60, 0x6e, 0xf8, 0x10, 0xeb,
    0x12, 0x4e, 0x39, 0x43, 0x40, 0x4c, 0xa7, 0x46, 0xa1, 0x2c, 0x51, 0xc7, 0xbf, 0x77, 0x68, 0x39,
    0x0f, 0x8d, 0x84, 0x2a, 0xc9, 0xcb, 0x62, 0x34, 0x97, 0x79, 0xa7, 0x53, 0x7a, 0x78, 0x32, 0x7d,
    0x54, 0x5a, 0xae, 0xb3, 0x3b, 0x2d, 0x42, 0xc7, 0xd1, 0xdc, 0x36, 0x80, 0xa4, 0xb2, 0x36,
];

#[test]
fn expands_multiple_blocks_and_a_partial_final_block() {
    let limits = Mgf1Sha256Limits::new(EXPECTED_MASK.len()).unwrap();
    let mut output = [0u8; EXPECTED_MASK.len()];

    generate_mgf1_sha256_mask(&SEED, &mut output, limits).unwrap();

    assert_eq!(output, EXPECTED_MASK);
}

#[test]
fn every_prefix_matches_the_complete_expansion() {
    let limits = Mgf1Sha256Limits::new(EXPECTED_MASK.len()).unwrap();
    for length in [1, 31, 32, 33, 64, 95] {
        let mut output = vec![0u8; length];
        generate_mgf1_sha256_mask(&SEED, &mut output, limits).unwrap();
        assert_eq!(output, EXPECTED_MASK[..length]);
    }
}

#[test]
fn empty_output_is_valid_under_a_nonzero_policy() {
    let limits = Mgf1Sha256Limits::new(1).unwrap();
    let mut output = [];
    generate_mgf1_sha256_mask(&SEED, &mut output, limits).unwrap();
    assert!(output.is_empty());
}

#[test]
fn policy_failure_leaves_the_complete_output_unchanged() {
    let limits = Mgf1Sha256Limits::new(32).unwrap();
    let mut output = [0xa5; 33];

    let error = generate_mgf1_sha256_mask(&SEED, &mut output, limits).unwrap_err();

    assert_eq!(error.kind(), Mgf1Sha256ErrorKind::MaskTooLongForPolicy);
    assert_eq!(output, [0xa5; 33]);
}

#[test]
fn limit_policy_rejects_zero_and_reports_its_bound() {
    let error = Mgf1Sha256Limits::new(0).unwrap_err();
    assert_eq!(
        error.kind(),
        Mgf1Sha256LimitsErrorKind::ZeroMaximumMaskBytes
    );

    let limits = Mgf1Sha256Limits::new(4096).unwrap();
    assert_eq!(limits.maximum_mask_bytes(), 4096);
}

#[cfg(target_pointer_width = "64")]
#[test]
fn limit_policy_rejects_lengths_beyond_the_rfc_domain() {
    let too_long = usize::try_from(MAXIMUM_MGF1_MASK_LENGTH + 1).unwrap();
    let error = Mgf1Sha256Limits::new(too_long).unwrap_err();
    assert_eq!(error.kind(), Mgf1Sha256LimitsErrorKind::MaximumMaskTooLong);
}

#[test]
fn public_constants_match_rfc_8017_sha256_mgf1() {
    assert_eq!(SHA256_DIGEST_LENGTH, 32);
    assert_eq!(MGF1_COUNTER_LENGTH, 4);
    assert_eq!(MAXIMUM_MGF1_MASK_LENGTH, 137_438_953_472);
}

#[test]
fn errors_are_stable_and_redacted() {
    let limits_error = Mgf1Sha256Limits::new(0).unwrap_err();
    assert_eq!(
        format!("{limits_error:?}"),
        "Mgf1Sha256LimitsError { kind: ZeroMaximumMaskBytes }"
    );
    assert_eq!(
        limits_error.to_string(),
        "SHA-256 MGF1 limits error ZeroMaximumMaskBytes"
    );

    let limits = Mgf1Sha256Limits::new(1).unwrap();
    let mut output = [0x7d; 2];
    let error = generate_mgf1_sha256_mask(&SEED, &mut output, limits).unwrap_err();
    assert_eq!(
        format!("{error:?}"),
        "Mgf1Sha256Error { kind: MaskTooLongForPolicy }"
    );
    assert_eq!(error.to_string(), "SHA-256 MGF1 error MaskTooLongForPolicy");
    assert!(!format!("{error:?}").contains("125"));
    assert!(!error.to_string().contains("125"));
}
