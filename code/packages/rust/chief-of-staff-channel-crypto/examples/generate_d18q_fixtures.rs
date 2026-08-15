//! Generate the deterministic shared D18Q version 1 fixture manifest.

use std::env;
use std::fs;
use std::path::Path;

use chief_of_staff_channel_crypto::grant_profile::{
    grant_serialize, plan_rotation, seal_channel_key_with_material, KeyGrantFields,
    RotationReceiver,
};
use chief_of_staff_channel_crypto::wire::{decode_key_grant, encode_key_grant};
use chief_of_staff_channel_crypto::{
    ChannelId, ChannelMasterKey, KeyEpoch, OriginatorSigningKey, ReceiverKeyPair,
    SealedChannelKeyGrant,
};
use coding_adventures_ed25519::sign;
use coding_adventures_hkdf::{hkdf, HashAlgorithm};
use coding_adventures_json_serializer::serialize;
use coding_adventures_json_value::JsonValue;
use coding_adventures_x25519::{generate_keypair, x25519};

const BASE64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const KEY_GRANT_CONTEXT: &[u8] = b"chief-channel-key-grant-v1";
const KEY_WRAP_CONTEXT: &[u8] = b"chief-channel-key-wrap-v1";
const CHANNEL_ID: [u8; 16] = [
    0x01, 0x8f, 0x47, 0xa0, 0x9b, 0x6c, 0x7d, 0xef, 0x92, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0,
];
const OTHER_CHANNEL_ID: [u8; 16] = [
    0x01, 0x8f, 0x47, 0xa0, 0x9b, 0x6c, 0x7d, 0xef, 0x92, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf1,
];
const ORIGINATOR_ID: &[u8] = &[0x00, 0xff, b'o', b'r', b'i', b'g', 0x80];
const RECEIVER_A_ID: &[u8] = &[0x00, b'A', 0xff];
const RECEIVER_B_ID: &[u8] = b"receiver-B";
const SIGNING_SEED: [u8; 32] = [0x11; 32];
const EPOCH_ZERO_CMK: [u8; 32] = [0x22; 32];
const EPOCH_ONE_CMK: [u8; 32] = [0x33; 32];
const RECEIVER_A_PRIVATE: [u8; 32] = [0x41; 32];
const RECEIVER_B_PRIVATE: [u8; 32] = [0x42; 32];
const EPHEMERAL_A: [u8; 32] = [0x51; 32];
const EPHEMERAL_B: [u8; 32] = [0x52; 32];
const NONCE_A: [u8; 24] = [0x61; 24];
const NONCE_B: [u8; 24] = [0x62; 24];

struct PositiveCase<'a> {
    name: &'a str,
    receiver_id: &'a [u8],
    receiver_private_key: [u8; 32],
    cmk: [u8; 32],
    epoch: u64,
    ephemeral_private_key: [u8; 32],
    wrapping_nonce: [u8; 24],
}

#[allow(dead_code)]
fn main() {
    let mut arguments = env::args().skip(1);
    let output = arguments
        .next()
        .expect("usage: generate_d18q_fixtures OUTPUT GENERATOR_BLOB_SHA1");
    let generator_blob_sha1 = arguments
        .next()
        .expect("usage: generate_d18q_fixtures OUTPUT GENERATOR_BLOB_SHA1");
    assert!(arguments.next().is_none(), "unexpected extra argument");
    let encoded = generate_manifest(&generator_blob_sha1);
    if let Some(parent) = Path::new(&output).parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(output, encoded).unwrap();
}

