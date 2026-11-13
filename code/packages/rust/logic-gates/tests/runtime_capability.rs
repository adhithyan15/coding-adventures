//! VER08b runtime capability conformance for `logic-gates`.
//!
//! `required_capabilities.json` declares `"capabilities": []` (pure
//! computation).  This test proves it at run time: EVERY public stateless
//! operation -- the eleven scalar gates, the three n-input gates, mux2, mux4,
//! mux_n, demux, decoder, encoder, priority_encoder and tri_state -- is exercised
//! inside a process that has been sandboxed, mechanically from that manifest,
//! to the minimal armed baseline (allocator + exit + TAP output).  An
//! independent `strace` observer must agree, negative controls must be caught,
//! and (when Docker works) the same policy is run in a container with no
//! network, a read-only root and no capabilities.
//!
//! `sequential.rs` is the next slice (LG01) and is not exercised here: the gate
//! covers what the Warrant file covers, no more.
//!
//! Phase A (`prepare`, unconstrained) builds every input vector.  Phase B (the
//! returned closure, armed) only calls the library and returns named checks: no
//! file, socket, clock, environment or process access is possible there, and
//! none is allowed to appear.  Phase B calls only VALID inputs: a rejected input
//! panics, and a panic prints to stderr, which the baseline does not allow; the
//! rejections are pinned unjailed, by `tests/bit_domain.rs` and the Warrant
//! harness.
//!
//! On non-Linux hosts the gate reports `NOT-LIVE` (RCG010); it never reports a
//! pass it did not earn.  Set `RCG_STRICT=1` to turn not-live into failure.

use capability_runtime_conformance::{
    run_package_gate, Check, Exercise, GateConfig, PolicyOptions, Rings,
};
use logic_gates::{
    and_gate, and_n, decoder, demux, encoder, mux2, mux4, mux_n, nand_and, nand_gate, nand_not,
    nand_or, nand_xor, nor_gate, not_gate, or_gate, or_n, priority_encoder, tri_state, xnor_gate,
    xor_gate, xor_n, MAX_DECODER_WIDTH,
};
use std::path::PathBuf;

fn check(name: &str, ok: bool) -> Check {
    Check {
        name: name.to_string(),
        ok,
    }
}

/// The bits of `value`, least significant first, `width` of them: the slice
/// convention of every operation in the crate.
fn bits(value: usize, width: usize) -> Vec<u8> {
    (0..width).map(|i| ((value >> i) & 1) as u8).collect()
}

