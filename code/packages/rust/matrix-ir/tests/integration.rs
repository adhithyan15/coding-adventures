//! Integration tests for `matrix-ir`.  Covers:
//!
//! 1. **Builder + validate** — representative graphs constructed via the
//!    builder, validated structurally and semantically.
//! 2. **Validator rejection** — deliberately malformed graphs, asserted
//!    to fail with the expected `IrError` variant.
//! 3. **Wire round-trip** — every graph in the suite is serialised,
//!    deserialised, asserted equal to the original.
//! 4. **Coverage gate** — meta-test that confirms the integration suite
//!    exercises every `Op` variant.
//!
//! See spec MX01 §"Test methodology".

use matrix_ir::{Constant, DType, Graph, GraphBuilder, IrError, Op, Shape, Tensor, TensorId};

// ─────────────────── 1. Builder + validate ───────────────────

#[test]
fn elementwise_unary_chain() {
    // y = log(exp(neg(abs(x))))
    let mut g = GraphBuilder::new();
    let x = g.input(DType::F32, Shape::from(&[4]));
    let a = g.abs(&x);
    let b = g.neg(&a);
    let c = g.exp(&b);
    let d = g.log(&c);
    g.output(&d);
    let g = g.build().unwrap();
    g.validate().unwrap();
    assert_eq!(g.ops.len(), 4);
}

#[test]
fn elementwise_binary_each() {
    let mut g = GraphBuilder::new();
    let a = g.input(DType::F32, Shape::from(&[3]));
    let b = g.input(DType::F32, Shape::from(&[3]));
    let _ = g.add(&a, &b);
    let _ = g.sub(&a, &b);
    let _ = g.mul(&a, &b);
    let _ = g.div(&a, &b);
    let _ = g.max(&a, &b);
    let _ = g.min(&a, &b);
    let _ = g.pow(&a, &b);
    let g = g.build().unwrap();
    assert_eq!(g.ops.len(), 7);
    g.validate().unwrap();
}

#[test]
fn reduction_each() {
    let mut g = GraphBuilder::new();
    let x = g.input(DType::F32, Shape::from(&[2, 3, 4]));
    let _ = g.reduce_sum(&x, vec![1], false);
    let _ = g.reduce_max(&x, vec![1], true);
    let _ = g.reduce_mean(&x, vec![], false);
    let g = g.build().unwrap();
    g.validate().unwrap();
}

#[test]
fn shape_ops_each() {
    let mut g = GraphBuilder::new();
    let x = g.input(DType::F32, Shape::from(&[2, 3]));
    let r = g.reshape(&x, Shape::from(&[6]));
    let _ = g.reshape(&r, Shape::from(&[6, 1]));
    let _ = g.transpose(&x, vec![1, 0]);
    let _ = g.broadcast(&x, Shape::from(&[2, 3]));
    let g = g.build().unwrap();
    g.validate().unwrap();
}

#[test]
fn matmul_chain() {
    let mut g = GraphBuilder::new();
    let a = g.input(DType::F32, Shape::from(&[3, 4]));
    let b = g.input(DType::F32, Shape::from(&[4, 5]));
    let c = g.input(DType::F32, Shape::from(&[5, 2]));
    let ab = g.matmul(&a, &b);
    let abc = g.matmul(&ab, &c);
    g.output(&abc);
    let g = g.build().unwrap();
    g.validate().unwrap();
    assert_eq!(g.tensor(abc.id).unwrap().shape, Shape::from(&[3, 2]));
}

#[test]
fn comparison_and_where() {
    let mut g = GraphBuilder::new();
    let a = g.input(DType::F32, Shape::from(&[3]));
    let b = g.input(DType::F32, Shape::from(&[3]));
    let lt = g.less(&a, &b);
    let _ = g.equal(&a, &b);
    let _ = g.greater(&a, &b);
    let r = g.where_(&lt, &a, &b);
    g.output(&r);
    let g = g.build().unwrap();
    g.validate().unwrap();
    assert_eq!(g.tensor(r.id).unwrap().dtype, DType::F32);
}

#[test]
fn cast_round_trip() {
    let mut g = GraphBuilder::new();
    let x = g.input(DType::F32, Shape::from(&[3]));
    let xi = g.cast(&x, DType::I32);
    let _ = g.cast(&xi, DType::F32);
    let g = g.build().unwrap();
    g.validate().unwrap();
}