/// Generate the complete canonical D18Q fixture manifest.
pub fn generate_manifest(generator_blob_sha1: &str) -> Vec<u8> {
    assert_eq!(generator_blob_sha1.len(), 40);
    let signer = OriginatorSigningKey::from_seed(SIGNING_SEED);
    let cases = [
        PositiveCase {
            name: "epoch-zero-receiver-a",
            receiver_id: RECEIVER_A_ID,
            receiver_private_key: RECEIVER_A_PRIVATE,
            cmk: EPOCH_ZERO_CMK,
            epoch: 0,
            ephemeral_private_key: EPHEMERAL_A,
            wrapping_nonce: NONCE_A,
        },
        PositiveCase {
            name: "epoch-zero-receiver-b",
            receiver_id: RECEIVER_B_ID,
            receiver_private_key: RECEIVER_B_PRIVATE,
            cmk: EPOCH_ZERO_CMK,
            epoch: 0,
            ephemeral_private_key: EPHEMERAL_B,
            wrapping_nonce: NONCE_B,
        },
        PositiveCase {
            name: "maximum-epoch-receiver-a",
            receiver_id: RECEIVER_A_ID,
            receiver_private_key: RECEIVER_A_PRIVATE,
            cmk: [0x99; 32],
            epoch: u64::MAX,
            ephemeral_private_key: [0x59; 32],
            wrapping_nonce: [0x69; 24],
        },
    ];
    let positives: Vec<JsonValue> = cases
        .iter()
        .map(|case| positive_case(case, &signer))
        .collect();
    let base_bytes = case_record(&cases[0], &signer);
    let base_grant = decode_key_grant(&base_bytes).unwrap();

    let manifest = JsonValue::Object(vec![
        (
            "fixture_format".into(),
            string("D18Q-channel-key-grant-fixtures-v1"),
        ),
        (
            "spec".into(),
            string("code/specs/D18Q-chief-of-staff-channel-key-grant-profile.md"),
        ),
        (
            "generator_blob_sha1".into(),
            string(generator_blob_sha1),
        ),
        (
            "warning".into(),
            string("All CMKs, private keys, shared secrets, wrapping keys, and nonces are deterministic test-only material. Never log them or use them outside conformance tests."),
        ),
        (
            "constants".into(),
            JsonValue::Object(vec![
                ("key_grant_context_ascii".into(), string("chief-channel-key-grant-v1")),
                ("key_wrap_context_ascii".into(), string("chief-channel-key-wrap-v1")),
                ("max_identity_bytes".into(), string("4096")),
                ("wire_magic_ascii".into(), string("D18G")),
                ("wire_version".into(), string("1")),
            ]),
        ),
        (
            "test_signing_key".into(),
            JsonValue::Object(vec![
                ("seed_hex".into(), string(encode_hex(&SIGNING_SEED))),
                ("public_key_hex".into(), string(encode_hex(&signer.public_key()))),
            ]),
        ),
        ("positive_cases".into(), JsonValue::Array(positives)),
        (
            "structural_negative_cases".into(),
            structural_negatives(&base_bytes),
        ),
        (
            "truncated_prefix_recipe".into(),
            JsonValue::Object(vec![
                ("source_case".into(), string("epoch-zero-receiver-a")),
                ("first_length".into(), string("0")),
                ("last_length_exclusive".into(), string(base_bytes.len().to_string())),
                ("expected_error".into(), string("truncated_record")),
            ]),
        ),
        (
            "oversize_recipes".into(),
            JsonValue::Array(vec![
                oversize_recipe("originator_id", 5, 4097),
                oversize_recipe("receiver_id", 9 + ORIGINATOR_ID.len(), 4097),
            ]),
        ),
        (
            "field_negative_cases".into(),
            JsonValue::Array(vec![
                named_error("empty-originator", "invalid_field"),
                named_error("empty-receiver", "invalid_field"),
                named_error("invalid-uuid-version", "invalid_field"),
                named_error("invalid-uuid-variant", "invalid_field"),
                named_error("oversized-originator", "length_limit_exceeded"),
                named_error("oversized-receiver", "length_limit_exceeded"),
            ]),
        ),
        (
            "seal_negative_cases".into(),
            JsonValue::Array(vec![named_error(
                "low-order-receiver-public-key",
                "invalid_key_agreement",
            )]),
        ),
        (
            "opening_negative_cases".into(),
            opening_negatives(&base_grant, &signer),
        ),
        (
            "receiver_state_trace".into(),
            receiver_state_trace(&signer),
        ),
        ("rotation_case".into(), rotation_case(&signer)),
        (
            "secret_erasure_capabilities".into(),
            JsonValue::Array(
                ["guaranteed", "best_effort", "not_enforceable"]
                    .into_iter()
                    .map(string)
                    .collect(),
            ),
        ),
        ("rust_secret_erasure_capability".into(), string("guaranteed")),
        (
            "stable_error_codes".into(),
            JsonValue::Array(
                [
                    "invalid_magic",
                    "unsupported_version",
                    "truncated_record",
                    "trailing_bytes",
                    "length_limit_exceeded",
                    "invalid_field",
                    "randomness_unavailable",
                    "invalid_key_agreement",
                    "key_derivation_failed",
                    "invalid_signature",
                    "unexpected_originator",
                    "unexpected_receiver",
                    "unexpected_channel",
                    "authentication_failed",
                    "invalid_wrapped_key",
                    "conflicting_grant",
                    "decreasing_epoch",
                    "epoch_exhausted",
                    "missing_epoch_key",
                ]
                .into_iter()
                .map(string)
                .collect(),
            ),
        ),
    ]);
    let mut encoded = serialize(&manifest).unwrap();
    encoded.push('\n');
    encoded.into_bytes()
}

