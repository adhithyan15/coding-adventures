//! Integration tests for the LUT module.

use fpga::lut::LUT;

#[test]
fn test_lut_default_output_zero() {
    let lut = LUT::new(4, None);
    assert_eq!(lut.evaluate(&[0, 0, 0, 0]), 0);
    assert_eq!(lut.evaluate(&[1, 1, 1, 1]), 0);
}

#[test]
fn test_lut_and_gate() {
    let mut tt = vec![0u8; 16];
    tt[3] = 1; // I0=1, I1=1
    let lut = LUT::new(4, Some(&tt));

    assert_eq!(lut.evaluate(&[0, 0, 0, 0]), 0);
    assert_eq!(lut.evaluate(&[1, 0, 0, 0]), 0);
    assert_eq!(lut.evaluate(&[0, 1, 0, 0]), 0);
    assert_eq!(lut.evaluate(&[1, 1, 0, 0]), 1);
}

#[test]
fn test_lut_xor_gate() {
    let mut tt = vec![0u8; 16];
    tt[1] = 1; // I0=1, I1=0
    tt[2] = 1; // I0=0, I1=1
    let lut = LUT::new(4, Some(&tt));

    assert_eq!(lut.evaluate(&[0, 0, 0, 0]), 0);
    assert_eq!(lut.evaluate(&[1, 0, 0, 0]), 1);
    assert_eq!(lut.evaluate(&[0, 1, 0, 0]), 1);
    assert_eq!(lut.evaluate(&[1, 1, 0, 0]), 0);
}

#[test]
fn test_lut_or_gate() {
    let mut tt = vec![0u8; 16];
    tt[1] = 1; // I0=1, I1=0
    tt[2] = 1; // I0=0, I1=1
    tt[3] = 1; // I0=1, I1=1
    let lut = LUT::new(4, Some(&tt));

    assert_eq!(lut.evaluate(&[0, 0, 0, 0]), 0);
    assert_eq!(lut.evaluate(&[1, 0, 0, 0]), 1);
    assert_eq!(lut.evaluate(&[0, 1, 0, 0]), 1);
    assert_eq!(lut.evaluate(&[1, 1, 0, 0]), 1);
}

#[test]
fn test_lut_reconfigure() {
    let mut lut = LUT::new(4, None);
    assert_eq!(lut.evaluate(&[1, 1, 0, 0]), 0);

    let mut tt = vec![0u8; 16];
    tt[3] = 1;
    lut.configure(&tt);
    assert_eq!(lut.evaluate(&[1, 1, 0, 0]), 1);
}

#[test]
fn test_lut_truth_table_getter() {
    let tt = vec![0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    let lut = LUT::new(4, Some(&tt));
    assert_eq!(lut.truth_table(), tt);
}

#[test]
fn test_lut_2_input() {
    let mut tt = vec![0u8; 4];
    tt[3] = 1; // AND(I0, I1)
    let lut = LUT::new(2, Some(&tt));
    assert_eq!(lut.evaluate(&[0, 0]), 0);
    assert_eq!(lut.evaluate(&[1, 1]), 1);
    assert_eq!(lut.k(), 2);
}

#[test]
fn test_lut_6_input() {
    let mut tt = vec![0u8; 64];
    tt[63] = 1; // all ones
    let lut = LUT::new(6, Some(&tt));
    assert_eq!(lut.evaluate(&[1, 1, 1, 1, 1, 1]), 1);
    assert_eq!(lut.evaluate(&[0, 1, 1, 1, 1, 1]), 0);
}