/// Phase A: build every input vector.  Nothing here is needed by phase B except
/// through the captured values.
fn prepare() -> Exercise {
    // Every bit vector of width 2..=6 for the n-input gates (124 vectors), and
    // every one-hot and every ordinary word for the encoders and multiplexers.
    let mut vectors: Vec<Vec<u8>> = Vec::new();
    for width in 2..=6usize {
        for value in 0..(1usize << width) {
            vectors.push(bits(value, width));
        }
    }
    // Decoder inputs of width 1..=6 (every value), plus the widest the crate allows.
    let mut decoder_inputs: Vec<(usize, Vec<u8>)> = Vec::new();
    for width in 1..=6usize {
        for value in 0..(1usize << width) {
            decoder_inputs.push((value, bits(value, width)));
        }
    }
    let widest = vec![1u8; MAX_DECODER_WIDTH];
    // One-hot inputs of width 2, 4, 8, 16, 32 (every position).
    let mut one_hot: Vec<(usize, Vec<u8>)> = Vec::new();
    for width in [2usize, 4, 8, 16, 32] {
        for position in 0..width {
            let mut v = vec![0u8; width];
            v[position] = 1;
            one_hot.push((position, v));
        }
    }
    // Priority-encoder words of width 4 and 8 (every value).
    let mut priority: Vec<(usize, Vec<u8>)> = Vec::new();
    for width in [4usize, 8] {
        for value in 0..(1usize << width) {
            priority.push((value, bits(value, width)));
        }
    }
    // Multiplexer cases: data word and select index for 2, 4, 8 and 16 inputs.
    let mut mux_cases: Vec<(Vec<u8>, usize, Vec<u8>)> = Vec::new();
    for select_width in 1..=4usize {
        let inputs = 1usize << select_width;
        for data in [0usize, 1, 0x5555, 0xAAAA, 0xFFFF, 0x1234, 0x8001] {
            let data = data & ((1usize << inputs) - 1);
            for index in 0..inputs {
                mux_cases.push((bits(data, inputs), index, bits(index, select_width)));
            }
        }
    }
    // Demux cases: select width 1..=4, every select value, data 0 and 1.
    let mut demux_cases: Vec<(u8, usize, Vec<u8>)> = Vec::new();
    for select_width in 1..=4usize {
        for index in 0..(1usize << select_width) {
            for data in [0u8, 1] {
                demux_cases.push((data, index, bits(index, select_width)));
            }
        }
    }

    Box::new(move || {
        let mut checks = Vec::new();

        // The seven primitives and the four NAND-built gates, against Rust's own
        // Boolean operators (the oracle is the host language, as in LG01).
        let mut gates_ok = true;
        for a in 0..=1u8 {
            gates_ok &= not_gate(a) == 1 - a;
            gates_ok &= nand_not(a) == 1 - a;
            for b in 0..=1u8 {
                gates_ok &= and_gate(a, b) == (a & b);
                gates_ok &= or_gate(a, b) == (a | b);
                gates_ok &= xor_gate(a, b) == (a ^ b);
                gates_ok &= nand_gate(a, b) == 1 - (a & b);
                gates_ok &= nor_gate(a, b) == 1 - (a | b);
                gates_ok &= xnor_gate(a, b) == 1 - (a ^ b);
                gates_ok &= nand_and(a, b) == (a & b);
                gates_ok &= nand_or(a, b) == (a | b);
                gates_ok &= nand_xor(a, b) == (a ^ b);
            }
        }
        checks.push(check("gates: every truth table (7 primitives, 4 NAND-built)", gates_ok));

        let nary_ok = vectors.iter().all(|v| {
            let ones = v.iter().filter(|&&b| b == 1).count();
            and_n(v) == u8::from(ones == v.len())
                && or_n(v) == u8::from(ones > 0)
                && xor_n(v) == (ones % 2) as u8
        });
        checks.push(check("and_n / or_n / xor_n: every vector of width 2..=6", nary_ok));

        let mux2_ok = (0..8usize).all(|c| {
            let (d0, d1, s) = ((c & 1) as u8, ((c >> 1) & 1) as u8, ((c >> 2) & 1) as u8);
            mux2(d0, d1, s) == if s == 1 { d1 } else { d0 }
        });
        checks.push(check("mux2: every input", mux2_ok));

        let mux4_ok = (0..64usize).all(|c| {
            let d = [(c & 1) as u8, ((c >> 1) & 1) as u8, ((c >> 2) & 1) as u8, ((c >> 3) & 1) as u8];
            let sel = [((c >> 4) & 1) as u8, ((c >> 5) & 1) as u8];
            mux4(d[0], d[1], d[2], d[3], &sel) == d[(c >> 4) & 3]
        });
        checks.push(check("mux4: every input", mux4_ok));

        let mux_n_ok = mux_cases
            .iter()
            .all(|(data, index, sel)| mux_n(data, sel) == data[*index]);
        checks.push(check("mux_n: 2, 4, 8 and 16 inputs, every select", mux_n_ok));

        let decoder_ok = decoder_inputs.iter().all(|(value, input)| {
            let out = decoder(input);
            out.len() == 1usize << input.len()
                && out.iter().enumerate().all(|(i, &line)| line == u8::from(i == *value))
        });
        checks.push(check("decoder: every input of width 1..=6 is one-hot at its value", decoder_ok));
        let wide = decoder(&widest);
        checks.push(check(
            "decoder: the widest allowed decoder (16 inputs) has 65,536 lines",
            wide.len() == 65_536 && wide[65_535] == 1 && wide[..65_535].iter().all(|&b| b == 0),
        ));

        let demux_ok = demux_cases.iter().all(|(data, index, sel)| {
            let out = demux(*data, sel, 1usize << sel.len());
            out.iter().enumerate().all(|(i, &line)| line == if i == *index { *data } else { 0 })
        });
        checks.push(check("demux: data reaches only the selected line", demux_ok));

        let encoder_ok = one_hot.iter().all(|(position, input)| {
            let out = encoder(input);
            let width = input.len().trailing_zeros() as usize;
            out == bits(*position, width)
        });
        checks.push(check("encoder: every one-hot input encodes its position", encoder_ok));

        let priority_ok = priority.iter().all(|(value, input)| {
            let (index, valid) = priority_encoder(input);
            let expected = if *value == 0 { 0 } else { usize::BITS as usize - 1 - value.leading_zeros() as usize };
            let width = input.len().trailing_zeros() as usize;
            valid == u8::from(*value != 0) && index == bits(expected, width)
        });
        checks.push(check("priority_encoder: highest active input wins, valid flag", priority_ok));

        checks.push(check(
            "tri_state: enabled passes the data, disabled floats",
            tri_state(0, 1) == Some(0)
                && tri_state(1, 1) == Some(1)
                && tri_state(0, 0).is_none()
                && tri_state(1, 0).is_none(),
        ));

        checks
    })
}

fn main() {
    let evidence_dir = option_env!("CARGO_TARGET_TMPDIR")
        .map(PathBuf::from)
        .map(|dir| dir.join("capability-runtime"));
    let config = GateConfig {
        package: "rust/logic-gates",
        manifest_json: include_str!("../required_capabilities.json"),
        evidence_dir,
        prepare,
        options: PolicyOptions::default(),
        rings: Rings::ALL,
    };
    std::process::exit(run_package_gate(&config));
}
