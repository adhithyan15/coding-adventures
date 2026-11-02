use coding_adventures_hkdf::{FixedHkdfHash, HkdfPrk};
use coding_adventures_zeroize::Zeroizing;
use std::error::Error;
use tls13_hkdf_label::{HkdfExpandLabelError, HkdfLabelError};
use tls13_traffic_secret_update::{derive_next_traffic_secret, TrafficSecretUpdateError};

fn decode_hex<const N: usize>(value: &str) -> [u8; N] {
    assert_eq!(value.len(), N * 2);
    let mut output = [0u8; N];
    for (index, byte) in output.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16).unwrap();
    }
    output
}

// These vectors were generated independently with Python's standard-library
// HMAC over the exact RFC 8446 HkdfLabel bytes. No repository HKDF or TLS code
// produced the expected values.
#[test]
fn derives_independent_sha256_vector() {
    let input: [u8; 32] = core::array::from_fn(|index| index as u8);
    let current = HkdfPrk::import_exact(FixedHkdfHash::Sha256, &input).unwrap();
    let expected =
        decode_hex::<32>("2cecd0a17506ef5fa73edc062d6e7b5397cf074ec1b4d8f99a120772932f0b45");
    let mut output = Zeroizing::new([0u8; 32]);

    derive_next_traffic_secret(&current, &mut output[..]).unwrap();

    assert_eq!(&output[..], &expected);
}

#[test]
fn derives_independent_sha384_vector() {
    let input: [u8; 48] = core::array::from_fn(|index| index as u8);
    let current = HkdfPrk::import_exact(FixedHkdfHash::Sha384, &input).unwrap();
    let expected = decode_hex::<48>(concat!(
        "401331b63e9d59f202e8f041042d9516f4cd7fa2e2ee14631d3b49fc340d7af3",
        "7fc2c0c9f252d8036f81ec5b85cbe5db"
    ));
    let mut output = Zeroizing::new([0u8; 48]);

    derive_next_traffic_secret(&current, &mut output[..]).unwrap();

    assert_eq!(&output[..], &expected);
}

#[test]
fn advances_each_hash_again_from_the_first_successor() {
    let sha256_input: [u8; 32] = core::array::from_fn(|index| index as u8);
    let sha256 = HkdfPrk::import_exact(FixedHkdfHash::Sha256, &sha256_input).unwrap();
    let mut sha256_first = Zeroizing::new([0u8; 32]);
    derive_next_traffic_secret(&sha256, &mut sha256_first[..]).unwrap();
    let sha256_first_owner =
        HkdfPrk::import_exact(FixedHkdfHash::Sha256, &sha256_first[..]).unwrap();
    let mut sha256_second = Zeroizing::new([0u8; 32]);
    derive_next_traffic_secret(&sha256_first_owner, &mut sha256_second[..]).unwrap();
    assert_eq!(
        &sha256_second[..],
        &decode_hex::<32>("2d3fb1fe267c2e0126351cc169afb7ca25c0d8867167c55c5533e3715558a6b9")
    );

    let sha384_input: [u8; 48] = core::array::from_fn(|index| index as u8);
    let sha384 = HkdfPrk::import_exact(FixedHkdfHash::Sha384, &sha384_input).unwrap();
    let mut sha384_first = Zeroizing::new([0u8; 48]);
    derive_next_traffic_secret(&sha384, &mut sha384_first[..]).unwrap();
    let sha384_first_owner =
        HkdfPrk::import_exact(FixedHkdfHash::Sha384, &sha384_first[..]).unwrap();
    let mut sha384_second = Zeroizing::new([0u8; 48]);
    derive_next_traffic_secret(&sha384_first_owner, &mut sha384_second[..]).unwrap();
    assert_eq!(
        &sha384_second[..],
        &decode_hex::<48>(concat!(
            "1fed39ab8257f49d1c0c45bfbf71ffa8525b22ea44fdbbe0f7dfff32ea98fa53",
            "864320d595ea4ab13a8292965996db9b"
        ))
    );
}

#[test]
fn current_secret_is_borrowed_and_reusable() {
    let current = HkdfPrk::import_exact(FixedHkdfHash::Sha256, &[0x42; 32]).unwrap();
    let mut first = Zeroizing::new([0u8; 32]);
    let mut second = Zeroizing::new([0u8; 32]);

    derive_next_traffic_secret(&current, &mut first[..]).unwrap();
    derive_next_traffic_secret(&current, &mut second[..]).unwrap();

    assert_eq!(&first[..], &second[..]);
}

#[test]
fn output_length_mismatch_is_atomic_for_both_hashes() {
    let sha256 = HkdfPrk::import_exact(FixedHkdfHash::Sha256, &[0x11; 32]).unwrap();
    let sha384 = HkdfPrk::import_exact(FixedHkdfHash::Sha384, &[0x22; 48]).unwrap();

    for mut output in [vec![0xA5; 31], vec![0xA5; 33]] {
        assert_eq!(
            derive_next_traffic_secret(&sha256, &mut output).unwrap_err(),
            TrafficSecretUpdateError::OutputLengthMismatch
        );
        assert!(output.iter().all(|byte| *byte == 0xA5));
    }
    for mut output in [vec![0x5A; 47], vec![0x5A; 49]] {
        assert_eq!(
            derive_next_traffic_secret(&sha384, &mut output).unwrap_err(),
            TrafficSecretUpdateError::OutputLengthMismatch
        );
        assert!(output.iter().all(|byte| *byte == 0x5A));
    }
}

#[test]
fn stable_error_omits_secret_bytes_and_observed_lengths() {
    let current = HkdfPrk::import_exact(FixedHkdfHash::Sha256, &[0xAB; 32]).unwrap();
    let mut output = [0xCD; 31];
    let error = derive_next_traffic_secret(&current, &mut output).unwrap_err();

    assert_eq!(error, TrafficSecretUpdateError::OutputLengthMismatch);
    assert_eq!(
        error.to_string(),
        "TLS 1.3 updated traffic-secret output must match the selected hash length"
    );
    assert_eq!(format!("{error:?}"), "OutputLengthMismatch");
    assert!(!error.to_string().contains("31"));
    assert!(!error.to_string().contains("ab"));
    assert!(!error.to_string().contains("cd"));
}

#[test]
fn derivation_error_preserves_only_the_stable_underlying_category() {
    let error = TrafficSecretUpdateError::Derivation(HkdfExpandLabelError::Framing(
        HkdfLabelError::EmptyLabel,
    ));

    assert_eq!(
        error.to_string(),
        "TLS 1.3 traffic-secret update failed: TLS 1.3 HKDF framing failed: \
         TLS 1.3 HKDF label is empty"
    );
    assert!(error.source().is_some());

    let mismatch = TrafficSecretUpdateError::OutputLengthMismatch;
    assert!(mismatch.source().is_none());
}
