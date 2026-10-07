//! Tests for the D18U sealed secret record, grouped by spec rule.
//!
//! The codec tests build envelopes by hand, byte by byte, rather than
//! round-tripping through `encode_record`. A round-trip test cannot catch an
//! encoder and decoder that agree with each other and disagree with the spec;
//! a hand-built envelope pins the wire format itself.

use std::collections::BTreeSet;
use std::sync::Arc;

use chief_of_staff_vault_runtime::{
    AllowedAgents, ChiefVaultRuntime, SecretPolicy, VaultDeliveryMode, VaultLeaseRequest,
    VaultRuntimeError,
};
use chief_of_staff_vault_secret_store::{
    decode_record, encode_record, validate_policy, ChiefSecretStore, NameError, RecordError,
    SecretName, StoreError, MAGIC, MAX_AGENT_ID_BYTES, MAX_ALLOWED_AGENTS, MAX_PAYLOAD_BYTES,
    MAX_RECORDS, MAX_RECORD_BYTES, MAX_SECRET_NAME_BYTES, NAMESPACE, VERSION,
};
use coding_adventures_vault_sealed_store::SealedStore;
use storage_core::{InMemoryStorageBackend, StorageBackend};

// ── Helpers ──────────────────────────────────────────────────────────────────

const KEK: [u8; 32] = [0x42; 32];

fn policy(agents: AllowedAgents, mode: VaultDeliveryMode) -> SecretPolicy {
    SecretPolicy {
        privilege_tier: 2,
        allowed_agents: agents,
        allowed_mode: mode,
        rotated_at_ms: 1_790_000_000_000,
    }
}

fn weather_policy() -> SecretPolicy {
    policy(
        AllowedAgents::only(["weather-agent"]),
        VaultDeliveryMode::Leased,
    )
}

/// A hand-built envelope: fixed header, then whatever `tail` supplies.
fn header(tier: u8, mode: u8, rotated: u64) -> Vec<u8> {
    let mut v = MAGIC.to_vec();
    v.push(VERSION);
    v.push(tier);
    v.push(mode);
    v.extend_from_slice(&rotated.to_be_bytes());
    v
}

fn push_str(v: &mut Vec<u8>, s: &[u8]) {
    v.extend_from_slice(&(s.len() as u32).to_be_bytes());
    v.extend_from_slice(s);
}

/// `Any`, mode Leased, tier 0, then the payload.
fn any_record(payload: &[u8]) -> Vec<u8> {
    let mut v = header(0, 1, 7);
    v.push(0);
    push_str(&mut v, payload);
    v
}

fn only_record(agents: &[&[u8]], payload: &[u8]) -> Vec<u8> {
    let mut v = header(1, 2, 7);
    v.push(1);
    v.extend_from_slice(&(agents.len() as u16).to_be_bytes());
    for a in agents {
        push_str(&mut v, a);
    }
    push_str(&mut v, payload);
    v
}

fn malformed(bytes: &[u8]) -> &'static str {
    match decode_record(bytes) {
        Err(RecordError::Malformed(why)) => why,
        other => panic!("expected Malformed, got {other:?}"),
    }
}

/// Two handles over one backend: one to set up, one owned by the adapter.
/// Mirrors how the CLI writes and the daemon later reads the same files.
fn store_over(backend: Arc<dyn StorageBackend>) -> ChiefSecretStore {
    let sealed = SealedStore::new(backend);
    if sealed.status().expect("status").initialized {
        sealed.unseal_with_kek(&KEK).expect("unseal");
    } else {
        sealed.init_with_kek(&KEK).expect("init");
    }
    ChiefSecretStore::new(sealed)
}

fn fresh() -> (Arc<dyn StorageBackend>, ChiefSecretStore) {
    let backend: Arc<dyn StorageBackend> = Arc::new(InMemoryStorageBackend::new());
    let store = store_over(backend.clone());
    (backend, store)
}

fn name(s: &str) -> SecretName {
    SecretName::parse(s).expect("valid name")
}

fn lease<'a>(agent: Option<&'a str>, secret: &'a str) -> VaultLeaseRequest<'a> {
    VaultLeaseRequest {
        requesting_agent_id: agent,
        secret_name: secret,
        ttl_ms: 60_000,
    }
}

// ── U-N1: secret names ───────────────────────────────────────────────────────

