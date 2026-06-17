//! Integration tests for the **MXF-3 `f64` path** of `array-runtime`, exercised
//! through the crate's *public* API only (no access to private helpers).
//!
//! The invariant under test: for `f64` inputs, the planned/executor path
//! ([`execute`] / [`execute_sum`]) returns the **bit-exact** `f64` result, equal
//! to the reference CPU path in [`ops`] to full double precision. We deliberately
//! pick values that are **not** representable in `f32` (e.g. `1 + 2^-40`), so the
//! old `f64 → f32 → f64` round-trip would have lost them — passing these proves
//! the round-trip is gone.

use coding_adventures_array_runtime::{execute, execute_sum, ops, Array, BinOp, DType, Kernel};

/// `1 + 2^-40` is a distinct `f64` but rounds to exactly `1.0` as an `f32`.
fn f32_unrepresentable() -> f64 {
    let v = 1.0 + 2f64.powi(-40);
    assert_eq!(
        v as f32 as f64, 1.0,
        "value must collapse under an f32 round-trip"
    );
    assert_ne!(v, 1.0, "value must be distinct from 1.0 in f64");
    v
}

/// Assert two arrays are equal bit-for-bit (not within a tolerance).
fn assert_bit_exact(executed: &Array, reference: &Array) {
    assert_eq!(executed.shape(), reference.shape(), "shape mismatch");
    for (e, r) in executed.data().iter().zip(reference.data()) {
        assert_eq!(e.to_bits(), r.to_bits(), "not bit-exact: {e} vs {r}");
    }
}

#[test]
fn f64_elementwise_execute_equals_reference_bit_exactly() {
    let x = f32_unrepresentable();
    let a = Array::from_vec(vec![x, x, x, x]);
    let one = Array::from_vec(vec![1.0, 1.0, 1.0, 1.0]);
    for op in [BinOp::Add, BinOp::Sub, BinOp::Mul] {
        let executed = execute(Kernel::Elementwise(op), &a, &one).unwrap();
        let reference = ops::elementwise(op, &a, &one).unwrap();
        assert_bit_exact(&executed, &reference);
    }
    // The subtraction's exact f64 result is 2^-40 — a value the f32 path would
    // have turned into 0.0. Confirms we are genuinely past f32 precision.
    let sub = execute(Kernel::Elementwise(BinOp::Sub), &a, &one).unwrap();
    assert_eq!(sub.data()[0].to_bits(), 2f64.powi(-40).to_bits());
}

#[test]
fn f64_matmul_execute_equals_reference_bit_exactly() {
    // [1, 2^-20] · [1, 2^-20]^T = 1 + 2^-40 (exact in f64, lost in f32).
    let tiny = 2f64.powi(-20);
    let a = Array::from_rows(vec![vec![1.0, tiny]]).unwrap();
    let b = Array::from_rows(vec![vec![1.0], vec![tiny]]).unwrap();
    let executed = execute(Kernel::MatMul, &a, &b).unwrap();
    let reference = ops::matmul(&a, &b).unwrap();
    assert_bit_exact(&executed, &reference);
    assert_eq!(
        reference.data()[0].to_bits(),
        (1.0 + 2f64.powi(-40)).to_bits()
    );
}

#[test]
fn f64_sum_execute_equals_reference_bit_exactly() {
    // 1.0 plus eight 2^-40 increments: a running sum that is not representable
    // in f32. execute_sum folds the same left-to-right order as ops::sum, so the
    // two agree bit-for-bit.
    let mut data = vec![1.0];
    for _ in 0..8 {
        data.push(2f64.powi(-40));
    }
    let a = Array::from_vec(data);
    let executed = execute_sum(&a).unwrap();
    let reference = ops::sum(&a);
    assert!(executed.is_scalar());
    assert_eq!(executed.data()[0].to_bits(), reference.to_bits());
    assert_ne!(reference, 1.0, "the sub-f32 bits must have survived");
}

#[test]
fn larger_f64_matmul_is_bit_exact() {
    // A 3x3 · 3x3 product where one cell's exact value carries sub-f32 bits.
    let e = 2f64.powi(-25); // representable added to 1.0 in f64, lost in f32
    let a = Array::from_rows(vec![
        vec![1.0, e, 0.0],
        vec![0.0, 1.0, e],
        vec![e, 0.0, 1.0],
    ])
    .unwrap();
    let b = Array::from_rows(vec![
        vec![1.0, 0.0, e],
        vec![e, 1.0, 0.0],
        vec![0.0, e, 1.0],
    ])
    .unwrap();
    let executed = execute(Kernel::MatMul, &a, &b).unwrap();
    let reference = ops::matmul(&a, &b).unwrap();
    assert_bit_exact(&executed, &reference);
}

#[test]
fn oversized_dim_is_rejected_for_f64_lowering() {
    // The usize→u32 shape cast is a trust boundary; a dim past u32::MAX must be a
    // clean error (no panic, no truncation), for the f64 dtype too. Uses
    // plan_backend (which lowers without allocating the buffer).
    use coding_adventures_array_runtime::plan_backend;
    let big = (u32::MAX as usize) + 1;
    let add = Kernel::Elementwise(BinOp::Add);
    assert!(plan_backend(add, DType::F64, &[big], &[big], false).is_err());
    // execute_sum guards the same boundary; we can't allocate u32::MAX+1 doubles,
    // so we assert the guard's predicate directly (kept in lockstep with the code).
    assert!(u32::try_from(big).is_err());
}
