//! NN31 fixture proof for the Rust execution core.
//!
//! `matrix-cpu` owns the Node-free graph execution helper, and the N-API crate
//! re-exports it at its binding edge. Testing the helper here avoids linking
//! Node host symbols while exercising the same implementation.

const GRAPH_JSON: &str = include_str!(
    "../../../../specs/fixtures/backend-parity-v1/matrix-ir/00-dense-batch.graph.json"
);
const INPUT_HEX: &str =
    include_str!("../../../../specs/fixtures/backend-parity-v1/payloads/00-input-x.f32le.hex");
const EXPECTED_HEX: &str = include_str!(
    "../../../../specs/fixtures/backend-parity-v1/payloads/00-expected-output.f32le.hex"
);

fn decode_hex(value: &str) -> Vec<u8> {
    let value = value.trim();
    assert!(!value.is_empty());
    assert_eq!(value.len() % 2, 0);
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let digits = std::str::from_utf8(pair).expect("fixture hex is UTF-8");
            u8::from_str_radix(digits, 16).expect("fixture hex is valid")
        })
        .collect()
}

fn decode_f32s(bytes: &[u8]) -> Vec<f32> {
    assert_eq!(bytes.len() % 4, 0);
    bytes
        .chunks_exact(4)
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect()
}

#[test]
fn dense_batch_executes_through_the_rust_core() {
    let graph = matrix_ir_json::decode(GRAPH_JSON).expect("NN31 MatrixIR fixture decodes");
    let input = decode_hex(INPUT_HEX);
    let expected = decode_hex(EXPECTED_HEX);
    let outputs =
        matrix_cpu::run_graph_on_cpu(&graph, &[input]).expect("Rust CPU execution succeeds");

    assert_eq!(outputs.len(), 1);
    assert_eq!(outputs[0], expected);
    assert_eq!(decode_f32s(&outputs[0]), vec![3.0, 5.0, 7.0]);
}