#[test]
fn names_accept_the_documented_alphabet() {
    for ok in [
        "a",
        "0",
        "weather.api-key_v2",
        "x".repeat(MAX_SECRET_NAME_BYTES).as_str(),
    ] {
        assert_eq!(SecretName::parse(ok).expect(ok).as_str(), ok);
    }
    assert_eq!(name("bank").to_string(), "bank");
}

#[test]
fn names_reject_everything_path_shaped() {
    let cases: &[(&str, NameError)] = &[
        ("", NameError::Empty),
        (&"x".repeat(MAX_SECRET_NAME_BYTES + 1), NameError::TooLong),
        (".hidden", NameError::BadStart),
        ("-flag", NameError::BadStart),
        ("_x", NameError::BadStart),
        ("Bank", NameError::BadStart),
        ("bAnk", NameError::InvalidCharacter),
        ("a/b", NameError::InvalidCharacter),
        ("a\\b", NameError::InvalidCharacter),
        ("a b", NameError::InvalidCharacter),
        ("a\0b", NameError::InvalidCharacter),
        ("bаnk", NameError::InvalidCharacter), // Cyrillic а
        ("a..b", NameError::DotDot),
        ("a..", NameError::DotDot),
    ];
    for (input, want) in cases {
        assert_eq!(SecretName::parse(input), Err(*want), "{input:?}");
        assert!(!want.to_string().is_empty());
    }
}

// ── Encoding: the wire format is pinned ──────────────────────────────────────

#[test]
fn encoder_writes_the_exact_spec_layout() {
    let p = policy(
        AllowedAgents::only(["zeta", "alpha"]),
        VaultDeliveryMode::Both,
    );
    let got = encode_record(&p, b"s3cret").expect("encode");
    let mut want = header(2, 2, 1_790_000_000_000);
    want.push(1);
    want.extend_from_slice(&2u16.to_be_bytes());
    push_str(&mut want, b"alpha"); // ascending, regardless of insertion order
    push_str(&mut want, b"zeta");
    push_str(&mut want, b"s3cret");
    assert_eq!(&*got, &want);
}

#[test]
fn every_mode_and_agent_shape_round_trips() {
    for mode in [
        VaultDeliveryMode::Direct,
        VaultDeliveryMode::Leased,
        VaultDeliveryMode::Both,
    ] {
        for agents in [AllowedAgents::Any, AllowedAgents::only(["a", "b", "c"])] {
            for tier in 0..=3 {
                let mut p = policy(agents.clone(), mode);
                p.privilege_tier = tier;
                let bytes = encode_record(&p, b"payload").expect("encode");
                let back = decode_record(&bytes).expect("decode");
                assert_eq!(back.policy, p);
                assert_eq!(back.payload.as_bytes(), b"payload");
            }
        }
    }
}

#[test]
fn the_largest_valid_record_fits_the_declared_bound() {
    let agents: BTreeSet<String> = (0..MAX_ALLOWED_AGENTS)
        .map(|i| format!("{i:03}{}", "a".repeat(MAX_AGENT_ID_BYTES - 3)))
        .collect();
    let p = policy(AllowedAgents::Only(agents), VaultDeliveryMode::Both);
    let bytes = encode_record(&p, &vec![7u8; MAX_PAYLOAD_BYTES]).expect("encode");
    assert_eq!(bytes.len(), MAX_RECORD_BYTES);
    assert_eq!(decode_record(&bytes).expect("decode").policy, p);
}

// ── U-E3 / U-E5 and policy bounds at encode time ─────────────────────────────

#[test]
fn encoder_refuses_policies_the_format_cannot_hold() {
    let mut high = weather_policy();
    high.privilege_tier = 4;
    let too_many: BTreeSet<String> = (0..=MAX_ALLOWED_AGENTS).map(|i| format!("a{i}")).collect();
    let cases = [
        high,
        policy(
            AllowedAgents::Only(BTreeSet::new()),
            VaultDeliveryMode::Both,
        ),
        policy(AllowedAgents::Only(too_many), VaultDeliveryMode::Both),
        policy(AllowedAgents::only([""]), VaultDeliveryMode::Both),
        policy(
            AllowedAgents::only(["x".repeat(MAX_AGENT_ID_BYTES + 1)]),
            VaultDeliveryMode::Both,
        ),
        policy(AllowedAgents::only(["bad\nid"]), VaultDeliveryMode::Both),
    ];
    for p in cases {
        let err = encode_record(&p, b"x").err().expect("must refuse");
        assert!(matches!(err, RecordError::InvalidPolicy(_)), "{err:?}");
        assert_eq!(validate_policy(&p), Err(err));
        assert!(err.to_string().starts_with("invalid secret policy"));
    }
}

