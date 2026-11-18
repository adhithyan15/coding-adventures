use core::ptr;

use coding_adventures_sha256::sha256;
use coding_adventures_sha384::sha384;
use coding_adventures_sha512::sum512;
use tls13_certificate_verify_input::{
    CertificateVerifyRole, InputError, TranscriptHashAlgorithm, MAXIMUM_INPUT_LENGTH,
};
use tls13_certificate_verify_transcript_binding::{
    bind_certificate_verify_transcript, hash_transcript_bound_certificate_verify_input,
};
use tls13_cipher_suite_policy::{
    CipherSuite, CipherSuitePolicy, CipherSuitePolicyEntry, SelectedCipherSuite,
};
use tls13_signature_scheme_policy::{
    AuthorizedSignatureScheme, SignatureScheme, SignatureSchemePolicy,
};
use tls13_transcript::{PendingTls13Transcript, SelectedTls13Transcript, TranscriptPolicy};

fn selected_suite(suite: CipherSuite) -> SelectedCipherSuite {
    let entry = CipherSuitePolicyEntry::new(suite, 4096).expect("valid suite entry");
    CipherSuitePolicy::new(&[entry])
        .expect("valid suite policy")
        .select(suite.wire_id())
        .expect("suite is allowed")
}

fn selected_transcript(suite: CipherSuite, bytes: &[u8]) -> SelectedTls13Transcript {
    let mut pending =
        PendingTls13Transcript::new(TranscriptPolicy::new(4096).expect("valid transcript policy"));
    pending.update(bytes).expect("test transcript fits");
    pending.select(selected_suite(suite))
}

fn authorized(scheme: SignatureScheme) -> AuthorizedSignatureScheme {
    SignatureSchemePolicy::new(&[scheme])
        .expect("valid signature policy")
        .select(scheme.wire_id())
        .expect("scheme is allowed")
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
