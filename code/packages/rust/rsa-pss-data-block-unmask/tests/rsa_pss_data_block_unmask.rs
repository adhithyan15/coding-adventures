use rsa_pss_data_block_unmask::{
    unmask_rsa_pss_data_block, RsaPssDataBlockUnmaskErrorKind, RsaPssDataBlockUnmaskPolicy,
    RsaPssDataBlockUnmaskPolicyErrorKind,
};

#[test]
fn unmasks_complete_data_block_and_clears_high_bits() {
    let masked = [0x80, 0xaa, 0x55, 0xff];
    let mask = [0xff, 0x0f, 0xf0, 0x0f];
    let mut output = [0xa5; 4];
    let policy = RsaPssDataBlockUnmaskPolicy::new(4).unwrap();

    let unmasked = unmask_rsa_pss_data_block(&masked, &mask, &mut output, 1, policy).unwrap();
    assert_eq!(unmasked.data_block(), &[0x7f, 0xa5, 0xa5, 0xf0]);
    assert_eq!(unmasked.unused_high_bits(), 1);
    assert_eq!(format!("{unmasked:?}"), "UnmaskedRsaPssDataBlock");
}

#[test]
fn supports_every_valid_unused_high_bit_count() {
    let policy = RsaPssDataBlockUnmaskPolicy::new(1).unwrap();
    for unused_high_bits in 0..=7 {
        let mut output = [0u8; 1];
        let unmasked =
            unmask_rsa_pss_data_block(&[0xff], &[0], &mut output, unused_high_bits, policy)
                .unwrap();
        assert_eq!(unmasked.data_block(), &[u8::MAX >> unused_high_bits]);
        assert_eq!(unmasked.unused_high_bits(), unused_high_bits);
    }
}

#[test]
fn policy_reports_and_enforces_public_ceiling() {
    let policy = RsaPssDataBlockUnmaskPolicy::new(3).unwrap();
    assert_eq!(policy.maximum_data_block_bytes(), 3);

    let mut output = [0xa5; 4];
    let error = unmask_rsa_pss_data_block(&[0; 4], &[0; 4], &mut output, 0, policy).unwrap_err();
    assert_eq!(
        error.kind(),
        RsaPssDataBlockUnmaskErrorKind::MaskedDataBlockTooLongForPolicy
    );
    assert_eq!(output, [0xa5; 4]);
}

#[test]
fn policy_rejects_zero_ceiling() {
    let error = RsaPssDataBlockUnmaskPolicy::new(0).unwrap_err();
    assert_eq!(
        error.kind(),
        RsaPssDataBlockUnmaskPolicyErrorKind::ZeroMaximumDataBlockBytes
    );
}

#[test]
fn rejects_empty_input_before_modifying_output() {
    let policy = RsaPssDataBlockUnmaskPolicy::new(4).unwrap();
    let mut output = [0xa5; 1];
    let error = unmask_rsa_pss_data_block(&[], &[], &mut output, 0, policy).unwrap_err();
    assert_eq!(
        error.kind(),
        RsaPssDataBlockUnmaskErrorKind::EmptyMaskedDataBlock
    );
    assert_eq!(output, [0xa5]);
}

#[test]
fn rejects_mask_length_before_modifying_output() {
    let policy = RsaPssDataBlockUnmaskPolicy::new(4).unwrap();
    for mask in [&[0; 2][..], &[0; 4][..]] {
        let mut output = [0xa5; 3];
        let error = unmask_rsa_pss_data_block(&[1; 3], mask, &mut output, 0, policy).unwrap_err();
        assert_eq!(
            error.kind(),
            RsaPssDataBlockUnmaskErrorKind::DataBlockMaskLengthMismatch
        );
        assert_eq!(output, [0xa5; 3]);
    }
}

#[test]
fn rejects_output_length_before_modifying_output() {
    let policy = RsaPssDataBlockUnmaskPolicy::new(4).unwrap();
    for output_length in [2, 4] {
        let mut output = vec![0xa5; output_length];
        let error =
            unmask_rsa_pss_data_block(&[1; 3], &[2; 3], &mut output, 0, policy).unwrap_err();
        assert_eq!(
            error.kind(),
            RsaPssDataBlockUnmaskErrorKind::OutputLengthMismatch
        );
        assert!(output.iter().all(|octet| *octet == 0xa5));
    }
}

#[test]
fn rejects_invalid_unused_high_bits_before_modifying_output() {
    let policy = RsaPssDataBlockUnmaskPolicy::new(4).unwrap();
    let mut output = [0xa5; 3];
    let error = unmask_rsa_pss_data_block(&[1; 3], &[2; 3], &mut output, 8, policy).unwrap_err();
    assert_eq!(
        error.kind(),
        RsaPssDataBlockUnmaskErrorKind::InvalidUnusedHighBits
    );
    assert_eq!(output, [0xa5; 3]);
}

#[test]
fn errors_are_stable_and_redacted() {
    let policy_error = RsaPssDataBlockUnmaskPolicy::new(0).unwrap_err();
    assert_eq!(
        format!("{policy_error:?}"),
        "RsaPssDataBlockUnmaskPolicyError { kind: ZeroMaximumDataBlockBytes }"
    );
    assert_eq!(
        policy_error.to_string(),
        "RSA-PSS data-block-unmask policy error ZeroMaximumDataBlockBytes"
    );

    let policy = RsaPssDataBlockUnmaskPolicy::new(4).unwrap();
    let mut output = [0xa5; 3];
    let error =
        unmask_rsa_pss_data_block(&[0x7d; 3], &[0x7e; 2], &mut output, 0, policy).unwrap_err();
    assert_eq!(
        format!("{error:?}"),
        "RsaPssDataBlockUnmaskError { kind: DataBlockMaskLengthMismatch }"
    );
    assert_eq!(
        error.to_string(),
        "RSA-PSS data-block-unmask error DataBlockMaskLengthMismatch"
    );
    for rendered in [
        format!("{policy_error:?}"),
        policy_error.to_string(),
        format!("{error:?}"),
        error.to_string(),
    ] {
        assert!(!rendered.contains("0x7d"));
        assert!(!rendered.contains("0x7e"));
    }
}
