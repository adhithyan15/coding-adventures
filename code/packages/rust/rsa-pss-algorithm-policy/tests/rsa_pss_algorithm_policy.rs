use rsa_pss_algorithm_policy::{
    validate_rsa_pss_algorithm_parameters, RsaPssAlgorithmParameters, RsaPssAlgorithmPolicy,
    RsaPssAlgorithmPolicyErrorKind, RsaPssAlgorithmProfile, RsaPssAlgorithmValidationErrorKind,
    RsaPssDigestAlgorithm, RSA_PSS_TRAILER_FIELD_BC,
};

const SHA256_32: RsaPssAlgorithmProfile = RsaPssAlgorithmProfile::new(
    RsaPssDigestAlgorithm::Sha256,
    RsaPssDigestAlgorithm::Sha256,
    32,
);
const SHA384_48: RsaPssAlgorithmProfile = RsaPssAlgorithmProfile::new(
    RsaPssDigestAlgorithm::Sha384,
    RsaPssDigestAlgorithm::Sha384,
    48,
);
const SHA512_64: RsaPssAlgorithmProfile = RsaPssAlgorithmProfile::new(
    RsaPssDigestAlgorithm::Sha512,
    RsaPssDigestAlgorithm::Sha512,
    64,
);

#[test]
fn validates_each_allowlisted_digest_profile() {
    let profiles = [SHA256_32, SHA384_48, SHA512_64];
    let policy = RsaPssAlgorithmPolicy::new(&profiles, 64).unwrap();
    for profile in profiles {
        let parameters = RsaPssAlgorithmParameters::new(
            profile.message_hash(),
            profile.mgf1_hash(),
            profile.salt_length(),
            RSA_PSS_TRAILER_FIELD_BC,
        );
        assert_eq!(parameters.profile(), profile);
        assert_eq!(parameters.trailer_field(), 1);
        let validated = validate_rsa_pss_algorithm_parameters(parameters, policy).unwrap();
        assert_eq!(validated.profile(), profile);
        assert_eq!(validated.message_hash(), profile.message_hash());
        assert_eq!(validated.mgf1_hash(), profile.mgf1_hash());
        assert_eq!(validated.salt_length(), profile.salt_length());
        assert_eq!(format!("{validated:?}"), "ValidatedRsaPssAlgorithmProfile");
    }
}

#[test]
fn reports_digest_output_lengths() {
    assert_eq!(RsaPssDigestAlgorithm::Sha256.output_length(), 32);
    assert_eq!(RsaPssDigestAlgorithm::Sha384.output_length(), 48);
    assert_eq!(RsaPssDigestAlgorithm::Sha512.output_length(), 64);
}

#[test]
fn policy_exposes_complete_public_configuration() {
    let profiles = [SHA256_32, SHA384_48];
    let policy = RsaPssAlgorithmPolicy::new(&profiles, 96).unwrap();
    assert_eq!(policy.allowed_profiles(), profiles);
    assert_eq!(policy.maximum_salt_bytes(), 96);
}

#[test]
fn policy_rejects_empty_allowlist() {
    let error = RsaPssAlgorithmPolicy::new(&[], 64).unwrap_err();
    assert_eq!(
        error.kind(),
        RsaPssAlgorithmPolicyErrorKind::EmptyAllowedProfiles
    );
}

#[test]
fn policy_rejects_profile_over_salt_ceiling() {
    let error = RsaPssAlgorithmPolicy::new(&[SHA512_64], 63).unwrap_err();
    assert_eq!(
        error.kind(),
        RsaPssAlgorithmPolicyErrorKind::AllowedProfileSaltLengthExceedsMaximum
    );
}

#[test]
fn policy_rejects_duplicate_profile() {
    let error = RsaPssAlgorithmPolicy::new(&[SHA256_32, SHA256_32], 64).unwrap_err();
    assert_eq!(
        error.kind(),
        RsaPssAlgorithmPolicyErrorKind::DuplicateAllowedProfile
    );
}

#[test]
fn supports_explicit_cross_hash_and_zero_salt_profiles() {
    let cross_hash = RsaPssAlgorithmProfile::new(
        RsaPssDigestAlgorithm::Sha256,
        RsaPssDigestAlgorithm::Sha512,
        0,
    );
    let profiles = [cross_hash];
    let policy = RsaPssAlgorithmPolicy::new(&profiles, 0).unwrap();
    let parameters = RsaPssAlgorithmParameters::new(
        RsaPssDigestAlgorithm::Sha256,
        RsaPssDigestAlgorithm::Sha512,
        0,
        1,
    );
    let validated = validate_rsa_pss_algorithm_parameters(parameters, policy).unwrap();
    assert_eq!(validated.profile(), cross_hash);
}

#[test]
fn rejects_in_deterministic_trailer_ceiling_profile_order() {
    let policy = RsaPssAlgorithmPolicy::new(&[SHA256_32], 32).unwrap();

    let wrong_trailer = RsaPssAlgorithmParameters::new(
        RsaPssDigestAlgorithm::Sha512,
        RsaPssDigestAlgorithm::Sha384,
        64,
        2,
    );
    assert_eq!(
        validate_rsa_pss_algorithm_parameters(wrong_trailer, policy)
            .unwrap_err()
            .kind(),
        RsaPssAlgorithmValidationErrorKind::UnsupportedTrailerField
    );

    let excessive_salt = RsaPssAlgorithmParameters::new(
        RsaPssDigestAlgorithm::Sha512,
        RsaPssDigestAlgorithm::Sha384,
        64,
        1,
    );
    assert_eq!(
        validate_rsa_pss_algorithm_parameters(excessive_salt, policy)
            .unwrap_err()
            .kind(),
        RsaPssAlgorithmValidationErrorKind::SaltLengthExceedsMaximum
    );

    let unlisted = RsaPssAlgorithmParameters::new(
        RsaPssDigestAlgorithm::Sha256,
        RsaPssDigestAlgorithm::Sha384,
        32,
        1,
    );
    assert_eq!(
        validate_rsa_pss_algorithm_parameters(unlisted, policy)
            .unwrap_err()
            .kind(),
        RsaPssAlgorithmValidationErrorKind::ProfileNotAllowed
    );
}

#[test]
fn errors_are_stable_and_redacted() {
    let policy_error = RsaPssAlgorithmPolicy::new(&[SHA512_64], 63).unwrap_err();
    assert_eq!(
        format!("{policy_error:?}"),
        "RsaPssAlgorithmPolicyError { kind: AllowedProfileSaltLengthExceedsMaximum }"
    );
    assert_eq!(
        policy_error.to_string(),
        "RSA-PSS algorithm policy error AllowedProfileSaltLengthExceedsMaximum"
    );

    let policy = RsaPssAlgorithmPolicy::new(&[SHA256_32], 64).unwrap();
    let parameters = RsaPssAlgorithmParameters::new(
        RsaPssDigestAlgorithm::Sha384,
        RsaPssDigestAlgorithm::Sha384,
        48,
        1,
    );
    let error = validate_rsa_pss_algorithm_parameters(parameters, policy).unwrap_err();
    assert_eq!(
        format!("{error:?}"),
        "RsaPssAlgorithmValidationError { kind: ProfileNotAllowed }"
    );
    assert_eq!(
        error.to_string(),
        "RSA-PSS algorithm validation error ProfileNotAllowed"
    );
    for rendered in [
        format!("{policy_error:?}"),
        policy_error.to_string(),
        format!("{error:?}"),
        error.to_string(),
    ] {
        assert!(!rendered.contains("63"));
        assert!(!rendered.contains("48"));
    }
}