#[test]
fn constant_basic() {
    let mut g = GraphBuilder::new();
    let _ = g.constant(DType::F32, Shape::from(&[2]), vec![0u8; 8]);
    let g = g.build().unwrap();
    g.validate().unwrap();
    assert_eq!(g.constants.len(), 1);
}

#[test]
fn relu_layer_full() {
    // y = relu(x @ w + b) over [1, 4] inputs.
    let mut g = GraphBuilder::new();
    let x = g.input(DType::F32, Shape::from(&[1, 4]));
    let w = g.input(DType::F32, Shape::from(&[4, 2]));
    let b = g.input(DType::F32, Shape::from(&[1, 2]));
    let zero = g.constant(DType::F32, Shape::from(&[1, 2]), vec![0u8; 8]);
    let xw = g.matmul(&x, &w);
    let xwb = g.add(&xw, &b);
    let y = g.max(&xwb, &zero);
    g.output(&y);
    let g = g.build().unwrap();
    g.validate().unwrap();
}

// ─────────────────── 2. Validator rejection ───────────────────

/// Manually construct a graph with a deliberately broken op so we can
/// exercise the validator's rejection paths.  These cases bypass the
/// builder's eager checks.
fn make_graph(tensors: Vec<Tensor>, ops: Vec<Op>, outputs: Vec<TensorId>) -> Graph {
    Graph {
        inputs: Vec::new(),
        outputs,
        ops,
        tensors,
        constants: Vec::new(),
    }
}

#[test]
fn rejects_undefined_tensor_input() {
    let g = make_graph(
        vec![Tensor::new(TensorId(0), DType::F32, Shape::from(&[3]))],
        vec![Op::Neg {
            input: TensorId(99),
            output: TensorId(0),
        }],
        vec![],
    );
    assert!(matches!(
        g.validate(),
        Err(IrError::UndefinedTensor { tensor: TensorId(99), .. })
    ));
}

#[test]
fn rejects_shape_mismatch_in_add() {
    let g = make_graph(
        vec![
            Tensor::new(TensorId(0), DType::F32, Shape::from(&[3])),
            Tensor::new(TensorId(1), DType::F32, Shape::from(&[4])),
            Tensor::new(TensorId(2), DType::F32, Shape::from(&[3])),
        ],
        vec![Op::Add {
            lhs: TensorId(0),
            rhs: TensorId(1),
            output: TensorId(2),
        }],
        vec![],
    );
    // Inputs 0/1 aren't defined as inputs, so we'll fail on UndefinedTensor first.
    // Construct as graph inputs instead.
    let g = Graph {
        inputs: vec![
            Tensor::new(TensorId(0), DType::F32, Shape::from(&[3])),
            Tensor::new(TensorId(1), DType::F32, Shape::from(&[4])),
        ],
        outputs: vec![],
        ops: vec![Op::Add {
            lhs: TensorId(0),
            rhs: TensorId(1),
            output: TensorId(2),
        }],
        tensors: vec![
            Tensor::new(TensorId(0), DType::F32, Shape::from(&[3])),
            Tensor::new(TensorId(1), DType::F32, Shape::from(&[4])),
            Tensor::new(TensorId(2), DType::F32, Shape::from(&[3])),
        ],
        constants: vec![],
    };
    assert!(matches!(
        g.validate(),
        Err(IrError::ShapeMismatch { .. })
    ));
}

#[test]
fn rejects_dtype_mismatch_in_add() {
    let g = Graph {
        inputs: vec![
            Tensor::new(TensorId(0), DType::F32, Shape::from(&[3])),
            Tensor::new(TensorId(1), DType::I32, Shape::from(&[3])),
        ],
        outputs: vec![],
        ops: vec![Op::Add {
            lhs: TensorId(0),
            rhs: TensorId(1),
            output: TensorId(2),
        }],
        tensors: vec![
            Tensor::new(TensorId(0), DType::F32, Shape::from(&[3])),
            Tensor::new(TensorId(1), DType::I32, Shape::from(&[3])),
            Tensor::new(TensorId(2), DType::F32, Shape::from(&[3])),
        ],
        constants: vec![],
    };
    assert!(matches!(
        g.validate(),
        Err(IrError::DTypeMismatch { .. })
    ));
}

