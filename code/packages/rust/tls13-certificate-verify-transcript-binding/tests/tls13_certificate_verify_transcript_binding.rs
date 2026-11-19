use core::ptr;

use coding_adventures_sha256::sha256;
use coding_adventures_sha384::sha384;
use coding_adventures_sha512::sum512;
use der_asn1::{Asn1Decoder, Asn1Limits};
use rsa_pss_sha256_signature::RsaPssSha256SignatureLimits;
use rsa_pss_sha384_signature::RsaPssSha384SignatureLimits;
use rsa_pss_sha512_signature::RsaPssSha512SignatureLimits;
use tls13_certificate_verify_input::{
    CertificateVerifyRole, InputError, TranscriptHashAlgorithm, MAXIMUM_INPUT_LENGTH,
};
use tls13_certificate_verify_message::CertificateVerifyMessage;
use tls13_certificate_verify_transcript_binding::{
    bind_certificate_verify_transcript, hash_transcript_bound_certificate_verify_input,
    verify_and_advance_owned_certificate_verify_transcript,
    verify_transcript_bound_certificate_verify, CertificateVerifyTranscriptAdvanceErrorKind,
};
use tls13_cipher_suite_policy::{
    CipherSuite, CipherSuitePolicy, CipherSuitePolicyEntry, SelectedCipherSuite,
};
use tls13_handshake::HandshakeErrorKind;
use tls13_rsa_pss_certificate_verify::Tls13RsaPssCertificateVerifyErrorKind;
use tls13_rsa_pss_signature_routing::{
    Tls13RsaPssSignatureLimits, Tls13RsaPssSignatureRoutingErrorKind,
};
use tls13_signature_scheme_policy::{
    AuthorizedSignatureScheme, SignatureScheme, SignatureSchemePolicy,
};
use tls13_transcript::{
    PendingTls13Transcript, SelectedTls13Transcript, TranscriptErrorKind, TranscriptPolicy,
};
use x509_rsa_public_key::{validate_rsa_public_key, RsaPublicKeyPolicy, ValidatedRsaPublicKey};
use x509_subject_public_key_info::decode_x509_subject_public_key_info;

const VERIFY_TRANSCRIPT_BYTES: &[u8] = b"selected transcript prefix through Certificate";

