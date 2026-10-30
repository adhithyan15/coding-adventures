use coding_adventures_chacha20_poly1305::{
    aead_decrypt_fixed, aead_encrypt, aead_encrypt_fixed, FixedAeadError,
};

fn fixed<const N: usize>(value: &str) -> [u8; N] {
    hex::decode(value)
        .expect("test vector must be hexadecimal")
        .try_into()
        .unwrap_or_else(|_| panic!("test vector must contain exactly {N} bytes"))
}

fn vector_key() -> [u8; 32] {
    fixed::<32>("808182838485868788898a8b8c8d8e8f909192939495969798999a9b9c9d9e9f")
}

fn vector_nonce() -> [u8; 12] {
    fixed::<12>("070000004041424344454647")
}

#[test]
fn fixed_aead_matches_rfc_8439_section_2_8_2() {
    let key = vector_key();
    let nonce = vector_nonce();
    let aad = fixed::<12>("50515253c0c1c2c3c4c5c6c7");
    let plaintext = b"Ladies and Gentlemen of the class of '99: If I could offer you only one tip for the future, sunscreen would be it.";
    let expected_ciphertext = hex::decode(
        "d31a8d34648e60db7b86afbc53ef7ec2\
         a4aded51296e08fea9e2b5a736ee62d6\
         3dbea45e8ca9671282fafb69da92728b\
         1a71de0a9e060b2905d6a5b67ecd3b36\
         92ddbd7f2d778b8c9803aee328091b58\
         fab324e4fad675945585808b4831d7bc\
         3ff4def08e4b7a9de576d26586cec64b\
         6116",
    )
    .expect("RFC ciphertext must be hexadecimal");
    let expected_tag = fixed::<16>("1ae10b594f09e26a7e902ecbd0600691");

    let mut ciphertext = vec![0xa5; plaintext.len()];
    let mut tag = [0x5a; 16];
    aead_encrypt_fixed(plaintext, &key, &nonce, &aad, &mut ciphertext, &mut tag)
        .expect("RFC encryption must succeed");

    assert_eq!(ciphertext, expected_ciphertext);
    assert_eq!(tag, expected_tag);

    let mut recovered = vec![0xa5; plaintext.len()];
    aead_decrypt_fixed(&ciphertext, &key, &nonce, &aad, &tag, &mut recovered)
        .expect("RFC decryption must succeed");
    assert_eq!(recovered, plaintext);
}

#[test]
fn fixed_aead_matches_existing_rfc_composition_at_block_edges() {
    let key = fixed::<32>("000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f");
    let nonce = fixed::<12>("000000000000000000000002");

    for plaintext_length in [0, 1, 15, 16, 17, 63, 64, 65, 255] {
        for aad_length in [0, 1, 15, 16, 17, 31] {
            let plaintext: Vec<u8> = (0..plaintext_length)
                .map(|index| (index % 251) as u8)
                .collect();
            let aad: Vec<u8> = (0..aad_length).map(|index| 0xf0 ^ (index as u8)).collect();
            let (expected_ciphertext, expected_tag) = aead_encrypt(&plaintext, &key, &nonce, &aad);

            let mut ciphertext = vec![0xa5; plaintext.len()];
            let mut tag = [0x5a; 16];
            aead_encrypt_fixed(&plaintext, &key, &nonce, &aad, &mut ciphertext, &mut tag)
                .expect("edge-length encryption must succeed");
            assert_eq!(ciphertext, expected_ciphertext);
            assert_eq!(tag, expected_tag);

            let mut recovered = vec![0xa5; plaintext.len()];
            aead_decrypt_fixed(&ciphertext, &key, &nonce, &aad, &tag, &mut recovered)
                .expect("edge-length decryption must succeed");
            assert_eq!(recovered, plaintext);
        }
    }
}

#[test]
fn authentication_failures_never_release_plaintext() {
    let key = vector_key();
    let nonce = vector_nonce();
    let aad = b"authenticated header";
    let plaintext = b"plaintext must remain hidden until authentication succeeds";
    let mut ciphertext = vec![0; plaintext.len()];
    let mut tag = [0; 16];
    aead_encrypt_fixed(plaintext, &key, &nonce, aad, &mut ciphertext, &mut tag)
        .expect("setup encryption must succeed");

    let mut wrong_tag = tag;
    wrong_tag[7] ^= 0x80;
    let mut output = vec![0xa5; plaintext.len()];
    assert_eq!(
        aead_decrypt_fixed(&ciphertext, &key, &nonce, aad, &wrong_tag, &mut output),
        Err(FixedAeadError::AuthenticationFailed),
    );
    assert_eq!(output, vec![0xa5; plaintext.len()]);

    let mut tampered_ciphertext = ciphertext.clone();
    tampered_ciphertext[3] ^= 0x01;
    assert_eq!(
        aead_decrypt_fixed(&tampered_ciphertext, &key, &nonce, aad, &tag, &mut output),
        Err(FixedAeadError::AuthenticationFailed),
    );
    assert_eq!(output, vec![0xa5; plaintext.len()]);

    assert_eq!(
        aead_decrypt_fixed(
            &ciphertext,
            &key,
            &nonce,
            b"different header",
            &tag,
            &mut output,
        ),
        Err(FixedAeadError::AuthenticationFailed),
    );
    assert_eq!(output, vec![0xa5; plaintext.len()]);
}

#[test]
fn output_length_failures_are_atomic_for_both_operations() {
    let key = vector_key();
    let nonce = vector_nonce();
    let plaintext = b"length mismatch";

    let mut short_ciphertext = vec![0xa5; plaintext.len() - 1];
    let mut tag = [0x5a; 16];
    assert_eq!(
        aead_encrypt_fixed(
            plaintext,
            &key,
            &nonce,
            b"aad",
            &mut short_ciphertext,
            &mut tag,
        ),
        Err(FixedAeadError::OutputLengthMismatch),
    );
    assert_eq!(short_ciphertext, vec![0xa5; plaintext.len() - 1]);
    assert_eq!(tag, [0x5a; 16]);

    let mut ciphertext = vec![0; plaintext.len()];
    aead_encrypt_fixed(plaintext, &key, &nonce, b"aad", &mut ciphertext, &mut tag)
        .expect("setup encryption must succeed");
    let mut long_plaintext = vec![0xa5; plaintext.len() + 1];
    assert_eq!(
        aead_decrypt_fixed(&ciphertext, &key, &nonce, b"aad", &tag, &mut long_plaintext,),
        Err(FixedAeadError::OutputLengthMismatch),
    );
    assert_eq!(long_plaintext, vec![0xa5; plaintext.len() + 1]);
}

#[test]
fn fixed_aead_errors_are_stable_and_payload_free() {
    assert_eq!(
        FixedAeadError::OutputLengthMismatch.to_string(),
        "AEAD output length must equal input length",
    );
    assert_eq!(
        FixedAeadError::LengthOverflow.to_string(),
        "AEAD input length exceeds the RFC 8439 encoding",
    );
    assert_eq!(
        FixedAeadError::CounterExhausted.to_string(),
        "AEAD request exceeds the ChaCha20 block counter",
    );
    assert_eq!(
        FixedAeadError::AuthenticationFailed.to_string(),
        "AEAD authentication failed",
    );
}
