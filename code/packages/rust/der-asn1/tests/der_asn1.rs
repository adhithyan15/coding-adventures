use der_asn1::{
    decode_bit_string, decode_boolean, decode_enumerated, decode_ia5_string,
    decode_implicit_bit_string, decode_implicit_ia5_string, decode_implicit_object_identifier,
    decode_implicit_octet_string, decode_integer, decode_null, decode_object_identifier,
    decode_octet_string, Asn1Decoder, Asn1ErrorKind, Asn1Limits,
};

fn decode(input: &[u8]) -> (Asn1Decoder, der_asn1::Asn1Element<'_>) {
    let mut decoder = Asn1Decoder::new(Asn1Limits::default());
    let element = decoder.decode_exact(input).expect("valid root");
    (decoder, element)
}

#[test]
fn canonical_primitive_values_borrow_the_input() {
    let (_, boolean) = decode(&[0x01, 0x01, 0xff]);
    assert!(decode_boolean(boolean).unwrap());

    let (_, integer) = decode(&[0x02, 0x02, 0x00, 0x80]);
    let integer = decode_integer(integer).unwrap();
    assert!(!integer.is_negative());
    assert_eq!(integer.signed_bytes(), &[0x00, 0x80]);
    assert_eq!(integer.to_u64().unwrap(), 128);

    let (_, octets) = decode(&[0x04, 0x03, 0xaa, 0xbb, 0xcc]);
    assert_eq!(decode_octet_string(octets).unwrap(), &[0xaa, 0xbb, 0xcc]);

    let (_, ia5) = decode(&[0x16, 0x03, b'a', b'b', b'c']);
    assert_eq!(decode_ia5_string(ia5).unwrap(), "abc");

    let (_, null) = decode(&[0x05, 0x00]);
    decode_null(null).unwrap();
}

#[test]
fn implicit_primitive_values_require_the_exact_context_tag() {
    let (_, octets) = decode(&[0x87, 0x04, 192, 0, 2, 1]);
    assert_eq!(
        decode_implicit_octet_string(octets, 7).unwrap(),
        &[192, 0, 2, 1]
    );

    let (_, ia5) = decode(&[
        0x82, 0x0b, b'e', b'x', b'a', b'm', b'p', b'l', b'e', b'.', b'c', b'o', b'm',
    ]);
    assert_eq!(decode_implicit_ia5_string(ia5, 2).unwrap(), "example.com");

    let (_, oid) = decode(&[0x88, 0x03, 0x2a, 0x03, 0x04]);
    let oid = decode_implicit_object_identifier(oid, 8, Asn1Limits::default()).unwrap();
    assert!(oid.equals(&[1, 2, 3, 4]));

    for input in [&[0x04, 0x00][..], &[0xa7, 0x00], &[0x86, 0x00]] {
        let (_, element) = decode(input);
        assert_eq!(
            decode_implicit_octet_string(element, 7).unwrap_err().kind(),
            Asn1ErrorKind::UnexpectedTag
        );
    }
}

#[test]
fn implicit_bit_strings_reuse_canonical_content_validation() {
    let (_, element) = decode(&[0x81, 0x02, 0x03, 0xa0]);
    let bits = decode_implicit_bit_string(element, 1).unwrap();
    assert_eq!(bits.bytes(), &[0xa0]);
    assert_eq!(bits.unused_bits(), 3);
    assert_eq!(bits.bit_len(), 5);

    assert_eq!(
        decode_implicit_bit_string(element, 2).unwrap_err().kind(),
        Asn1ErrorKind::UnexpectedTag
    );

    let (_, non_zero_padding) = decode(&[0x81, 0x02, 0x03, 0xa1]);
    assert_eq!(
        decode_implicit_bit_string(non_zero_padding, 1)
            .unwrap_err()
            .kind(),
        Asn1ErrorKind::NonZeroBitPadding
    );
}

#[test]
fn ia5_string_rejects_non_ascii_at_the_exact_value_offset() {
    let (_, universal) = decode(&[0x16, 0x03, b'a', 0x80, b'b']);
    let error = decode_ia5_string(universal).unwrap_err();
    assert_eq!(error.kind(), Asn1ErrorKind::NonAsciiIa5String);
    assert_eq!(error.offset(), 3);

    let (_, implicit) = decode(&[0x82, 0x02, b'a', 0xff]);
    let error = decode_implicit_ia5_string(implicit, 2).unwrap_err();
    assert_eq!(error.kind(), Asn1ErrorKind::NonAsciiIa5String);
    assert_eq!(error.offset(), 3);
    assert!(!error.to_string().contains("ff"));
}

