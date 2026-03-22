//! Integration tests for the combinational circuits module.
//!
//! These tests exercise MUX, DEMUX, decoder, encoder, priority encoder,
//! and tri-state buffer circuits against their expected truth tables and
//! edge cases.

use logic_gates::combinational::*;

// ===========================================================================
// MUX2 — 2-to-1 Multiplexer
// ===========================================================================

#[test]
fn test_mux2_exhaustive() {
    // For every combination of d0, d1, sel — verify the correct input is selected
    for d0 in 0..=1u8 {
        for d1 in 0..=1u8 {
            assert_eq!(mux2(d0, d1, 0), d0, "mux2({d0}, {d1}, 0) should be {d0}");
            assert_eq!(mux2(d0, d1, 1), d1, "mux2({d0}, {d1}, 1) should be {d1}");
        }
    }
}

// ===========================================================================
// MUX4 — 4-to-1 Multiplexer
// ===========================================================================

#[test]
fn test_mux4_selects_correct_input() {
    let inputs = [1u8, 0, 0, 0];
    assert_eq!(mux4(inputs[0], inputs[1], inputs[2], inputs[3], &[0, 0]), 1);

    let inputs = [0u8, 1, 0, 0];
    assert_eq!(mux4(inputs[0], inputs[1], inputs[2], inputs[3], &[1, 0]), 1);

    let inputs = [0u8, 0, 1, 0];
    assert_eq!(mux4(inputs[0], inputs[1], inputs[2], inputs[3], &[0, 1]), 1);

    let inputs = [0u8, 0, 0, 1];
    assert_eq!(mux4(inputs[0], inputs[1], inputs[2], inputs[3], &[1, 1]), 1);
}

#[test]
fn test_mux4_with_all_same_data() {
    // When all data inputs are the same, output is always that value
    for sel0 in 0..=1u8 {
        for sel1 in 0..=1u8 {
            assert_eq!(mux4(1, 1, 1, 1, &[sel0, sel1]), 1);
            assert_eq!(mux4(0, 0, 0, 0, &[sel0, sel1]), 0);
        }
    }
}

// ===========================================================================
// MUX_N — N-to-1 Multiplexer (recursive)
// ===========================================================================

#[test]
fn test_mux_n_selects_each_of_8_inputs() {
    for target in 0..8usize {
        let mut data = vec![0u8; 8];
        data[target] = 1;
        let sel: Vec<u8> = (0..3)
            .map(|bit| ((target >> bit) & 1) as u8)
            .collect();
        assert_eq!(
            mux_n(&data, &sel),
            1,
            "mux_n should select input {target} with sel={sel:?}"
        );
    }
}

#[test]
fn test_mux_n_selects_each_of_16_inputs() {
    for target in 0..16usize {
        let mut data = vec![0u8; 16];
        data[target] = 1;
        let sel: Vec<u8> = (0..4)
            .map(|bit| ((target >> bit) & 1) as u8)
            .collect();
        assert_eq!(mux_n(&data, &sel), 1);
    }
}

// ===========================================================================
// DEMUX — 1-to-N Demultiplexer
// ===========================================================================

#[test]
fn test_demux_4_outputs_data_1() {
    // Route data=1 to each of 4 outputs
    for target in 0..4usize {
        let sel: Vec<u8> = (0..2)
            .map(|bit| ((target >> bit) & 1) as u8)
            .collect();
        let result = demux(1, &sel, 4);
        for (i, &v) in result.iter().enumerate() {
            if i == target {
                assert_eq!(v, 1, "demux output {i} should be 1 for target {target}");
            } else {
                assert_eq!(v, 0, "demux output {i} should be 0 for target {target}");
            }
        }
    }
}

#[test]
fn test_demux_data_0_all_outputs_zero() {
    for sel0 in 0..=1u8 {
        for sel1 in 0..=1u8 {
            assert_eq!(demux(0, &[sel0, sel1], 4), vec![0, 0, 0, 0]);
        }
    }
}

#[test]
fn test_demux_8_outputs() {
    let result = demux(1, &[1, 0, 1], 8); // index = 1 + 0 + 4 = 5
    assert_eq!(result, vec![0, 0, 0, 0, 0, 1, 0, 0]);
}

// ===========================================================================
// DECODER — Binary to One-Hot
// ===========================================================================

#[test]
fn test_decoder_1bit_all() {
    assert_eq!(decoder(&[0]), vec![1, 0]);
    assert_eq!(decoder(&[1]), vec![0, 1]);
}