fn positive_case(case: &PositiveCase<'_>, signer: &OriginatorSigningKey) -> JsonValue {
    let receiver = ReceiverKeyPair::from_private_key(case.receiver_private_key).unwrap();
    let fields = KeyGrantFields::new(
        ORIGINATOR_ID.to_vec(),
        case.receiver_id.to_vec(),
        ChannelId(CHANNEL_ID),
        KeyEpoch(case.epoch),
    )
    .unwrap();
    let grant = seal_channel_key_with_material(
        &fields,
        &ChannelMasterKey::from_bytes(case.cmk),
        &receiver.public_key(),
        signer,
        case.ephemeral_private_key,
        case.wrapping_nonce,
    )
    .unwrap();
    let d18g = grant_serialize(&grant).unwrap();
    let ephemeral_public = generate_keypair(&case.ephemeral_private_key).unwrap();
    let shared_secret = x25519(&case.ephemeral_private_key, &receiver.public_key()).unwrap();
    let epoch_bytes = case.epoch.to_be_bytes();
    let hkdf_salt = frame(&[&CHANNEL_ID, &epoch_bytes]);
    let hkdf_info = frame(&[KEY_WRAP_CONTEXT, case.receiver_id]);
    let wrapping_key = hkdf(
        &hkdf_salt,
        &shared_secret,
        &hkdf_info,
        32,
        HashAlgorithm::Sha256,
    )
    .unwrap();
    let aad = grant_aad(
        ORIGINATOR_ID,
        case.receiver_id,
        KeyEpoch(case.epoch),
        &ephemeral_public,
    );
    let signature_input = grant_signature_input(
        ORIGINATOR_ID,
        case.receiver_id,
        CHANNEL_ID,
        KeyEpoch(case.epoch),
        &ephemeral_public,
        &case.wrapping_nonce,
        &grant.wrapped_cmk(),
    );
    assert_eq!(
        sign(&signature_input, &signer_secret(signer)),
        grant.originator_signature()
    );

    JsonValue::Object(vec![
        ("name".into(), string(case.name)),
        (
            "originator_id_b64".into(),
            string(encode_base64(ORIGINATOR_ID)),
        ),
        (
            "receiver_id_b64".into(),
            string(encode_base64(case.receiver_id)),
        ),
        ("channel_id_hex".into(), string(encode_hex(&CHANNEL_ID))),
        ("key_epoch".into(), string(case.epoch.to_string())),
        ("cmk_hex".into(), string(encode_hex(&case.cmk))),
        (
            "receiver_private_key_hex".into(),
            string(encode_hex(&case.receiver_private_key)),
        ),
        (
            "receiver_public_key_hex".into(),
            string(encode_hex(&receiver.public_key())),
        ),
        (
            "ephemeral_private_key_hex".into(),
            string(encode_hex(&case.ephemeral_private_key)),
        ),
        (
            "ephemeral_public_key_hex".into(),
            string(encode_hex(&ephemeral_public)),
        ),
        (
            "shared_secret_hex".into(),
            string(encode_hex(&shared_secret)),
        ),
        ("hkdf_salt_b64".into(), string(encode_base64(&hkdf_salt))),
        ("hkdf_info_b64".into(), string(encode_base64(&hkdf_info))),
        ("wrapping_key_hex".into(), string(encode_hex(&wrapping_key))),
        (
            "wrapping_nonce_hex".into(),
            string(encode_hex(&case.wrapping_nonce)),
        ),
        ("grant_aad_b64".into(), string(encode_base64(&aad))),
        (
            "wrapped_cmk_hex".into(),
            string(encode_hex(&grant.wrapped_cmk())),
        ),
        (
            "signature_input_b64".into(),
            string(encode_base64(&signature_input)),
        ),
        (
            "signature_hex".into(),
            string(encode_hex(&grant.originator_signature())),
        ),
        ("d18g_b64".into(), string(encode_base64(&d18g))),
        (
            "expected_opened_cmk_hex".into(),
            string(encode_hex(&case.cmk)),
        ),
    ])
}

