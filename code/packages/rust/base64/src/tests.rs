//! Tests for the base64 codec.
//!
//! The oracle is RFC 4648 §10's own table, transcribed verbatim rather than
//! generated from this implementation. A test that computes its expectation
//! the way the code computes its answer agrees with any bug they share.

use super::*;

/// RFC 4648 §10, exactly as printed.
///
/// These seven cover every length modulo 3, which is where the padding rules
/// differ and where hand-written base64 goes wrong.
const RFC_4648_VECTORS: &[(&str, &str)] = &[
    ("", ""),
    ("f", "Zg=="),
    ("fo", "Zm8="),
    ("foo", "Zm9v"),
    ("foob", "Zm9vYg=="),
    ("fooba", "Zm9vYmE="),
    ("foobar", "Zm9vYmFy"),
];

#[test]
fn matches_the_rfc_4648_test_vectors() {
    for (plain, encoded) in RFC_4648_VECTORS {
        assert_eq!(
            encode(plain.as_bytes(), &STANDARD),
            *encoded,
            "encoding {plain:?}"
        );
        assert_eq!(
            decode(encoded, &STANDARD).unwrap(),
            plain.as_bytes(),
            "decoding {encoded:?}"
        );
    }
}

#[test]
fn the_unpadded_variant_is_the_padded_one_without_the_equals() {
    for (plain, encoded) in RFC_4648_VECTORS {
        let expected = encoded.trim_end_matches('=');
        assert_eq!(encode(plain.as_bytes(), &STANDARD_NO_PAD), expected);
        assert_eq!(
            decode(expected, &STANDARD_NO_PAD).unwrap(),
            plain.as_bytes()
        );
    }
}

/// The two alphabets differ only in symbols 62 and 63.
///
/// Byte patterns are chosen so those symbols actually appear -- an alphabet
/// test that never emits `+` or `/` would pass with the tables swapped.
#[test]
fn url_safe_swaps_the_last_two_symbols() {
    let input = [0xfb, 0xff, 0xbf];
    let standard = encode(&input, &STANDARD);
    let url_safe = encode(&input, &URL_SAFE);

    assert!(
        standard.contains('+') && standard.contains('/'),
        "the fixture must exercise symbols 62 and 63: {standard}"
    );
    assert!(!url_safe.contains('+') && !url_safe.contains('/'));
    assert_eq!(url_safe, standard.replace('+', "-").replace('/', "_"));

    assert_eq!(decode(&url_safe, &URL_SAFE).unwrap(), input);
}

/// Every length from 0 to 1024, not a handful of sizes.
///
/// The tail cases are what separate a correct implementation from a nearly
/// correct one, and they recur every three bytes.
#[test]
fn round_trips_every_length_up_to_1024() {
    for alphabet in [&STANDARD, &STANDARD_NO_PAD, &URL_SAFE, &URL_SAFE_NO_PAD] {
        for length in 0..=1024usize {
            // A varied byte pattern rather than zeros: all-zero input encodes
            // to all-`A`, which would not exercise the symbol table.
            let input: Vec<u8> = (0..length).map(|i| (i * 37 + i / 3) as u8).collect();
            let encoded = encode(&input, alphabet);
            assert_eq!(
                encoded.len(),
                encoded_len(length, alphabet),
                "encoded_len disagreed with encode at length {length}"
            );
            assert_eq!(
                decode(&encoded, alphabet).unwrap(),
                input,
                "round trip failed at length {length}"
            );
        }
    }
}

#[test]
fn encode_into_appends_rather_than_replacing() {
    let mut buffer = String::from("prefix:");
    encode_into(b"foobar", &STANDARD, &mut buffer);
    assert_eq!(buffer, "prefix:Zm9vYmFy");
}

// ---------------------------------------------------------------------------
// Strictness. Each of these is a way a lax decoder invents bytes nobody wrote.
// ---------------------------------------------------------------------------

#[test]
fn rejects_a_byte_outside_the_alphabet() {
    let error = decode("Zm9v*mFy", &STANDARD).unwrap_err();
    assert_eq!(
        error,
        DecodeError::InvalidByte {
            offset: 4,
            byte: b'*'
        }
    );
}

/// Whitespace is not stripped.
///
/// Callers that want to accept wrapped input should strip it themselves and
/// say so; doing it here would silently accept a class of input the encoder
/// never produces.
#[test]
fn rejects_whitespace_including_newlines() {
    // Rejection is the property. WHICH error depends on where the whitespace
    // lands: at a length that is already impossible the length check fires
    // first, and only a well-formed length reaches the symbol scan.
    for input in ["Zm9v YmFy", "Zm9v\nYmFy", "Zm9vYmFy\n", " Zm9vYmFy"] {
        assert!(
            decode(input, &STANDARD).is_err(),
            "whitespace should be rejected: {input:?}"
        );
    }

    // Length 8 -- a valid base64 length -- so this one reaches the scan and
    // names the offending byte, which is the diagnostic a caller wants.
    assert_eq!(
        decode("Zm9vYm y", &STANDARD).unwrap_err(),
        DecodeError::InvalidByte {
            offset: 6,
            byte: b' '
        }
    );
}

