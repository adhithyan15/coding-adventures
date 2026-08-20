use serde_json::Value;
use std::path::PathBuf;
use zip::{
    crc32, raw_deflate, raw_inflate, raw_inflate_counted, ZipReader, ZipWriter,
    RAW_INFLATE_MAX_OUTPUT,
};

fn fixture() -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/fixtures/zip-raw-rfc1951-v1/cases.json");
    serde_json::from_str(&std::fs::read_to_string(path).expect("read neutral fixture"))
        .expect("parse neutral fixture")
}

fn hex_bytes(text: &str) -> Vec<u8> {
    assert_eq!(text.len() % 2, 0, "hex input must contain whole bytes");
    text.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| {
            let pair = std::str::from_utf8(pair).expect("ASCII hex");
            u8::from_str_radix(pair, 16).expect("valid fixture hex")
        })
        .collect()
}

fn expected_output(value: &Value) -> Vec<u8> {
    if let Some(hex) = value.get("hex").and_then(Value::as_str) {
        return hex_bytes(hex);
    }
    let repeated = hex_bytes(
        value
            .get("repeat_hex")
            .and_then(Value::as_str)
            .expect("repeat_hex"),
    );
    let count = value
        .get("count")
        .and_then(Value::as_u64)
        .expect("repeat count") as usize;
    repeated.repeat(count)
}

#[test]
fn consumes_every_closed_raw_rfc1951_case() {
    let fixture = fixture();
    let cases = fixture["cases"].as_array().expect("fixture cases");
    assert_eq!(cases.len(), 34, "closed profile case count changed");

    for case in cases {
        let id = case["id"].as_str().expect("case id");
        match case["operation"].as_str().expect("operation") {
            "inflate" => {
                let input = hex_bytes(case["input_hex"].as_str().expect("input_hex"));
                let limit = case
                    .get("max_output")
                    .and_then(Value::as_i64)
                    .unwrap_or(RAW_INFLATE_MAX_OUTPUT);
                let result = raw_inflate_counted(&input, limit)
                    .unwrap_or_else(|error| panic!("{id}: unexpected {}", error.code.as_str()));
                assert_eq!(
                    result.output,
                    expected_output(&case["expected"]["output"]),
                    "{id}: output"
                );
                assert_eq!(
                    raw_inflate(&input, limit).expect("uncounted raw wrapper"),
                    result.output,
                    "{id}: uncounted wrapper"
                );
                assert_eq!(
                    result.bytes_consumed,
                    case["expected"]["bytes_consumed"].as_u64().unwrap() as usize,
                    "{id}: bytes consumed"
                );
            }
            "inflate-error" => {
                let input = hex_bytes(case["input_hex"].as_str().expect("input_hex"));
                let limit = case
                    .get("max_output")
                    .and_then(Value::as_i64)
                    .unwrap_or(RAW_INFLATE_MAX_OUTPUT);
                let error = raw_inflate_counted(&input, limit).unwrap_err();
                assert_eq!(
                    error.code.as_str(),
                    case["expected"]["error_id"].as_str().unwrap(),
                    "{id}: stable error"
                );
                assert_eq!(
                    error.to_string(),
                    error.code.as_str(),
                    "{id}: payload-blind display"
                );
            }
            "deflate-interoperability" => {
                let input = hex_bytes(case["input_hex"].as_str().expect("input_hex"));
                let encoded = raw_deflate(&input);
                let independently_decoded = deflate::inflate(&encoded)
                    .unwrap_or_else(|error| panic!("{id}: independent decode: {error}"));
                assert_eq!(
                    independently_decoded,
                    expected_output(&case["expected"]["output"]),
                    "{id}: independently decoded encoder output"
                );
            }
            "crc32" => {
                let mut checksum = 0;
                for chunk in case["chunks_hex"].as_array().expect("CRC chunks") {
                    checksum = crc32(&hex_bytes(chunk.as_str().unwrap()), checksum);
                }
                assert_eq!(
                    format!("{checksum:08x}"),
                    case["expected"]["crc32_hex"].as_str().unwrap(),
                    "{id}: incremental CRC-32"
                );
            }
            operation => panic!("{id}: unknown fixture operation {operation}"),
        }
    }
}

fn read_u32(data: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap())
}

fn write_u32(data: &mut [u8], offset: usize, value: u32) {
    data[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

#[test]
fn zip_reader_rejects_a_suffix_inside_the_declared_compressed_payload() {
    let mut writer = ZipWriter::new();
    writer.add_file("payload.bin", &vec![b'A'; 1024], true);
    let mut archive = writer.finish();

    let old_eocd = archive.len() - 22;
    let old_cd = read_u32(&archive, old_eocd + 16) as usize;
    let old_compressed = read_u32(&archive, 18);
    archive.insert(old_cd, 0x00);

    let new_cd = old_cd + 1;
    let new_eocd = old_eocd + 1;
    write_u32(&mut archive, 18, old_compressed + 1);
    write_u32(&mut archive, new_cd + 20, old_compressed + 1);
    write_u32(&mut archive, new_eocd + 16, new_cd as u32);

    let reader = ZipReader::new(&archive).expect("structurally valid archive");
    let error = reader.read(&reader.entries()[0]).unwrap_err();
    assert_eq!(error, "zip: compressed payload contains trailing bytes");
}