#[test]
fn implicit_object_identifier_reuses_canonicality_and_arc_limits() {
    for input in [&[0x88, 0x00][..], &[0x88, 0x01, 0x80], &[0x88, 0x01, 0x81]] {
        let (_, element) = decode(input);
        assert!(
            decode_implicit_object_identifier(element, 8, Asn1Limits::default()).is_err(),
            "accepted {input:02x?}"
        );
    }

    let limits = Asn1Limits {
        max_oid_arcs: 3,
        ..Asn1Limits::default()
    };
    let (_, element) = decode(&[0x88, 0x03, 0x2a, 0x03, 0x04]);
    assert_eq!(
        decode_implicit_object_identifier(element, 8, limits)
            .unwrap_err()
            .kind(),
        Asn1ErrorKind::OidArcLimitExceeded
    );
}

#[test]
fn boolean_rejects_alternate_ber_encodings() {
    for input in [
        &[0x01, 0x00][..],
        &[0x01, 0x02, 0x00, 0x00],
        &[0x01, 0x01, 0x01],
    ] {
        let (_, element) = decode(input);
        assert!(decode_boolean(element).is_err(), "accepted {input:02x?}");
    }
}

#[test]
fn integer_enforces_minimal_twos_complement() {
    for input in [
        &[0x02, 0x00][..],
        &[0x02, 0x02, 0x00, 0x7f],
        &[0x02, 0x02, 0xff, 0x80],
    ] {
        let (_, element) = decode(input);
        assert!(decode_integer(element).is_err(), "accepted {input:02x?}");
    }

    let (_, negative) = decode(&[0x02, 0x01, 0xff]);
    let negative = decode_integer(negative).unwrap();
    assert!(negative.is_negative());
    assert_eq!(
        negative.to_u64().unwrap_err().kind(),
        Asn1ErrorKind::NegativeInteger
    );

    let (_, largest) = decode(&[
        0x02, 0x09, 0x00, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    ]);
    assert_eq!(decode_integer(largest).unwrap().to_u64().unwrap(), u64::MAX);

    let (_, too_wide) = decode(&[
        0x02, 0x09, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ]);
    assert_eq!(
        decode_integer(too_wide)
            .unwrap()
            .to_u64()
            .unwrap_err()
            .kind(),
        Asn1ErrorKind::IntegerOverflow
    );
}

#[test]
fn enumerated_borrows_canonical_twos_complement_values() {
    for (input, expected) in [
        (&[0x0a, 0x01, 0x00][..], 0),
        (&[0x0a, 0x01, 0x7f], 127),
        (&[0x0a, 0x02, 0x00, 0x80], 128),
        (
            &[
                0x0a, 0x09, 0x00, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            ],
            u64::MAX,
        ),
    ] {
        let (_, element) = decode(input);
        let value = decode_enumerated(element).unwrap();
        assert!(!value.is_negative());
        assert_eq!(value.to_u64().unwrap(), expected);
        assert_eq!(value.signed_bytes(), &input[2..]);
    }

    let (_, negative) = decode(&[0x0a, 0x01, 0xff]);
    let negative = decode_enumerated(negative).unwrap();
    assert!(negative.is_negative());
    assert_eq!(negative.signed_bytes(), &[0xff]);
    assert_eq!(
        negative.to_u64().unwrap_err().kind(),
        Asn1ErrorKind::NegativeInteger
    );
}

#[test]
fn enumerated_rejects_wrong_tags_and_noncanonical_values() {
    for (input, kind) in [
        (&[0x0a, 0x00][..], Asn1ErrorKind::EmptyInteger),
        (&[0x0a, 0x02, 0x00, 0x7f], Asn1ErrorKind::NonMinimalInteger),
        (&[0x0a, 0x02, 0xff, 0x80], Asn1ErrorKind::NonMinimalInteger),
        (&[0x2a, 0x01, 0x00], Asn1ErrorKind::UnexpectedTag),
        (&[0x02, 0x01, 0x00], Asn1ErrorKind::UnexpectedTag),
    ] {
        let (_, element) = decode(input);
        assert_eq!(decode_enumerated(element).unwrap_err().kind(), kind);
    }

    let (_, enumerated) = decode(&[0x0a, 0x01, 0x00]);
    assert_eq!(
        decode_integer(enumerated).unwrap_err().kind(),
        Asn1ErrorKind::UnexpectedTag
    );

    let (_, too_wide) = decode(&[
        0x0a, 0x09, 0x01, 0xde, 0xad, 0xbe, 0xef, 0xca, 0xfe, 0xba, 0xbe,
    ]);
    let error = decode_enumerated(too_wide).unwrap().to_u64().unwrap_err();
    assert_eq!(error.kind(), Asn1ErrorKind::IntegerOverflow);
    assert_eq!(error.offset(), 2);
    assert!(!error.to_string().contains("deadbeef"));
}