// Generated with OpenSSL 3.6.3 from one 2048-bit RSA key over the exact
// server CertificateVerify input derived from VERIFY_TRANSCRIPT_BYTES, using
// fixed PSS salt lengths 32, 48, and 64.
const PUBLIC_KEY_SPKI_HEX: &str = concat!(
    "30820122300d06092a864886f70d01010105000382010f003082010a0282010100",
    "a9a0b8549be6b8be9edb4261bb68f5cdbbc6f668164288368e65f9a615627de3",
    "534435e2e56f80839333265798aac333815d2f5447af611ee6de16a3e86313ea",
    "2be8a53df0c84a3140350095f052d525b386458a9d15d3edb745a5feca28a1d0",
    "e68b13f48274358e97d94136c2cf39c333f970d309323621a880c96a1b7407f2",
    "a0a8af6cec62e8348b861625e60e0945cd4055c3f9b486f67640a4797c5034d",
    "a14e6c6076ff2e5f19345f94540a8054add245613ab4bbab98ef83e4c3944082",
    "e29c492e3a7535cbb10d45686a2f02c1c1fc08f3c6d93cb9c9efabdc3e0494e",
    "609c807eaaaf5d46e9b2b0b1651c7c8c4a09cec15b5b89a3f76d88010ba8303",
    "ed90203010001",
);
const SHA256_SIGNATURE_HEX: &str = concat!(
    "1daecc6104adf9fdbbc8030b07962ed996bd838fbf21bcc8c54b61922349f02a",
    "22b8ee7ff4830685d98ad22001b6660db8a2228f516dae3478a3c7887ffe8baa",
    "bebf03c0f1baf77af8f1945d2c3634ea0ff5831efc0256cbdb20f1054c691b5d",
    "ff3ab84039265fc095e13d7a7caf6e9ddb38beac7c2c0cc406924f347e39dd90",
    "2d001ad51367942c79edfa88abe8023fde7727fc87579b7899d61f64e67d5685",
    "73437167593f3ff31f0427125e71d3fda83d8d9e72d664d1a80ba7612c8a389",
    "6803024e1763b294df3bc67630d86bd5e89058a019aaac99702ae73b1dc37346",
    "766d4bad7737e4cb49c7a281a03346c18fe6e4e45e937ea72596105aae5972c15",
);
const SHA384_SIGNATURE_HEX: &str = concat!(
    "397925f7b8c867bf9c28e111235cd45b21dd35d577ba0ea8982fd21eac63184e",
    "caed0dbec5c6f29df08f5514046e320aab951b261b87376bd6a07710403309da",
    "f474dc708bbaca469ff0dc55befe9fa27f95e4e1348b3bbba910e06263c720b6",
    "8aba6f052e52e5d867962ff183252c6f0a2af6a38e236cb6328de648f8288834",
    "ff61c18abeaecbbe99519c592e2b981477e64390c9834b5e4391a7a2f7c4d66",
    "b56af6f915eb38f873db94a670a5e9187b3f73a227deee282893261c58bfed3d",
    "c37de6193492b421be36faabd7b9997e6ee60ec22b76b8c3ab80274b5afb340f",
    "0569dd2acdc593501c13165a806e8f3793d70ef591a660936895014328df4b909",
);
const SHA512_SIGNATURE_HEX: &str = concat!(
    "9d82fd696b1f292e5312b9af5548f26988770f5fb3b72e860680375fc501a838",
    "7808a799d72e808d4d4b15d17f6b6a7b5524cd72511648e12844a3a56953a49",
    "faeec0885fa7d8908b601a7a00568f684a81c8451929855ccdd60895cc20eb0b",
    "ab1c33d4bd0441717ef53d0ebb5e8a01b4ae91d57505fdf0fba1c69aa6c9b29",
    "e16c977445cf1a8da4bd9b6acb32e5b5317893fae72eab0bbfa62e98d0a31e3",
    "a4a40c3704c8e57f94262e2f14f9b6c7a8e2d173a5258ced0a360a7fdb10adb",
    "7b65433521e8325a74bf35d4acf5a69256195146decc5f69f779236720950b0d",
    "b01b337902bf840cd283b82c3a85dd35e8d24664a7551d0c251c4f4827ca3fda0822",
);

fn decode_hex(hexadecimal: &str) -> Vec<u8> {
    assert!(hexadecimal.len().is_multiple_of(2));
    hexadecimal
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

fn validated_key(encoded: &[u8]) -> ValidatedRsaPublicKey<'_> {
    let mut decoder = Asn1Decoder::new(Asn1Limits::default());
    let root = decoder.decode_exact(encoded).unwrap();
    let subject_public_key_info = decode_x509_subject_public_key_info(&mut decoder, root).unwrap();
    let policy = RsaPublicKeyPolicy::new(2048, 2048, &[65_537]).unwrap();
    validate_rsa_public_key(subject_public_key_info, policy, Asn1Limits::default()).unwrap()
}

fn selected_suite(suite: CipherSuite) -> SelectedCipherSuite {
    let entry = CipherSuitePolicyEntry::new(suite, 4096).expect("valid suite entry");
    CipherSuitePolicy::new(&[entry])
        .expect("valid suite policy")
        .select(suite.wire_id())
        .expect("suite is allowed")
}

fn selected_transcript(suite: CipherSuite, bytes: &[u8]) -> SelectedTls13Transcript {
    selected_transcript_with_limit(suite, bytes, 4096)
}

fn selected_transcript_with_limit(
    suite: CipherSuite,
    bytes: &[u8],
    maximum_observed_bytes: u64,
) -> SelectedTls13Transcript {
    let mut pending = PendingTls13Transcript::new(
        TranscriptPolicy::new(maximum_observed_bytes).expect("valid transcript policy"),
    );
    pending.update(bytes).expect("test transcript fits");
    pending.select(selected_suite(suite))
}

fn authorized(scheme: SignatureScheme) -> AuthorizedSignatureScheme {
    SignatureSchemePolicy::new(&[scheme])
        .expect("valid signature policy")
        .select(scheme.wire_id())
        .expect("scheme is allowed")
}

fn certificate_verify_body(scheme: SignatureScheme, signature: &[u8]) -> Vec<u8> {
    let mut body = Vec::with_capacity(4 + signature.len());
    body.extend_from_slice(&scheme.wire_id().to_be_bytes());
    body.extend_from_slice(&(signature.len() as u16).to_be_bytes());
    body.extend_from_slice(signature);
    body
}

