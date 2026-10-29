use tls13_encrypted_extensions_message::decode_encrypted_extensions_message;
use tls13_extension_block::{ExtensionBlockError, ExtensionBlockLimits, ExtensionType};

fn limits(bytes: usize, count: usize) -> ExtensionBlockLimits {
    ExtensionBlockLimits::new(bytes, count).expect("valid test limits")
}

#[test]
fn decodes_the_protocol_defined_empty_body() {
    let encoded = [0_u8, 0];
    let message = decode_encrypted_extensions_message(&encoded, limits(64, 8))
        .expect("empty extensions are valid");

    assert_eq!(message.body(), &encoded);
    assert!(core::ptr::eq(message.body().as_ptr(), encoded.as_ptr()));
    assert!(message.extensions().is_empty());
    assert_eq!(message.extensions().len(), 0);
    assert_eq!(message.extensions().payload_length(), 0);
}

#[test]
fn retains_exact_body_and_opaque_extensions_in_wire_order() {
    let encoded = [
        0, 11, // extension-vector payload length
        0, 16, 0, 2, 0xaa, 0xbb, // ALPN type with still-opaque data
        0xff, 0x01, 0, 1, 0xcc, // unknown private-use type
    ];
    let message = decode_encrypted_extensions_message(&encoded, limits(64, 8))
        .expect("valid EncryptedExtensions body");
    let entries = message.extensions().entries();

    assert_eq!(message.body(), &encoded);
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].extension_type().wire_value(), 16);
    assert_eq!(entries[0].data(), &[0xaa, 0xbb]);
    assert_eq!(entries[1].extension_type().wire_value(), 0xff01);
    assert_eq!(entries[1].data(), &[0xcc]);
    assert_eq!(
        message
            .extensions()
            .find(ExtensionType::from_wire_value(16))
            .expect("ALPN entry")
            .data(),
        &[0xaa, 0xbb]
    );
}

#[test]
fn preserves_structural_and_resource_failure_categories() {
    assert_eq!(
        decode_encrypted_extensions_message(&[], limits(64, 8)).unwrap_err(),
        ExtensionBlockError::MissingBlockLength
    );
    assert_eq!(
        decode_encrypted_extensions_message(&[0, 1], limits(64, 8)).unwrap_err(),
        ExtensionBlockError::BlockLengthMismatch
    );
    assert_eq!(
        decode_encrypted_extensions_message(&[0, 4, 0, 16, 0, 1], limits(64, 8)).unwrap_err(),
        ExtensionBlockError::TruncatedExtensionData
    );
    assert_eq!(
        decode_encrypted_extensions_message(&[0, 8, 0, 16, 0, 0, 0, 16, 0, 0], limits(64, 8))
            .unwrap_err(),
        ExtensionBlockError::DuplicateExtension
    );
    assert_eq!(
        decode_encrypted_extensions_message(&[0, 4, 0, 16, 0, 0], limits(3, 8)).unwrap_err(),
        ExtensionBlockError::BlockTooLong
    );
    assert_eq!(
        decode_encrypted_extensions_message(&[0, 8, 0, 16, 0, 0, 0, 43, 0, 0], limits(64, 1))
            .unwrap_err(),
        ExtensionBlockError::TooManyExtensions
    );
}

#[test]
fn debug_output_redacts_body_and_extension_data() {
    let encoded = [0, 5, 0, 16, 0, 1, 0xaa];
    let message = decode_encrypted_extensions_message(&encoded, limits(64, 8))
        .expect("valid EncryptedExtensions body");
    let debug = format!("{message:?}");

    assert!(debug.contains("extension_count: 1"));
    assert!(debug.contains("extension_payload_length: 5"));
    assert!(debug.contains("[OPAQUE]"));
    assert!(!debug.contains("170"));
    assert!(!debug.contains("0xaa"));
}
