//! Per-op evaluation kernels.
//!
//! Each function takes a slice of input bytes (already in dtype-encoded
//! little-endian) and produces a slice of output bytes.  Float-typed
//! ops use IEEE-754 standard semantics; integer ops use saturating or
//! wrapping arithmetic per the spec MX01 contract.
//!
//! ## Layout convention
//!
//! Tensors are dtype-encoded little-endian.  For an N-element f32
//! tensor the buffer is `4*N` bytes; for a u8 tensor it's `N` bytes;
//! for an i32 tensor it's `4*N` bytes.  This matches the matrix-ir
//! constant layout (spec MX01 §"Constants").

use matrix_ir::DType;

/// Helper: read N f32 values from bytes.
pub fn read_f32_vec(bytes: &[u8], n: usize) -> Vec<f32> {
    debug_assert_eq!(bytes.len(), n * 4, "f32 buffer wrong size");
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let off = i * 4;
        let arr: [u8; 4] = bytes[off..off + 4].try_into().unwrap();
        out.push(f32::from_le_bytes(arr));
    }
    out
}

/// Helper: write N f32 values into bytes.
pub fn write_f32_vec(out: &mut [u8], values: &[f32]) {
    debug_assert_eq!(out.len(), values.len() * 4);
    for (i, &v) in values.iter().enumerate() {
        let off = i * 4;
        out[off..off + 4].copy_from_slice(&v.to_le_bytes());
    }
}

/// Helper: read N i32 values from bytes.
pub fn read_i32_vec(bytes: &[u8], n: usize) -> Vec<i32> {
    debug_assert_eq!(bytes.len(), n * 4, "i32 buffer wrong size");
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let off = i * 4;
        let arr: [u8; 4] = bytes[off..off + 4].try_into().unwrap();
        out.push(i32::from_le_bytes(arr));
    }
    out
}

/// Helper: write N i32 values into bytes.
pub fn write_i32_vec(out: &mut [u8], values: &[i32]) {
    debug_assert_eq!(out.len(), values.len() * 4);
    for (i, &v) in values.iter().enumerate() {
        let off = i * 4;
        out[off..off + 4].copy_from_slice(&v.to_le_bytes());
    }
}

/// Bytes-per-element.  Mirrors `DType::size_bytes`; duplicated so this
/// file is self-contained.
pub fn dtype_size(d: DType) -> usize {
    d.size_bytes()
}

// ──────────────────────────── Elementwise unary ────────────────────────────

/// Apply a per-element f32 function to the input bytes, writing the
/// result into `output`.
pub fn unary_f32(input: &[u8], output: &mut [u8], n: usize, f: impl Fn(f32) -> f32) {
    let xs = read_f32_vec(input, n);
    let ys: Vec<f32> = xs.iter().map(|&x| f(x)).collect();
    write_f32_vec(output, &ys);
}

/// Apply a per-element i32 function.
pub fn unary_i32(input: &[u8], output: &mut [u8], n: usize, f: impl Fn(i32) -> i32) {
    let xs = read_i32_vec(input, n);
    let ys: Vec<i32> = xs.iter().map(|&x| f(x)).collect();
    write_i32_vec(output, &ys);
}

/// Apply a per-element u8 function.
pub fn unary_u8(input: &[u8], output: &mut [u8], f: impl Fn(u8) -> u8) {
    for (i, &x) in input.iter().enumerate() {
        output[i] = f(x);
    }
}

// ──────────────────────────── Elementwise binary ────────────────────────────

pub fn binary_f32(lhs: &[u8], rhs: &[u8], output: &mut [u8], n: usize, f: impl Fn(f32, f32) -> f32) {
    let xs = read_f32_vec(lhs, n);
    let ys = read_f32_vec(rhs, n);
    let zs: Vec<f32> = xs.iter().zip(ys.iter()).map(|(&a, &b)| f(a, b)).collect();
    write_f32_vec(output, &zs);
}

pub fn binary_i32(lhs: &[u8], rhs: &[u8], output: &mut [u8], n: usize, f: impl Fn(i32, i32) -> i32) {
    let xs = read_i32_vec(lhs, n);
    let ys = read_i32_vec(rhs, n);
    let zs: Vec<i32> = xs.iter().zip(ys.iter()).map(|(&a, &b)| f(a, b)).collect();
    write_i32_vec(output, &zs);
}