#[test]
fn selected_suite_snapshot_alone_chooses_framing_hash_metadata() {
    let transcript_bytes = b"exact caller-accepted handshake encodings";
    for (suite, role, algorithm, expected_hash) in [
        (
            CipherSuite::TlsAes128GcmSha256,
            CertificateVerifyRole::Server,
            TranscriptHashAlgorithm::Sha256,
            sha256(transcript_bytes).to_vec(),
        ),
        (
            CipherSuite::TlsAes256GcmSha384,
            CertificateVerifyRole::Client,
            TranscriptHashAlgorithm::Sha384,
            sha384(transcript_bytes).to_vec(),
        ),
    ] {
        let transcript = selected_transcript(suite, transcript_bytes);
        let snapshot = transcript.snapshot();
        let mut output = [0xa5; MAXIMUM_INPUT_LENGTH + 7];
        let input_length = {
            let bound = bind_certificate_verify_transcript(&transcript, role, &mut output)
                .expect("maximum output is sufficient");

            assert!(ptr::eq(bound.transcript(), &transcript));
            assert_eq!(bound.snapshot(), snapshot);
            assert_eq!(bound.input().role(), role);
            assert_eq!(bound.input().transcript_hash_algorithm(), algorithm);

            let input = bound.input().as_bytes();
            assert_eq!(&input[..64], &[0x20; 64]);
            assert_eq!(&input[64..97], role.context());
            assert_eq!(input[97], 0);
            assert_eq!(&input[98..], expected_hash);
            input.len()
        };
        assert!(output[input_length..].iter().all(|byte| *byte == 0xa5));
    }
}

#[test]
fn short_output_fails_before_caller_storage_changes() {
    for suite in [
        CipherSuite::TlsAes128GcmSha256,
        CipherSuite::TlsAes256GcmSha384,
    ] {
        let transcript = selected_transcript(suite, b"bounded transcript");
        let required = match transcript.snapshot().as_bytes().len() {
            32 => TranscriptHashAlgorithm::Sha256.input_length(),
            48 => TranscriptHashAlgorithm::Sha384.input_length(),
            _ => unreachable!("selected TLS transcript hashes are fixed"),
        };
        let mut output = vec![0xa5; required - 1];
        let before = output.clone();

        let error = bind_certificate_verify_transcript(
            &transcript,
            CertificateVerifyRole::Server,
            &mut output,
        )
        .expect_err("one-byte-short output must fail");

        assert_eq!(error, InputError::OutputTooShort);
        assert_eq!(output, before);
    }
}

#[test]
fn authorized_scheme_hashing_retains_the_exact_bound_transcript_input() {
    let transcript = selected_transcript(
        CipherSuite::TlsChacha20Poly1305Sha256,
        b"selected transcript prefix through Certificate",
    );

    for scheme in [
        SignatureScheme::RsaPssRsaeSha256,
        SignatureScheme::RsaPssRsaeSha384,
        SignatureScheme::RsaPssRsaeSha512,
    ] {
        let mut output = [0_u8; MAXIMUM_INPUT_LENGTH];
        let bound = bind_certificate_verify_transcript(
            &transcript,
            CertificateVerifyRole::Server,
            &mut output,
        )
        .expect("maximum output is sufficient");
        let hashed = hash_transcript_bound_certificate_verify_input(authorized(scheme), &bound);

        assert!(ptr::eq(hashed.bound_input(), &bound));
        assert!(ptr::eq(hashed.hashed_input().input(), bound.input()));
        assert_eq!(hashed.hashed_input().authorized_scheme().scheme(), scheme);

        let expected = match scheme {
            SignatureScheme::RsaPssRsaeSha256 => sha256(bound.input().as_bytes()).to_vec(),
            SignatureScheme::RsaPssRsaeSha384 => sha384(bound.input().as_bytes()).to_vec(),
            SignatureScheme::RsaPssRsaeSha512 => sum512(bound.input().as_bytes()).to_vec(),
        };
        assert_eq!(hashed.hashed_input().digest().as_bytes(), expected);
        assert_eq!(
            format!("{hashed:?}"),
            "HashedTranscriptBoundCertificateVerifyInput"
        );
    }
}

