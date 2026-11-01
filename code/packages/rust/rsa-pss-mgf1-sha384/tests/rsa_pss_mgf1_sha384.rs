use rsa_pss_mgf1_sha384::{
    generate_mgf1_sha384_mask, Mgf1Sha384ErrorKind, Mgf1Sha384Limits, Mgf1Sha384LimitsErrorKind,
    MAXIMUM_MGF1_MASK_LENGTH, MGF1_COUNTER_LENGTH, SHA384_DIGEST_LENGTH,
};

const SEED: [u8; SHA384_DIGEST_LENGTH] = [
    0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f,
    0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f,
    0x20, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x28, 0x29, 0x2a, 0x2b, 0x2c, 0x2d, 0x2e, 0x2f,
];

// Independent RFC 8017 evaluation for seed 00..2f and maskLen 143.
const EXPECTED_MASK: [u8; 143] = [
    0xe4, 0x5a, 0xc6, 0xc0, 0xca, 0xff, 0xf3, 0x43, 0xb2, 0x68, 0xd4, 0xcb, 0xd7, 0x73, 0x32, 0x84,
    0x13, 0x67, 0x2a, 0x76, 0x4d, 0xf9, 0x9a, 0xb8, 0x23, 0xb5, 0x30, 0x74, 0xd9, 0x41, 0x52, 0xbd,
    0x27, 0xfc, 0x38, 0xbc, 0xff, 0xdb, 0x7c, 0x1d, 0xc1, 0xb6, 0xa3, 0x65, 0x6b, 0x2d, 0x48, 0x19,
    0x35, 0x2c, 0x48, 0x2d, 0xa4, 0x0a, 0xad, 0x3b, 0x37, 0xf3, 0x33, 0xc7, 0xaf, 0xa8, 0x1a, 0x92,
    0xb7, 0xb5, 0x45, 0x51, 0xf3, 0x00, 0x9e, 0xfa, 0x4b, 0xdb, 0x89, 0x37, 0x49, 0x2c, 0x5a, 0xfc,
    0xa1, 0xb1, 0x41, 0xc9, 0x91, 0x59, 0xb4, 0xf0, 0xf8, 0x19, 0x97, 0x7a, 0x4e, 0x10, 0xeb, 0x51,
    0x61, 0xed, 0xd4, 0xb1, 0x73, 0x47, 0x17, 0xde, 0x41, 0x06, 0xf9, 0xc1, 0x84, 0xa1, 0x7a, 0x9b,
    0x5e, 0xe6, 0x1a, 0x43, 0x99, 0xdd, 0x75, 0x5f, 0x32, 0x2f, 0x5d, 0x70, 0x7a, 0x58, 0x1c, 0xc1,
    0x40, 0x3b, 0x41, 0x14, 0x30, 0x4b, 0x97, 0xf6, 0xf7, 0x94, 0xca, 0xca, 0xa4, 0xcf, 0xcc,
];

#[test]
fn expands_multiple_blocks_and_a_partial_final_block() {
    let limits = Mgf1Sha384Limits::new(EXPECTED_MASK.len()).unwrap();
    let mut output = [0u8; EXPECTED_MASK.len()];
    generate_mgf1_sha384_mask(&SEED, &mut output, limits).unwrap();
    assert_eq!(output, EXPECTED_MASK);
}

#[test]
fn every_prefix_matches_the_complete_expansion() {
    let limits = Mgf1Sha384Limits::new(EXPECTED_MASK.len()).unwrap();
    for length in [1, 47, 48, 49, 96, 143] {
        let mut output = vec![0u8; length];
        generate_mgf1_sha384_mask(&SEED, &mut output, limits).unwrap();
        assert_eq!(output, EXPECTED_MASK[..length]);
    }
}

#[test]
fn empty_output_is_valid_under_a_nonzero_policy() {
    let limits = Mgf1Sha384Limits::new(1).unwrap();
    let mut output = [];
    generate_mgf1_sha384_mask(&SEED, &mut output, limits).unwrap();
    assert!(output.is_empty());
}

#[test]
fn policy_failure_leaves_the_complete_output_unchanged() {
    let limits = Mgf1Sha384Limits::new(48).unwrap();
    let mut output = [0xa5; 49];
    let error = generate_mgf1_sha384_mask(&SEED, &mut output, limits).unwrap_err();
    assert_eq!(error.kind(), Mgf1Sha384ErrorKind::MaskTooLongForPolicy);
    assert_eq!(output, [0xa5; 49]);
}

#[test]
fn limit_policy_rejects_zero_and_reports_its_bound() {
    let error = Mgf1Sha384Limits::new(0).unwrap_err();
    assert_eq!(
        error.kind(),
        Mgf1Sha384LimitsErrorKind::ZeroMaximumMaskBytes
    );
    let limits = Mgf1Sha384Limits::new(4096).unwrap();
    assert_eq!(limits.maximum_mask_bytes(), 4096);
}

#[cfg(target_pointer_width = "64")]
#[test]
fn limit_policy_rejects_lengths_beyond_the_rfc_domain() {
    let too_long = usize::try_from(MAXIMUM_MGF1_MASK_LENGTH + 1).unwrap();
    let error = Mgf1Sha384Limits::new(too_long).unwrap_err();
    assert_eq!(error.kind(), Mgf1Sha384LimitsErrorKind::MaximumMaskTooLong);
}

#[test]
fn public_constants_match_rfc_8017_sha384_mgf1() {
    assert_eq!(SHA384_DIGEST_LENGTH, 48);
    assert_eq!(MGF1_COUNTER_LENGTH, 4);
    assert_eq!(MAXIMUM_MGF1_MASK_LENGTH, 206_158_430_208);
}

#[test]
fn errors_are_stable_and_redacted() {
    let limits_error = Mgf1Sha384Limits::new(0).unwrap_err();
    assert_eq!(
        format!("{limits_error:?}"),
        "Mgf1Sha384LimitsError { kind: ZeroMaximumMaskBytes }"
    );
    assert_eq!(
        limits_error.to_string(),
        "SHA-384 MGF1 limits error ZeroMaximumMaskBytes"
    );

    let limits = Mgf1Sha384Limits::new(1).unwrap();
    let mut output = [0x7d; 2];
    let error = generate_mgf1_sha384_mask(&SEED, &mut output, limits).unwrap_err();
    assert_eq!(
        format!("{error:?}"),
        "Mgf1Sha384Error { kind: MaskTooLongForPolicy }"
    );
    assert_eq!(error.to_string(), "SHA-384 MGF1 error MaskTooLongForPolicy");
    assert!(!format!("{error:?}").contains("125"));
    assert!(!error.to_string().contains("125"));
}