pub fn binary_u8(lhs: &[u8], rhs: &[u8], output: &mut [u8], f: impl Fn(u8, u8) -> u8) {
    for i in 0..lhs.len() {
        output[i] = f(lhs[i], rhs[i]);
    }
}

// ──────────────────────────── MatMul ────────────────────────────

/// f32 matmul: `c[m,n] = a[m,k] × b[k,n]` (row-major).
pub fn matmul_f32(a: &[u8], b: &[u8], c: &mut [u8], m: usize, k: usize, n: usize) {
    let av = read_f32_vec(a, m * k);
    let bv = read_f32_vec(b, k * n);
    let mut cv = vec![0.0f32; m * n];
    for i in 0..m {
        for j in 0..n {
            let mut acc = 0.0f32;
            for kk in 0..k {
                acc += av[i * k + kk] * bv[kk * n + j];
            }
            cv[i * n + j] = acc;
        }
    }
    write_f32_vec(c, &cv);
}

pub fn matmul_i32(a: &[u8], b: &[u8], c: &mut [u8], m: usize, k: usize, n: usize) {
    let av = read_i32_vec(a, m * k);
    let bv = read_i32_vec(b, k * n);
    let mut cv = vec![0i32; m * n];
    for i in 0..m {
        for j in 0..n {
            let mut acc = 0i32;
            for kk in 0..k {
                acc = acc.wrapping_add(av[i * k + kk].wrapping_mul(bv[kk * n + j]));
            }
            cv[i * n + j] = acc;
        }
    }
    write_i32_vec(c, &cv);
}

pub fn matmul_u8(a: &[u8], b: &[u8], c: &mut [u8], m: usize, k: usize, n: usize) {
    for i in 0..m {
        for j in 0..n {
            let mut acc: u8 = 0;
            for kk in 0..k {
                acc = acc.wrapping_add(a[i * k + kk].wrapping_mul(b[kk * n + j]));
            }
            c[i * n + j] = acc;
        }
    }
}

// ──────────────────────────── Reduction helpers ────────────────────────────

/// Compute strides for a row-major shape.  `dims = [d0, d1, ..., dr-1]` →
/// `strides = [d1*d2*...*dr-1, d2*...*dr-1, ..., 1]`.
pub fn row_major_strides(dims: &[u32]) -> Vec<usize> {
    let r = dims.len();
    if r == 0 {
        return Vec::new();
    }
    let mut strides = vec![0usize; r];
    strides[r - 1] = 1;
    for i in (0..r - 1).rev() {
        strides[i] = strides[i + 1] * dims[i + 1] as usize;
    }
    strides
}

/// Total number of elements in a shape.
pub fn numel(dims: &[u32]) -> usize {
    dims.iter().map(|&d| d as usize).product()
}

/// Decompose a flat index into multidim coordinates per `dims`.
pub fn unravel(mut flat: usize, dims: &[u32]) -> Vec<usize> {
    let r = dims.len();
    let mut coords = vec![0usize; r];
    for i in (0..r).rev() {
        let d = dims[i] as usize;
        coords[i] = flat % d;
        flat /= d;
    }
    coords
}

/// Compose multidim coordinates into a flat row-major index.
pub fn ravel(coords: &[usize], strides: &[usize]) -> usize {
    coords
        .iter()
        .zip(strides.iter())
        .map(|(c, s)| c * s)
        .sum()
}

