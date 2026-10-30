use x509_ip_name_constraint_policy::{
    evaluate_ip_name_constraints, IpConstraintSyntaxError, IpNameConstraintErrorKind,
    PresentedIpAddressSyntaxError, MAX_EXCLUDED_IP_CONSTRAINTS, MAX_PERMITTED_IP_CONSTRAINTS,
    MAX_PRESENTED_IP_ADDRESSES,
};

fn ipv4_constraint(address: [u8; 4], mask: [u8; 4]) -> [u8; 8] {
    let mut value = [0; 8];
    value[..4].copy_from_slice(&address);
    value[4..].copy_from_slice(&mask);
    value
}

fn ipv6_constraint(address: [u8; 16], mask: [u8; 16]) -> [u8; 32] {
    let mut value = [0; 32];
    value[..16].copy_from_slice(&address);
    value[16..].copy_from_slice(&mask);
    value
}

fn prefix_mask<const N: usize>(prefix_bits: usize) -> [u8; N] {
    let mut mask = [0; N];
    for bit in 0..prefix_bits {
        mask[bit / 8] |= 0x80 >> (bit % 8);
    }
    mask
}

#[test]
fn evaluates_ipv4_exact_and_subnet_constraints() {
    let exact = ipv4_constraint([192, 0, 2, 7], [255, 255, 255, 255]);
    let subnet = ipv4_constraint([198, 51, 100, 0], [255, 255, 255, 0]);
    let addresses: [&[u8]; 2] = [&[192, 0, 2, 7], &[198, 51, 100, 42]];

    let evidence = evaluate_ip_name_constraints(&[&exact, &subnet], &[], &addresses).unwrap();
    assert_eq!(evidence.count(), 2);
    assert!(!evidence.is_empty());

    let error = evaluate_ip_name_constraints(&[&exact], &[], &[&[192, 0, 2, 8]]).unwrap_err();
    assert_eq!(
        error.kind(),
        IpNameConstraintErrorKind::PresentedAddressNotPermitted
    );
}

#[test]
fn evaluates_ipv6_constraints_and_separates_address_families() {
    let mut base = [0; 16];
    base[..4].copy_from_slice(&[0x20, 0x01, 0x0d, 0xb8]);
    let constraint = ipv6_constraint(base, prefix_mask::<16>(32));
    let mut inside = base;
    inside[15] = 1;
    let ipv4 = [32, 1, 13, 184];

    assert!(evaluate_ip_name_constraints(&[&constraint], &[], &[&inside]).is_ok());
    let error = evaluate_ip_name_constraints(&[&constraint], &[], &[&ipv4]).unwrap_err();
    assert_eq!(
        error.kind(),
        IpNameConstraintErrorKind::PresentedAddressNotPermitted
    );
}

#[test]
fn excluded_constraints_take_precedence() {
    let permitted = ipv4_constraint([192, 0, 2, 0], [255, 255, 255, 0]);
    let excluded = ipv4_constraint([192, 0, 2, 128], [255, 255, 255, 128]);

    assert!(
        evaluate_ip_name_constraints(&[&permitted], &[&excluded], &[&[192, 0, 2, 42]],).is_ok()
    );
    let error = evaluate_ip_name_constraints(&[&permitted], &[&excluded], &[&[192, 0, 2, 200]])
        .unwrap_err();
    assert_eq!(
        error.kind(),
        IpNameConstraintErrorKind::PresentedAddressExcluded
    );
}

#[test]
fn absent_permitted_set_accepts_any_nonexcluded_family() {
    let excluded = ipv4_constraint([10, 0, 0, 0], [255, 0, 0, 0]);
    let evidence = evaluate_ip_name_constraints(&[], &[&excluded], &[&[203, 0, 113, 9]]).unwrap();
    assert_eq!(evidence.count(), 1);
    assert!(evaluate_ip_name_constraints(&[], &[], &[])
        .unwrap()
        .is_empty());
}

#[test]
fn accepts_every_ipv4_and_ipv6_cidr_prefix() {
    for prefix in 0..=32 {
        let constraint = ipv4_constraint([192, 0, 2, 129], prefix_mask::<4>(prefix));
        assert!(evaluate_ip_name_constraints(&[&constraint], &[], &[&[192, 0, 2, 129]]).is_ok());
    }
    for prefix in 0..=128 {
        let constraint = ipv6_constraint([0x20; 16], prefix_mask::<16>(prefix));
        assert!(evaluate_ip_name_constraints(&[&constraint], &[], &[&[0x20; 16]]).is_ok());
    }
}