#[test]
fn bit_string_checks_unused_bits_and_padding() {
    let (_, element) = decode(&[0x03, 0x02, 0x03, 0xa8]);
    let bits = decode_bit_string(element).unwrap();
    assert_eq!(bits.unused_bits(), 3);
    assert_eq!(bits.bytes(), &[0xa8]);
    assert_eq!(bits.bit_len(), 5);

    for input in [
        &[0x03, 0x00][..],
        &[0x03, 0x01, 0x01],
        &[0x03, 0x02, 0x08, 0x00],
        &[0x03, 0x02, 0x03, 0xab],
    ] {
        let (_, element) = decode(input);
        assert!(decode_bit_string(element).is_err(), "accepted {input:02x?}");
    }
}

#[test]
fn null_and_tags_are_exact() {
    let (_, nonempty_null) = decode(&[0x05, 0x01, 0x00]);
    assert_eq!(
        decode_null(nonempty_null).unwrap_err().kind(),
        Asn1ErrorKind::NonEmptyNull
    );

    let (_, constructed_boolean) = decode(&[0x21, 0x01, 0xff]);
    assert_eq!(
        decode_boolean(constructed_boolean).unwrap_err().kind(),
        Asn1ErrorKind::UnexpectedTag
    );
}

#[test]
fn object_identifier_decodes_first_and_later_arcs_without_allocation() {
    let (_, element) = decode(&[0x06, 0x06, 0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d]);
    let oid = decode_object_identifier(element, Asn1Limits::default()).unwrap();
    assert_eq!(oid.arcs().collect::<Vec<_>>(), [1, 2, 840, 113549]);
    assert!(oid.equals(&[1, 2, 840, 113549]));
    assert!(!oid.equals(&[1, 2, 840]));

    let (_, large_first) = decode(&[0x06, 0x02, 0x88, 0x37]);
    assert_eq!(
        decode_object_identifier(large_first, Asn1Limits::default())
            .unwrap()
            .arcs()
            .collect::<Vec<_>>(),
        [2, 999]
    );
}

#[test]
fn object_identifier_rejects_malformed_and_over_budget_values() {
    for input in [
        &[0x06, 0x00][..],
        &[0x06, 0x01, 0x80],
        &[0x06, 0x01, 0x81],
        &[
            0x06, 0x0a, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x7f,
        ],
    ] {
        let (_, element) = decode(input);
        assert!(
            decode_object_identifier(element, Asn1Limits::default()).is_err(),
            "accepted {input:02x?}"
        );
    }

    let limits = Asn1Limits {
        max_oid_arcs: 3,
        ..Asn1Limits::default()
    };
    let (_, element) = decode(&[0x06, 0x03, 0x2a, 0x03, 0x04]);
    assert_eq!(
        decode_object_identifier(element, limits)
            .unwrap_err()
            .kind(),
        Asn1ErrorKind::OidArcLimitExceeded
    );
}

#[test]
fn sequence_set_and_explicit_wrappers_are_exact() {
    let (mut decoder, sequence) = decode(&[0x30, 0x05, 0x02, 0x01, 0x01, 0x05, 0x00]);
    let mut children = decoder.sequence(sequence).unwrap();
    assert_eq!(
        decode_integer(children.read(&mut decoder).unwrap().unwrap())
            .unwrap()
            .to_u64()
            .unwrap(),
        1
    );
    decode_null(children.read(&mut decoder).unwrap().unwrap()).unwrap();
    assert!(children.read(&mut decoder).unwrap().is_none());
    children.finish().unwrap();

    let (decoder, set) = decode(&[0x31, 0x00]);
    assert!(decoder.set(set).is_ok());
    assert_eq!(
        decoder.sequence(set).unwrap_err().kind(),
        Asn1ErrorKind::UnexpectedTag
    );

    let (mut decoder, explicit) = decode(&[0xa3, 0x03, 0x02, 0x01, 0x02]);
    let child = decoder.explicit(explicit, 3).unwrap();
    assert_eq!(decode_integer(child).unwrap().to_u64().unwrap(), 2);

    let (mut decoder, two_children) = decode(&[0xa3, 0x04, 0x05, 0x00, 0x05, 0x00]);
    assert!(decoder.explicit(two_children, 3).is_err());
}