fn case_record(case: &PositiveCase<'_>, signer: &OriginatorSigningKey) -> Vec<u8> {
    let receiver = ReceiverKeyPair::from_private_key(case.receiver_private_key).unwrap();
    let fields = KeyGrantFields::new(
        ORIGINATOR_ID.to_vec(),
        case.receiver_id.to_vec(),
        ChannelId(CHANNEL_ID),
        KeyEpoch(case.epoch),
    )
    .unwrap();
    let grant = seal_channel_key_with_material(
        &fields,
        &ChannelMasterKey::from_bytes(case.cmk),
        &receiver.public_key(),
        signer,
        case.ephemeral_private_key,
        case.wrapping_nonce,
    )
    .unwrap();
    grant_serialize(&grant).unwrap()
}

fn structural_negatives(base: &[u8]) -> JsonValue {
    let mut wrong_magic = base.to_vec();
    wrong_magic[0] = b'X';
    let mut wrong_version = base.to_vec();
    wrong_version[4] = 2;
    let mut trailing = base.to_vec();
    trailing.push(0);
    JsonValue::Array(vec![
        record_error("wrong-magic", &wrong_magic, "invalid_magic"),
        record_error("unsupported-version", &wrong_version, "unsupported_version"),
        record_error("trailing-byte", &trailing, "trailing_bytes"),
    ])
}