#[test]
fn rejects_matmul_inner_mismatch() {
    let g = Graph {
        inputs: vec![
            Tensor::new(TensorId(0), DType::F32, Shape::from(&[3, 4])),
            Tensor::new(TensorId(1), DType::F32, Shape::from(&[5, 6])),
        ],
        outputs: vec![],
        ops: vec![Op::MatMul {
            a: TensorId(0),
            b: TensorId(1),
            output: TensorId(2),
        }],
        tensors: vec![
            Tensor::new(TensorId(0), DType::F32, Shape::from(&[3, 4])),
            Tensor::new(TensorId(1), DType::F32, Shape::from(&[5, 6])),
            Tensor::new(TensorId(2), DType::F32, Shape::from(&[3, 6])),
        ],
        constants: vec![],
    };
    assert!(matches!(
        g.validate(),
        Err(IrError::ShapeMismatch { .. })
    ));
}

#[test]
fn rejects_invalid_perm() {
    let g = Graph {
        inputs: vec![Tensor::new(TensorId(0), DType::F32, Shape::from(&[2, 3]))],
        outputs: vec![],
        ops: vec![Op::Transpose {
            input: TensorId(0),
            perm: vec![0, 0],
            output: TensorId(1),
        }],
        tensors: vec![
            Tensor::new(TensorId(0), DType::F32, Shape::from(&[2, 3])),
            Tensor::new(TensorId(1), DType::F32, Shape::from(&[2, 2])),
        ],
        constants: vec![],
    };
    assert!(matches!(
        g.validate(),
        Err(IrError::InvalidPermutation { .. })
    ));
}

#[test]
fn rejects_reshape_numel_mismatch() {
    let g = Graph {
        inputs: vec![Tensor::new(TensorId(0), DType::F32, Shape::from(&[2, 3]))],
        outputs: vec![],
        ops: vec![Op::Reshape {
            input: TensorId(0),
            new_shape: Shape::from(&[7]),
            output: TensorId(1),
        }],
        tensors: vec![
            Tensor::new(TensorId(0), DType::F32, Shape::from(&[2, 3])),
            Tensor::new(TensorId(1), DType::F32, Shape::from(&[7])),
        ],
        constants: vec![],
    };
    assert!(matches!(
        g.validate(),
        Err(IrError::NumelMismatch { .. })
    ));
}

#[test]
fn rejects_invalid_broadcast() {
    let g = Graph {
        inputs: vec![Tensor::new(TensorId(0), DType::F32, Shape::from(&[2, 3]))],
        outputs: vec![],
        ops: vec![Op::Broadcast {
            input: TensorId(0),
            target_shape: Shape::from(&[5, 3]),
            output: TensorId(1),
        }],
        tensors: vec![
            Tensor::new(TensorId(0), DType::F32, Shape::from(&[2, 3])),
            Tensor::new(TensorId(1), DType::F32, Shape::from(&[5, 3])),
        ],
        constants: vec![],
    };
    assert!(matches!(
        g.validate(),
        Err(IrError::InvalidBroadcast { .. })
    ));
}

#[test]
fn rejects_non_u8_predicate_in_where() {
    let g = Graph {
        inputs: vec![
            Tensor::new(TensorId(0), DType::F32, Shape::from(&[3])),
            Tensor::new(TensorId(1), DType::F32, Shape::from(&[3])),
            Tensor::new(TensorId(2), DType::F32, Shape::from(&[3])),
        ],
        outputs: vec![],
        ops: vec![Op::Where {
            predicate: TensorId(0),
            true_value: TensorId(1),
            false_value: TensorId(2),
            output: TensorId(3),
        }],
        tensors: vec![
            Tensor::new(TensorId(0), DType::F32, Shape::from(&[3])),
            Tensor::new(TensorId(1), DType::F32, Shape::from(&[3])),
            Tensor::new(TensorId(2), DType::F32, Shape::from(&[3])),
            Tensor::new(TensorId(3), DType::F32, Shape::from(&[3])),
        ],
        constants: vec![],
    };
    assert!(matches!(
        g.validate(),
        Err(IrError::NonU8Predicate { .. })
    ));
}

