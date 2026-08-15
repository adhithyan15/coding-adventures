use std::fs;
use std::path::PathBuf;

use chief_of_staff_channel_crypto::grant_profile::{
    grant_deserialize, grant_serialize, open_channel_key_grant, plan_rotation,
    seal_channel_key_with_material, secret_erasure_capability, KeyGrantFields,
    KeyGrantProfileError, ReceiverEpochKeys, RotationReceiver,
};
use chief_of_staff_channel_crypto::{
    ChannelId, ChannelMasterKey, GrantInstallOutcome, KeyEpoch, OriginatorSigningKey,
    ReceiverKeyPair,
};
use coding_adventures_hkdf::{hkdf, HashAlgorithm};
use coding_adventures_json_parser::try_parse_json;
use coding_adventures_json_value::{from_ast, JsonValue};
use coding_adventures_sha1::sum1;
use coding_adventures_x25519::{generate_keypair, x25519};

#[path = "../examples/generate_d18q_fixtures.rs"]
mod fixture_generator;

const KEY_GRANT_CONTEXT: &[u8] = b"chief-channel-key-grant-v1";
const KEY_WRAP_CONTEXT: &[u8] = b"chief-channel-key-wrap-v1";
const CHANNEL_ID: [u8; 16] = [
    0x01, 0x8f, 0x47, 0xa0, 0x9b, 0x6c, 0x7d, 0xef, 0x92, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0,
];

#[test]
fn checked_manifest_is_byte_identical_to_the_recorded_generator() {
    let bytes = manifest_bytes();
    let manifest = parse_manifest(&bytes);
    let root = object(&manifest);
    assert_eq!(
        string(root, "fixture_format"),
        "D18Q-channel-key-grant-fixtures-v1"
    );
    assert!(string(root, "warning").contains("test-only"));
    assert!(string(root, "warning").contains("Never log"));
    let generator_blob_sha1 = string(root, "generator_blob_sha1");
    assert_eq!(generator_blob_sha1.len(), 40);
    let generator_source = include_bytes!("../examples/generate_d18q_fixtures.rs");
    let mut git_blob = format!("blob {}\0", generator_source.len()).into_bytes();
    git_blob.extend_from_slice(generator_source);
    assert_eq!(encode_hex(&sum1(&git_blob)), generator_blob_sha1);
    assert_eq!(
        fixture_generator::generate_manifest(generator_blob_sha1),
        bytes
    );
}