fn opening_negatives(base: &SealedChannelKeyGrant, signer: &OriginatorSigningKey) -> JsonValue {
    let mut cases = Vec::new();
    cases.push(opening_case(
        "unexpected-originator",
        base.clone(),
        b"other-originator",
        RECEIVER_A_ID,
        CHANNEL_ID,
        RECEIVER_A_PRIVATE,
        "unexpected_originator",
    ));
    cases.push(opening_case(
        "unexpected-receiver",
        base.clone(),
        ORIGINATOR_ID,
        b"other-receiver",
        CHANNEL_ID,
        RECEIVER_A_PRIVATE,
        "unexpected_receiver",
    ));
    cases.push(opening_case(
        "unexpected-channel",
        base.clone(),
        ORIGINATOR_ID,
        RECEIVER_A_ID,
        OTHER_CHANNEL_ID,
        RECEIVER_A_PRIVATE,
        "unexpected_channel",
    ));
    let mut bad_signature = base.clone();
    bad_signature.originator_signature[0] ^= 1;
    cases.push(opening_case(
        "invalid-signature",
        bad_signature,
        ORIGINATOR_ID,
        RECEIVER_A_ID,
        CHANNEL_ID,
        RECEIVER_A_PRIVATE,
        "invalid_signature",
    ));
    let mut signature_before_agreement = base.clone();
    signature_before_agreement.ephemeral_public_key = [0; 32];
    cases.push(opening_case(
        "invalid-signature-before-key-agreement",
        signature_before_agreement,
        ORIGINATOR_ID,
        RECEIVER_A_ID,
        CHANNEL_ID,
        RECEIVER_A_PRIVATE,
        "invalid_signature",
    ));
    let mut low_order = base.clone();
    low_order.ephemeral_public_key = [0; 32];
    resign(&mut low_order, signer);
    cases.push(opening_case(
        "low-order-ephemeral-public-key",
        low_order,
        ORIGINATOR_ID,
        RECEIVER_A_ID,
        CHANNEL_ID,
        RECEIVER_A_PRIVATE,
        "invalid_key_agreement",
    ));
    cases.push(opening_case(
        "wrong-receiver-private-key",
        base.clone(),
        ORIGINATOR_ID,
        RECEIVER_A_ID,
        CHANNEL_ID,
        [0x7f; 32],
        "authentication_failed",
    ));
    let mut nonce = base.clone();
    nonce.wrapping_nonce[0] ^= 1;
    resign(&mut nonce, signer);
    cases.push(opening_case(
        "wrong-wrapping-nonce",
        nonce,
        ORIGINATOR_ID,
        RECEIVER_A_ID,
        CHANNEL_ID,
        RECEIVER_A_PRIVATE,
        "authentication_failed",
    ));
    for (name, index) in [("mutated-wrapped-cmk", 0usize), ("mutated-tag", 47usize)] {
        let mut grant = base.clone();
        grant.wrapped_cmk[index] ^= 1;
        resign(&mut grant, signer);
        cases.push(opening_case(
            name,
            grant,
            ORIGINATOR_ID,
            RECEIVER_A_ID,
            CHANNEL_ID,
            RECEIVER_A_PRIVATE,
            "authentication_failed",
        ));
    }
    let mut epoch = base.clone();
    epoch.key_epoch = KeyEpoch(1);
    resign(&mut epoch, signer);
    cases.push(opening_case(
        "epoch-derivation-binding",
        epoch,
        ORIGINATOR_ID,
        RECEIVER_A_ID,
        CHANNEL_ID,
        RECEIVER_A_PRIVATE,
        "authentication_failed",
    ));
    let mut receiver = base.clone();
    receiver.receiver_id = b"receiver-A-alias".to_vec();
    resign(&mut receiver, signer);
    cases.push(opening_case(
        "receiver-derivation-binding",
        receiver,
        ORIGINATOR_ID,
        b"receiver-A-alias",
        CHANNEL_ID,
        RECEIVER_A_PRIVATE,
        "authentication_failed",
    ));
    let mut channel = base.clone();
    channel.channel_id = ChannelId(OTHER_CHANNEL_ID);
    resign(&mut channel, signer);
    cases.push(opening_case(
        "channel-aad-binding",
        channel,
        ORIGINATOR_ID,
        RECEIVER_A_ID,
        OTHER_CHANNEL_ID,
        RECEIVER_A_PRIVATE,
        "authentication_failed",
    ));
    let mut originator = base.clone();
    originator.originator_id = b"originator-alias".to_vec();
    resign(&mut originator, signer);
    cases.push(opening_case(
        "originator-aad-binding",
        originator,
        b"originator-alias",
        RECEIVER_A_ID,
        CHANNEL_ID,
        RECEIVER_A_PRIVATE,
        "authentication_failed",
    ));
    JsonValue::Array(cases)
}

fn receiver_state_trace(signer: &OriginatorSigningKey) -> JsonValue {
    let base_case = PositiveCase {
        name: "base",
        receiver_id: RECEIVER_A_ID,
        receiver_private_key: RECEIVER_A_PRIVATE,
        cmk: EPOCH_ZERO_CMK,
        epoch: 0,
        ephemeral_private_key: EPHEMERAL_A,
        wrapping_nonce: NONCE_A,
    };
    let conflict_case = PositiveCase {
        name: "conflict",
        cmk: [0x23; 32],
        ephemeral_private_key: [0x53; 32],
        wrapping_nonce: [0x63; 24],
        ..base_case
    };
    let skipped_case = PositiveCase {
        name: "skipped-epoch-three",
        cmk: [0x73; 32],
        epoch: 3,
        ephemeral_private_key: [0x54; 32],
        wrapping_nonce: [0x64; 24],
        ..base_case
    };
    let mut failed_higher = decode_key_grant(&case_record(
        &PositiveCase {
            name: "failed-higher",
            cmk: [0x24; 32],
            epoch: 2,
            ephemeral_private_key: [0x55; 32],
            wrapping_nonce: [0x65; 24],
            ..base_case
        },
        signer,
    ))
    .unwrap();
    failed_higher.wrapped_cmk[0] ^= 1;
    resign(&mut failed_higher, signer);
    JsonValue::Object(vec![
        (
            "grants".into(),
            JsonValue::Object(vec![
                (
                    "epoch_zero_b64".into(),
                    string(encode_base64(&case_record(&base_case, signer))),
                ),
                (
                    "same_epoch_conflict_b64".into(),
                    string(encode_base64(&case_record(&conflict_case, signer))),
                ),
                (
                    "failed_higher_epoch_b64".into(),
                    string(encode_base64(&encode_key_grant(&failed_higher).unwrap())),
                ),
                (
                    "skipped_epoch_three_b64".into(),
                    string(encode_base64(&case_record(&skipped_case, signer))),
                ),
            ]),
        ),
        (
            "steps".into(),
            JsonValue::Array(vec![
                trace_step(
                    "install-epoch-zero",
                    "epoch_zero_b64",
                    "installed",
                    "0",
                    &[0],
                ),
                trace_step(
                    "retry-epoch-zero",
                    "epoch_zero_b64",
                    "idempotent",
                    "0",
                    &[0],
                ),
                trace_step(
                    "same-epoch-conflict",
                    "same_epoch_conflict_b64",
                    "conflicting_grant",
                    "0",
                    &[0],
                ),
                trace_step(
                    "failed-higher-open",
                    "failed_higher_epoch_b64",
                    "authentication_failed",
                    "0",
                    &[0],
                ),
                trace_step(
                    "install-skipped-epoch-three",
                    "skipped_epoch_three_b64",
                    "installed",
                    "3",
                    &[0, 3],
                ),
                trace_step(
                    "decreasing-epoch",
                    "epoch_zero_b64",
                    "decreasing_epoch",
                    "3",
                    &[0, 3],
                ),
            ]),
        ),
        ("missing_epoch".into(), string("1")),
        ("missing_epoch_error".into(), string("missing_epoch_key")),
    ])
}