#[test]
fn encoder_refuses_empty_and_oversized_payloads() {
    for payload in [Vec::new(), vec![0u8; MAX_PAYLOAD_BYTES + 1]] {
        let err = encode_record(&weather_policy(), &payload)
            .err()
            .expect("must refuse");
        assert!(matches!(err, RecordError::InvalidPayload(_)));
        assert!(err.to_string().starts_with("invalid secret payload"));
    }
}

// ── U-E1: decode is total and closed ─────────────────────────────────────────

#[test]
fn decoder_accepts_hand_built_records() {
    let any = decode_record(&any_record(b"k")).expect("any");
    assert_eq!(any.policy.allowed_agents, AllowedAgents::Any);
    assert_eq!(any.policy.allowed_mode, VaultDeliveryMode::Leased);
    let only = decode_record(&only_record(&[b"a", b"b"], b"k")).expect("only");
    assert_eq!(only.policy.allowed_agents, AllowedAgents::only(["a", "b"]));
    assert_eq!(only.policy.allowed_mode, VaultDeliveryMode::Both);
    assert!(format!("{only:?}").contains("<redacted>"));
}

#[test]
fn decoder_rejects_every_header_violation() {
    let mut wrong_magic = any_record(b"k");
    wrong_magic[0] = b'X';
    assert_eq!(malformed(&wrong_magic), "wrong magic");

    let mut wrong_version = any_record(b"k");
    wrong_version[8] = 2;
    assert_eq!(malformed(&wrong_version), "unknown version");

    let mut mode = any_record(b"k");
    mode[10] = 3;
    assert_eq!(malformed(&mode), "unknown delivery mode");

    let mut tier = any_record(b"k");
    tier[9] = 4;
    assert_eq!(malformed(&tier), "privilege_tier is above 3");

    let mut tag = any_record(b"k");
    tag[19] = 2;
    assert_eq!(malformed(&tag), "unknown allowed-agents tag");
}

#[test]
fn decoder_rejects_every_truncation_point() {
    let full = only_record(&[b"alpha", b"beta"], b"secret");
    for cut in 0..full.len() {
        let why = malformed(&full[..cut]);
        assert_eq!(why, "record is truncated", "cut at {cut}");
    }
}

#[test]
fn decoder_rejects_trailing_bytes_and_oversized_input() {
    let mut trailing = any_record(b"k");
    trailing.push(0);
    assert_eq!(malformed(&trailing), "trailing bytes after payload");

    let huge = vec![0u8; MAX_RECORD_BYTES + 1];
    assert_eq!(malformed(&huge), "record is larger than any valid record");
}

#[test]
fn decoder_rejects_out_of_range_payload_lengths() {
    assert_eq!(malformed(&any_record(b"")), "payload length out of range");
    let mut big = header(0, 1, 0);
    big.push(0);
    big.extend_from_slice(&((MAX_PAYLOAD_BYTES as u32) + 1).to_be_bytes());
    assert_eq!(malformed(&big), "payload length out of range");
}

#[test]
fn decoder_rejects_bad_allow_lists() {
    assert_eq!(
        malformed(&only_record(&[], b"k")),
        "allow-list names no agent"
    );

    let too_many: Vec<Vec<u8>> = (0..=MAX_ALLOWED_AGENTS)
        .map(|i| format!("a{i:03}").into_bytes())
        .collect();
    let refs: Vec<&[u8]> = too_many.iter().map(Vec::as_slice).collect();
    assert_eq!(
        malformed(&only_record(&refs, b"k")),
        "more than 64 allowed agents"
    );

    let long = vec![b'a'; MAX_AGENT_ID_BYTES + 1];
    assert_eq!(
        malformed(&only_record(&[&long], b"k")),
        "an agent id is longer than 256 bytes"
    );
    assert_eq!(
        malformed(&only_record(&[&[0xff, 0xfe]], b"k")),
        "an agent id is not UTF-8"
    );
    assert_eq!(
        malformed(&only_record(&[b""], b"k")),
        "an agent id is empty"
    );
    assert_eq!(
        malformed(&only_record(&[b"a\x07"], b"k")),
        "an agent id contains a control character"
    );
}

// ── U-E4: one policy, one encoding ───────────────────────────────────────────

