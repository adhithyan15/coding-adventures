//! Integration tests for the Slice module.

use fpga::slice::Slice;

#[test]
fn test_slice_combinational_and_xor() {
    let mut s = Slice::new(4);
    let mut and_tt = vec![0u8; 16];
    and_tt[3] = 1;
    let mut xor_tt = vec![0u8; 16];
    xor_tt[1] = 1;
    xor_tt[2] = 1;
    s.configure(&and_tt, &xor_tt, false, false, false);

    let out = s.evaluate(&[1, 1, 0, 0], &[1, 0, 0, 0], 0, 0);
    assert_eq!(out.output_a, 1); // AND(1,1)
    assert_eq!(out.output_b, 1); // XOR(1,0)
    assert_eq!(out.carry_out, 0);
}

#[test]
fn test_slice_carry_chain_generate() {
    let mut s = Slice::new(4);
    let mut and_tt = vec![0u8; 16];
    and_tt[3] = 1;
    let mut xor_tt = vec![0u8; 16];
    xor_tt[1] = 1;
    xor_tt[2] = 1;
    s.configure(&and_tt, &xor_tt, false, false, true);

    // A=1, B=1: generate=AND(1,1)=1, propagate=XOR(1,1)=0
    // carry_out = (1 AND 0) OR (0 AND (1 XOR 0)) = 0 OR 0 = 0
    // Wait, LUT A gets inputs [1,1,0,0], LUT B gets inputs [1,1,0,0]
    // LUT A out = AND(1,1) = 1, LUT B out = XOR(1,1) = 0
    // carry_out = OR(AND(1,0), AND(carry_in, XOR(1,0))) = OR(0, AND(0,1)) = 0
    let out = s.evaluate(&[1, 1, 0, 0], &[1, 1, 0, 0], 0, 0);
    assert_eq!(out.carry_out, 0);

    // With carry_in = 1 and propagate = 1 (XOR(1,0) = 1)
    // LUT A = AND(1,0)=0, LUT B = XOR(1,0)=1
    // carry_out = OR(AND(0,1), AND(1, XOR(0,1))) = OR(0, AND(1,1)) = 1
    let out = s.evaluate(&[1, 0, 0, 0], &[1, 0, 0, 0], 0, 1);
    assert_eq!(out.carry_out, 1);
}

#[test]
fn test_slice_carry_disabled() {
    let mut s = Slice::new(4);
    let mut and_tt = vec![0u8; 16];
    and_tt[3] = 1;
    s.configure(&and_tt, &and_tt, false, false, false);

    let out = s.evaluate(&[1, 1, 0, 0], &[1, 1, 0, 0], 0, 1);
    assert_eq!(out.carry_out, 0); // disabled
}

#[test]
fn test_slice_properties() {
    let s = Slice::new(4);
    assert_eq!(s.k(), 4);
}
