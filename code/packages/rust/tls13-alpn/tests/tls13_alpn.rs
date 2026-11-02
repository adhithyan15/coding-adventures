use tls13_alpn::{
    AlpnError, ClientAlpnOffer, DecodedAlpnProtocolList, ALPN_EXTENSION_TYPE,
    MAX_ALPN_EXTENSION_DATA_LENGTH, MAX_ALPN_PROTOCOL_COUNT, MAX_ALPN_PROTOCOL_NAME_LENGTH,
};

#[test]
fn rfc_protocol_ids_round_trip_in_preference_order() {
    let protocols: [&[u8]; 2] = [b"h2", b"http/1.1"];
    let offer = ClientAlpnOffer::new(&protocols).expect("valid offer");
    let mut output = [0xa5; 16];

    assert_eq!(ALPN_EXTENSION_TYPE, 16);
    assert_eq!(offer.len(), 2);
    assert!(!offer.is_empty());
    assert_eq!(offer.get(0), Some(&b"h2"[..]));
    assert_eq!(offer.get(1), Some(&b"http/1.1"[..]));
    assert_eq!(offer.get(2), None);
    assert_eq!(offer.encoded_len(), 14);
    assert_eq!(offer.encode(&mut output).expect("encodes"), 14);
    assert_eq!(
        &output[..14],
        &[0, 12, 2, b'h', b'2', 8, b'h', b't', b't', b'p', b'/', b'1', b'.', b'1']
    );
    assert_eq!(&output[14..], &[0xa5, 0xa5]);

    let decoded = DecodedAlpnProtocolList::decode(&output[..14], 2).expect("decodes");
    assert_eq!(decoded.len(), 2);
    assert!(!decoded.is_empty());
    assert_eq!(decoded.get(0), Some(&b"h2"[..]));
    assert_eq!(decoded.get(1), Some(&b"http/1.1"[..]));
    assert_eq!(decoded.get(2), None);
    assert_eq!(decoded.iter().collect::<Vec<_>>(), protocols);
}

#[test]
fn protocol_names_are_opaque_bytes() {
    let first = [0x00, 0xff, 0x80];
    let second = [0x01];
    let protocols: [&[u8]; 2] = [&first, &second];
    let offer = ClientAlpnOffer::new(&protocols).expect("opaque values are valid");
    let mut encoded = [0_u8; 8];
    let length = offer.encode(&mut encoded).expect("encodes");
    let decoded = DecodedAlpnProtocolList::decode(&encoded[..length], 2).expect("decodes");

    assert_eq!(decoded.iter().collect::<Vec<_>>(), protocols);
}

#[test]
fn client_offer_rejects_invalid_names_duplicates_and_counts() {
    assert_eq!(
        ClientAlpnOffer::new(&[]).unwrap_err(),
        AlpnError::EmptyProtocolList
    );

    let empty: [&[u8]; 1] = [b""];
    assert_eq!(
        ClientAlpnOffer::new(&empty).unwrap_err(),
        AlpnError::EmptyProtocolName
    );

    let long = [0x42; MAX_ALPN_PROTOCOL_NAME_LENGTH + 1];
    let long_offer: [&[u8]; 1] = [&long];
    assert_eq!(
        ClientAlpnOffer::new(&long_offer).unwrap_err(),
        AlpnError::ProtocolNameTooLong
    );

    let duplicates: [&[u8]; 2] = [b"h2", b"h2"];
    assert_eq!(
        ClientAlpnOffer::new(&duplicates).unwrap_err(),
        AlpnError::DuplicateProtocol
    );

    let storage: Vec<[u8; 1]> = (0..=MAX_ALPN_PROTOCOL_COUNT as u8)
        .map(|value| [value])
        .collect();
    let too_many: Vec<&[u8]> = storage.iter().map(<[u8; 1]>::as_slice).collect();
    assert_eq!(
        ClientAlpnOffer::new(&too_many).unwrap_err(),
        AlpnError::TooManyProtocols
    );
}

#[test]
fn client_encoding_checks_capacity_before_writing() {
    let protocols: [&[u8]; 1] = [b"http/1.1"];
    let offer = ClientAlpnOffer::new(&protocols).expect("valid offer");
    let mut output = [0xa5; 10];

    assert_eq!(offer.encoded_len(), 11);
    assert_eq!(offer.encode(&mut output), Err(AlpnError::OutputTooSmall));
    assert_eq!(output, [0xa5; 10]);
}

