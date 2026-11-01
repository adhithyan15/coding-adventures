use std::mem::size_of;

use coding_adventures_sha256::sha256;
use coding_adventures_sha384::sha384;
use tls13_cipher_suite_policy::{
    CipherSuite, CipherSuitePolicy, CipherSuitePolicyEntry, HkdfHash, SelectedCipherSuite,
};
use tls13_transcript::{
    PendingTls13Transcript, TranscriptError, TranscriptErrorKind, TranscriptPolicy,
    TLS13_MAX_TRANSCRIPT_BYTES,
};

fn selected_suite(suite: CipherSuite) -> SelectedCipherSuite {
    let entry = CipherSuitePolicyEntry::new(suite, 1024).expect("valid suite entry");
    CipherSuitePolicy::new(&[entry])
        .expect("valid suite policy")
        .select(suite.wire_id())
        .expect("suite is allowed")
}

fn transcript(limit: u64) -> PendingTls13Transcript {
    PendingTls13Transcript::new(TranscriptPolicy::new(limit).expect("valid transcript policy"))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[test]
fn policy_requires_a_nonzero_sha256_bounded_limit() {
    for limit in [0, TLS13_MAX_TRANSCRIPT_BYTES + 1] {
        assert_eq!(
            TranscriptPolicy::new(limit)
                .expect_err("invalid limit must fail")
                .kind(),
            TranscriptErrorKind::TranscriptLimitOutOfRange
        );
    }

    let maximum = TranscriptPolicy::new(TLS13_MAX_TRANSCRIPT_BYTES).expect("maximum is valid");
    assert_eq!(maximum.maximum_observed_bytes(), TLS13_MAX_TRANSCRIPT_BYTES);
}

#[test]
fn preselection_hashes_both_algorithms_and_selection_keeps_the_suite_binding() {
    let segments: [&[u8]; 3] = [b"client", b"-", b"hello"];
    let whole = b"client-hello";

    for (suite, algorithm, expected) in [
        (
            CipherSuite::TlsAes128GcmSha256,
            HkdfHash::Sha256,
            sha256(whole).to_vec(),
        ),
        (
            CipherSuite::TlsAes256GcmSha384,
            HkdfHash::Sha384,
            sha384(whole).to_vec(),
        ),
        (
            CipherSuite::TlsChacha20Poly1305Sha256,
            HkdfHash::Sha256,
            sha256(whole).to_vec(),
        ),
    ] {
        let mut pending = transcript(64);
        for segment in segments {
            pending.update(segment).expect("segment fits");
        }
        assert_eq!(pending.observed_bytes(), whole.len() as u64);

        let selected_suite = selected_suite(suite);
        let selected = pending.select(selected_suite);
        assert_eq!(selected.selected_suite(), selected_suite);
        assert_eq!(selected.snapshot().algorithm(), algorithm);
        assert_eq!(selected.snapshot().as_bytes(), expected);
        assert_eq!(selected.observed_bytes(), whole.len() as u64);
    }
}

#[test]
fn snapshots_are_non_destructive_and_updates_continue_the_same_history() {
    let mut pending = transcript(64);
    pending.update(b"ab").expect("prefix fits");
    let mut selected = pending.select(selected_suite(CipherSuite::TlsAes128GcmSha256));

    let prefix = selected.snapshot();
    assert_eq!(prefix.as_bytes(), sha256(b"ab"));
    assert_eq!(selected.snapshot(), prefix);

    selected.update(b"c").expect("suffix fits");
    assert_eq!(selected.snapshot().as_bytes(), sha256(b"abc"));
}

#[test]
fn cumulative_limit_failure_is_atomic_before_and_after_selection() {
    let mut pending = transcript(3);
    pending.update(b"ab").expect("prefix fits");
    assert_eq!(
        pending
            .update(b"cd")
            .expect_err("pending limit must fail")
            .kind(),
        TranscriptErrorKind::TranscriptLimitExceeded
    );
    assert_eq!(pending.observed_bytes(), 2);

    let mut selected = pending.select(selected_suite(CipherSuite::TlsAes128GcmSha256));
    let before = selected.snapshot();
    assert_eq!(
        selected
            .update(b"cd")
            .expect_err("selected limit must fail")
            .kind(),
        TranscriptErrorKind::TranscriptLimitExceeded
    );
    assert_eq!(selected.snapshot(), before);
    assert_eq!(selected.observed_bytes(), 2);
}

#[test]
fn hello_retry_request_rewrite_matches_independent_sha_vectors() {
    let client_hello = [1, 0, 0, 3, 1, 2, 3];
    let cases = [
        (
            CipherSuite::TlsAes128GcmSha256,
            "64459fff8d1913bf676ed64a76e05832aa8827658cbe7c1bcdccda9d30b9c5bc",
        ),
        (
            CipherSuite::TlsAes256GcmSha384,
            "1587f968180088b7fb76e813ab672a832160566f09240cfc79078d0438256d3de3f2c1df89863e76edfc88190c00a43f",
        ),
    ];

    for (suite, expected) in cases {
        let mut pending = transcript(128);
        pending.update(&client_hello).expect("ClientHello fits");
        let mut selected = pending.select(selected_suite(suite));
        selected
            .apply_hello_retry_request()
            .expect("first rewrite succeeds");

        assert_eq!(hex(selected.snapshot().as_bytes()), expected);
        assert!(selected.hello_retry_request_applied());
        assert_eq!(selected.observed_bytes(), client_hello.len() as u64);
    }
}

#[test]
fn hello_retry_request_is_one_shot_and_rejection_preserves_state() {
    let mut pending = transcript(64);
    pending
        .update(&[1, 0, 0, 0])
        .expect("ClientHello framing fits");
    let mut selected = pending.select(selected_suite(CipherSuite::TlsAes128GcmSha256));
    selected
        .apply_hello_retry_request()
        .expect("first rewrite succeeds");
    let before = selected.snapshot();

    assert_eq!(
        selected
            .apply_hello_retry_request()
            .expect_err("second rewrite must fail")
            .kind(),
        TranscriptErrorKind::HelloRetryRequestAlreadyApplied
    );
    assert_eq!(selected.snapshot(), before);
}

#[test]
fn caller_accepted_bytes_after_retry_extend_the_synthetic_history() {
    let client_hello = [1, 0, 0, 0];
    let hello_retry_request = [2, 0, 0, 1, 0xaa];
    let client_hello_hash = sha256(&client_hello);
    let mut synthetic = Vec::from([254, 0, 0, 32]);
    synthetic.extend_from_slice(&client_hello_hash);
    synthetic.extend_from_slice(&hello_retry_request);

    let mut pending = transcript(64);
    pending.update(&client_hello).expect("ClientHello fits");
    let mut selected = pending.select(selected_suite(CipherSuite::TlsAes128GcmSha256));
    selected
        .apply_hello_retry_request()
        .expect("rewrite succeeds");
    selected
        .update(&hello_retry_request)
        .expect("HelloRetryRequest fits");

    assert_eq!(selected.snapshot().as_bytes(), sha256(&synthetic));
    assert_eq!(
        selected.observed_bytes(),
        (client_hello.len() + hello_retry_request.len()) as u64
    );
}

#[test]
fn errors_are_payload_free_stable_categories() {
    assert_eq!(
        size_of::<TranscriptError>(),
        size_of::<TranscriptErrorKind>()
    );
    let error = TranscriptPolicy::new(0).expect_err("zero limit fails");
    assert_eq!(error.kind(), TranscriptErrorKind::TranscriptLimitOutOfRange);
    assert_eq!(
        error.to_string(),
        "TLS 1.3 transcript error TranscriptLimitOutOfRange"
    );
}
