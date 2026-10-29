use tls13_alert_message::{
    decode_alert_message, encode_alert_message, AlertDescription, AlertDisposition, AlertErrorKind,
    AlertLevel, ALERT_MESSAGE_LENGTH,
};

const STANDARD_DESCRIPTIONS: [AlertDescription; 27] = [
    AlertDescription::CLOSE_NOTIFY,
    AlertDescription::UNEXPECTED_MESSAGE,
    AlertDescription::BAD_RECORD_MAC,
    AlertDescription::RECORD_OVERFLOW,
    AlertDescription::HANDSHAKE_FAILURE,
    AlertDescription::BAD_CERTIFICATE,
    AlertDescription::UNSUPPORTED_CERTIFICATE,
    AlertDescription::CERTIFICATE_REVOKED,
    AlertDescription::CERTIFICATE_EXPIRED,
    AlertDescription::CERTIFICATE_UNKNOWN,
    AlertDescription::ILLEGAL_PARAMETER,
    AlertDescription::UNKNOWN_CA,
    AlertDescription::ACCESS_DENIED,
    AlertDescription::DECODE_ERROR,
    AlertDescription::DECRYPT_ERROR,
    AlertDescription::PROTOCOL_VERSION,
    AlertDescription::INSUFFICIENT_SECURITY,
    AlertDescription::INTERNAL_ERROR,
    AlertDescription::INAPPROPRIATE_FALLBACK,
    AlertDescription::USER_CANCELED,
    AlertDescription::MISSING_EXTENSION,
    AlertDescription::UNSUPPORTED_EXTENSION,
    AlertDescription::UNRECOGNIZED_NAME,
    AlertDescription::BAD_CERTIFICATE_STATUS_RESPONSE,
    AlertDescription::UNKNOWN_PSK_IDENTITY,
    AlertDescription::CERTIFICATE_REQUIRED,
    AlertDescription::NO_APPLICATION_PROTOCOL,
];

#[test]
fn standard_registry_matches_rfc_8446_and_has_no_duplicates() {
    assert_eq!(ALERT_MESSAGE_LENGTH, 2);
    assert_eq!(STANDARD_DESCRIPTIONS.len(), 27);

    for (index, description) in STANDARD_DESCRIPTIONS.iter().enumerate() {
        assert!(description.is_standard());
        assert_eq!(
            AlertDescription::from_wire(description.wire_value()),
            *description
        );
        for other in &STANDARD_DESCRIPTIONS[index + 1..] {
            assert_ne!(description.wire_value(), other.wire_value());
        }
    }
}

#[test]
fn closure_disposition_ignores_every_legacy_level_value() {
    for legacy_level in 0..=u8::MAX {
        let close = decode_alert_message(&[legacy_level, 0]).expect("close_notify decodes");
        assert_eq!(close.legacy_level(), legacy_level);
        assert_eq!(close.description(), AlertDescription::CLOSE_NOTIFY);
        assert_eq!(close.disposition(), AlertDisposition::CloseNotify);

        let canceled = decode_alert_message(&[legacy_level, 90]).expect("user_canceled decodes");
        assert_eq!(canceled.disposition(), AlertDisposition::UserCanceled);
    }
}

#[test]
fn defined_error_alerts_are_errors_regardless_of_legacy_level() {
    for description in STANDARD_DESCRIPTIONS {
        if matches!(
            description.disposition(),
            AlertDisposition::CloseNotify | AlertDisposition::UserCanceled
        ) {
            continue;
        }
        for legacy_level in [0, 1, 2, 255] {
            let alert = decode_alert_message(&[legacy_level, description.wire_value()])
                .expect("defined error alert decodes");
            assert_eq!(alert.disposition(), AlertDisposition::Error);
        }
    }
}

#[test]
fn unknown_descriptions_fail_closed_as_errors() {
    for value in 0..=u8::MAX {
        let description = AlertDescription::from_wire(value);
        if description.is_standard() {
            continue;
        }
        let alert = decode_alert_message(&[1, value]).expect("unknown alert decodes");
        assert_eq!(alert.description().wire_value(), value);
        assert_eq!(alert.disposition(), AlertDisposition::Error);
    }
}