#[test]
fn rejects_noncontiguous_masks_in_either_collection() {
    let invalid_v4 = ipv4_constraint([192, 0, 2, 0], [255, 0, 255, 0]);
    let mut invalid_v6_mask = prefix_mask::<16>(64);
    invalid_v6_mask[15] = 1;
    let invalid_v6 = ipv6_constraint([0; 16], invalid_v6_mask);

    let permitted_error = evaluate_ip_name_constraints(&[&invalid_v4], &[], &[]).unwrap_err();
    assert_eq!(
        permitted_error.kind(),
        IpNameConstraintErrorKind::InvalidPermittedConstraint(
            IpConstraintSyntaxError::NonContiguousMask
        )
    );
    assert_eq!(permitted_error.item_index(), 0);

    let excluded_error = evaluate_ip_name_constraints(&[], &[&invalid_v6], &[]).unwrap_err();
    assert_eq!(
        excluded_error.kind(),
        IpNameConstraintErrorKind::InvalidExcludedConstraint(
            IpConstraintSyntaxError::NonContiguousMask
        )
    );
}

#[test]
fn rejects_invalid_constraint_and_presented_lengths() {
    for length in 0..=33 {
        if length == 8 || length == 32 {
            continue;
        }
        let value = vec![0; length];
        let error = evaluate_ip_name_constraints(&[&value], &[], &[]).unwrap_err();
        assert_eq!(
            error.kind(),
            IpNameConstraintErrorKind::InvalidPermittedConstraint(
                IpConstraintSyntaxError::InvalidLength
            )
        );
    }

    for length in 0..=17 {
        if length == 4 || length == 16 {
            continue;
        }
        let value = vec![0; length];
        let error = evaluate_ip_name_constraints(&[], &[], &[&value]).unwrap_err();
        assert_eq!(
            error.kind(),
            IpNameConstraintErrorKind::InvalidPresentedAddress(
                PresentedIpAddressSyntaxError::InvalidLength
            )
        );
    }
}

#[test]
fn validates_all_constraints_before_accepting_an_address() {
    let valid = ipv4_constraint([192, 0, 2, 0], [255, 255, 255, 0]);
    let invalid = ipv4_constraint([198, 51, 100, 0], [255, 0, 255, 0]);
    let error =
        evaluate_ip_name_constraints(&[&valid, &invalid], &[], &[&[192, 0, 2, 42]]).unwrap_err();
    assert_eq!(
        error.kind(),
        IpNameConstraintErrorKind::InvalidPermittedConstraint(
            IpConstraintSyntaxError::NonContiguousMask
        )
    );
    assert_eq!(error.item_index(), 1);
}

#[test]
fn collection_ceilings_are_exact_and_fail_before_evaluation() {
    let constraint = ipv4_constraint([0; 4], [0; 4]);
    let permitted = vec![constraint.as_slice(); MAX_PERMITTED_IP_CONSTRAINTS];
    let excluded = vec![constraint.as_slice(); MAX_EXCLUDED_IP_CONSTRAINTS];
    let address = [192, 0, 2, 1];
    let presented = vec![address.as_slice(); MAX_PRESENTED_IP_ADDRESSES];
    assert_eq!(
        evaluate_ip_name_constraints(&permitted, &[], &presented)
            .unwrap()
            .count(),
        MAX_PRESENTED_IP_ADDRESSES
    );

    for (result, expected) in [
        (
            evaluate_ip_name_constraints(
                &[permitted.clone(), vec![&constraint]].concat(),
                &[],
                &[],
            ),
            IpNameConstraintErrorKind::TooManyPermittedConstraints,
        ),
        (
            evaluate_ip_name_constraints(&[], &[excluded, vec![&constraint]].concat(), &[]),
            IpNameConstraintErrorKind::TooManyExcludedConstraints,
        ),
        (
            evaluate_ip_name_constraints(&[], &[], &[presented, vec![&address]].concat()),
            IpNameConstraintErrorKind::TooManyPresentedAddresses,
        ),
    ] {
        let error = result.unwrap_err();
        assert_eq!(error.kind(), expected);
        assert_eq!(error.item_index(), 256);
    }
}

#[test]
fn errors_are_stable_and_redacted() {
    let invalid = ipv4_constraint([203, 0, 113, 77], [255, 0, 255, 0]);
    let error = evaluate_ip_name_constraints(&[&invalid], &[], &[]).unwrap_err();
    assert_eq!(
        format!("{error:?}"),
        "IpNameConstraintError { kind: InvalidPermittedConstraint(NonContiguousMask), item_index: 0 }"
    );
    assert_eq!(
        error.to_string(),
        "X.509 IP name constraint error InvalidPermittedConstraint(NonContiguousMask) at item 0"
    );
    assert!(!format!("{error:?}").contains("203"));
    assert!(!error.to_string().contains("113"));
}