/// Reduce f32 along the given axes using `fold` (initialized at `init`).
/// Returns the reduced bytes and the output shape.
pub fn reduce_f32(
    input: &[u8],
    in_dims: &[u32],
    axes: &[u32],
    keep_dims: bool,
    init: f32,
    fold: impl Fn(f32, f32) -> f32,
) -> (Vec<u8>, Vec<u32>) {
    let rank = in_dims.len();
    let n_in = numel(in_dims);
    let xs = read_f32_vec(input, n_in);

    // Compute output shape.
    let reduce_all = axes.is_empty();
    let mut out_dims: Vec<u32> = Vec::new();
    if reduce_all {
        if keep_dims {
            out_dims = vec![1; rank];
        }
    } else {
        for (i, &d) in in_dims.iter().enumerate() {
            let i = i as u32;
            if axes.contains(&i) {
                if keep_dims {
                    out_dims.push(1);
                }
            } else {
                out_dims.push(d);
            }
        }
    }
    let n_out = numel(&out_dims).max(1);
    let mut acc = vec![init; n_out];

    let in_strides = row_major_strides(in_dims);
    let out_strides = row_major_strides(&out_dims);

    for flat in 0..n_in {
        let in_coords = unravel_with_strides(flat, &in_strides, rank);
        // Map input coords → output coords.
        let out_coords: Vec<usize> = if reduce_all {
            if keep_dims {
                vec![0; rank]
            } else {
                Vec::new()
            }
        } else {
            let mut oc = Vec::new();
            for (i, &c) in in_coords.iter().enumerate() {
                let i = i as u32;
                if axes.contains(&i) {
                    if keep_dims {
                        oc.push(0);
                    }
                } else {
                    oc.push(c);
                }
            }
            oc
        };
        let out_flat = ravel(&out_coords, &out_strides);
        acc[out_flat] = fold(acc[out_flat], xs[flat]);
    }

    let mut out = vec![0u8; n_out * 4];
    write_f32_vec(&mut out, &acc);
    (out, out_dims)
}

pub fn reduce_i32(
    input: &[u8],
    in_dims: &[u32],
    axes: &[u32],
    keep_dims: bool,
    init: i32,
    fold: impl Fn(i32, i32) -> i32,
) -> (Vec<u8>, Vec<u32>) {
    let rank = in_dims.len();
    let n_in = numel(in_dims);
    let xs = read_i32_vec(input, n_in);
    let reduce_all = axes.is_empty();
    let mut out_dims: Vec<u32> = Vec::new();
    if reduce_all {
        if keep_dims {
            out_dims = vec![1; rank];
        }
    } else {
        for (i, &d) in in_dims.iter().enumerate() {
            let i = i as u32;
            if axes.contains(&i) {
                if keep_dims {
                    out_dims.push(1);
                }
            } else {
                out_dims.push(d);
            }
        }
    }
    let n_out = numel(&out_dims).max(1);
    let mut acc = vec![init; n_out];
    let in_strides = row_major_strides(in_dims);
    let out_strides = row_major_strides(&out_dims);
    for flat in 0..n_in {
        let in_coords = unravel_with_strides(flat, &in_strides, rank);
        let out_coords: Vec<usize> = if reduce_all {
            if keep_dims {
                vec![0; rank]
            } else {
                Vec::new()
            }
        } else {
            let mut oc = Vec::new();
            for (i, &c) in in_coords.iter().enumerate() {
                let i = i as u32;
                if axes.contains(&i) {
                    if keep_dims {
                        oc.push(0);
                    }
                } else {
                    oc.push(c);
                }
            }
            oc
        };
        let out_flat = ravel(&out_coords, &out_strides);
        acc[out_flat] = fold(acc[out_flat], xs[flat]);
    }
    let mut out = vec![0u8; n_out * 4];
    write_i32_vec(&mut out, &acc);
    (out, out_dims)
}

pub fn reduce_u8(
    input: &[u8],
    in_dims: &[u32],
    axes: &[u32],
    keep_dims: bool,
    init: u8,
    fold: impl Fn(u8, u8) -> u8,
) -> (Vec<u8>, Vec<u32>) {
    let rank = in_dims.len();
    let n_in = numel(in_dims);
    let reduce_all = axes.is_empty();
    let mut out_dims: Vec<u32> = Vec::new();
    if reduce_all {
        if keep_dims {
            out_dims = vec![1; rank];
        }
    } else {
        for (i, &d) in in_dims.iter().enumerate() {
            let i = i as u32;
            if axes.contains(&i) {
                if keep_dims {
                    out_dims.push(1);
                }
            } else {
                out_dims.push(d);
            }
        }
    }
    let n_out = numel(&out_dims).max(1);
    let mut acc = vec![init; n_out];
    let in_strides = row_major_strides(in_dims);
    let out_strides = row_major_strides(&out_dims);
    for flat in 0..n_in {
        let in_coords = unravel_with_strides(flat, &in_strides, rank);
        let out_coords: Vec<usize> = if reduce_all {
            if keep_dims {
                vec![0; rank]
            } else {
                Vec::new()
            }
        } else {
            let mut oc = Vec::new();
            for (i, &c) in in_coords.iter().enumerate() {
                let i = i as u32;
                if axes.contains(&i) {
                    if keep_dims {
                        oc.push(0);
                    }
                } else {
                    oc.push(c);
                }
            }
            oc
        };
        let out_flat = ravel(&out_coords, &out_strides);
        acc[out_flat] = fold(acc[out_flat], input[flat]);
    }
    (acc, out_dims)
}

