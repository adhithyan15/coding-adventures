//! Bridge from CAS matrices to the shared matrix execution backend.
//!
//! The matrix execution layer only supports machine numeric dtypes today, so
//! this module handles concrete integer/float matrices and leaves symbolic or
//! exact-rational inputs to the CAS fallback paths.

use compute_ir::{
    BufferId, ComputeGraph, OpTiming as PlanOpTiming, PlacedOp, PlacedTensor, Residency,
    CPU_EXECUTOR,
};
use executor_protocol::{ExecutorRequest, ExecutorResponse};
use matrix_cpu::CpuExecutor;
use matrix_ir::{DType, Op, Shape, TensorId};
use symbolic_ir::{flt, int, IRNode};

use crate::matrix::{matrix, MatrixError, MatrixResult};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BackendBinaryOp {
    Add,
    Sub,
    Mul,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BackendDType {
    I32,
    F32,
}

impl BackendDType {
    fn matrix_dtype(self) -> DType {
        match self {
            BackendDType::I32 => DType::I32,
            BackendDType::F32 => DType::F32,
        }
    }

    fn size_bytes(self) -> usize {
        self.matrix_dtype().size_bytes()
    }
}

pub(crate) fn try_transpose(rows: &[Vec<IRNode>]) -> Option<MatrixResult<IRNode>> {
    let nrows = rows.len();
    let ncols = rows.first().map_or(0, |row| row.len());
    let dtype = dtype_for_rows(rows)?;
    let input = encode_rows(rows, dtype)?;
    Some(
        run_unary(
            Op::Transpose {
                input: TensorId(0),
                perm: vec![1, 0],
                output: TensorId(1),
            },
            &[nrows, ncols],
            &[ncols, nrows],
            input,
            dtype,
        )
        .and_then(|bytes| decode_rows(&bytes, ncols, nrows, dtype))
        .and_then(matrix),
    )
}

pub(crate) fn try_elementwise(
    lhs: &[Vec<IRNode>],
    rhs: &[Vec<IRNode>],
    op: BackendBinaryOp,
) -> Option<MatrixResult<IRNode>> {
    let nrows = lhs.len();
    let ncols = lhs.first().map_or(0, |row| row.len());
    let dtype = dtype_for_two_rows(lhs, rhs)?;
    let lhs_bytes = encode_rows(lhs, dtype)?;
    let rhs_bytes = encode_rows(rhs, dtype)?;

    Some(
        run_binary(
            op,
            &[nrows, ncols],
            &[nrows, ncols],
            lhs_bytes,
            rhs_bytes,
            dtype,
        )
        .and_then(|bytes| decode_rows(&bytes, nrows, ncols, dtype))
        .and_then(matrix),
    )
}

pub(crate) fn try_scalar_multiply(
    scalar: &IRNode,
    rows: &[Vec<IRNode>],
) -> Option<MatrixResult<IRNode>> {
    let nrows = rows.len();
    let ncols = rows.first().map_or(0, |row| row.len());
    let dtype = dtype_for_scalar_and_rows(scalar, rows)?;
    let lhs = encode_scalar_matrix(scalar, nrows * ncols, dtype)?;
    let rhs = encode_rows(rows, dtype)?;

    Some(
        run_binary(
            BackendBinaryOp::Mul,
            &[nrows, ncols],
            &[nrows, ncols],
            lhs,
            rhs,
            dtype,
        )
        .and_then(|bytes| decode_rows(&bytes, nrows, ncols, dtype))
        .and_then(matrix),
    )
}

pub(crate) fn try_matmul(lhs: &[Vec<IRNode>], rhs: &[Vec<IRNode>]) -> Option<MatrixResult<IRNode>> {
    let lhs_rows = lhs.len();
    let lhs_cols = lhs.first().map_or(0, |row| row.len());
    let rhs_rows = rhs.len();
    let rhs_cols = rhs.first().map_or(0, |row| row.len());
    let dtype = dtype_for_two_rows(lhs, rhs)?;
    let lhs_bytes = encode_rows(lhs, dtype)?;
    let rhs_bytes = encode_rows(rhs, dtype)?;

    Some(
        run_matmul(
            &[lhs_rows, lhs_cols],
            &[rhs_rows, rhs_cols],
            lhs_bytes,
            rhs_bytes,
            dtype,
        )
        .and_then(|bytes| decode_rows(&bytes, lhs_rows, rhs_cols, dtype))
        .and_then(matrix),
    )
}

fn dtype_for_rows(rows: &[Vec<IRNode>]) -> Option<BackendDType> {
    let mut dtype = BackendDType::I32;
    for row in rows {
        for cell in row {
            dtype = merge_dtype(dtype, dtype_for_node(cell)?)?;
        }
    }
    Some(dtype)
}

fn dtype_for_two_rows(lhs: &[Vec<IRNode>], rhs: &[Vec<IRNode>]) -> Option<BackendDType> {
    merge_dtype(dtype_for_rows(lhs)?, dtype_for_rows(rhs)?)
}

fn dtype_for_scalar_and_rows(scalar: &IRNode, rows: &[Vec<IRNode>]) -> Option<BackendDType> {
    merge_dtype(dtype_for_node(scalar)?, dtype_for_rows(rows)?)
}

fn dtype_for_node(node: &IRNode) -> Option<BackendDType> {
    match node {
        IRNode::Integer(value) if i32::try_from(*value).is_ok() => Some(BackendDType::I32),
        IRNode::Float(value) if value.is_finite() => Some(BackendDType::F32),
        _ => None,
    }
}

fn merge_dtype(lhs: BackendDType, rhs: BackendDType) -> Option<BackendDType> {
    match (lhs, rhs) {
        (BackendDType::I32, BackendDType::I32) => Some(BackendDType::I32),
        (BackendDType::I32, BackendDType::F32) | (BackendDType::F32, BackendDType::I32) => {
            Some(BackendDType::F32)
        }
        (BackendDType::F32, BackendDType::F32) => Some(BackendDType::F32),
    }
}

fn encode_rows(rows: &[Vec<IRNode>], dtype: BackendDType) -> Option<Vec<u8>> {
    let values = rows.iter().flat_map(|row| row.iter());
    encode_nodes(values, dtype)
}

fn encode_scalar_matrix(scalar: &IRNode, len: usize, dtype: BackendDType) -> Option<Vec<u8>> {
    encode_nodes(std::iter::repeat(scalar).take(len), dtype)
}

fn encode_nodes<'a>(
    nodes: impl Iterator<Item = &'a IRNode>,
    dtype: BackendDType,
) -> Option<Vec<u8>> {
    let mut bytes = Vec::new();
    for node in nodes {
        match (dtype, node) {
            (BackendDType::I32, IRNode::Integer(value)) => {
                let value = i32::try_from(*value).ok()?;
                bytes.extend_from_slice(&value.to_le_bytes());
            }
            (BackendDType::F32, IRNode::Integer(value)) => {
                bytes.extend_from_slice(&(*value as f32).to_le_bytes());
            }
            (BackendDType::F32, IRNode::Float(value)) if value.is_finite() => {
                bytes.extend_from_slice(&(*value as f32).to_le_bytes());
            }
            _ => return None,
        }
    }
    Some(bytes)
}