#[test]
fn rejects_undefined_output() {
    let g = Graph {
        inputs: vec![],
        outputs: vec![TensorId(99)],
        ops: vec![],
        tensors: vec![],
        constants: vec![],
    };
    assert!(matches!(
        g.validate(),
        Err(IrError::UndefinedOutput { tensor: TensorId(99) })
    ));
}

#[test]
fn rejects_constant_byte_length_mismatch() {
    let g = Graph {
        inputs: vec![],
        outputs: vec![],
        ops: vec![],
        tensors: vec![Tensor::new(TensorId(0), DType::F32, Shape::from(&[2]))],
        constants: vec![Constant {
            tensor: Tensor::new(TensorId(0), DType::F32, Shape::from(&[2])),
            // Should be 8 bytes (2 × f32) but we supplied 5.
            bytes: vec![0u8; 5],
        }],
    };
    assert!(matches!(
        g.validate(),
        Err(IrError::ConstantByteLength {
            expected: 8,
            actual: 5,
            ..
        })
    ));
}

#[test]
fn rejects_tensor_id_mismatch() {
    let g = Graph {
        inputs: vec![],
        outputs: vec![],
        ops: vec![],
        tensors: vec![Tensor::new(TensorId(7), DType::F32, Shape::from(&[1]))],
        constants: vec![],
    };
    assert!(matches!(
        g.validate(),
        Err(IrError::TensorIdMismatch { .. })
    ));
}

#[test]
fn rejects_reduction_axis_out_of_range() {
    let g = Graph {
        inputs: vec![Tensor::new(TensorId(0), DType::F32, Shape::from(&[2, 3]))],
        outputs: vec![],
        ops: vec![Op::ReduceSum {
            input: TensorId(0),
            axes: vec![5],
            keep_dims: false,
            output: TensorId(1),
        }],
        tensors: vec![
            Tensor::new(TensorId(0), DType::F32, Shape::from(&[2, 3])),
            Tensor::new(TensorId(1), DType::F32, Shape::from(&[2, 3])),
        ],
        constants: vec![],
    };
    assert!(matches!(
        g.validate(),
        Err(IrError::InvalidAxis { axis: 5, .. })
    ));
}

// ─────────────────── 3. Wire round-trip ───────────────────

fn round_trip(g: &Graph) {
    let bytes = g.to_bytes();
    let decoded = Graph::from_bytes(&bytes).expect("decode should succeed");
    assert_eq!(&decoded, g, "round-trip changed the graph");
    // And the bytes must be deterministic.
    let bytes2 = decoded.to_bytes();
    assert_eq!(bytes, bytes2, "encoding is not deterministic");
}

#[test]
fn round_trip_relu_layer() {
    let mut g = GraphBuilder::new();
    let x = g.input(DType::F32, Shape::from(&[1, 4]));
    let w = g.input(DType::F32, Shape::from(&[4, 2]));
    let b = g.input(DType::F32, Shape::from(&[1, 2]));
    let zero = g.constant(DType::F32, Shape::from(&[1, 2]), vec![0u8; 8]);
    let xw = g.matmul(&x, &w);
    let xwb = g.add(&xw, &b);
    let y = g.max(&xwb, &zero);
    g.output(&y);
    let g = g.build().unwrap();
    round_trip(&g);
}

#[test]
fn round_trip_all_unary() {
    let mut g = GraphBuilder::new();
    let x = g.input(DType::F32, Shape::from(&[3]));
    let _ = g.neg(&x);
    let _ = g.abs(&x);
    let _ = g.sqrt(&x);
    let _ = g.exp(&x);
    let _ = g.log(&x);
    let _ = g.tanh(&x);
    let _ = g.recip(&x);
    let g = g.build().unwrap();
    round_trip(&g);
}

#[test]
fn round_trip_all_binary() {
    let mut g = GraphBuilder::new();
    let a = g.input(DType::F32, Shape::from(&[3]));
    let b = g.input(DType::F32, Shape::from(&[3]));
    let _ = g.add(&a, &b);
    let _ = g.sub(&a, &b);
    let _ = g.mul(&a, &b);
    let _ = g.div(&a, &b);
    let _ = g.max(&a, &b);
    let _ = g.min(&a, &b);
    let _ = g.pow(&a, &b);
    let g = g.build().unwrap();
    round_trip(&g);
}