#[test]
fn test_decoder_2bit_all() {
    assert_eq!(decoder(&[0, 0]), vec![1, 0, 0, 0]);
    assert_eq!(decoder(&[1, 0]), vec![0, 1, 0, 0]);
    assert_eq!(decoder(&[0, 1]), vec![0, 0, 1, 0]);
    assert_eq!(decoder(&[1, 1]), vec![0, 0, 0, 1]);
}

#[test]
fn test_decoder_3bit_boundary() {
    // Input 000 -> output[0] = 1
    assert_eq!(decoder(&[0, 0, 0]), vec![1, 0, 0, 0, 0, 0, 0, 0]);
    // Input 111 -> output[7] = 1
    assert_eq!(decoder(&[1, 1, 1]), vec![0, 0, 0, 0, 0, 0, 0, 1]);
    // Input 101 = index 5 -> output[5] = 1
    assert_eq!(decoder(&[1, 0, 1]), vec![0, 0, 0, 0, 0, 1, 0, 0]);
}

#[test]
fn test_decoder_always_one_hot() {
    // For any 3-bit input, exactly one output should be 1
    for i in 0..8u8 {
        let input = vec![(i >> 0) & 1, (i >> 1) & 1, (i >> 2) & 1];
        let output = decoder(&input);
        assert_eq!(output.iter().sum::<u8>(), 1, "decoder output must be one-hot");
        assert_eq!(output.len(), 8);
    }
}

// ===========================================================================
// ENCODER — One-Hot to Binary
// ===========================================================================

#[test]
fn test_encoder_4_all_positions() {
    assert_eq!(encoder(&[1, 0, 0, 0]), vec![0, 0]); // index 0
    assert_eq!(encoder(&[0, 1, 0, 0]), vec![1, 0]); // index 1
    assert_eq!(encoder(&[0, 0, 1, 0]), vec![0, 1]); // index 2
    assert_eq!(encoder(&[0, 0, 0, 1]), vec![1, 1]); // index 3
}

#[test]
fn test_encoder_8_input() {
    assert_eq!(encoder(&[0, 0, 0, 0, 0, 1, 0, 0]), vec![1, 0, 1]); // index 5
    assert_eq!(encoder(&[0, 0, 0, 0, 0, 0, 0, 1]), vec![1, 1, 1]); // index 7
}

#[test]
fn test_encoder_decoder_roundtrip() {
    // Encoding the output of a decoder should give back the original input
    for i in 0..4usize {
        let input = vec![((i >> 0) & 1) as u8, ((i >> 1) & 1) as u8];
        let decoded = decoder(&input);
        let encoded = encoder(&decoded);
        assert_eq!(encoded, input, "roundtrip failed for index {i}");
    }
}

// ===========================================================================
// PRIORITY ENCODER
// ===========================================================================

#[test]
fn test_priority_encoder_single_inputs() {
    for i in 0..4usize {
        let mut inputs = vec![0u8; 4];
        inputs[i] = 1;
        let (bits, valid) = priority_encoder(&inputs);
        let expected: Vec<u8> = (0..2).map(|b| ((i >> b) & 1) as u8).collect();
        assert_eq!(bits, expected);
        assert_eq!(valid, 1);
    }
}

#[test]
fn test_priority_encoder_multiple_highest_wins() {
    // I1 and I3 active — I3 wins
    let (bits, valid) = priority_encoder(&[0, 1, 0, 1]);
    assert_eq!(bits, vec![1, 1]); // index 3
    assert_eq!(valid, 1);

    // I0, I1, I2 active — I2 wins
    let (bits, valid) = priority_encoder(&[1, 1, 1, 0]);
    assert_eq!(bits, vec![0, 1]); // index 2
    assert_eq!(valid, 1);
}

#[test]
fn test_priority_encoder_no_active() {
    let (bits, valid) = priority_encoder(&[0, 0, 0, 0]);
    assert_eq!(bits, vec![0, 0]);
    assert_eq!(valid, 0);
}

#[test]
fn test_priority_encoder_8_inputs() {
    // I5 is highest active
    let (bits, valid) = priority_encoder(&[1, 0, 1, 0, 0, 1, 0, 0]);
    assert_eq!(bits, vec![1, 0, 1]); // index 5
    assert_eq!(valid, 1);
}

// ===========================================================================
// TRI-STATE BUFFER
// ===========================================================================

#[test]
fn test_tri_state_exhaustive() {
    assert_eq!(tri_state(0, 0), None);
    assert_eq!(tri_state(1, 0), None);
    assert_eq!(tri_state(0, 1), Some(0));
    assert_eq!(tri_state(1, 1), Some(1));
}
