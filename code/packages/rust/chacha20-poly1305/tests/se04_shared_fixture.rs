use coding_adventures_chacha20_poly1305::{
    hchacha20_subkey, xchacha20_encrypt, xchacha20_poly1305_aead_decrypt,
    xchacha20_poly1305_aead_encrypt,
};
use serde_json::Value;
use std::path::PathBuf;

fn fixture() -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/fixtures/se04-xchacha20-poly1305-v1/cases.json");
    serde_json::from_str(&std::fs::read_to_string(path).expect("read SE04 fixture"))
        .expect("parse SE04 fixture")
}

fn hex_bytes(value: &Value) -> Vec<u8> {
    hex::decode(value.as_str().expect("fixture hex string")).expect("valid fixture hex")
}

fn fixed<const N: usize>(value: &Value) -> [u8; N] {
    hex_bytes(value)
        .try_into()
        .unwrap_or_else(|bytes: Vec<u8>| panic!("expected {N} bytes, got {}", bytes.len()))
}

#[test]
fn consumes_closed_se04_fixture() {
    let fixture = fixture();
    assert_eq!(fixture["schema_version"], 1);
    assert_eq!(fixture["profile"], "se04-xchacha20-poly1305-v1");
    assert_eq!(fixture["authentication_failure"], "authentication_failed");
    assert_eq!(fixture["hchacha20_cases"].as_array().unwrap().len(), 1);
    assert_eq!(fixture["xchacha20_cases"].as_array().unwrap().len(), 2);
    assert_eq!(fixture["aead_cases"].as_array().unwrap().len(), 3);
    assert_eq!(fixture["mutations"].as_array().unwrap().len(), 5);

    for test_case in fixture["hchacha20_cases"].as_array().unwrap() {
        let key = fixed::<32>(&test_case["key_hex"]);
        let nonce = fixed::<16>(&test_case["nonce_hex"]);
        assert_eq!(
            hchacha20_subkey(&key, &nonce),
            fixed::<32>(&test_case["subkey_hex"]),
            "{}",
            test_case["id"]
        );
    }

    for test_case in fixture["xchacha20_cases"].as_array().unwrap() {
        let key = fixed::<32>(&test_case["key_hex"]);
        let nonce = fixed::<24>(&test_case["nonce_hex"]);
        let input = hex_bytes(&test_case["input_hex"]);
        let counter = test_case["counter"].as_u64().unwrap() as u32;
        let output = xchacha20_encrypt(&input, &key, &nonce, counter);
        assert_eq!(
            output,
            hex_bytes(&test_case["output_hex"]),
            "{}",
            test_case["id"]
        );
        assert_eq!(
            xchacha20_encrypt(&output, &key, &nonce, counter),
            input,
            "{}",
            test_case["id"]
        );
    }

    for test_case in fixture["aead_cases"].as_array().unwrap() {
        let key = fixed::<32>(&test_case["key_hex"]);
        let nonce = fixed::<24>(&test_case["nonce_hex"]);
        let aad = hex_bytes(&test_case["aad_hex"]);
        let plaintext = hex_bytes(&test_case["plaintext_hex"]);
        let expected_ciphertext = hex_bytes(&test_case["ciphertext_hex"]);
        let expected_tag = fixed::<16>(&test_case["tag_hex"]);
        assert_eq!(
            xchacha20_poly1305_aead_encrypt(&plaintext, &key, &nonce, &aad),
            (expected_ciphertext.clone(), expected_tag),
            "{}",
            test_case["id"]
        );
        assert_eq!(
            xchacha20_poly1305_aead_decrypt(
                &expected_ciphertext,
                &key,
                &nonce,
                &aad,
                &expected_tag,
            ),
            Some(plaintext),
            "{}",
            test_case["id"]
        );
    }

    let aead_cases = fixture["aead_cases"].as_array().unwrap();
    for mutation in fixture["mutations"].as_array().unwrap() {
        let source_id = mutation["source_case"].as_str().unwrap();
        let source = aead_cases
            .iter()
            .find(|test_case| test_case["id"] == source_id)
            .expect("mutation source case");
        let xor_byte = fixed::<1>(&mutation["xor_hex"])[0];

        for byte_index in mutation["byte_indices"].as_array().unwrap() {
            let byte_index = byte_index.as_u64().unwrap() as usize;
            let mut ciphertext = hex_bytes(&source["ciphertext_hex"]);
            let mut key = fixed::<32>(&source["key_hex"]);
            let mut nonce = fixed::<24>(&source["nonce_hex"]);
            let mut aad = hex_bytes(&source["aad_hex"]);
            let mut tag = fixed::<16>(&source["tag_hex"]);

            match mutation["target"].as_str().unwrap() {
                "ciphertext" => ciphertext[byte_index] ^= xor_byte,
                "key" => key[byte_index] ^= xor_byte,
                "nonce" => nonce[byte_index] ^= xor_byte,
                "aad" => aad[byte_index] ^= xor_byte,
                "tag" => tag[byte_index] ^= xor_byte,
                target => panic!("unknown mutation target {target}"),
            }

            assert_eq!(
                xchacha20_poly1305_aead_decrypt(&ciphertext, &key, &nonce, &aad, &tag),
                None,
                "{} byte {} must return only authentication failure",
                mutation["target"],
                byte_index
            );
        }
    }
}