/// Helper: decompose flat index using precomputed strides.
fn unravel_with_strides(flat: usize, strides: &[usize], rank: usize) -> Vec<usize> {
    let mut coords = vec![0usize; rank];
    let mut remainder = flat;
    for i in 0..rank {
        if strides[i] == 0 {
            coords[i] = 0;
        } else {
            coords[i] = remainder / strides[i];
            remainder %= strides[i];
        }
    }
    coords
}

// ──────────────────────────── Transpose ────────────────────────────

pub fn transpose_bytes(
    input: &[u8],
    in_dims: &[u32],
    perm: &[u32],
    elem_bytes: usize,
) -> (Vec<u8>, Vec<u32>) {
    let n = numel(in_dims);
    let mut out = vec![0u8; n * elem_bytes];
    let in_strides = row_major_strides(in_dims);
    let out_dims: Vec<u32> = perm.iter().map(|&p| in_dims[p as usize]).collect();
    let out_strides = row_major_strides(&out_dims);
    for flat_in in 0..n {
        let in_coords = unravel_with_strides(flat_in, &in_strides, in_dims.len());
        let out_coords: Vec<usize> = perm.iter().map(|&p| in_coords[p as usize]).collect();
        let flat_out = ravel(&out_coords, &out_strides);
        out[flat_out * elem_bytes..(flat_out + 1) * elem_bytes]
            .copy_from_slice(&input[flat_in * elem_bytes..(flat_in + 1) * elem_bytes]);
    }
    (out, out_dims)
}

// ──────────────────────────── Broadcast ────────────────────────────

pub fn broadcast_bytes(
    input: &[u8],
    in_dims: &[u32],
    target_dims: &[u32],
    elem_bytes: usize,
) -> Vec<u8> {
    let n = numel(target_dims);
    let mut out = vec![0u8; n * elem_bytes];
    let in_strides = row_major_strides(in_dims);
    let target_strides = row_major_strides(target_dims);
    for flat_t in 0..n {
        let t_coords = unravel_with_strides(flat_t, &target_strides, target_dims.len());
        let in_coords: Vec<usize> = t_coords
            .iter()
            .zip(in_dims.iter())
            .map(|(c, &d)| if d == 1 { 0 } else { *c })
            .collect();
        let flat_in = ravel(&in_coords, &in_strides);
        out[flat_t * elem_bytes..(flat_t + 1) * elem_bytes]
            .copy_from_slice(&input[flat_in * elem_bytes..(flat_in + 1) * elem_bytes]);
    }
    out
}

// ──────────────────────────── Cast ────────────────────────────