#[test]
fn positives_lock_every_derivation_input_and_production_d18g_byte() {
    let manifest = parse_manifest(&manifest_bytes());
    let root = object(&manifest);
    let signing = object(field(root, "test_signing_key"));
    let signing_seed: [u8; 32] = decode_hex(string(signing, "seed_hex")).try_into().unwrap();
    let signer = OriginatorSigningKey::from_seed(signing_seed);
    let expected_signing_public: [u8; 32] = decode_hex(string(signing, "public_key_hex"))
        .try_into()
        .unwrap();
    assert_eq!(signer.public_key(), expected_signing_public);
    let positives = array(field(root, "positive_cases"));
    assert_eq!(positives.len(), 3);
    assert_eq!(
        positives
            .iter()
            .map(|value| string(object(value), "name"))
            .collect::<Vec<_>>(),
        vec![
            "epoch-zero-receiver-a",
            "epoch-zero-receiver-b",
            "maximum-epoch-receiver-a"
        ]
    );

    for case in positives {
        let case = object(case);
        let name = string(case, "name");
        let originator_id = decode_base64(string(case, "originator_id_b64"));
        let receiver_id = decode_base64(string(case, "receiver_id_b64"));
        let channel = ChannelId(
            decode_hex(string(case, "channel_id_hex"))
                .try_into()
                .unwrap(),
        );
        let epoch = KeyEpoch(string(case, "key_epoch").parse().unwrap());
        let cmk_bytes: [u8; 32] = decode_hex(string(case, "cmk_hex")).try_into().unwrap();
        let receiver_private: [u8; 32] = decode_hex(string(case, "receiver_private_key_hex"))
            .try_into()
            .unwrap();
        let receiver = ReceiverKeyPair::from_private_key(receiver_private).unwrap();
        let expected_receiver_public: [u8; 32] =
            decode_hex(string(case, "receiver_public_key_hex"))
                .try_into()
                .unwrap();
        assert_eq!(
            receiver.public_key(),
            expected_receiver_public,
            "{name}: receiver public key"
        );
        let ephemeral_private: [u8; 32] = decode_hex(string(case, "ephemeral_private_key_hex"))
            .try_into()
            .unwrap();
        let ephemeral_public = generate_keypair(&ephemeral_private).unwrap();
        let expected_ephemeral_public: [u8; 32] =
            decode_hex(string(case, "ephemeral_public_key_hex"))
                .try_into()
                .unwrap();
        assert_eq!(
            ephemeral_public, expected_ephemeral_public,
            "{name}: ephemeral public key"
        );
        let shared_secret = x25519(&ephemeral_private, &receiver.public_key()).unwrap();
        let expected_shared_secret: [u8; 32] = decode_hex(string(case, "shared_secret_hex"))
            .try_into()
            .unwrap();
        assert_eq!(
            shared_secret, expected_shared_secret,
            "{name}: shared secret"
        );
        let epoch_bytes = epoch.0.to_be_bytes();
        let salt = frame(&[&channel.0, &epoch_bytes]);
        let info = frame(&[KEY_WRAP_CONTEXT, &receiver_id]);
        assert_eq!(
            salt,
            decode_base64(string(case, "hkdf_salt_b64")),
            "{name}: salt"
        );
        assert_eq!(
            info,
            decode_base64(string(case, "hkdf_info_b64")),
            "{name}: info"
        );
        assert_eq!(
            hkdf(&salt, &shared_secret, &info, 32, HashAlgorithm::Sha256).unwrap(),
            decode_hex(string(case, "wrapping_key_hex")),
            "{name}: wrapping key"
        );
        let fields =
            KeyGrantFields::new(originator_id.clone(), receiver_id.clone(), channel, epoch)
                .unwrap();
        let nonce: [u8; 24] = decode_hex(string(case, "wrapping_nonce_hex"))
            .try_into()
            .unwrap();
        let grant = seal_channel_key_with_material(
            &fields,
            &ChannelMasterKey::from_bytes(cmk_bytes),
            &receiver.public_key(),
            &signer,
            ephemeral_private,
            nonce,
        )
        .unwrap();
        let record = decode_base64(string(case, "d18g_b64"));
        assert_eq!(grant_serialize(&grant).unwrap(), record, "{name}: D18G");
        assert_eq!(
            grant.wrapped_cmk().to_vec(),
            decode_hex(string(case, "wrapped_cmk_hex"))
        );
        assert_eq!(
            grant.originator_signature().to_vec(),
            decode_hex(string(case, "signature_hex"))
        );
        let aad = frame(&[
            KEY_GRANT_CONTEXT,
            &originator_id,
            &channel.0,
            &epoch_bytes,
            &receiver_id,
            &ephemeral_public,
        ]);
        assert_eq!(
            aad,
            decode_base64(string(case, "grant_aad_b64")),
            "{name}: AAD"
        );
        let signature_input = frame(&[
            KEY_GRANT_CONTEXT,
            &originator_id,
            &channel.0,
            &epoch_bytes,
            &receiver_id,
            &ephemeral_public,
            &nonce,
            &grant.wrapped_cmk(),
        ]);
        assert_eq!(
            signature_input,
            decode_base64(string(case, "signature_input_b64")),
            "{name}: signature input"
        );
        let decoded = grant_deserialize(&record).unwrap();
        assert_eq!(
            grant_serialize(&decoded).unwrap(),
            record,
            "{name}: round trip"
        );
        let opened = open_channel_key_grant(
            &decoded,
            &originator_id,
            &receiver_id,
            channel,
            &receiver,
            &signer.public_key(),
        )
        .unwrap();
        assert_eq!(
            opened.as_bytes().as_slice(),
            decode_hex(string(case, "expected_opened_cmk_hex")),
            "{name}: opened CMK"
        );
    }
}