#[test]
fn diagnostics_retain_no_transcript_or_framed_input_bytes() {
    let transcript = selected_transcript(
        CipherSuite::TlsAes128GcmSha256,
        b"caller marker 223 should remain absent",
    );
    let mut output = [0_u8; MAXIMUM_INPUT_LENGTH];
    let bound =
        bind_certificate_verify_transcript(&transcript, CertificateVerifyRole::Server, &mut output)
            .expect("maximum output is sufficient");

    let debug = format!("{bound:?}");
    assert_eq!(debug, "TranscriptBoundCertificateVerifyInput");
    assert!(!debug.contains("223"));
    assert!(!debug.contains("server CertificateVerify"));
}

#[test]
fn verifies_all_three_signatures_and_retains_outer_transcript_proof() {
    let encoded_key = decode_hex(PUBLIC_KEY_SPKI_HEX);
    let key = validated_key(&encoded_key);

    let cases = [
        (
            SignatureScheme::RsaPssRsaeSha256,
            SHA256_SIGNATURE_HEX,
            223,
            Tls13RsaPssSignatureLimits::Sha256(RsaPssSha256SignatureLimits::new(300, 300).unwrap()),
        ),
        (
            SignatureScheme::RsaPssRsaeSha384,
            SHA384_SIGNATURE_HEX,
            207,
            Tls13RsaPssSignatureLimits::Sha384(RsaPssSha384SignatureLimits::new(300, 300).unwrap()),
        ),
        (
            SignatureScheme::RsaPssRsaeSha512,
            SHA512_SIGNATURE_HEX,
            191,
            Tls13RsaPssSignatureLimits::Sha512(RsaPssSha512SignatureLimits::new(300, 300).unwrap()),
        ),
    ];

    for (scheme, signature_hex, scratch_length, limits) in cases {
        let transcript = selected_transcript(
            CipherSuite::TlsChacha20Poly1305Sha256,
            VERIFY_TRANSCRIPT_BYTES,
        );
        let mut output = [0_u8; MAXIMUM_INPUT_LENGTH];
        let bound = bind_certificate_verify_transcript(
            &transcript,
            CertificateVerifyRole::Server,
            &mut output,
        )
        .unwrap();
        let hashed = hash_transcript_bound_certificate_verify_input(authorized(scheme), &bound);
        let signature = decode_hex(signature_hex);
        let message = CertificateVerifyMessage::new(scheme.wire_id(), &signature).unwrap();
        let mut scratch = vec![0xa5; scratch_length];

        let verified = verify_transcript_bound_certificate_verify(
            &hashed,
            &message,
            key,
            &mut scratch,
            limits,
        )
        .unwrap();

        assert!(ptr::eq(verified.hashed_input(), &hashed));
        assert!(ptr::eq(
            verified.verification().hashed_input(),
            hashed.hashed_input()
        ));
        assert_eq!(verified.verification().signature().message(), &message);
        assert_eq!(
            verified.verification().signature().public_key().modulus(),
            key.modulus()
        );
        assert_eq!(
            format!("{verified:?}"),
            "VerifiedTranscriptBoundCertificateVerify"
        );
    }
}