fn rotation_case(signer: &OriginatorSigningKey) -> JsonValue {
    let receiver_b = ReceiverKeyPair::from_private_key(RECEIVER_B_PRIVATE).unwrap();
    let plan = plan_rotation(
        ORIGINATOR_ID,
        ChannelId(CHANNEL_ID),
        KeyEpoch(0),
        ChannelMasterKey::from_bytes(EPOCH_ONE_CMK),
        vec![RotationReceiver::with_material(
            RECEIVER_B_ID.to_vec(),
            receiver_b.public_key(),
            [0x71; 32],
            [0x81; 24],
        )
        .unwrap()],
        signer,
    )
    .unwrap();
    JsonValue::Object(vec![
        ("name".into(), string("receivers-a-plus-b-to-b-only")),
        ("current_epoch".into(), string("0")),
        ("new_epoch".into(), string(plan.new_epoch().0.to_string())),
        (
            "new_cmk_hex".into(),
            string(encode_hex(plan.new_cmk().as_bytes())),
        ),
        (
            "authorized_receiver_ids_b64".into(),
            JsonValue::Array(vec![string(encode_base64(RECEIVER_B_ID))]),
        ),
        (
            "new_grants_b64".into(),
            JsonValue::Array(
                plan.grants()
                    .iter()
                    .map(|grant| string(encode_base64(&grant_serialize(grant).unwrap())))
                    .collect(),
            ),
        ),
        (
            "receiver_a_retains_epochs".into(),
            JsonValue::Array(vec![string("0")]),
        ),
        (
            "receiver_b_retains_epochs".into(),
            JsonValue::Array(vec![string("0"), string("1")]),
        ),
        ("receiver_a_new_grant".into(), JsonValue::Null),
    ])
}

fn trace_step(name: &str, grant: &str, expected: &str, latest: &str, keys: &[u64]) -> JsonValue {
    JsonValue::Object(vec![
        ("name".into(), string(name)),
        ("grant".into(), string(grant)),
        ("expected".into(), string(expected)),
        ("latest_epoch".into(), string(latest)),
        (
            "retained_epochs".into(),
            JsonValue::Array(keys.iter().map(|value| string(value.to_string())).collect()),
        ),
    ])
}