#[test]
fn structural_and_high_level_failures_have_the_declared_codes() {
    let manifest = parse_manifest(&manifest_bytes());
    let root = object(&manifest);
    let positives = array(field(root, "positive_cases"));
    let base = decode_base64(string(object(&positives[0]), "d18g_b64"));

    for case in array(field(root, "structural_negative_cases")) {
        let case = object(case);
        let error = grant_deserialize(&decode_base64(string(case, "d18g_b64"))).unwrap_err();
        assert_eq!(
            error.code(),
            string(case, "expected_error"),
            "{}",
            string(case, "name")
        );
    }

    let recipe = object(field(root, "truncated_prefix_recipe"));
    let last: usize = string(recipe, "last_length_exclusive").parse().unwrap();
    assert_eq!(last, base.len());
    for end in 0..last {
        assert_eq!(
            grant_deserialize(&base[..end]).unwrap_err().code(),
            string(recipe, "expected_error"),
            "truncated prefix {end}"
        );
    }

    for recipe in array(field(root, "oversize_recipes")) {
        let recipe = object(recipe);
        let offset: usize = string(recipe, "length_offset").parse().unwrap();
        let length: u32 = string(recipe, "declared_length").parse().unwrap();
        let mut record = base.clone();
        record[offset..offset + 4].copy_from_slice(&length.to_be_bytes());
        assert_eq!(
            grant_deserialize(&record).unwrap_err().code(),
            string(recipe, "expected_error"),
            "{}",
            string(recipe, "field")
        );
    }

    for case in array(field(root, "field_negative_cases")) {
        let case = object(case);
        let name = string(case, "name");
        let mut originator = b"originator".to_vec();
        let mut receiver = b"receiver".to_vec();
        let mut channel = CHANNEL_ID;
        match name {
            "empty-originator" => originator.clear(),
            "empty-receiver" => receiver.clear(),
            "invalid-uuid-version" => channel[6] = 0x60,
            "invalid-uuid-variant" => channel[8] = 0x10,
            "oversized-originator" => originator = vec![0; 4097],
            "oversized-receiver" => receiver = vec![0; 4097],
            _ => panic!("unknown field case {name}"),
        }
        let error =
            KeyGrantFields::new(originator, receiver, ChannelId(channel), KeyEpoch(0)).unwrap_err();
        assert_eq!(error.code(), string(case, "expected_error"), "{name}");
    }

    let seal_case = object(&array(field(root, "seal_negative_cases"))[0]);
    let fields = KeyGrantFields::new(
        b"originator".to_vec(),
        b"receiver".to_vec(),
        ChannelId(CHANNEL_ID),
        KeyEpoch(0),
    )
    .unwrap();
    let error = seal_channel_key_with_material(
        &fields,
        &ChannelMasterKey::from_bytes([0x22; 32]),
        &[0; 32],
        &OriginatorSigningKey::from_seed([0x11; 32]),
        [0x51; 32],
        [0x61; 24],
    )
    .unwrap_err();
    assert_eq!(error.code(), string(seal_case, "expected_error"));
}

#[test]
fn opening_failures_follow_the_declared_validation_order() {
    let manifest = parse_manifest(&manifest_bytes());
    let root = object(&manifest);
    let signing = object(field(root, "test_signing_key"));
    let public_key: [u8; 32] = decode_hex(string(signing, "public_key_hex"))
        .try_into()
        .unwrap();
    let cases = array(field(root, "opening_negative_cases"));
    assert_eq!(cases.len(), 14);
    for case in cases {
        let case = object(case);
        let name = string(case, "name");
        let grant = grant_deserialize(&decode_base64(string(case, "d18g_b64"))).unwrap();
        let receiver_private: [u8; 32] = decode_hex(string(case, "receiver_private_key_hex"))
            .try_into()
            .unwrap();
        let receiver = ReceiverKeyPair::from_private_key(receiver_private).unwrap();
        let channel = ChannelId(
            decode_hex(string(case, "expected_channel_id_hex"))
                .try_into()
                .unwrap(),
        );
        let error = open_channel_key_grant(
            &grant,
            &decode_base64(string(case, "expected_originator_id_b64")),
            &decode_base64(string(case, "expected_receiver_id_b64")),
            channel,
            &receiver,
            &public_key,
        )
        .err()
        .unwrap();
        assert_eq!(error.code(), string(case, "expected_error"), "{name}");
    }
}

