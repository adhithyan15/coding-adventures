use tls13_key_share::{
    decode_client_key_shares, encode_client_key_shares, validate_client_key_share, KeyShareLimits,
};
use tls13_key_share_retry::{
    bind_retried_client_key_share, bind_retry_selected_group, RetryKeyShareError,
};
use tls13_named_group_policy::{NamedGroup, NamedGroupPolicy, PolicyError};
use tls13_supported_groups::{decode_supported_groups, NamedGroupListLimits};

const X25519_KEY: [u8; 32] = [0x42; 32];
const P256_KEY: [u8; 65] = [0x24; 65];

fn policy() -> NamedGroupPolicy {
    NamedGroupPolicy::new(&[NamedGroup::X25519, NamedGroup::Secp256r1]).unwrap()
}

fn supported(bytes: &[u8]) -> tls13_supported_groups::NamedGroupList<'_> {
    decode_supported_groups(bytes, NamedGroupListLimits::new(2).unwrap()).unwrap()
}

fn shares<'input>(
    bytes: &'input [u8],
    policy: &NamedGroupPolicy,
) -> tls13_key_share::DecodedClientKeyShares<'input> {
    decode_client_key_shares(bytes, policy, KeyShareLimits::new(2).unwrap()).unwrap()
}

fn one_share(group: u16, key: &[u8], policy: &NamedGroupPolicy) -> Vec<u8> {
    let entry = validate_client_key_share(policy, group, key).unwrap();
    let mut output = vec![0_u8; key.len() + 6];
    encode_client_key_shares(&[entry], &mut output).unwrap();
    output
}

#[test]
fn binds_empty_initial_offer_to_the_only_retried_share() {
    let policy = policy();
    let supported_bytes = [0x00, 0x04, 0x00, 0x1d, 0x00, 0x17];
    let initial_bytes = [0x00, 0x00];
    let retry_bytes = [0x00, 0x1d];
    let retried_bytes = one_share(0x001d, &X25519_KEY, &policy);

    let selected = bind_retry_selected_group(
        &retry_bytes,
        &policy,
        supported(&supported_bytes),
        shares(&initial_bytes, &policy),
    )
    .unwrap();
    assert_eq!(selected.extension_data(), &retry_bytes);
    assert_eq!(
        selected.supported_groups().extension_data(),
        &supported_bytes
    );
    assert!(selected.initial_client_shares().is_empty());
    assert_eq!(format!("{selected:?}"), "RetrySelectedGroup");

    let bound = bind_retried_client_key_share(selected, shares(&retried_bytes, &policy)).unwrap();
    assert_eq!(
        bound.selected_group().parameters().group(),
        NamedGroup::X25519
    );
    assert_eq!(
        bound
            .retried_client_shares()
            .entries()
            .next()
            .unwrap()
            .key_exchange(),
        X25519_KEY
    );
    assert_eq!(bound.selection().extension_data(), &retry_bytes);
    assert_eq!(format!("{bound:?}"), "BoundRetryKeyShare");
}

#[test]
fn permits_retrying_a_supported_group_absent_from_a_nonempty_offer() {
    let policy = policy();
    let supported_bytes = [0x00, 0x04, 0x00, 0x1d, 0x00, 0x17];
    let initial_bytes = one_share(0x001d, &X25519_KEY, &policy);
    let retried_bytes = one_share(0x0017, &P256_KEY, &policy);
    let selected = bind_retry_selected_group(
        &[0x00, 0x17],
        &policy,
        supported(&supported_bytes),
        shares(&initial_bytes, &policy),
    )
    .unwrap();

    let bound = bind_retried_client_key_share(selected, shares(&retried_bytes, &policy)).unwrap();
    assert_eq!(
        bound.selected_group().parameters().group(),
        NamedGroup::Secp256r1
    );
}

#[test]
fn rejects_non_exact_retry_selected_group_framing() {
    let policy = policy();
    let supported_bytes = [0x00, 0x02, 0x00, 0x1d];
    for malformed in [&[][..], &[0x00][..], &[0x00, 0x1d, 0x00][..]] {
        assert_eq!(
            bind_retry_selected_group(
                malformed,
                &policy,
                supported(&supported_bytes),
                shares(&[0x00, 0x00], &policy),
            )
            .unwrap_err(),
            RetryKeyShareError::InvalidSelectedGroupLength
        );
    }
}

