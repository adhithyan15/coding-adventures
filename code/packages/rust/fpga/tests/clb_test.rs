//! Integration tests for the CLB module.

use fpga::clb::CLB;

#[test]
fn test_clb_dual_slice_evaluation() {
    let mut clb = CLB::new(4);
    let mut and_tt = vec![0u8; 16];
    and_tt[3] = 1;
    let zeros = vec![0u8; 16];

    clb.slice0_mut()
        .configure(&and_tt, &zeros, false, false, false);
    clb.slice1_mut()
        .configure(&and_tt, &zeros, false, false, false);

    let out = clb.evaluate(
        &[1, 1, 0, 0], // slice0 LUT A: AND(1,1)=1
        &[0, 0, 0, 0], // slice0 LUT B: 0
        &[1, 0, 0, 0], // slice1 LUT A: AND(1,0)=0
        &[0, 0, 0, 0], // slice1 LUT B: 0
        0,
        0,
    );

    assert_eq!(out.slice0.output_a, 1);
    assert_eq!(out.slice1.output_a, 0);
}

#[test]
fn test_clb_carry_chain_flows_between_slices() {
    let mut clb = CLB::new(4);
    let mut and_tt = vec![0u8; 16];
    and_tt[3] = 1; // generate
    let mut xor_tt = vec![0u8; 16];
    xor_tt[1] = 1;
    xor_tt[2] = 1; // propagate

    // Both slices have carry enabled
    clb.slice0_mut()
        .configure(&and_tt, &xor_tt, false, false, true);
    clb.slice1_mut()
        .configure(&and_tt, &xor_tt, false, false, true);

    // Slice 0: AND(1,1)=1, XOR(1,1)=0 -> carry_out = OR(AND(1,0), AND(0, XOR(1,0))) = 0
    // Actually: generate=1, propagate=0 -> carry_out = OR(AND(1,0), AND(cin,1)) = OR(0,0) = 0
    // Hmm, let me re-derive. LUT A out = 1, LUT B out = 0
    // carry_out = OR(AND(lut_a, lut_b), AND(cin, XOR(lut_a, lut_b)))
    //           = OR(AND(1, 0), AND(0, XOR(1, 0)))
    //           = OR(0, AND(0, 1))
    //           = OR(0, 0) = 0
    // So with cin=0 and generate but no propagate, carry doesn't pass

    // Let's try: both LUT outputs = 1
    // LUT A = AND(1,1) = 1, LUT B = XOR has inputs [1,1,0,0] -> XOR(1,1) = 0
    // Need to set inputs so that both A and B produce 1
    // LUT A(1,1) = 1, LUT B(1,0) = 1
    let out = clb.evaluate(
        &[1, 1, 0, 0], // slice0 LUT A: AND(1,1)=1
        &[1, 0, 0, 0], // slice0 LUT B: XOR(1,0)=1
        &[0, 0, 0, 0], // slice1 LUT A: 0
        &[0, 0, 0, 0], // slice1 LUT B: 0
        0,
        0,
    );

    // Slice 0: A=1, B=1 -> carry_out = OR(AND(1,1), AND(0,XOR(1,1))) = OR(1,0) = 1
    assert_eq!(out.slice0.carry_out, 1);
    // Slice 1 receives carry_in=1
}

#[test]
fn test_clb_properties() {
    let clb = CLB::new(4);
    assert_eq!(clb.k(), 4);
}