#[test]
fn receiver_trace_is_atomic_monotonic_and_allows_skipped_epochs() {
    let manifest = parse_manifest(&manifest_bytes());
    let root = object(&manifest);
    let signing = object(field(root, "test_signing_key"));
    let public_key: [u8; 32] = decode_hex(string(signing, "public_key_hex"))
        .try_into()
        .unwrap();
    let positive = object(&array(field(root, "positive_cases"))[0]);
    let originator = decode_base64(string(positive, "originator_id_b64"));
    let receiver_id = decode_base64(string(positive, "receiver_id_b64"));
    let receiver_private: [u8; 32] = decode_hex(string(positive, "receiver_private_key_hex"))
        .try_into()
        .unwrap();
    let mut state = ReceiverEpochKeys::new(
        originator,
        receiver_id,
        ChannelId(CHANNEL_ID),
        ReceiverKeyPair::from_private_key(receiver_private).unwrap(),
        public_key,
    )
    .unwrap();
    let trace = object(field(root, "receiver_state_trace"));
    let grants = object(field(trace, "grants"));
    for step in array(field(trace, "steps")) {
        let step = object(step);
        let name = string(step, "name");
        let grant =
            grant_deserialize(&decode_base64(string(grants, string(step, "grant")))).unwrap();
        let actual = match state.install_grant(grant) {
            Ok(GrantInstallOutcome::Installed) => "installed",
            Ok(GrantInstallOutcome::Idempotent) => "idempotent",
            Err(error) => error.code(),
        };
        assert_eq!(actual, string(step, "expected"), "{name}");
        assert_eq!(
            state.latest_epoch().unwrap().0.to_string(),
            string(step, "latest_epoch"),
            "{name}: latest"
        );
        let expected: Vec<u64> = array(field(step, "retained_epochs"))
            .iter()
            .map(|value| json_string(value).parse().unwrap())
            .collect();
        let actual: Vec<u64> = (0..=3)
            .filter(|epoch| state.key(KeyEpoch(*epoch)).is_ok())
            .collect();
        assert_eq!(actual, expected, "{name}: retained keys");
    }
    let missing: u64 = string(trace, "missing_epoch").parse().unwrap();
    assert_eq!(
        state.key(KeyEpoch(missing)).err().unwrap().code(),
        string(trace, "missing_epoch_error")
    );
}