#[test]
fn decoder_enforces_exact_framing_and_caller_work_bound() {
    assert_eq!(
        DecodedAlpnProtocolList::decode(&[0, 3, 2, b'h', b'2'], 0).unwrap_err(),
        AlpnError::InvalidProtocolCountLimit
    );
    assert_eq!(
        DecodedAlpnProtocolList::decode(&[0, 3, 2, b'h', b'2'], MAX_ALPN_PROTOCOL_COUNT + 1)
            .unwrap_err(),
        AlpnError::InvalidProtocolCountLimit
    );
    assert_eq!(
        DecodedAlpnProtocolList::decode(&[], 1).unwrap_err(),
        AlpnError::MalformedProtocolList
    );
    assert_eq!(
        DecodedAlpnProtocolList::decode(&[0], 1).unwrap_err(),
        AlpnError::MalformedProtocolList
    );
    assert_eq!(
        DecodedAlpnProtocolList::decode(&[0, 2, 1, b'a', 0], 2).unwrap_err(),
        AlpnError::MalformedProtocolList
    );
    assert_eq!(
        DecodedAlpnProtocolList::decode(&[0, 0], 1).unwrap_err(),
        AlpnError::EmptyProtocolList
    );
    assert_eq!(
        DecodedAlpnProtocolList::decode(&[0, 1, 0], 1).unwrap_err(),
        AlpnError::EmptyProtocolName
    );
    assert_eq!(
        DecodedAlpnProtocolList::decode(&[0, 2, 2, b'a'], 1).unwrap_err(),
        AlpnError::MalformedProtocolList
    );
    assert_eq!(
        DecodedAlpnProtocolList::decode(&[0, 6, 1, b'a', 1, b'b', 1, b'c'], 2).unwrap_err(),
        AlpnError::TooManyProtocols
    );
    assert_eq!(
        DecodedAlpnProtocolList::decode(&[0, 4, 1, b'a', 1, b'a'], 2).unwrap_err(),
        AlpnError::DuplicateProtocol
    );

    let oversized = vec![0_u8; MAX_ALPN_EXTENSION_DATA_LENGTH + 1];
    assert_eq!(
        DecodedAlpnProtocolList::decode(&oversized, 1).unwrap_err(),
        AlpnError::ExtensionDataTooLong
    );
}

#[test]
fn server_selection_must_be_exactly_one_offered_protocol() {
    let protocols: [&[u8]; 2] = [b"h2", b"http/1.1"];
    let offer = ClientAlpnOffer::new(&protocols).expect("valid offer");

    let selected = offer
        .validate_server_selection(&[0, 3, 2, b'h', b'2'])
        .expect("offered selection");
    assert_eq!(selected.as_bytes(), b"h2");

    assert_eq!(
        offer.validate_server_selection(&[0, 9, 8, b'h', b't', b't', b'p', b'/', b'1', b'.', b'0']),
        Err(AlpnError::UnofferedServerSelection)
    );
    assert_eq!(
        offer.validate_server_selection(&[
            0, 12, 2, b'h', b'2', 8, b'h', b't', b't', b'p', b'/', b'1', b'.', b'1'
        ]),
        Err(AlpnError::MalformedServerSelection)
    );
    assert_eq!(
        offer.validate_server_selection(&[0, 0]),
        Err(AlpnError::MalformedServerSelection)
    );
    assert_eq!(
        offer.validate_server_selection(&[0, 3, 0, b'h', b'2']),
        Err(AlpnError::MalformedServerSelection)
    );
}

#[test]
fn package_maximum_count_and_name_length_round_trip() {
    let storage: Vec<Vec<u8>> = (0..MAX_ALPN_PROTOCOL_COUNT)
        .map(|index| {
            let mut protocol = vec![index as u8; MAX_ALPN_PROTOCOL_NAME_LENGTH];
            protocol[0] = index as u8;
            protocol
        })
        .collect();
    let protocols: Vec<&[u8]> = storage.iter().map(Vec::as_slice).collect();
    let offer = ClientAlpnOffer::new(&protocols).expect("maximum offer is valid");
    let mut encoded = vec![0_u8; offer.encoded_len()];
    offer.encode(&mut encoded).expect("maximum offer encodes");
    let decoded = DecodedAlpnProtocolList::decode(&encoded, MAX_ALPN_PROTOCOL_COUNT)
        .expect("maximum offer decodes");

    assert_eq!(decoded.len(), MAX_ALPN_PROTOCOL_COUNT);
    assert_eq!(decoded.get(0), Some(storage[0].as_slice()));
    assert_eq!(
        decoded.get(MAX_ALPN_PROTOCOL_COUNT - 1),
        Some(storage[MAX_ALPN_PROTOCOL_COUNT - 1].as_slice())
    );
}

#[test]
fn diagnostics_do_not_expose_protocol_bytes_or_sizes() {
    let protocols: [&[u8]; 1] = [b"sensitive-protocol"];
    let offer = ClientAlpnOffer::new(&protocols).expect("valid offer");
    let decoded =
        DecodedAlpnProtocolList::decode(&[0, 3, 2, b'h', b'2'], 1).expect("valid decoded list");
    let selection = offer
        .validate_server_selection(&[
            0, 19, 18, b's', b'e', b'n', b's', b'i', b't', b'i', b'v', b'e', b'-', b'p', b'r',
            b'o', b't', b'o', b'c', b'o', b'l',
        ])
        .expect("offered selection");

    let offer_debug = format!("{offer:?}");
    let decoded_debug = format!("{decoded:?}");
    let iterator_debug = format!("{:?}", decoded.iter());
    let selection_debug = format!("{selection:?}");
    assert!(!offer_debug.contains("sensitive-protocol"));
    assert!(!decoded_debug.contains("h2"));
    assert_eq!(iterator_debug, "AlpnProtocolIter");
    assert!(!selection_debug.contains("sensitive-protocol"));
    for error in [
        AlpnError::InvalidProtocolCountLimit,
        AlpnError::EmptyProtocolList,
        AlpnError::TooManyProtocols,
        AlpnError::EmptyProtocolName,
        AlpnError::ProtocolNameTooLong,
        AlpnError::DuplicateProtocol,
        AlpnError::ExtensionDataTooLong,
        AlpnError::MalformedProtocolList,
        AlpnError::OutputTooSmall,
        AlpnError::MalformedServerSelection,
        AlpnError::UnofferedServerSelection,
    ] {
        assert!(!error.to_string().contains("sensitive"));
    }
}