#[test]
fn implicit_constructed_wrappers_are_exact_bounded_cursors() {
    // NameConstraints permittedSubtrees [0] containing one GeneralSubtree
    // SEQUENCE whose base is the dNSName constraint ".tr".
    let (mut decoder, permitted) = decode(&[0xa0, 0x07, 0x30, 0x05, 0x82, 0x03, b'.', b't', b'r']);
    let mut subtrees = decoder.implicit_constructed(permitted, 0).unwrap();
    let subtree = subtrees.read(&mut decoder).unwrap().unwrap();
    let mut fields = decoder.sequence(subtree).unwrap();
    let base = fields.read(&mut decoder).unwrap().unwrap();
    assert_eq!(decode_implicit_ia5_string(base, 2).unwrap(), ".tr");
    assert!(fields.read(&mut decoder).unwrap().is_none());
    fields.finish().unwrap();
    assert!(subtrees.read(&mut decoder).unwrap().is_none());
    subtrees.finish().unwrap();

    for input in [&[0x30, 0x00][..], &[0x80, 0x00], &[0xa1, 0x00]] {
        let (decoder, element) = decode(input);
        assert_eq!(
            decoder.implicit_constructed(element, 0).unwrap_err().kind(),
            Asn1ErrorKind::UnexpectedTag
        );
    }
}

#[test]
fn depth_and_total_element_limits_are_shared() {
    let limits = Asn1Limits {
        max_depth: 1,
        ..Asn1Limits::default()
    };
    let mut decoder = Asn1Decoder::new(limits);
    let root = decoder.decode_exact(&[0x30, 0x00]).unwrap();
    assert_eq!(
        decoder.sequence(root).unwrap_err().kind(),
        Asn1ErrorKind::DepthLimitExceeded
    );

    let mut decoder = Asn1Decoder::new(limits);
    let root = decoder.decode_exact(&[0xa0, 0x00]).unwrap();
    assert_eq!(
        decoder.implicit_constructed(root, 0).unwrap_err().kind(),
        Asn1ErrorKind::DepthLimitExceeded
    );

    let limits = Asn1Limits {
        max_total_elements: 2,
        ..Asn1Limits::default()
    };
    let mut decoder = Asn1Decoder::new(limits);
    let root = decoder
        .decode_exact(&[0x30, 0x06, 0x05, 0x00, 0x05, 0x00, 0x05, 0x00])
        .unwrap();
    let mut children = decoder.sequence(root).unwrap();
    children.read(&mut decoder).unwrap().unwrap();
    let remaining = children.remaining();
    assert_eq!(
        children.read(&mut decoder).unwrap_err().kind(),
        Asn1ErrorKind::ElementLimitExceeded
    );
    assert_eq!(children.remaining(), remaining);
    assert_eq!(decoder.elements_read(), 2);
}

#[test]
fn failed_child_read_changes_neither_cursor_nor_budget() {
    let (mut decoder, root) = decode(&[0x30, 0x02, 0x02, 0x01]);
    let mut children = decoder.sequence(root).unwrap();
    let remaining = children.remaining();
    let count = decoder.elements_read();
    assert!(children.read(&mut decoder).is_err());
    assert_eq!(children.remaining(), remaining);
    assert_eq!(decoder.elements_read(), count);
}

#[test]
fn cursor_rejects_a_decoder_with_different_limits() {
    let (decoder, root) = decode(&[0x30, 0x02, 0x05, 0x00]);
    let mut children = decoder.sequence(root).unwrap();
    let remaining = children.remaining();

    let different_limits = Asn1Limits {
        max_total_elements: Asn1Limits::default().max_total_elements - 1,
        ..Asn1Limits::default()
    };
    let mut different_decoder = Asn1Decoder::new(different_limits);
    assert_eq!(
        children.read(&mut different_decoder).unwrap_err().kind(),
        Asn1ErrorKind::DecoderLimitMismatch
    );
    assert_eq!(children.remaining(), remaining);
    assert_eq!(different_decoder.elements_read(), 0);
}

#[test]
fn root_rejects_trailing_data_and_zero_depth_budget() {
    let mut decoder = Asn1Decoder::new(Asn1Limits::default());
    assert!(decoder.decode_exact(&[0x05, 0x00, 0x00]).is_err());
    assert_eq!(decoder.elements_read(), 0);

    let limits = Asn1Limits {
        max_depth: 0,
        ..Asn1Limits::default()
    };
    let mut decoder = Asn1Decoder::new(limits);
    assert_eq!(
        decoder.decode_exact(&[0x05, 0x00]).unwrap_err().kind(),
        Asn1ErrorKind::DepthLimitExceeded
    );
}