fn opening_case(
    name: &str,
    grant: SealedChannelKeyGrant,
    originator: &[u8],
    receiver: &[u8],
    channel: [u8; 16],
    receiver_private: [u8; 32],
    expected: &str,
) -> JsonValue {
    JsonValue::Object(vec![
        ("name".into(), string(name)),
        (
            "d18g_b64".into(),
            string(encode_base64(&encode_key_grant(&grant).unwrap())),
        ),
        (
            "expected_originator_id_b64".into(),
            string(encode_base64(originator)),
        ),
        (
            "expected_receiver_id_b64".into(),
            string(encode_base64(receiver)),
        ),
        (
            "expected_channel_id_hex".into(),
            string(encode_hex(&channel)),
        ),
        (
            "receiver_private_key_hex".into(),
            string(encode_hex(&receiver_private)),
        ),
        ("expected_error".into(), string(expected)),
    ])
}

fn resign(grant: &mut SealedChannelKeyGrant, signer: &OriginatorSigningKey) {
    let input = grant_signature_input(
        &grant.originator_id,
        &grant.receiver_id,
        grant.channel_id.0,
        grant.key_epoch,
        &grant.ephemeral_public_key,
        &grant.wrapping_nonce,
        &grant.wrapped_cmk,
    );
    grant.originator_signature = sign(&input, &signer_secret(signer));
}

fn signer_secret(signer: &OriginatorSigningKey) -> [u8; 64] {
    let (_, secret) = coding_adventures_ed25519::generate_keypair(&SIGNING_SEED);
    let embedded_public: [u8; 32] = secret[32..].try_into().unwrap();
    assert_eq!(signer.public_key(), embedded_public);
    secret
}

fn grant_aad(
    originator_id: &[u8],
    receiver_id: &[u8],
    epoch: KeyEpoch,
    ephemeral_public_key: &[u8; 32],
) -> Vec<u8> {
    let epoch = epoch.0.to_be_bytes();
    frame(&[
        KEY_GRANT_CONTEXT,
        originator_id,
        &CHANNEL_ID,
        &epoch,
        receiver_id,
        ephemeral_public_key,
    ])
}

fn grant_signature_input(
    originator_id: &[u8],
    receiver_id: &[u8],
    channel_id: [u8; 16],
    epoch: KeyEpoch,
    ephemeral_public_key: &[u8; 32],
    nonce: &[u8; 24],
    wrapped_cmk: &[u8; 48],
) -> Vec<u8> {
    let epoch = epoch.0.to_be_bytes();
    frame(&[
        KEY_GRANT_CONTEXT,
        originator_id,
        &channel_id,
        &epoch,
        receiver_id,
        ephemeral_public_key,
        nonce,
        wrapped_cmk,
    ])
}

fn frame(fields: &[&[u8]]) -> Vec<u8> {
    let mut output = Vec::new();
    for field in fields {
        output.extend_from_slice(&(field.len() as u64).to_be_bytes());
        output.extend_from_slice(field);
    }
    output
}

fn record_error(name: &str, record: &[u8], expected: &str) -> JsonValue {
    JsonValue::Object(vec![
        ("name".into(), string(name)),
        ("d18g_b64".into(), string(encode_base64(record))),
        ("expected_error".into(), string(expected)),
    ])
}

fn named_error(name: &str, expected: &str) -> JsonValue {
    JsonValue::Object(vec![
        ("name".into(), string(name)),
        ("expected_error".into(), string(expected)),
    ])
}

fn oversize_recipe(field: &str, length_offset: usize, declared_length: usize) -> JsonValue {
    JsonValue::Object(vec![
        ("field".into(), string(field)),
        ("length_offset".into(), string(length_offset.to_string())),
        (
            "declared_length".into(),
            string(declared_length.to_string()),
        ),
        ("expected_error".into(), string("length_limit_exceeded")),
    ])
}

fn string(value: impl Into<String>) -> JsonValue {
    JsonValue::String(value.into())
}

fn encode_base64(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let word = ((chunk[0] as u32) << 16)
            | ((chunk.get(1).copied().unwrap_or(0) as u32) << 8)
            | chunk.get(2).copied().unwrap_or(0) as u32;
        output.push(BASE64[((word >> 18) & 0x3f) as usize] as char);
        output.push(BASE64[((word >> 12) & 0x3f) as usize] as char);
        output.push(if chunk.len() > 1 {
            BASE64[((word >> 6) & 0x3f) as usize] as char
        } else {
            '='
        });
        output.push(if chunk.len() > 2 {
            BASE64[(word & 0x3f) as usize] as char
        } else {
            '='
        });
    }
    output
}

fn encode_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}