#[test]
fn rotation_revokes_a_prospectively_and_reproduces_the_ordered_plan() {
    let manifest = parse_manifest(&manifest_bytes());
    let root = object(&manifest);
    let positives = array(field(root, "positive_cases"));
    let a = object(&positives[0]);
    let b = object(&positives[1]);
    let signing = object(field(root, "test_signing_key"));
    let signing_seed: [u8; 32] = decode_hex(string(signing, "seed_hex")).try_into().unwrap();
    let signer = OriginatorSigningKey::from_seed(signing_seed);
    let public = signer.public_key();
    let originator = decode_base64(string(a, "originator_id_b64"));
    let receiver_a_id = decode_base64(string(a, "receiver_id_b64"));
    let receiver_b_id = decode_base64(string(b, "receiver_id_b64"));
    let receiver_a_private: [u8; 32] = decode_hex(string(a, "receiver_private_key_hex"))
        .try_into()
        .unwrap();
    let receiver_b_private: [u8; 32] = decode_hex(string(b, "receiver_private_key_hex"))
        .try_into()
        .unwrap();
    let receiver_a = ReceiverKeyPair::from_private_key(receiver_a_private).unwrap();
    let receiver_b = ReceiverKeyPair::from_private_key(receiver_b_private).unwrap();
    let receiver_b_public = receiver_b.public_key();
    let mut state_a = ReceiverEpochKeys::new(
        originator.clone(),
        receiver_a_id,
        ChannelId(CHANNEL_ID),
        receiver_a,
        public,
    )
    .unwrap();
    let mut state_b = ReceiverEpochKeys::new(
        originator.clone(),
        receiver_b_id.clone(),
        ChannelId(CHANNEL_ID),
        receiver_b,
        public,
    )
    .unwrap();
    state_a
        .install_grant(grant_deserialize(&decode_base64(string(a, "d18g_b64"))).unwrap())
        .unwrap();
    state_b
        .install_grant(grant_deserialize(&decode_base64(string(b, "d18g_b64"))).unwrap())
        .unwrap();
    let rotation = object(field(root, "rotation_case"));
    let plan = plan_rotation(
        &originator,
        ChannelId(CHANNEL_ID),
        KeyEpoch(0),
        ChannelMasterKey::from_bytes(
            decode_hex(string(rotation, "new_cmk_hex"))
                .try_into()
                .unwrap(),
        ),
        vec![RotationReceiver::with_material(
            receiver_b_id,
            receiver_b_public,
            [0x71; 32],
            [0x81; 24],
        )
        .unwrap()],
        &signer,
    )
    .unwrap();
    let expected_grants = array(field(rotation, "new_grants_b64"));
    assert_eq!(plan.grants().len(), 1);
    assert_eq!(
        grant_serialize(&plan.grants()[0]).unwrap(),
        decode_base64(json_string(&expected_grants[0]))
    );
    state_b.install_grant(plan.grants()[0].clone()).unwrap();
    assert!(state_a.key(KeyEpoch(0)).is_ok());
    assert_eq!(
        state_a.key(KeyEpoch(1)).err().unwrap(),
        KeyGrantProfileError::MissingEpochKey
    );
    assert!(state_b.key(KeyEpoch(0)).is_ok());
    assert_eq!(
        state_b.key(KeyEpoch(1)).unwrap().as_bytes(),
        plan.new_cmk().as_bytes()
    );
    assert_eq!(
        plan.new_epoch().0.to_string(),
        string(rotation, "new_epoch")
    );

    let exhausted = plan_rotation(
        &originator,
        ChannelId(CHANNEL_ID),
        KeyEpoch(u64::MAX),
        ChannelMasterKey::from_bytes([0x33; 32]),
        vec![RotationReceiver::with_material(
            b"receiver".to_vec(),
            receiver_b_public,
            [0x71; 32],
            [0x81; 24],
        )
        .unwrap()],
        &signer,
    );
    assert_eq!(
        exhausted.err().unwrap(),
        KeyGrantProfileError::EpochExhausted
    );
}

