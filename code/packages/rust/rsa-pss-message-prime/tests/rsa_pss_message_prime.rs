use rsa_pss_message_prime::{
    encode_rsa_pss_message_prime, RsaPssMessagePrimeErrorKind, RsaPssMessagePrimePolicy,
    RsaPssMessagePrimePolicyErrorKind, RSA_PSS_MESSAGE_PRIME_PREFIX_BYTES,
};

#[test]
fn encodes_typical_sha256_profile_and_exact_borrows() {
    let message_hash: Vec<u8> = (0..32).collect();
    let salt: Vec<u8> = (32..64).collect();
    let policy = RsaPssMessagePrimePolicy::new(32, 32, 72).unwrap();
    let mut output = [0xff; 72];

    let encoded = encode_rsa_pss_message_prime(&message_hash, &salt, &mut output, policy).unwrap();
    assert_eq!(&encoded.bytes()[..8], &[0; 8]);
    assert_eq!(encoded.message_hash(), message_hash);
    assert_eq!(encoded.salt(), salt);
    assert_eq!(encoded.bytes().len(), 72);
    assert_eq!(format!("{encoded:?}"), "EncodedRsaPssMessagePrime");
}

#[test]
fn supports_zero_length_salt() {
    let policy = RsaPssMessagePrimePolicy::new(64, 0, 72).unwrap();
    let mut output = [0xff; 72];
    let encoded = encode_rsa_pss_message_prime(&[0x5a; 64], &[], &mut output, policy).unwrap();
    assert_eq!(&encoded.bytes()[..8], &[0; 8]);
    assert_eq!(encoded.message_hash(), &[0x5a; 64]);
    assert!(encoded.salt().is_empty());
}

#[test]
fn supports_data_driven_hash_and_salt_lengths() {
    for (hash_length, salt_length) in [(1, 0), (28, 8), (32, 32), (48, 20), (64, 64)] {
        let length = RSA_PSS_MESSAGE_PRIME_PREFIX_BYTES + hash_length + salt_length;
        let policy = RsaPssMessagePrimePolicy::new(hash_length, salt_length, length).unwrap();
        let message_hash = vec![0x11; hash_length];
        let salt = vec![0x22; salt_length];
        let mut output = vec![0xff; length];
        let encoded =
            encode_rsa_pss_message_prime(&message_hash, &salt, &mut output, policy).unwrap();
        assert_eq!(encoded.message_hash(), message_hash);
        assert_eq!(encoded.salt(), salt);
    }
}

#[test]
fn policy_reports_public_bounds() {
    let policy = RsaPssMessagePrimePolicy::new(48, 24, 128).unwrap();
    assert_eq!(policy.message_hash_length(), 48);
    assert_eq!(policy.salt_length(), 24);
    assert_eq!(policy.maximum_message_prime_bytes(), 128);
    assert_eq!(policy.message_prime_length(), 80);
}

#[test]
fn policy_rejects_zero_hash_and_zero_ceiling() {
    let zero_hash = RsaPssMessagePrimePolicy::new(0, 0, 8).unwrap_err();
    assert_eq!(
        zero_hash.kind(),
        RsaPssMessagePrimePolicyErrorKind::ZeroMessageHashLength
    );

    let zero_ceiling = RsaPssMessagePrimePolicy::new(1, 0, 0).unwrap_err();
    assert_eq!(
        zero_ceiling.kind(),
        RsaPssMessagePrimePolicyErrorKind::ZeroMaximumMessagePrimeBytes
    );
}

#[test]
fn policy_rejects_each_overflow_and_an_unusable_ceiling() {
    for (hash_length, salt_length) in [(usize::MAX, 0), (1, usize::MAX)] {
        let error =
            RsaPssMessagePrimePolicy::new(hash_length, salt_length, usize::MAX).unwrap_err();
        assert_eq!(
            error.kind(),
            RsaPssMessagePrimePolicyErrorKind::LengthOverflow
        );
    }

    let too_small = RsaPssMessagePrimePolicy::new(32, 32, 71).unwrap_err();
    assert_eq!(
        too_small.kind(),
        RsaPssMessagePrimePolicyErrorKind::MaximumMessagePrimeTooSmall
    );
}

#[test]
fn rejects_message_hash_length_before_modifying_output() {
    let policy = RsaPssMessagePrimePolicy::new(4, 2, 14).unwrap();
    let mut output = [0xa5; 14];
    let error = encode_rsa_pss_message_prime(&[1, 2, 3], &[4, 5], &mut output, policy).unwrap_err();
    assert_eq!(
        error.kind(),
        RsaPssMessagePrimeErrorKind::MessageHashLengthMismatch
    );
    assert_eq!(output, [0xa5; 14]);
}

#[test]
fn rejects_salt_length_before_modifying_output() {
    let policy = RsaPssMessagePrimePolicy::new(4, 2, 14).unwrap();
    let mut output = [0xa5; 14];
    let error = encode_rsa_pss_message_prime(&[1, 2, 3, 4], &[5], &mut output, policy).unwrap_err();
    assert_eq!(
        error.kind(),
        RsaPssMessagePrimeErrorKind::SaltLengthMismatch
    );
    assert_eq!(output, [0xa5; 14]);
}

#[test]
fn rejects_short_and_long_output_before_modifying_it() {
    let policy = RsaPssMessagePrimePolicy::new(4, 2, 15).unwrap();
    for output_length in [13, 15] {
        let mut output = vec![0xa5; output_length];
        let error =
            encode_rsa_pss_message_prime(&[1, 2, 3, 4], &[5, 6], &mut output, policy).unwrap_err();
        assert_eq!(
            error.kind(),
            RsaPssMessagePrimeErrorKind::OutputLengthMismatch
        );
        assert!(output.iter().all(|octet| *octet == 0xa5));
    }
}

#[test]
fn errors_are_stable_and_redacted() {
    let policy_error = RsaPssMessagePrimePolicy::new(125, 16, 32).unwrap_err();
    assert_eq!(
        format!("{policy_error:?}"),
        "RsaPssMessagePrimePolicyError { kind: MaximumMessagePrimeTooSmall }"
    );
    assert_eq!(
        policy_error.to_string(),
        "RSA-PSS M-prime policy error MaximumMessagePrimeTooSmall"
    );

    let policy = RsaPssMessagePrimePolicy::new(4, 2, 14).unwrap();
    let mut output = [0xa5; 14];
    let error = encode_rsa_pss_message_prime(&[0x7d; 3], &[1, 2], &mut output, policy).unwrap_err();
    assert_eq!(
        format!("{error:?}"),
        "RsaPssMessagePrimeError { kind: MessageHashLengthMismatch }"
    );
    assert_eq!(
        error.to_string(),
        "RSA-PSS M-prime error MessageHashLengthMismatch"
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
