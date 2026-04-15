use resp_protocol::{decode, decode_all, RespValue};

fn bulk(value: &str) -> RespValue {
    RespValue::BulkString(Some(value.as_bytes().to_vec()))
}

#[test]
fn empty_inline_queries_decode_to_empty_arrays() {
    assert_eq!(decode(b"\r\n").unwrap(), Some((RespValue::Array(Some(vec![])), 2)));
}

#[test]
fn inline_commands_decode_into_bulk_string_arguments() {
    assert_eq!(
        decode(b"SET x foo\r\n").unwrap(),
        Some((RespValue::Array(Some(vec![bulk("SET"), bulk("x"), bulk("foo")])), 11))
    );
}

#[test]
fn arrays_of_bulk_strings_decode_correctly() {
    assert_eq!(
        decode(b"*3\r\n$3\r\nSET\r\n$1\r\nx\r\n$3\r\nfoo\r\n").unwrap(),
        Some((RespValue::Array(Some(vec![bulk("SET"), bulk("x"), bulk("foo")])), 29))
    );
}

#[test]
fn malformed_resp_lengths_are_reported_as_errors() {
    assert!(decode(b"*-10\r\n").is_err());
    assert!(decode(b"*9223372036854775808\r\n").is_err());
    assert!(decode(b"*3\r\n$3\r\nSET\r\n$1\r\nx\r\n$-10\r\n").is_err());
    assert!(decode(b"*3\r\n$3\r\nSET\r\n$1\r\nx\r\n$9223372036854775808\r\n").is_err());
}

#[test]
fn decode_all_collects_multiple_messages() {
    let (messages, consumed) = decode_all(b"+OK\r\n:1\r\n").unwrap();
    assert_eq!(consumed, 9);
    assert_eq!(
        messages,
        vec![RespValue::SimpleString("OK".to_string()), RespValue::Integer(1)]
    );
}
