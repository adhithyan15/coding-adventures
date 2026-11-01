use coding_adventures_hkdf::{FixedHkdfHash, HkdfPrk};
use tls13_cipher_suite_policy::{
    CipherSuite, CipherSuitePolicy, CipherSuitePolicyEntry, SelectedCipherSuite,
};
use tls13_finished::{
    derive_finished_key, FinishedError, FinishedKey, FinishedKeyStorage, FinishedRole,
};

fn decode_hex<const N: usize>(value: &str) -> [u8; N] {
    assert_eq!(value.len(), N * 2);
    let mut decoded = [0; N];
    for (index, byte) in decoded.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16).unwrap();
    }
    decoded
}

fn selected(suite: CipherSuite) -> SelectedCipherSuite {
    let entry = CipherSuitePolicyEntry::new(suite, 1024).unwrap();
    CipherSuitePolicy::new(&[entry])
        .unwrap()
        .select(suite.wire_id())
        .unwrap()
}

fn with_sha256_finished_key<Result>(operation: impl FnOnce(FinishedKey<'_>) -> Result) -> Result {
    let base_key =
        decode_hex::<32>("b67b7d690cc16c4e75e54213cb2d37b4e9c912bcded9105d42befd59d391ad38");
    let prk = HkdfPrk::import_exact(FixedHkdfHash::Sha256, &base_key).unwrap();
    let mut storage = FinishedKeyStorage::new(
        selected(CipherSuite::TlsAes128GcmSha256),
        FinishedRole::Server,
    );
    operation(derive_finished_key(&prk, &mut storage).unwrap())
}

#[test]
fn matches_rfc8448_finished_key_and_independent_sha256_verify_data() {
    // RFC 8448 section 3 supplies the server handshake traffic secret and the
    // derived finished key 008d3b66...19fc8. The expected verify-data below
    // was independently generated with Python's hashlib/hmac over 00..1f.
    let transcript_hash = core::array::from_fn::<_, 32, _>(|index| index as u8);
    let expected =
        decode_hex::<32>("b99ed8cc98ac1ef9207cef835e06391aad261c9c876c885640f0070d601dd410");
    with_sha256_finished_key(|key| {
        let mut output = [0; 32];
        key.write_verify_data(&transcript_hash, &mut output)
            .unwrap();
        assert_eq!(output, expected);
    });
}

#[test]
fn matches_independent_sha384_finished_vector() {
    let base_key = core::array::from_fn::<_, 48, _>(|index| index as u8);
    let transcript_hash = core::array::from_fn::<_, 48, _>(|index| 0xa0 + index as u8);
    let expected = decode_hex::<48>(
        "158131dd54a2bfa4d2349389dd00440fc8a75fd59001633b7aa719130ee258b99673b7191d8f8340b74cf7720f1f456f",
    );
    let prk = HkdfPrk::import_exact(FixedHkdfHash::Sha384, &base_key).unwrap();
    let mut storage = FinishedKeyStorage::new(
        selected(CipherSuite::TlsAes256GcmSha384),
        FinishedRole::Server,
    );
    let key = derive_finished_key(&prk, &mut storage).unwrap();
    let mut output = [0; 48];

    key.write_verify_data(&transcript_hash, &mut output)
        .unwrap();

    assert_eq!(output, expected);
}

#[test]
fn chacha_suite_uses_the_same_sha256_finished_policy() {
    let base_key =
        decode_hex::<32>("b67b7d690cc16c4e75e54213cb2d37b4e9c912bcded9105d42befd59d391ad38");
    let prk = HkdfPrk::import_exact(FixedHkdfHash::Sha256, &base_key).unwrap();
    let mut storage = FinishedKeyStorage::new(
        selected(CipherSuite::TlsChacha20Poly1305Sha256),
        FinishedRole::Client,
    );
    let key = derive_finished_key(&prk, &mut storage).unwrap();
    let transcript_hash = [0x42; 32];
    let mut output = [0; 32];

    key.write_verify_data(&transcript_hash, &mut output)
        .unwrap();

    assert_ne!(output, [0; 32]);
    assert_eq!(
        key.selected_suite().parameters().suite(),
        CipherSuite::TlsChacha20Poly1305Sha256
    );
    assert_eq!(key.role(), FinishedRole::Client);
}

#[test]
fn peer_verification_returns_exact_transcript_bound_evidence() {
    with_sha256_finished_key(|key| {
        let transcript_hash = [0x24; 32];
        let mut verify_data = [0; 32];
        key.write_verify_data(&transcript_hash, &mut verify_data)
            .unwrap();

        let verified = key.verify_peer(&transcript_hash, &verify_data).unwrap();

        assert_eq!(verified.selected_suite(), key.selected_suite());
        assert_eq!(verified.role(), FinishedRole::Server);
        assert_eq!(verified.transcript_hash(), transcript_hash);
        assert!(!format!("{verified:?}").contains("242424"));
    });
}

#[test]
fn peer_verification_rejects_a_difference_at_every_position() {
    with_sha256_finished_key(|key| {
        let transcript_hash = [0x55; 32];
        let mut valid = [0; 32];
        key.write_verify_data(&transcript_hash, &mut valid).unwrap();

        for index in 0..valid.len() {
            let mut invalid = valid;
            invalid[index] ^= 1;
            assert_eq!(
                key.verify_peer(&transcript_hash, &invalid).unwrap_err(),
                FinishedError::VerifyDataMismatch,
                "difference at index {index}"
            );
        }
    });
}

#[test]
fn public_length_failures_leave_generation_output_unchanged() {
    with_sha256_finished_key(|key| {
        let mut output = [0xa5; 32];
        let before = output;

        assert_eq!(
            key.write_verify_data(&[0; 31], &mut output).unwrap_err(),
            FinishedError::TranscriptHashLengthMismatch
        );
        assert_eq!(output, before);
        assert_eq!(
            key.write_verify_data(&[0; 32], &mut output[..31])
                .unwrap_err(),
            FinishedError::OutputLengthMismatch
        );
        assert_eq!(output, before);
        assert_eq!(
            key.verify_peer(&[0; 32], &[0; 31]).unwrap_err(),
            FinishedError::PeerVerifyDataLengthMismatch
        );
    });
}

#[test]
fn suite_and_base_key_hash_must_match() {
    let sha256 = HkdfPrk::import_exact(FixedHkdfHash::Sha256, &[0x11; 32]).unwrap();
    let sha384 = HkdfPrk::import_exact(FixedHkdfHash::Sha384, &[0x22; 48]).unwrap();

    assert_eq!(
        derive_finished_key(
            &sha256,
            &mut FinishedKeyStorage::new(
                selected(CipherSuite::TlsAes256GcmSha384),
                FinishedRole::Server,
            ),
        )
        .unwrap_err(),
        FinishedError::HashMismatch
    );
    assert_eq!(
        derive_finished_key(
            &sha384,
            &mut FinishedKeyStorage::new(
                selected(CipherSuite::TlsAes128GcmSha256),
                FinishedRole::Client,
            ),
        )
        .unwrap_err(),
        FinishedError::HashMismatch
    );
}

#[test]
fn debug_and_errors_are_redacted_stable_categories() {
    with_sha256_finished_key(|key| {
        let debug = format!("{key:?}");
        assert!(debug.contains("REDACTED"));
        assert!(!debug.contains("008d3b66"));
    });
    assert_eq!(
        FinishedError::VerifyDataMismatch.to_string(),
        "TLS 1.3 peer Finished verification failed"
    );
    assert_eq!(
        FinishedError::TranscriptHashLengthMismatch.to_string(),
        "TLS 1.3 Finished transcript-hash length mismatch"
    );
}