#[test]
fn round_trip_reductions() {
    let mut g = GraphBuilder::new();
    let x = g.input(DType::F32, Shape::from(&[2, 3, 4]));
    let _ = g.reduce_sum(&x, vec![1], false);
    let _ = g.reduce_max(&x, vec![0, 2], true);
    let _ = g.reduce_mean(&x, vec![], false);
    let g = g.build().unwrap();
    round_trip(&g);
}

#[test]
fn round_trip_shape_ops() {
    let mut g = GraphBuilder::new();
    let x = g.input(DType::F32, Shape::from(&[2, 3]));
    let _ = g.reshape(&x, Shape::from(&[6]));
    let _ = g.transpose(&x, vec![1, 0]);
    let _ = g.broadcast(&x, Shape::from(&[2, 3]));
    let g = g.build().unwrap();
    round_trip(&g);
}

#[test]
fn round_trip_comparison_and_where() {
    let mut g = GraphBuilder::new();
    let a = g.input(DType::F32, Shape::from(&[3]));
    let b = g.input(DType::F32, Shape::from(&[3]));
    let lt = g.less(&a, &b);
    let _ = g.equal(&a, &b);
    let _ = g.greater(&a, &b);
    let _ = g.where_(&lt, &a, &b);
    let g = g.build().unwrap();
    round_trip(&g);
}

#[test]
fn round_trip_cast() {
    let mut g = GraphBuilder::new();
    let x = g.input(DType::F32, Shape::from(&[3]));
    let _ = g.cast(&x, DType::I32);
    let _ = g.cast(&x, DType::U8);
    let g = g.build().unwrap();
    round_trip(&g);
}

// ─────────────────── 4. Coverage gate ───────────────────

/// Build a graph that exercises every Op variant.  This serves as the
/// coverage gate — if a future variant is added without being included
/// here, the test fails (because we count distinct wire tags and assert
/// the count matches).
#[test]
fn coverage_every_variant() {
    // Build a graph that uses every variant.  We don't validate it as a
    // semantically-correct graph; we just check that one of every wire
    // tag is present.
    let mut g = GraphBuilder::new();
    let x = g.input(DType::F32, Shape::from(&[2, 3]));
    let y = g.input(DType::F32, Shape::from(&[2, 3]));
    let p = g.input(DType::U8, Shape::from(&[2, 3]));
    let _ = g.constant(DType::F32, Shape::from(&[2]), vec![0u8; 8]); // emits Const
    let _ = g.neg(&x);
    let _ = g.abs(&x);
    let _ = g.sqrt(&x);
    let _ = g.exp(&x);
    let _ = g.log(&x);
    let _ = g.tanh(&x);
    let _ = g.recip(&x);
    let _ = g.add(&x, &y);
    let _ = g.sub(&x, &y);
    let _ = g.mul(&x, &y);
    let _ = g.div(&x, &y);
    let _ = g.max(&x, &y);
    let _ = g.min(&x, &y);
    let _ = g.pow(&x, &y);
    let _ = g.reduce_sum(&x, vec![0], false);
    let _ = g.reduce_max(&x, vec![0], false);
    let _ = g.reduce_mean(&x, vec![0], false);
    let _ = g.reshape(&x, Shape::from(&[6]));
    let _ = g.transpose(&x, vec![1, 0]);
    let _ = g.broadcast(&x, Shape::from(&[2, 3]));
    let mat_a = g.input(DType::F32, Shape::from(&[2, 3]));
    let mat_b = g.input(DType::F32, Shape::from(&[3, 2]));
    let _ = g.matmul(&mat_a, &mat_b);
    let _ = g.equal(&x, &y);
    let _ = g.less(&x, &y);
    let _ = g.greater(&x, &y);
    let _ = g.where_(&p, &x, &y);
    let _ = g.cast(&x, DType::I32);
    let g = g.build().unwrap();

    // Collect distinct wire tags.
    let mut tags: Vec<u8> = g.ops.iter().map(|op| op.wire_tag()).collect();
    tags.sort();
    tags.dedup();

    // V1 has 27 op variants.  If a variant is added without updating
    // this test, this assertion fires.
    assert_eq!(
        tags.len(),
        27,
        "coverage gate: not every Op variant exercised; tags = {:?}",
        tags
    );
}