#[test]
fn error_and_erasure_vocabulary_is_closed() {
    let manifest = parse_manifest(&manifest_bytes());
    let root = object(&manifest);
    let actual: Vec<&str> = array(field(root, "stable_error_codes"))
        .iter()
        .map(json_string)
        .collect();
    assert_eq!(
        actual,
        vec![
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
    );
    assert_eq!(
        array(field(root, "secret_erasure_capabilities"))
            .iter()
            .map(json_string)
            .collect::<Vec<_>>(),
        vec!["guaranteed", "best_effort", "not_enforceable"]
    );
    assert_eq!(
        secret_erasure_capability().as_str(),
        string(root, "rust_secret_erasure_capability")
    );
}

#[test]
fn manifest_fields_and_case_rosters_are_closed() {
    let manifest = parse_manifest(&manifest_bytes());
    let root = object(&manifest);
    assert_eq!(
        keys(root),
        vec![
            "fixture_format",
            "spec",
            "generator_blob_sha1",
            "warning",
            "constants",
            "test_signing_key",
            "positive_cases",
            "structural_negative_cases",
            "truncated_prefix_recipe",
            "oversize_recipes",
            "field_negative_cases",
            "seal_negative_cases",
            "opening_negative_cases",
            "receiver_state_trace",
            "rotation_case",
            "secret_erasure_capabilities",
            "rust_secret_erasure_capability",
            "stable_error_codes",
        ]
    );
    assert_eq!(
        string(root, "spec"),
        "code/specs/D18Q-chief-of-staff-channel-key-grant-profile.md"
    );
    assert_eq!(
        keys(object(field(root, "constants"))),
        vec![
            "key_grant_context_ascii",
            "key_wrap_context_ascii",
            "max_identity_bytes",
            "wire_magic_ascii",
            "wire_version",
        ]
    );
    assert_eq!(
        keys(object(field(root, "test_signing_key"))),
        vec!["seed_hex", "public_key_hex"]
    );
    for case in array(field(root, "positive_cases")) {
        assert_eq!(
            keys(object(case)),
            vec![
                "name",
                "originator_id_b64",
                "receiver_id_b64",
                "channel_id_hex",
                "key_epoch",
                "cmk_hex",
                "receiver_private_key_hex",
                "receiver_public_key_hex",
                "ephemeral_private_key_hex",
                "ephemeral_public_key_hex",
                "shared_secret_hex",
                "hkdf_salt_b64",
                "hkdf_info_b64",
                "wrapping_key_hex",
                "wrapping_nonce_hex",
                "grant_aad_b64",
                "wrapped_cmk_hex",
                "signature_input_b64",
                "signature_hex",
                "d18g_b64",
                "expected_opened_cmk_hex",
            ]
        );
    }
    assert_case_roster(
        root,
        "structural_negative_cases",
        &["wrong-magic", "unsupported-version", "trailing-byte"],
        &["name", "d18g_b64", "expected_error"],
    );
    assert_case_roster(
        root,
        "field_negative_cases",
        &[
            "empty-originator",
            "empty-receiver",
            "invalid-uuid-version",
            "invalid-uuid-variant",
            "oversized-originator",
            "oversized-receiver",
        ],
        &["name", "expected_error"],
    );
    assert_case_roster(
        root,
        "seal_negative_cases",
        &["low-order-receiver-public-key"],
        &["name", "expected_error"],
    );
    assert_case_roster(
        root,
        "opening_negative_cases",
        &[
            "unexpected-originator",
            "unexpected-receiver",
            "unexpected-channel",
            "invalid-signature",
            "invalid-signature-before-key-agreement",
            "low-order-ephemeral-public-key",
            "wrong-receiver-private-key",
            "wrong-wrapping-nonce",
            "mutated-wrapped-cmk",
            "mutated-tag",
            "epoch-derivation-binding",
            "receiver-derivation-binding",
            "channel-aad-binding",
            "originator-aad-binding",
        ],
        &[
            "name",
            "d18g_b64",
            "expected_originator_id_b64",
            "expected_receiver_id_b64",
            "expected_channel_id_hex",
            "receiver_private_key_hex",
            "expected_error",
        ],
    );
    assert_eq!(
        keys(object(field(root, "truncated_prefix_recipe"))),
        vec![
            "source_case",
            "first_length",
            "last_length_exclusive",
            "expected_error",
        ]
    );
    for recipe in array(field(root, "oversize_recipes")) {
        assert_eq!(
            keys(object(recipe)),
            vec![
                "field",
                "length_offset",
                "declared_length",
                "expected_error"
            ]
        );
    }
    let trace = object(field(root, "receiver_state_trace"));
    assert_eq!(
        keys(trace),
        vec!["grants", "steps", "missing_epoch", "missing_epoch_error"]
    );
    assert_eq!(
        array(field(trace, "steps"))
            .iter()
            .map(|step| string(object(step), "name"))
            .collect::<Vec<_>>(),
        vec![
            "install-epoch-zero",
            "retry-epoch-zero",
            "same-epoch-conflict",
            "failed-higher-open",
            "install-skipped-epoch-three",
            "decreasing-epoch",
        ]
    );
    for step in array(field(trace, "steps")) {
        assert_eq!(
            keys(object(step)),
            vec![
                "name",
                "grant",
                "expected",
                "latest_epoch",
                "retained_epochs"
            ]
        );
    }
    assert_eq!(
        keys(object(field(root, "rotation_case"))),
        vec![
            "name",
            "current_epoch",
            "new_epoch",
            "new_cmk_hex",
            "authorized_receiver_ids_b64",
            "new_grants_b64",
            "receiver_a_retains_epochs",
            "receiver_b_retains_epochs",
            "receiver_a_new_grant",
        ]
    );
}

fn manifest_bytes() -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../fixtures/chief-of-staff-channel-key-grant/v1/manifest.json");
    fs::read(path).unwrap()
}