#[test]
fn applies_named_group_policy_before_offer_relations() {
    let x25519_only = NamedGroupPolicy::new(&[NamedGroup::X25519]).unwrap();
    let initial = [0x00, 0x00];
    assert_eq!(
        bind_retry_selected_group(
            &[0x0a, 0x0a],
            &x25519_only,
            supported(&[0x00, 0x02, 0x0a, 0x0a]),
            shares(&initial, &x25519_only),
        )
        .unwrap_err(),
        RetryKeyShareError::NamedGroupPolicy(PolicyError::UnknownNamedGroup)
    );
    assert_eq!(
        bind_retry_selected_group(
            &[0x00, 0x17],
            &x25519_only,
            supported(&[0x00, 0x02, 0x00, 0x17]),
            shares(&initial, &x25519_only),
        )
        .unwrap_err(),
        RetryKeyShareError::NamedGroupPolicy(PolicyError::DisallowedNamedGroup)
    );
}

#[test]
fn rejects_a_group_absent_from_supported_groups_or_present_in_initial_shares() {
    let policy = policy();
    let empty = [0x00, 0x00];
    assert_eq!(
        bind_retry_selected_group(
            &[0x00, 0x17],
            &policy,
            supported(&[0x00, 0x02, 0x00, 0x1d]),
            shares(&empty, &policy),
        )
        .unwrap_err(),
        RetryKeyShareError::SelectedGroupNotSupported
    );

    let initial = one_share(0x001d, &X25519_KEY, &policy);
    assert_eq!(
        bind_retry_selected_group(
            &[0x00, 0x1d],
            &policy,
            supported(&[0x00, 0x02, 0x00, 0x1d]),
            shares(&initial, &policy),
        )
        .unwrap_err(),
        RetryKeyShareError::SelectedGroupAlreadyOffered
    );
}

#[test]
fn requires_exactly_one_matching_retried_share() {
    let policy = policy();
    let supported_bytes = [0x00, 0x04, 0x00, 0x1d, 0x00, 0x17];
    let select = || {
        bind_retry_selected_group(
            &[0x00, 0x1d],
            &policy,
            supported(&supported_bytes),
            shares(&[0x00, 0x00], &policy),
        )
        .unwrap()
    };

    assert_eq!(
        bind_retried_client_key_share(select(), shares(&[0x00, 0x00], &policy)).unwrap_err(),
        RetryKeyShareError::RetriedShareCount
    );

    let x25519 = validate_client_key_share(&policy, 0x001d, &X25519_KEY).unwrap();
    let p256 = validate_client_key_share(&policy, 0x0017, &P256_KEY).unwrap();
    let mut two = [0_u8; 107];
    encode_client_key_shares(&[x25519, p256], &mut two).unwrap();
    assert_eq!(
        bind_retried_client_key_share(select(), shares(&two, &policy)).unwrap_err(),
        RetryKeyShareError::RetriedShareCount
    );

    let p256_only = one_share(0x0017, &P256_KEY, &policy);
    assert_eq!(
        bind_retried_client_key_share(select(), shares(&p256_only, &policy)).unwrap_err(),
        RetryKeyShareError::RetriedGroupMismatch
    );
}

#[test]
fn diagnostics_retain_no_group_or_key_payloads() {
    let policy = policy();
    let selected = bind_retry_selected_group(
        &[0x00, 0x1d],
        &policy,
        supported(&[0x00, 0x02, 0x00, 0x1d]),
        shares(&[0x00, 0x00], &policy),
    )
    .unwrap();
    assert_eq!(format!("{selected:?}"), "RetrySelectedGroup");

    for error in [
        RetryKeyShareError::InvalidSelectedGroupLength,
        RetryKeyShareError::NamedGroupPolicy(PolicyError::UnknownNamedGroup),
        RetryKeyShareError::SelectedGroupNotSupported,
        RetryKeyShareError::SelectedGroupAlreadyOffered,
        RetryKeyShareError::RetriedShareCount,
        RetryKeyShareError::RetriedGroupMismatch,
    ] {
        for diagnostic in [format!("{error:?}"), error.to_string()] {
            for forbidden in ["001d", "0x1d", "66", "36", "32"] {
                assert!(!diagnostic.contains(forbidden));
            }
        }
    }
}