fn decode_rows(
    bytes: &[u8],
    nrows: usize,
    ncols: usize,
    dtype: BackendDType,
) -> MatrixResult<Vec<Vec<IRNode>>> {
    let mut rows = Vec::with_capacity(nrows);
    match dtype {
        BackendDType::I32 => {
            let expected = nrows * ncols * dtype.size_bytes();
            if bytes.len() != expected {
                return Err(MatrixError(format!(
                    "matrix backend: expected {expected} output bytes, got {}",
                    bytes.len()
                )));
            }
            for row in 0..nrows {
                let mut cells = Vec::with_capacity(ncols);
                for col in 0..ncols {
                    let offset = (row * ncols + col) * 4;
                    let arr: [u8; 4] = bytes[offset..offset + 4]
                        .try_into()
                        .map_err(|_| MatrixError("matrix backend: malformed i32 output".into()))?;
                    cells.push(int(i32::from_le_bytes(arr) as i64));
                }
                rows.push(cells);
            }
        }
        BackendDType::F32 => {
            let expected = nrows * ncols * dtype.size_bytes();
            if bytes.len() != expected {
                return Err(MatrixError(format!(
                    "matrix backend: expected {expected} output bytes, got {}",
                    bytes.len()
                )));
            }
            for row in 0..nrows {
                let mut cells = Vec::with_capacity(ncols);
                for col in 0..ncols {
                    let offset = (row * ncols + col) * 4;
                    let arr: [u8; 4] = bytes[offset..offset + 4]
                        .try_into()
                        .map_err(|_| MatrixError("matrix backend: malformed f32 output".into()))?;
                    cells.push(flt(f32::from_le_bytes(arr) as f64));
                }
                rows.push(cells);
            }
        }
    }
    Ok(rows)
}