fn parse_manifest(bytes: &[u8]) -> JsonValue {
    let source = std::str::from_utf8(bytes).unwrap();
    from_ast(&try_parse_json(source).unwrap()).unwrap()
}

fn object(value: &JsonValue) -> &[(String, JsonValue)] {
    match value {
        JsonValue::Object(value) => value,
        _ => panic!("expected object"),
    }
}

fn array(value: &JsonValue) -> &[JsonValue] {
    match value {
        JsonValue::Array(value) => value,
        _ => panic!("expected array"),
    }
}

fn field<'a>(object: &'a [(String, JsonValue)], name: &str) -> &'a JsonValue {
    &object.iter().find(|(key, _)| key == name).unwrap().1
}

fn keys(object: &[(String, JsonValue)]) -> Vec<&str> {
    object.iter().map(|(key, _)| key.as_str()).collect()
}

fn assert_case_roster(
    root: &[(String, JsonValue)],
    field_name: &str,
    expected_names: &[&str],
    expected_fields: &[&str],
) {
    let cases = array(field(root, field_name));
    assert_eq!(
        cases
            .iter()
            .map(|case| string(object(case), "name"))
            .collect::<Vec<_>>(),
        expected_names
    );
    for case in cases {
        assert_eq!(keys(object(case)), expected_fields);
    }
}

fn string<'a>(object: &'a [(String, JsonValue)], name: &str) -> &'a str {
    json_string(field(object, name))
}

fn json_string(value: &JsonValue) -> &str {
    match value {
        JsonValue::String(value) => value,
        _ => panic!("expected string"),
    }
}

fn frame(fields: &[&[u8]]) -> Vec<u8> {
    let mut output = Vec::new();
    for field in fields {
        output.extend_from_slice(&(field.len() as u64).to_be_bytes());
        output.extend_from_slice(field);
    }
    output
}

fn decode_base64(value: &str) -> Vec<u8> {
    assert_eq!(value.len() % 4, 0);
    let mut output = Vec::with_capacity(value.len() / 4 * 3);
    for chunk in value.as_bytes().chunks_exact(4) {
        let a = base64_digit(chunk[0]) as u32;
        let b = base64_digit(chunk[1]) as u32;
        let c = if chunk[2] == b'=' {
            0
        } else {
            base64_digit(chunk[2]) as u32
        };
        let d = if chunk[3] == b'=' {
            0
        } else {
            base64_digit(chunk[3]) as u32
        };
        let word = (a << 18) | (b << 12) | (c << 6) | d;
        output.push((word >> 16) as u8);
        if chunk[2] != b'=' {
            output.push((word >> 8) as u8);
        }
        if chunk[3] != b'=' {
            output.push(word as u8);
        }
    }
    output
}

fn base64_digit(byte: u8) -> u8 {
    match byte {
        b'A'..=b'Z' => byte - b'A',
        b'a'..=b'z' => byte - b'a' + 26,
        b'0'..=b'9' => byte - b'0' + 52,
        b'+' => 62,
        b'/' => 63,
        _ => panic!("invalid base64 digit"),
    }
}

fn decode_hex(value: &str) -> Vec<u8> {
    assert_eq!(value.len() % 2, 0);
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| (hex_digit(pair[0]) << 4) | hex_digit(pair[1]))
        .collect()
}

fn hex_digit(byte: u8) -> u8 {
    match byte {
        b'0'..=b'9' => byte - b'0',
        b'a'..=b'f' => byte - b'a' + 10,
        _ => panic!("invalid hex digit"),
    }
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