#[test]
fn decoder_rejects_non_canonical_agent_order() {
    assert_eq!(
        malformed(&only_record(&[b"beta", b"alpha"], b"k")),
        "agent ids are not strictly ascending"
    );
    assert_eq!(
        malformed(&only_record(&[b"alpha", b"alpha"], b"k")),
        "agent ids are not strictly ascending"
    );
}

// ── U-E7: nothing secret in diagnostics ──────────────────────────────────────

#[test]
fn errors_and_debug_output_carry_no_secret_bytes() {
    let secret = b"hunter2-very-secret";
    let decoded = decode_record(&any_record(secret)).expect("decode");
    assert!(!format!("{decoded:?}").contains("hunter2"));

    let mut bad = only_record(&[b"peer-agent-name"], secret);
    bad.push(0);
    let err = decode_record(&bad).expect_err("trailing");
    let shown = format!("{err} {err:?}");
    assert!(!shown.contains("hunter2") && !shown.contains("peer-agent-name"));

    let (_, store) = fresh();
    assert_eq!(format!("{store:?}"), "ChiefSecretStore(<redacted>)");
}

// ── The store: put, names, load, delete ──────────────────────────────────────

#[test]
fn put_then_load_round_trips_through_the_sealed_store() {
    let (_, store) = fresh();
    store
        .put(&name("weather"), &weather_policy(), b"api-key")
        .expect("put");
    store
        .put(
            &name("bank"),
            &policy(AllowedAgents::Any, VaultDeliveryMode::Direct),
            b"password",
        )
        .expect("put");

    assert_eq!(
        store.names().expect("names"),
        vec![name("bank"), name("weather")]
    );
    let loaded = store.load_all().expect("load");
    assert_eq!(loaded.len(), 2);
    assert_eq!(loaded[1].name, name("weather"));
    assert_eq!(loaded[1].record.policy, weather_policy());
    assert_eq!(loaded[1].record.payload.as_bytes(), b"api-key");
}

#[test]
fn a_second_process_reads_what_the_first_wrote() {
    // The CLI writes; the daemon, a separate SealedStore handle over the same
    // backend, reads at startup.
    let (backend, cli) = fresh();
    cli.put(&name("weather"), &weather_policy(), b"api-key")
        .expect("put");
    let daemon = store_over(backend);
    let loaded = daemon.load_all().expect("load");
    assert_eq!(loaded[0].record.payload.as_bytes(), b"api-key");
}

#[test]
fn put_overwrites_which_is_rotation() {
    let (_, store) = fresh();
    store
        .put(&name("weather"), &weather_policy(), b"old")
        .expect("put");
    let mut rotated = weather_policy();
    rotated.rotated_at_ms += 1;
    store
        .put(&name("weather"), &rotated, b"new")
        .expect("rotate");
    let loaded = store.load_all().expect("load");
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].record.payload.as_bytes(), b"new");
    assert_eq!(loaded[0].record.policy, rotated);
}

#[test]
fn put_refuses_an_unstorable_record_before_writing() {
    let (_, store) = fresh();
    let err = store
        .put(&name("weather"), &weather_policy(), b"")
        .expect_err("empty");
    assert!(matches!(
        err,
        StoreError::Record(RecordError::InvalidPayload(_))
    ));
    assert!(err.to_string().contains("payload is empty"));
    assert!(store.names().expect("names").is_empty());
}

#[test]
fn delete_removes_and_tolerates_absence() {
    let (_, store) = fresh();
    store
        .put(&name("weather"), &weather_policy(), b"k")
        .expect("put");
    store.delete(&name("weather")).expect("delete");
    store.delete(&name("weather")).expect("delete again");
    assert!(store.load_all().expect("load").is_empty());
}

#[test]
fn listing_pages_past_one_backend_page() {
    let (_, store) = fresh();
    // 300 spans three pages of 128, which exercises the rebuilt cursor.
    for i in 0..300 {
        store
            .put(&name(&format!("s{i:03}")), &weather_policy(), b"k")
            .expect("put");
    }
    let names = store.names().expect("names");
    assert_eq!(names.len(), 300);
    assert_eq!(names.first(), Some(&name("s000")));
    assert_eq!(names.last(), Some(&name("s299")));
}

// ── U-L1: all or nothing ─────────────────────────────────────────────────────