#[test]
fn rejects_message_or_limits_substitution_before_scratch_mutation_and_redacts() {
    let encoded_key = decode_hex(PUBLIC_KEY_SPKI_HEX);
    let key = validated_key(&encoded_key);
    let transcript = selected_transcript(
        CipherSuite::TlsChacha20Poly1305Sha256,
        VERIFY_TRANSCRIPT_BYTES,
    );
    let mut output = [0_u8; MAXIMUM_INPUT_LENGTH];
    let bound =
        bind_certificate_verify_transcript(&transcript, CertificateVerifyRole::Server, &mut output)
            .unwrap();
    let hashed = hash_transcript_bound_certificate_verify_input(
        authorized(SignatureScheme::RsaPssRsaeSha256),
        &bound,
    );
    let signature = decode_hex(SHA256_SIGNATURE_HEX);
    let wrong_message =
        CertificateVerifyMessage::new(SignatureScheme::RsaPssRsaeSha384.wire_id(), &signature)
            .unwrap();
    let correct_message =
        CertificateVerifyMessage::new(SignatureScheme::RsaPssRsaeSha256.wire_id(), &signature)
            .unwrap();
    let mut scratch = [0xa5; 223];
    let before = scratch;

    let wrong_message_error = verify_transcript_bound_certificate_verify(
        &hashed,
        &wrong_message,
        key,
        &mut scratch,
        Tls13RsaPssSignatureLimits::Sha256(RsaPssSha256SignatureLimits::new(300, 300).unwrap()),
    )
    .unwrap_err();
    assert_eq!(
        wrong_message_error.kind(),
        Tls13RsaPssCertificateVerifyErrorKind::SignatureRouting(
            Tls13RsaPssSignatureRoutingErrorKind::SignatureSchemeMismatch
        )
    );
    assert_eq!(scratch, before);

    let wrong_limits_error = verify_transcript_bound_certificate_verify(
        &hashed,
        &correct_message,
        key,
        &mut scratch,
        Tls13RsaPssSignatureLimits::Sha384(RsaPssSha384SignatureLimits::new(300, 300).unwrap()),
    )
    .unwrap_err();
    assert_eq!(
        wrong_limits_error.kind(),
        Tls13RsaPssCertificateVerifyErrorKind::SignatureRouting(
            Tls13RsaPssSignatureRoutingErrorKind::LimitsProfileMismatch
        )
    );
    assert_eq!(scratch, before);

    for diagnostic in [
        format!("{wrong_message_error:?}"),
        wrong_message_error.to_string(),
        format!("{wrong_limits_error:?}"),
        wrong_limits_error.to_string(),
    ] {
        assert!(!diagnostic.contains(std::str::from_utf8(VERIFY_TRANSCRIPT_BYTES).unwrap()));
        assert!(!diagnostic.contains(&SHA256_SIGNATURE_HEX[..32]));
    }
}

#[test]
fn changed_selected_transcript_invalidates_signature() {
    let encoded_key = decode_hex(PUBLIC_KEY_SPKI_HEX);
    let key = validated_key(&encoded_key);
    let transcript = selected_transcript(
        CipherSuite::TlsChacha20Poly1305Sha256,
        b"selected transcript prefix through Certificate!",
    );
    let mut output = [0_u8; MAXIMUM_INPUT_LENGTH];
    let bound =
        bind_certificate_verify_transcript(&transcript, CertificateVerifyRole::Server, &mut output)
            .unwrap();
    let hashed = hash_transcript_bound_certificate_verify_input(
        authorized(SignatureScheme::RsaPssRsaeSha256),
        &bound,
    );
    let signature = decode_hex(SHA256_SIGNATURE_HEX);
    let message =
        CertificateVerifyMessage::new(SignatureScheme::RsaPssRsaeSha256.wire_id(), &signature)
            .unwrap();
    let mut scratch = [0xa5; 223];

    let error = verify_transcript_bound_certificate_verify(
        &hashed,
        &message,
        key,
        &mut scratch,
        Tls13RsaPssSignatureLimits::Sha256(RsaPssSha256SignatureLimits::new(300, 300).unwrap()),
    )
    .expect_err("the original signature must not verify a changed transcript");

    assert!(matches!(
        error.kind(),
        Tls13RsaPssCertificateVerifyErrorKind::SignatureRouting(
            Tls13RsaPssSignatureRoutingErrorKind::Sha256(_)
        )
    ));
    assert!(!format!("{error:?}").contains(&SHA256_SIGNATURE_HEX[..32]));
}