fn run_unary(
    op: Op,
    input_shape: &[usize],
    output_shape: &[usize],
    input: Vec<u8>,
    dtype: BackendDType,
) -> MatrixResult<Vec<u8>> {
    run_backend(
        vec![(TensorId(0), input_shape.to_vec(), input)],
        TensorId(1),
        output_shape,
        dtype,
        op,
    )
}

fn run_binary(
    op: BackendBinaryOp,
    shape: &[usize],
    output_shape: &[usize],
    lhs: Vec<u8>,
    rhs: Vec<u8>,
    dtype: BackendDType,
) -> MatrixResult<Vec<u8>> {
    let op = match op {
        BackendBinaryOp::Add => Op::Add {
            lhs: TensorId(0),
            rhs: TensorId(1),
            output: TensorId(2),
        },
        BackendBinaryOp::Sub => Op::Sub {
            lhs: TensorId(0),
            rhs: TensorId(1),
            output: TensorId(2),
        },
        BackendBinaryOp::Mul => Op::Mul {
            lhs: TensorId(0),
            rhs: TensorId(1),
            output: TensorId(2),
        },
    };
    run_backend(
        vec![
            (TensorId(0), shape.to_vec(), lhs),
            (TensorId(1), shape.to_vec(), rhs),
        ],
        TensorId(2),
        output_shape,
        dtype,
        op,
    )
}

fn run_matmul(
    lhs_shape: &[usize],
    rhs_shape: &[usize],
    lhs: Vec<u8>,
    rhs: Vec<u8>,
    dtype: BackendDType,
) -> MatrixResult<Vec<u8>> {
    let output_shape = [lhs_shape[0], rhs_shape[1]];
    run_backend(
        vec![
            (TensorId(0), lhs_shape.to_vec(), lhs),
            (TensorId(1), rhs_shape.to_vec(), rhs),
        ],
        TensorId(2),
        &output_shape,
        dtype,
        Op::MatMul {
            a: TensorId(0),
            b: TensorId(1),
            output: TensorId(2),
        },
    )
}