/// The alphabets do not accept each other's symbols.
///
/// Silently taking both is how a URL-safe payload decodes correctly under a
/// standard decoder right up until a byte lands on `+`.
#[test]
fn each_alphabet_rejects_the_others_symbols() {
    let url_safe_text = encode(&[0xfb, 0xff, 0xbf], &URL_SAFE);
    assert!(url_safe_text.contains('-') || url_safe_text.contains('_'));
    assert!(matches!(
        decode(&url_safe_text, &STANDARD),
        Err(DecodeError::InvalidByte { .. })
    ));

    let standard_text = encode(&[0xfb, 0xff, 0xbf], &STANDARD);
    assert!(matches!(
        decode(&standard_text, &URL_SAFE),
        Err(DecodeError::InvalidByte { .. })
    ));
}

#[test]
fn rejects_an_impossible_length() {
    // 5 symbols: 4 make three bytes, and a lone 5th carries 6 bits that no
    // byte count can need.
    assert_eq!(
        decode("Zm9vY", &STANDARD_NO_PAD).unwrap_err(),
        DecodeError::InvalidLength { length: 5 }
    );
}

#[test]
fn rejects_padding_on_an_unpadded_alphabet() {
    assert!(matches!(
        decode("Zg==", &STANDARD_NO_PAD),
        Err(DecodeError::InvalidPadding { .. })
    ));
}

#[test]
fn rejects_wrong_and_missing_padding() {
    for input in [
        "Zg=",    // needs two
        "Zm8==",  // needs one
        "Zg===",  // three is never right
        "Zg",     // padded alphabet, no padding
        "Zm9vYg", // ditto, longer
    ] {
        assert!(
            matches!(
                decode(input, &STANDARD),
                Err(DecodeError::InvalidPadding { .. })
                    | Err(DecodeError::InvalidLength { .. })
            ),
            "should have rejected {input:?}"
        );
    }
}

/// The bits a decode discards must be zero.
///
/// `"Zg=="` is canonical for `f`; `"Zh=="` decodes the same byte from a
/// different string. Accepting both means two inputs map to one output, and
/// anything computed over the decoded bytes -- a hash, a signature, an
/// equality check -- stops being a function of the input.
#[test]
fn rejects_non_canonical_trailing_bits() {
    assert_eq!(decode("Zg==", &STANDARD).unwrap(), b"f");
    assert_eq!(
        decode("Zh==", &STANDARD).unwrap_err(),
        DecodeError::NonCanonical { offset: 1 }
    );

    // Three-symbol tail: the low two bits of the third symbol are discarded.
    assert_eq!(decode("Zm8=", &STANDARD).unwrap(), b"fo");
    assert_eq!(
        decode("Zm9=", &STANDARD).unwrap_err(),
        DecodeError::NonCanonical { offset: 2 }
    );
}

#[test]
fn errors_describe_themselves() {
    let rendered = DecodeError::InvalidByte {
        offset: 4,
        byte: b'*',
    }
    .to_string();
    assert!(rendered.contains("offset 4"), "{rendered}");
    assert!(rendered.contains("0x2a"), "{rendered}");

    for error in [
        DecodeError::InvalidLength { length: 5 },
        DecodeError::InvalidPadding { offset: 2 },
        DecodeError::NonCanonical { offset: 1 },
    ] {
        assert!(!error.to_string().is_empty());
    }
}

#[test]
fn length_helpers_agree_with_reality() {
    for length in 0..=64usize {
        let input = vec![0xa5; length];
        for alphabet in [&STANDARD, &STANDARD_NO_PAD, &URL_SAFE, &URL_SAFE_NO_PAD] {
            let encoded = encode(&input, alphabet);
            assert_eq!(encoded.len(), encoded_len(length, alphabet));
            assert!(
                decoded_len_estimate(encoded.len()) >= length,
                "estimate must not undercount at length {length}"
            );
        }
    }
}

#[test]
fn is_padded_reports_the_variant() {
    assert!(STANDARD.is_padded() && URL_SAFE.is_padded());
    assert!(!STANDARD_NO_PAD.is_padded() && !URL_SAFE_NO_PAD.is_padded());
}

/// All 256 byte values survive, in every alignment.
///
/// Rotating the offset walks each byte through all three positions of a group,
/// so a symbol-table error affecting only the middle byte cannot hide.
#[test]
fn every_byte_value_round_trips_in_every_position() {
    for offset in 0..3usize {
        let mut input = vec![0u8; offset];
        input.extend((0..=255u8).collect::<Vec<_>>());
        let encoded = encode(&input, &STANDARD);
        assert_eq!(decode(&encoded, &STANDARD).unwrap(), input);
    }
}