#[test]
fn a_corrupt_record_stops_the_load_and_is_named() {
    let (backend, store) = fresh();
    store
        .put(&name("good"), &weather_policy(), b"k")
        .expect("put");
    // Write a record that decrypts fine but is not a D18U envelope, through
    // a raw handle: the AEAD is happy, the format is not.
    let raw = SealedStore::new(backend);
    raw.unseal_with_kek(&KEK).expect("unseal");
    raw.put(NAMESPACE, "bad", b"not an envelope", None)
        .expect("raw put");

    let err = store.load_all().expect_err("must refuse");
    match &err {
        StoreError::CorruptRecord { name: n, error } => {
            assert_eq!(n, &name("bad"));
            assert_eq!(*error, RecordError::Malformed("wrong magic"));
        }
        other => panic!("{other:?}"),
    }
    assert!(err.to_string().contains("record bad"));

    // And register_all leaves the runtime untouched.
    let runtime = ChiefVaultRuntime::new();
    assert!(store.register_all(&runtime).is_err());
    assert_eq!(runtime.secret_policy("good"), None);
}

#[test]
fn a_key_that_is_not_a_secret_name_stops_the_load() {
    let (backend, store) = fresh();
    let raw = SealedStore::new(backend);
    raw.unseal_with_kek(&KEK).expect("unseal");
    raw.put(NAMESPACE, "Not-A-Name", b"x", None)
        .expect("raw put");
    let err = store.load_all().expect_err("must refuse");
    assert!(matches!(err, StoreError::UnnamedRecord));
    // The on-disk key is not echoed.
    assert!(!err.to_string().contains("Not-A-Name"));
}

#[test]
fn a_store_that_would_not_unseal_refuses_to_load() {
    // The wrong KEK cannot unseal, and a still-sealed store must surface as
    // an error rather than as an empty vault.
    let (backend, store) = fresh();
    store
        .put(&name("weather"), &weather_policy(), b"k")
        .expect("put");
    let sealed = SealedStore::new(backend);
    assert!(sealed.unseal_with_kek(&[0x00; 32]).is_err());
    let err = ChiefSecretStore::new(sealed)
        .load_all()
        .expect_err("sealed");
    assert!(matches!(err, StoreError::Sealed(_)));
    assert!(err.to_string().starts_with("chief secret store:"));
}

// ── U-L2: bounded ────────────────────────────────────────────────────────────

#[test]
fn more_than_the_record_bound_is_refused() {
    let (_, store) = fresh();
    for i in 0..=MAX_RECORDS {
        store
            .put(&name(&format!("s{i:04}")), &weather_policy(), b"k")
            .expect("put");
    }
    let err = store.load_all().expect_err("too many");
    assert!(matches!(err, StoreError::TooManyRecords));
    assert!(err.to_string().contains("1024"));
}

// ── End to end: provision, register, lease ───────────────────────────────────

#[test]
fn registered_secrets_carry_the_provisioned_policy_into_the_runtime() {
    let (_, store) = fresh();
    store
        .put(&name("weather"), &weather_policy(), b"api-key")
        .expect("put");
    store
        .put(
            &name("bank"),
            &policy(
                AllowedAgents::only(["weather-agent"]),
                VaultDeliveryMode::Direct,
            ),
            b"password",
        )
        .expect("put");

    let runtime = ChiefVaultRuntime::new();
    assert_eq!(store.register_all(&runtime).expect("register"), 2);
    assert_eq!(runtime.secret_policy("weather"), Some(weather_policy()));

    // The allowed agent can lease the leasable secret, and redeem it.
    let receipt = runtime
        .request_lease(lease(Some("weather-agent"), "weather"))
        .expect("lease");
    let payload = runtime.consume(&receipt.vault_ref).expect("consume");
    assert_eq!(payload.as_bytes(), b"api-key");

    // Anyone else is refused, and so is an absent identity (VLT06 P3).
    assert!(matches!(
        runtime.request_lease(lease(Some("peer"), "weather")),
        Err(VaultRuntimeError::AgentNotPermitted)
    ));
    assert!(matches!(
        runtime.request_lease(lease(None, "weather")),
        Err(VaultRuntimeError::AgentNotPermitted)
    ));

    // And the direct-only secret cannot be leased even by its allowed agent:
    // the stored mode survived the round trip.
    assert!(matches!(
        runtime.request_lease(lease(Some("weather-agent"), "bank")),
        Err(VaultRuntimeError::DeliveryModeNotPermitted)
    ));
}