#[test]
fn recognized_legacy_levels_are_metadata_only() {
    let warning = decode_alert_message(&[1, 50]).expect("warning-level error decodes");
    assert_eq!(warning.recognized_legacy_level(), Some(AlertLevel::Warning));
    assert_eq!(warning.disposition(), AlertDisposition::Error);

    let fatal = decode_alert_message(&[2, 0]).expect("fatal-level closure decodes");
    assert_eq!(fatal.recognized_legacy_level(), Some(AlertLevel::Fatal));
    assert_eq!(fatal.disposition(), AlertDisposition::CloseNotify);

    let unknown = decode_alert_message(&[3, 0]).expect("unknown-level closure decodes");
    assert_eq!(unknown.recognized_legacy_level(), None);
    assert_eq!(unknown.disposition(), AlertDisposition::CloseNotify);
}

#[test]
fn exact_decoder_rejects_truncation_and_padding() {
    for input in [&[][..], &[1][..], &[1, 0, 0][..]] {
        assert_eq!(
            decode_alert_message(input)
                .expect_err("non-exact alert must fail")
                .kind(),
            AlertErrorKind::InvalidMessageLength
        );
    }
}

#[test]
fn encoder_round_trips_standard_closure_and_error_alerts() {
    let cases = [
        (AlertLevel::Warning, AlertDescription::CLOSE_NOTIFY),
        (AlertLevel::Fatal, AlertDescription::USER_CANCELED),
        (AlertLevel::Fatal, AlertDescription::DECODE_ERROR),
        (AlertLevel::Fatal, AlertDescription::NO_APPLICATION_PROTOCOL),
    ];

    for (level, description) in cases {
        let mut output = [0xa5; 4];
        let used = encode_alert_message(level, description, &mut output).expect("alert encodes");
        assert_eq!(used, ALERT_MESSAGE_LENGTH);
        assert_eq!(output[0], level.wire_value());
        assert_eq!(output[1], description.wire_value());
        assert_eq!(&output[used..], &[0xa5, 0xa5]);

        let decoded = decode_alert_message(&output[..used]).expect("encoded alert decodes");
        assert_eq!(decoded.description(), description);
    }
}

#[test]
fn encoder_rejects_nonfatal_error_and_unknown_description_atomically() {
    let mut output = [0xa5; 4];
    assert_eq!(
        encode_alert_message(
            AlertLevel::Warning,
            AlertDescription::DECODE_ERROR,
            &mut output,
        )
        .expect_err("error alert must be fatal")
        .kind(),
        AlertErrorKind::ErrorAlertMustBeFatal
    );
    assert_eq!(output, [0xa5; 4]);

    assert_eq!(
        encode_alert_message(
            AlertLevel::Fatal,
            AlertDescription::from_wire(255),
            &mut output,
        )
        .expect_err("unknown description must not be sent")
        .kind(),
        AlertErrorKind::UnknownDescriptionForEncoding
    );
    assert_eq!(output, [0xa5; 4]);
}

#[test]
fn short_output_is_rejected_before_mutation() {
    for length in 0..ALERT_MESSAGE_LENGTH {
        let mut output = [0xa5; ALERT_MESSAGE_LENGTH];
        assert_eq!(
            encode_alert_message(
                AlertLevel::Warning,
                AlertDescription::CLOSE_NOTIFY,
                &mut output[..length],
            )
            .expect_err("short output must fail")
            .kind(),
            AlertErrorKind::OutputTooSmall
        );
        assert_eq!(output, [0xa5; ALERT_MESSAGE_LENGTH]);
    }
}

#[test]
fn diagnostics_do_not_disclose_wire_values() {
    let description = AlertDescription::from_wire(255);
    let description_debug = format!("{description:?}");
    assert!(description_debug.contains("standard: false"));
    assert!(description_debug.contains("Error"));
    assert!(!description_debug.contains("255"));

    let error = decode_alert_message(&[1]).expect_err("truncated alert must fail");
    assert_eq!(
        error.to_string(),
        "TLS 1.3 alert error InvalidMessageLength"
    );
    assert!(!format!("{error:?}").contains('1'));
}
