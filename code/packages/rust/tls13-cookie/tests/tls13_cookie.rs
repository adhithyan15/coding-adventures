use tls13_cookie::{
    bind_cookie_echo, decode_cookie, encode_cookie, CookieError, CookieLimits,
    COOKIE_LENGTH_PREFIX, MAX_COOKIE_LENGTH,
};

#[test]
fn decodes_one_exact_nonempty_cookie() {
    let encoded = [0x00, 0x04, 0xde, 0xad, 0xbe, 0xef];
    let cookie = decode_cookie(&encoded, CookieLimits::new(8).unwrap()).unwrap();

    assert_eq!(cookie.extension_data(), &encoded);
    assert_eq!(cookie.cookie(), &[0xde, 0xad, 0xbe, 0xef]);
    assert_eq!(cookie.len(), 4);
    assert!(!cookie.is_empty());
}

#[test]
fn encodes_atomically_and_leaves_extra_capacity_untouched() {
    let mut output = [0xa5; 8];
    let encoded = encode_cookie(
        &[0x11, 0x22, 0x33],
        CookieLimits::new(8).unwrap(),
        &mut output,
    )
    .unwrap();

    assert_eq!(encoded.extension_data(), &[0x00, 0x03, 0x11, 0x22, 0x33]);
    assert_eq!(encoded.cookie(), &[0x11, 0x22, 0x33]);
    assert_eq!(&output[5..], &[0xa5; 3]);
}

#[test]
fn validates_limits() {
    assert_eq!(
        CookieLimits::new(0),
        Err(CookieError::InvalidMaximumCookieLength)
    );
    assert_eq!(
        CookieLimits::new(MAX_COOKIE_LENGTH + 1),
        Err(CookieError::InvalidMaximumCookieLength)
    );
    assert_eq!(
        CookieLimits::new(MAX_COOKIE_LENGTH)
            .unwrap()
            .maximum_cookie_length(),
        MAX_COOKIE_LENGTH
    );
    assert_eq!(COOKIE_LENGTH_PREFIX, 2);
}

#[test]
fn rejects_malformed_and_over_policy_decoding() {
    let limits = CookieLimits::new(4).unwrap();

    assert_eq!(
        decode_cookie(&[], limits),
        Err(CookieError::TruncatedLength)
    );
    assert_eq!(
        decode_cookie(&[0x00], limits),
        Err(CookieError::TruncatedLength)
    );
    assert_eq!(
        decode_cookie(&[0x00, 0x00], limits),
        Err(CookieError::EmptyCookie)
    );
    assert_eq!(
        decode_cookie(&[0x00, 0x05], limits),
        Err(CookieError::CookieTooLong)
    );
    assert_eq!(
        decode_cookie(&[0x00, 0x03, 1, 2], limits),
        Err(CookieError::LengthMismatch)
    );
    assert_eq!(
        decode_cookie(&[0x00, 0x02, 1, 2, 3], limits),
        Err(CookieError::LengthMismatch)
    );
}

#[test]
fn encoding_failures_do_not_change_output() {
    let limits = CookieLimits::new(3).unwrap();
    let mut output = [0x6c; 4];

    assert!(matches!(
        encode_cookie(&[], limits, &mut output),
        Err(CookieError::EmptyCookie)
    ));
    assert_eq!(output, [0x6c; 4]);
    assert!(matches!(
        encode_cookie(&[1, 2, 3, 4], limits, &mut output),
        Err(CookieError::CookieTooLong)
    ));
    assert_eq!(output, [0x6c; 4]);
    assert!(matches!(
        encode_cookie(&[1, 2, 3], limits, &mut output),
        Err(CookieError::OutputTooSmall)
    ));
    assert_eq!(output, [0x6c; 4]);
}

#[test]
fn binds_the_exact_retry_cookie_echo() {
    let limits = CookieLimits::new(8).unwrap();
    let retry_data = [0x00, 0x03, 1, 2, 3];
    let client_data = [0x00, 0x03, 1, 2, 3];
    let retry = decode_cookie(&retry_data, limits).unwrap();
    let client = decode_cookie(&client_data, limits).unwrap();
    let echoed = bind_cookie_echo(retry, client).unwrap();

    assert_eq!(echoed.hello_retry_request().extension_data(), &retry_data);
    assert_eq!(echoed.client_hello().extension_data(), &client_data);
}

#[test]
fn rejects_a_changed_retry_cookie_echo() {
    let limits = CookieLimits::new(8).unwrap();
    let retry = decode_cookie(&[0x00, 0x03, 1, 2, 3], limits).unwrap();
    let client = decode_cookie(&[0x00, 0x03, 1, 2, 4], limits).unwrap();

    assert!(matches!(
        bind_cookie_echo(retry, client),
        Err(CookieError::EchoMismatch)
    ));
}

#[test]
fn accepts_the_wire_format_maximum() {
    let limits = CookieLimits::new(MAX_COOKIE_LENGTH).unwrap();
    let value = vec![0x3c; MAX_COOKIE_LENGTH];
    let mut output = vec![0_u8; COOKIE_LENGTH_PREFIX + MAX_COOKIE_LENGTH];
    let encoded = encode_cookie(&value, limits, &mut output).unwrap();

    assert_eq!(&encoded.extension_data()[..2], &[0xff, 0xff]);
    assert_eq!(encoded.cookie(), value);
    assert_eq!(
        decode_cookie(encoded.extension_data(), limits)
            .unwrap()
            .cookie(),
        value
    );
}

#[test]
fn diagnostics_do_not_expose_cookie_data_or_lengths() {
    let limits = CookieLimits::new(4).unwrap();
    let retry = decode_cookie(&[0x00, 0x04, 0xde, 0xad, 0xbe, 0xef], limits).unwrap();
    let client = decode_cookie(&[0x00, 0x04, 0xde, 0xad, 0xbe, 0xef], limits).unwrap();
    let echoed = bind_cookie_echo(retry, client).unwrap();

    assert_eq!(format!("{:?}", echoed.hello_retry_request()), "Tls13Cookie");
    assert_eq!(format!("{echoed:?}"), "EchoedCookie");

    for error in [
        CookieError::InvalidMaximumCookieLength,
        CookieError::TruncatedLength,
        CookieError::EmptyCookie,
        CookieError::CookieTooLong,
        CookieError::LengthMismatch,
        CookieError::OutputTooSmall,
        CookieError::EchoMismatch,
    ] {
        let debug = format!("{error:?}");
        let display = error.to_string();
        for forbidden in ["222", "173", "190", "239", "65535"] {
            assert!(!debug.contains(forbidden));
            assert!(!display.contains(forbidden));
        }
    }
}