fn run_backend(
    inputs: Vec<(TensorId, Vec<usize>, Vec<u8>)>,
    output_id: TensorId,
    output_shape: &[usize],
    dtype: BackendDType,
    op: Op,
) -> MatrixResult<Vec<u8>> {
    let exec = CpuExecutor::new();
    let mut placed_inputs = Vec::with_capacity(inputs.len());
    let mut tensors = Vec::with_capacity(inputs.len() + 1);

    for (tensor, shape, bytes) in inputs {
        let buffer = alloc(&exec, bytes.len())?;
        upload(&exec, buffer, bytes)?;
        let placed = placed(tensor, dtype, &shape, buffer)?;
        placed_inputs.push(placed.clone());
        tensors.push(placed);
    }

    let output_bytes = byte_len(output_shape, dtype)?;
    let output_buffer = alloc(&exec, output_bytes)?;
    let output = placed(output_id, dtype, output_shape, output_buffer)?;
    let output_residency = output.residency;
    tensors.push(output.clone());

    let graph = ComputeGraph {
        format_version: compute_ir::WIRE_FORMAT_VERSION,
        inputs: placed_inputs,
        outputs: vec![output],
        constants: vec![],
        ops: vec![
            PlacedOp::Alloc {
                residency: output_residency,
                bytes: output_bytes as u64,
            },
            PlacedOp::Compute {
                op,
                executor: CPU_EXECUTOR,
                timing: PlanOpTiming { estimated_ns: 0 },
            },
        ],
        tensors,
    };
    graph
        .validate()
        .map_err(|err| MatrixError(format!("matrix backend validate: {err:?}")))?;

    match exec.handle(ExecutorRequest::Dispatch { job_id: 1, graph }) {
        ExecutorResponse::DispatchDone { .. } => {}
        ExecutorResponse::Error { message, .. } => {
            return Err(MatrixError(format!("matrix backend dispatch: {message}")));
        }
        other => {
            return Err(MatrixError(format!(
                "matrix backend dispatch: unexpected response {other:?}"
            )));
        }
    }

    download(&exec, output_buffer, output_bytes)
}

fn alloc(exec: &CpuExecutor, bytes: usize) -> MatrixResult<BufferId> {
    match exec.handle(ExecutorRequest::AllocBuffer {
        bytes: bytes as u64,
    }) {
        ExecutorResponse::BufferAllocated { buffer } => Ok(buffer),
        ExecutorResponse::Error { message, .. } => {
            Err(MatrixError(format!("matrix backend alloc: {message}")))
        }
        other => Err(MatrixError(format!(
            "matrix backend alloc: unexpected response {other:?}"
        ))),
    }
}

fn upload(exec: &CpuExecutor, buffer: BufferId, data: Vec<u8>) -> MatrixResult<()> {
    match exec.handle(ExecutorRequest::UploadBuffer {
        buffer,
        offset: 0,
        data,
    }) {
        ExecutorResponse::BufferUploaded { .. } => Ok(()),
        ExecutorResponse::Error { message, .. } => {
            Err(MatrixError(format!("matrix backend upload: {message}")))
        }
        other => Err(MatrixError(format!(
            "matrix backend upload: unexpected response {other:?}"
        ))),
    }
}

fn download(exec: &CpuExecutor, buffer: BufferId, len: usize) -> MatrixResult<Vec<u8>> {
    match exec.handle(ExecutorRequest::DownloadBuffer {
        buffer,
        offset: 0,
        len: len as u64,
    }) {
        ExecutorResponse::BufferData { data, .. } => Ok(data),
        ExecutorResponse::Error { message, .. } => {
            Err(MatrixError(format!("matrix backend download: {message}")))
        }
        other => Err(MatrixError(format!(
            "matrix backend download: unexpected response {other:?}"
        ))),
    }
}

fn placed(
    id: TensorId,
    dtype: BackendDType,
    shape: &[usize],
    buffer: BufferId,
) -> MatrixResult<PlacedTensor> {
    let dims: Result<Vec<u32>, _> = shape.iter().copied().map(u32::try_from).collect();
    Ok(PlacedTensor {
        id,
        dtype: dtype.matrix_dtype(),
        shape: Shape::from(
            &dims.map_err(|_| MatrixError("matrix backend: shape too large".into()))?,
        ),
        residency: Residency {
            executor: CPU_EXECUTOR,
            buffer,
        },
    })
}

fn byte_len(shape: &[usize], dtype: BackendDType) -> MatrixResult<usize> {
    let elements = shape.iter().try_fold(1usize, |acc, dim| {
        acc.checked_mul(*dim)
            .ok_or_else(|| MatrixError("matrix backend: shape overflow".into()))
    })?;
    elements
        .checked_mul(dtype.size_bytes())
        .ok_or_else(|| MatrixError("matrix backend: byte size overflow".into()))
}