#[test]
fn authenticated_exact_body_advances_the_owned_transcript_once() {
    let encoded_key = decode_hex(PUBLIC_KEY_SPKI_HEX);
    let key = validated_key(&encoded_key);
    let scheme = SignatureScheme::RsaPssRsaeSha256;
    let signature = decode_hex(SHA256_SIGNATURE_HEX);
    let body = certificate_verify_body(scheme, &signature);
    let transcript = selected_transcript(
        CipherSuite::TlsChacha20Poly1305Sha256,
        VERIFY_TRANSCRIPT_BYTES,
    );
    let before_snapshot = transcript.snapshot();
    let mut expected_transcript = selected_transcript(
        CipherSuite::TlsChacha20Poly1305Sha256,
        VERIFY_TRANSCRIPT_BYTES,
    );
    let mut signed_input_output = [0_u8; MAXIMUM_INPUT_LENGTH];
    let mut handshake_output = vec![0xa5; 4 + body.len() + 7];
    let mut scratch = [0xa5; 223];

    let verified = verify_and_advance_owned_certificate_verify_transcript(
        transcript,
        CertificateVerifyRole::Server,
        authorized(scheme),
        &body,
        key,
        &mut signed_input_output,
        &mut handshake_output,
        &mut scratch,
        Tls13RsaPssSignatureLimits::Sha256(RsaPssSha256SignatureLimits::new(300, 300).unwrap()),
    )
    .unwrap();

    let encoded_length = 4 + body.len();
    assert_eq!(&handshake_output[..4], &[15, 0, 1, 4]);
    assert_eq!(&handshake_output[4..encoded_length], body);
    assert!(handshake_output[encoded_length..]
        .iter()
        .all(|byte| *byte == 0xa5));
    expected_transcript
        .update(&handshake_output[..encoded_length])
        .unwrap();

    assert_eq!(verified.before_snapshot(), before_snapshot);
    assert_eq!(verified.after_snapshot(), expected_transcript.snapshot());
    assert_eq!(
        verified.transcript().snapshot(),
        expected_transcript.snapshot()
    );
    assert_eq!(
        verified.transcript().observed_bytes(),
        (VERIFY_TRANSCRIPT_BYTES.len() + encoded_length) as u64
    );
    assert_eq!(verified.role(), CertificateVerifyRole::Server);
    assert_eq!(verified.authorized_scheme().scheme(), scheme);
    assert!(ptr::eq(verified.certificate_verify_body(), body.as_slice()));
    assert_eq!(verified.public_key(), key);
    assert_eq!(verified.encoded_handshake_length(), encoded_length);
    assert_eq!(
        format!("{verified:?}"),
        "OwnedVerifiedCertificateVerifyTranscript"
    );
}

#[test]
fn malformed_body_fails_before_output_mutation_and_redacts() {
    let encoded_key = decode_hex(PUBLIC_KEY_SPKI_HEX);
    let key = validated_key(&encoded_key);
    let transcript = selected_transcript(
        CipherSuite::TlsChacha20Poly1305Sha256,
        VERIFY_TRANSCRIPT_BYTES,
    );
    let malformed_body = [0x08, 0x04, 0x00, 0x02, 0xaa];
    let mut signed_input_output = [0xa5; MAXIMUM_INPUT_LENGTH];
    let mut handshake_output = [0xa5; 32];
    let mut scratch = [0xa5; 223];

    let error = verify_and_advance_owned_certificate_verify_transcript(
        transcript,
        CertificateVerifyRole::Server,
        authorized(SignatureScheme::RsaPssRsaeSha256),
        &malformed_body,
        key,
        &mut signed_input_output,
        &mut handshake_output,
        &mut scratch,
        Tls13RsaPssSignatureLimits::Sha256(RsaPssSha256SignatureLimits::new(300, 300).unwrap()),
    )
    .unwrap_err();

    assert_eq!(
        error.kind(),
        CertificateVerifyTranscriptAdvanceErrorKind::MessageFraming(
            tls13_certificate_verify_message::MessageError::SignatureLengthMismatch
        )
    );
    assert_eq!(signed_input_output, [0xa5; MAXIMUM_INPUT_LENGTH]);
    assert_eq!(handshake_output, [0xa5; 32]);
    assert_eq!(scratch, [0xa5; 223]);
    assert!(!format!("{error:?}").contains("aa"));
}