pub fn cast(input: &[u8], src: DType, dst: DType, n: usize) -> Vec<u8> {
    let mut out = vec![0u8; n * dtype_size(dst)];
    match (src, dst) {
        (DType::F32, DType::F32) => out.copy_from_slice(input),
        (DType::U8, DType::U8) => out.copy_from_slice(input),
        (DType::I32, DType::I32) => out.copy_from_slice(input),
        (DType::F32, DType::I32) => {
            let xs = read_f32_vec(input, n);
            let ys: Vec<i32> = xs.iter().map(|&x| x as i32).collect();
            write_i32_vec(&mut out, &ys);
        }
        (DType::F32, DType::U8) => {
            let xs = read_f32_vec(input, n);
            for (i, &x) in xs.iter().enumerate() {
                let v = if x < 0.0 {
                    0u8
                } else if x > 255.0 {
                    255u8
                } else {
                    x as u8
                };
                out[i] = v;
            }
        }
        (DType::I32, DType::F32) => {
            let xs = read_i32_vec(input, n);
            let ys: Vec<f32> = xs.iter().map(|&x| x as f32).collect();
            write_f32_vec(&mut out, &ys);
        }
        (DType::I32, DType::U8) => {
            let xs = read_i32_vec(input, n);
            for (i, &x) in xs.iter().enumerate() {
                let v = x.clamp(0, 255) as u8;
                out[i] = v;
            }
        }
        (DType::U8, DType::F32) => {
            let xs: Vec<f32> = input.iter().map(|&x| x as f32).collect();
            write_f32_vec(&mut out, &xs);
        }
        (DType::U8, DType::I32) => {
            let xs: Vec<i32> = input.iter().map(|&x| x as i32).collect();
            write_i32_vec(&mut out, &xs);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn f32_round_trip() {
        let mut buf = vec![0u8; 12];
        write_f32_vec(&mut buf, &[1.0, 2.0, 3.0]);
        let r = read_f32_vec(&buf, 3);
        assert_eq!(r, vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn matmul_2x2() {
        // [[1,2],[3,4]] × [[5,6],[7,8]] = [[19,22],[43,50]]
        let a = [1.0f32, 2.0, 3.0, 4.0];
        let b = [5.0f32, 6.0, 7.0, 8.0];
        let mut a_b = vec![0u8; 16];
        let mut b_b = vec![0u8; 16];
        write_f32_vec(&mut a_b, &a);
        write_f32_vec(&mut b_b, &b);
        let mut c_b = vec![0u8; 16];
        matmul_f32(&a_b, &b_b, &mut c_b, 2, 2, 2);
        let c = read_f32_vec(&c_b, 4);
        assert_eq!(c, vec![19.0, 22.0, 43.0, 50.0]);
    }

    #[test]
    fn transpose_2x3_to_3x2() {
        // [[0,1,2],[3,4,5]] → [[0,3],[1,4],[2,5]]
        let xs: [f32; 6] = [0.0, 1.0, 2.0, 3.0, 4.0, 5.0];
        let mut input = vec![0u8; 24];
        write_f32_vec(&mut input, &xs);
        let (out, out_dims) = transpose_bytes(&input, &[2, 3], &[1, 0], 4);
        assert_eq!(out_dims, vec![3, 2]);
        let r = read_f32_vec(&out, 6);
        assert_eq!(r, vec![0.0, 3.0, 1.0, 4.0, 2.0, 5.0]);
    }

    #[test]
    fn reduce_sum_along_axis() {
        // [[1,2],[3,4]] reduce sum axis=0 → [4, 6]
        let xs = [1.0f32, 2.0, 3.0, 4.0];
        let mut input = vec![0u8; 16];
        write_f32_vec(&mut input, &xs);
        let (out, out_dims) = reduce_f32(&input, &[2, 2], &[0], false, 0.0, |a, b| a + b);
        assert_eq!(out_dims, vec![2]);
        let r = read_f32_vec(&out, 2);
        assert_eq!(r, vec![4.0, 6.0]);
    }

    #[test]
    fn cast_f32_to_i32_truncates() {
        let xs = [1.7f32, -2.3, 3.9];
        let mut input = vec![0u8; 12];
        write_f32_vec(&mut input, &xs);
        let out = cast(&input, DType::F32, DType::I32, 3);
        let r = read_i32_vec(&out, 3);
        assert_eq!(r, vec![1, -2, 3]);
    }

    #[test]
    fn cast_f32_to_u8_clamps() {
        let xs = [-10.0f32, 100.5, 300.0];
        let mut input = vec![0u8; 12];
        write_f32_vec(&mut input, &xs);
        let out = cast(&input, DType::F32, DType::U8, 3);
        assert_eq!(out, vec![0u8, 100u8, 255u8]);
    }

    #[test]
    fn broadcast_1x3_to_2x3() {
        let xs = [1.0f32, 2.0, 3.0];
        let mut input = vec![0u8; 12];
        write_f32_vec(&mut input, &xs);
        let out = broadcast_bytes(&input, &[1, 3], &[2, 3], 4);
        let r = read_f32_vec(&out, 6);
        assert_eq!(r, vec![1.0, 2.0, 3.0, 1.0, 2.0, 3.0]);
    }
}