#[test]
fn invalid_signature_fails_before_handshake_framing_or_transcript_admission() {
    let encoded_key = decode_hex(PUBLIC_KEY_SPKI_HEX);
    let key = validated_key(&encoded_key);
    let scheme = SignatureScheme::RsaPssRsaeSha256;
    let signature = decode_hex(SHA256_SIGNATURE_HEX);
    let mut body = certificate_verify_body(scheme, &signature);
    body[4] ^= 1;
    let transcript = selected_transcript(
        CipherSuite::TlsChacha20Poly1305Sha256,
        VERIFY_TRANSCRIPT_BYTES,
    );
    let mut signed_input_output = [0_u8; MAXIMUM_INPUT_LENGTH];
    let mut handshake_output = vec![0xa5; 4 + body.len()];
    let before_handshake_output = handshake_output.clone();
    let mut scratch = [0xa5; 223];

    let error = verify_and_advance_owned_certificate_verify_transcript(
        transcript,
        CertificateVerifyRole::Server,
        authorized(scheme),
        &body,
        key,
        &mut signed_input_output,
        &mut handshake_output,
        &mut scratch,
        Tls13RsaPssSignatureLimits::Sha256(RsaPssSha256SignatureLimits::new(300, 300).unwrap()),
    )
    .unwrap_err();

    assert!(matches!(
        error.kind(),
        CertificateVerifyTranscriptAdvanceErrorKind::SignatureVerification(
            Tls13RsaPssCertificateVerifyErrorKind::SignatureRouting(
                Tls13RsaPssSignatureRoutingErrorKind::Sha256(_)
            )
        )
    ));
    assert_eq!(handshake_output, before_handshake_output);
    for diagnostic in [format!("{error:?}"), error.to_string()] {
        assert!(!diagnostic.contains(std::str::from_utf8(VERIFY_TRANSCRIPT_BYTES).unwrap()));
        assert!(!diagnostic.contains(&SHA256_SIGNATURE_HEX[..32]));
    }
}

#[test]
fn short_handshake_storage_withholds_authenticated_transcript_state() {
    let encoded_key = decode_hex(PUBLIC_KEY_SPKI_HEX);
    let key = validated_key(&encoded_key);
    let scheme = SignatureScheme::RsaPssRsaeSha256;
    let signature = decode_hex(SHA256_SIGNATURE_HEX);
    let body = certificate_verify_body(scheme, &signature);
    let transcript = selected_transcript(
        CipherSuite::TlsChacha20Poly1305Sha256,
        VERIFY_TRANSCRIPT_BYTES,
    );
    let mut signed_input_output = [0_u8; MAXIMUM_INPUT_LENGTH];
    let mut handshake_output = vec![0xa5; 3 + body.len()];
    let before_handshake_output = handshake_output.clone();
    let mut scratch = [0xa5; 223];

    let error = verify_and_advance_owned_certificate_verify_transcript(
        transcript,
        CertificateVerifyRole::Server,
        authorized(scheme),
        &body,
        key,
        &mut signed_input_output,
        &mut handshake_output,
        &mut scratch,
        Tls13RsaPssSignatureLimits::Sha256(RsaPssSha256SignatureLimits::new(300, 300).unwrap()),
    )
    .unwrap_err();

    assert_eq!(
        error.kind(),
        CertificateVerifyTranscriptAdvanceErrorKind::HandshakeFraming(
            HandshakeErrorKind::OutputTooSmall
        )
    );
    assert_eq!(handshake_output, before_handshake_output);
}

#[test]
fn transcript_limit_failure_withholds_partially_advanced_state() {
    let encoded_key = decode_hex(PUBLIC_KEY_SPKI_HEX);
    let key = validated_key(&encoded_key);
    let scheme = SignatureScheme::RsaPssRsaeSha256;
    let signature = decode_hex(SHA256_SIGNATURE_HEX);
    let body = certificate_verify_body(scheme, &signature);
    let transcript = selected_transcript_with_limit(
        CipherSuite::TlsChacha20Poly1305Sha256,
        VERIFY_TRANSCRIPT_BYTES,
        VERIFY_TRANSCRIPT_BYTES.len() as u64,
    );
    let mut signed_input_output = [0_u8; MAXIMUM_INPUT_LENGTH];
    let mut handshake_output = vec![0xa5; 4 + body.len()];
    let mut scratch = [0xa5; 223];

    let error = verify_and_advance_owned_certificate_verify_transcript(
        transcript,
        CertificateVerifyRole::Server,
        authorized(scheme),
        &body,
        key,
        &mut signed_input_output,
        &mut handshake_output,
        &mut scratch,
        Tls13RsaPssSignatureLimits::Sha256(RsaPssSha256SignatureLimits::new(300, 300).unwrap()),
    )
    .unwrap_err();

    assert_eq!(
        error.kind(),
        CertificateVerifyTranscriptAdvanceErrorKind::Transcript(
            TranscriptErrorKind::TranscriptLimitExceeded
        )
    );
    assert_eq!(&handshake_output[..4], &[15, 0, 1, 4]);
    assert_eq!(&handshake_output[4..], body);
}
