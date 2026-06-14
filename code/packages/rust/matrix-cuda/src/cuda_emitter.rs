//! `cuda_emitter` — CUDA C string generator for specialised kernels.
//!
//! MX06 Phase 4.  Direct port of [`matrix_metal::msl_emitter`] —
//! same API surface, same constant-folding logic, same SpecKey
//! coverage — emitting CUDA C instead of MSL.  The two emitters are
//! intentionally close in shape so reviewers can diff them side by
//! side and audit that every supported MSL specialisation has a
//! CUDA peer.
//!
//! # What this module is
//!
//! Takes a [`SpecKey`] from `matrix-profile` and a 64-bit handle
//! and returns a self-contained CUDA C source string with the
//! observed information baked in.  When the profiler reports a
//! stable F32 constant `K` on the RHS of an Add, the generic
//! `add_f32(a, b, out, n)` becomes:
//!
//! ```cuda
//! extern "C" __global__ void specialised_add_const_f32_0xDEADBEEF(
//!     const float* __restrict__ a,
//!     float* __restrict__ out,
//!     unsigned int n
//! ) {
//!     unsigned int gid = blockIdx.x * blockDim.x + threadIdx.x;
//!     if (gid >= n) return;
//!     out[gid] = a[gid] + 7.0f;   // K folded
//! }
//! ```
//!
//! Just like the Metal emitter:
//! - One fewer buffer argument (no `b`).
//! - The constant is a literal float, free to be folded into ALU work.
//! - The entry-point name embeds the handle so distinct
//!   specialisations coexist without collisions.
//!
//! # Why pure code-generation
//!
//! This module touches no device — no `CudaDevice`, no NVRTC, no
//! `cuda-compute`.  It runs on every platform (including macOS) and
//! is fully unit-testable via snapshot assertions on the emitted
//! string.  Phase 5 will hand the output to `Kernels::new` (or its
//! per-handle equivalent) for actual NVRTC compilation.
//!
//! # Scope (Phase 4)
//!
//! Mirrors the MSL emitter's coverage exactly:
//!
//! | `op_kind`    | Folded constant            | folded_slot       | Notes                                            |
//! |--------------|----------------------------|-------------------|--------------------------------------------------|
//! | `0x00..=0x06`| 4 B `f32` input value      | `Some(0)`         | Unary; output precomputed (write of `f(K)`)      |
//! | `0x07` Add   | 4 B `f32` operand          | any               | Commutative — slot doesn't matter                |
//! | `0x09` Mul   | 4 B `f32` operand          | any               | Commutative                                      |
//! | `0x0B` Max   | 4 B `f32` operand          | any               | Commutative                                      |
//! | `0x0C` Min   | 4 B `f32` operand          | any               | Commutative                                      |
//! | `0x08` Sub   | 4 B `f32` operand          | `Some(0)`/`Some(1)`| Non-commutative; LHS- and RHS-folded variants    |
//! | `0x0A` Div   | 4 B `f32` operand          | `Some(0)`/`Some(1)`| Non-commutative                                  |
//! | `0x0D` Pow   | 4 B `f32` operand          | `Some(0)`/`Some(1)`| Non-commutative                                  |
//! | `0x15` MatMul| `4*N²` B `f32` matrix      | `Some(1)`         | 2×2 or 4×4 only; RHS folded                       |
//!
//! Everything else returns `None`; the runtime falls back to the
//! generic dispatch path.

use matrix_ir::DType;
use matrix_profile::{RangeClass, SpecKey};

/// A single emitted CUDA C kernel, ready to hand to NVRTC via
/// [`cuda_compute::CudaDevice::compile`].
///
/// Plain-data carrier — the emitter is a pure function, so this
/// type can be passed across thread boundaries (it owns its
/// `String` fields).
///
/// Symmetric with `matrix_metal::EmittedKernel`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EmittedKernel {
    /// Full CUDA C source.  Self-contained — no extra headers
    /// required.  NVRTC's standard headers (`math.h` for `fmaxf`
    /// / `fminf` / `powf`) are available implicitly.
    pub source: String,

    /// Entry-point name passed to `CudaModule::function(name)`.
    /// Always embeds the 64-bit `handle` in zero-padded hex so
    /// distinct specialisations don't collide in one compiled
    /// module.
    pub entry_point: String,

    /// Number of input buffers the kernel expects.  The runtime,
    /// when routing `DispatchSpecialised`, must pass exactly this
    /// many `BufferId`s in `inputs`.
    pub input_buffer_count: usize,

    /// Number of output buffers the kernel writes.  Always 1 in
    /// Phase 4; preserved as a field so the type stays symmetric
    /// with the MSL emitter and so multi-output specialisations
    /// can be added later without changing the public type.
    pub output_buffer_count: usize,
}

/// Emit a CUDA C kernel for the given [`SpecKey`] under the given
/// 64-bit handle.  Returns `None` when Phase 4 doesn't know how to
/// specialise this shape — the runtime then falls back to generic
/// dispatch.
///
/// # Currently supported shapes
///
/// See the module-level doc-comment table.
pub fn emit_specialised_kernel(key: &SpecKey, handle: u64) -> Option<EmittedKernel> {
    if key.dtype != DType::F32 {
        return None;
    }
    let RangeClass::Constant { bytes } = &key.range_class else {
        return None;
    };

    // MatMul branches off here — its constant payload is the entire
    // RHS matrix, more than 4 bytes.
    if key.op_kind == 0x15 {
        return emit_matmul_with_folded_matrix(key, handle, bytes);
    }

    if bytes.len() != 4 {
        return None;
    }
    let arr: [u8; 4] = bytes.as_slice().try_into().ok()?;
    let constant = f32::from_le_bytes(arr);

    // Commutative binary: the same kernel works regardless of which
    // slot the policy folded.  Canonical form is `a[gid] OP K`.
    let commutative_template: Option<(&str, &str)> = match key.op_kind {
        0x07 => Some(("add", "{a} + {k}")),
        0x09 => Some(("mul", "{a} * {k}")),
        0x0B => Some(("max", "fmaxf({a}, {k})")),
        0x0C => Some(("min", "fminf({a}, {k})")),
        _ => None,
    };
    if let Some((op_name, expr_template)) = commutative_template {
        return Some(emit_binary_f32_with_rhs_const(
            handle,
            op_name,
            expr_template,
            constant,
        ));
    }

    // Non-commutative / unary: we **must** know which slot is the
    // constant.  No `folded_slot` → fall back.
    let folded_slot = key.folded_slot?;

    // Unary f32 ops with a folded input constant: the whole output
    // is precomputed to `f(K)`.  The kernel is a memset that
    // writes the literal value at every position — no input
    // buffer needed.
    let unary_eval: Option<(&str, f32)> = match key.op_kind {
        0x00 if folded_slot == 0 => Some(("neg", -constant)),
        0x01 if folded_slot == 0 => Some(("abs", constant.abs())),
        0x02 if folded_slot == 0 => Some(("sqrt", constant.sqrt())),
        0x03 if folded_slot == 0 => Some(("exp", constant.exp())),
        0x04 if folded_slot == 0 => Some(("log", constant.ln())),
        0x05 if folded_slot == 0 => Some(("tanh", constant.tanh())),
        0x06 if folded_slot == 0 => Some(("recip", 1.0_f32 / constant)),
        _ => None,
    };
    if let Some((op_name, precomputed)) = unary_eval {
        return Some(emit_unary_f32_folded_constant(handle, op_name, precomputed));
    }

    // Non-commutative binary: one of Sub / Div / Pow.  The LHS and
    // RHS variants differ in operand order — the emitter encodes
    // the choice into both the source body and the entry-point name.
    let (op_name, lhs_template, rhs_template) = match key.op_kind {
        0x08 => ("sub", "{k} - {a}", "{a} - {k}"),
        0x0A => ("div", "{k} / {a}", "{a} / {k}"),
        0x0D => ("pow", "powf({k}, {a})", "powf({a}, {k})"),
        _ => return None,
    };
    Some(emit_binary_f32_with_const_at_slot(
        handle,
        op_name,
        lhs_template,
        rhs_template,
        constant,
        folded_slot,
    ))
}

/// Emit a commutative-binary-f32-with-folded-constant kernel.
///
/// Entry-point name pattern:
///
/// ```text
/// specialised_<op_name>_const_f32_0xHHHHHHHHHHHHHHHH
/// ```
///
/// `expr_template` is the body fragment after `out[gid] = `, with
/// `{a}` substituted by `a[gid]` and `{k}` by the formatted
/// float literal.
fn emit_binary_f32_with_rhs_const(
    handle: u64,
    op_name: &str,
    expr_template: &str,
    constant: f32,
) -> EmittedKernel {
    let entry = format!("specialised_{op_name}_const_f32_0x{handle:016X}");
    let literal = format_f32_literal(constant);
    let body_expr = expr_template
        .replace("{a}", "a[gid]")
        .replace("{k}", &literal);

    let source = format!(
        "// MX06 Phase 4 — specialised {op_name}_f32 with folded constant {literal}.\n\
         // handle = 0x{handle:016X}\n\
         extern \"C\" __global__ void {entry}(\n\
         \x20   const float* __restrict__ a,\n\
         \x20   float* __restrict__ out,\n\
         \x20   unsigned int n\n\
         ) {{\n\
         \x20   unsigned int gid = blockIdx.x * blockDim.x + threadIdx.x;\n\
         \x20   if (gid >= n) return;\n\
         \x20   out[gid] = {body_expr};\n\
         }}\n",
        handle = handle,
        entry = entry,
        op_name = op_name,
        literal = literal,
        body_expr = body_expr,
    );

    EmittedKernel {
        source,
        entry_point: entry,
        input_buffer_count: 1,
        output_buffer_count: 1,
    }
}

/// Emit a non-commutative-binary-f32 kernel where the folded
/// constant lives on the LHS or the RHS, chosen by `folded_slot`.
///
/// Entry-point name pattern:
///
/// ```text
/// specialised_<op_name>_<lhs|rhs>_const_f32_0xHHHHHHHHHHHHHHHH
/// ```
///
/// `lhs_template` and `rhs_template` are the body fragments for
/// the two slot variants.  An out-of-range `folded_slot` falls back
/// to the RHS shape but renames the entry-point to `_unknown_` so
/// reviewers can spot the anomaly in any compiled-module listing.
/// This mirrors the MSL emitter's "soft" handling exactly.
fn emit_binary_f32_with_const_at_slot(
    handle: u64,
    op_name: &str,
    lhs_template: &str,
    rhs_template: &str,
    constant: f32,
    folded_slot: u8,
) -> EmittedKernel {
    let (variant_name, expr_template): (&str, &str) = match folded_slot {
        0 => ("lhs", lhs_template),
        1 => ("rhs", rhs_template),
        _ => ("unknown", rhs_template),
    };

    let entry = format!("specialised_{op_name}_{variant_name}_const_f32_0x{handle:016X}");
    let literal = format_f32_literal(constant);
    let body_expr = expr_template
        .replace("{a}", "a[gid]")
        .replace("{k}", &literal);

    let source = format!(
        "// MX06 Phase 4 — specialised {op_name}_f32 with folded {variant_name} constant {literal}.\n\
         // handle = 0x{handle:016X}\n\
         extern \"C\" __global__ void {entry}(\n\
         \x20   const float* __restrict__ a,\n\
         \x20   float* __restrict__ out,\n\
         \x20   unsigned int n\n\
         ) {{\n\
         \x20   unsigned int gid = blockIdx.x * blockDim.x + threadIdx.x;\n\
         \x20   if (gid >= n) return;\n\
         \x20   out[gid] = {body_expr};\n\
         }}\n",
        handle = handle,
        entry = entry,
        op_name = op_name,
        variant_name = variant_name,
        literal = literal,
        body_expr = body_expr,
    );

    EmittedKernel {
        source,
        entry_point: entry,
        input_buffer_count: 1,
        output_buffer_count: 1,
    }
}

/// Emit a `Op::MatMul(A[m, dim] × B[dim, dim] = C[m, dim])` kernel
/// where the entire RHS matrix is baked into the source as float
/// literals.
///
/// V1 supports `dim ∈ {2, 4}` with `folded_slot = Some(1)` (RHS
/// folded — the common case: variable input times a stable
/// transform).  16-element (4×4) is the upper bound because the
/// emitted source grows quadratically with `dim`.
fn emit_matmul_with_folded_matrix(
    key: &SpecKey,
    handle: u64,
    bytes: &[u8],
) -> Option<EmittedKernel> {
    if key.folded_slot != Some(1) {
        return None;
    }
    if bytes.len() % 4 != 0 {
        return None;
    }
    let n_floats = bytes.len() / 4;
    let dim = match n_floats {
        4 => 2usize,
        16 => 4usize,
        _ => return None,
    };
    let matrix: Vec<f32> = bytes
        .chunks(4)
        .map(|c| {
            let arr: [u8; 4] = c.try_into().unwrap();
            f32::from_le_bytes(arr)
        })
        .collect();

    // Build the `if (c == X)` arms.  For `dim`-by-`dim` matrix, the
    // output element at (r, c) is `sum_k a[r * dim + k] * B[k, c]`.
    let mut col_arms = String::new();
    for c in 0..dim {
        let mut sum_terms = Vec::with_capacity(dim);
        for k in 0..dim {
            let b_val = matrix[k * dim + c];
            let lit = format_f32_literal(b_val);
            sum_terms.push(format!("a[r * {dim} + {k}] * {lit}"));
        }
        let body = sum_terms.join(" + ");
        col_arms.push_str(&format!(
            "        if (c == {c}) {{ out[gid] = {body}; return; }}\n"
        ));
    }

    let entry = format!(
        "specialised_matmul_{dim}x{dim}_rhs_const_f32_0x{handle:016X}"
    );

    let source = format!(
        "// MX06 Phase 4 — specialised {dim}x{dim} matmul with RHS matrix folded.\n\
         // handle = 0x{handle:016X}\n\
         // Output element count `n = m * {dim}` — passed in as a uniform.\n\
         extern \"C\" __global__ void {entry}(\n\
         \x20   const float* __restrict__ a,\n\
         \x20   float* __restrict__ out,\n\
         \x20   unsigned int n\n\
         ) {{\n\
         \x20   unsigned int gid = blockIdx.x * blockDim.x + threadIdx.x;\n\
         \x20   if (gid >= n) return;\n\
         \x20   unsigned int r = gid / {dim};\n\
         \x20   unsigned int c = gid % {dim};\n\
         {col_arms}\
         }}\n",
        dim = dim,
        handle = handle,
        entry = entry,
        col_arms = col_arms,
    );

    Some(EmittedKernel {
        source,
        entry_point: entry,
        input_buffer_count: 1,
        output_buffer_count: 1,
    })
}

/// Emit a unary-f32 kernel whose single input is itself a constant —
/// the output is `f(K)` at every position.
///
/// Kernel signature:
/// - 0 inputs (folded away — the runtime passes an empty `inputs`)
/// - 1 output buffer
/// - `n` element count
///
/// Entry-point: `specialised_<op_name>_input_const_f32_0xHHHH…`.
fn emit_unary_f32_folded_constant(handle: u64, op_name: &str, precomputed: f32) -> EmittedKernel {
    let entry = format!("specialised_{op_name}_input_const_f32_0x{handle:016X}");
    let literal = format_f32_literal(precomputed);

    let source = format!(
        "// MX06 Phase 4 — specialised {op_name}_f32 with folded input.\n\
         // handle = 0x{handle:016X}\n\
         // precomputed = {literal}\n\
         extern \"C\" __global__ void {entry}(\n\
         \x20   float* __restrict__ out,\n\
         \x20   unsigned int n\n\
         ) {{\n\
         \x20   unsigned int gid = blockIdx.x * blockDim.x + threadIdx.x;\n\
         \x20   if (gid >= n) return;\n\
         \x20   out[gid] = {literal};\n\
         }}\n",
        handle = handle,
        entry = entry,
        op_name = op_name,
        literal = literal,
    );

    EmittedKernel {
        source,
        entry_point: entry,
        input_buffer_count: 0,
        output_buffer_count: 1,
    }
}

/// Format an `f32` as a CUDA-C-friendly float literal.
///
/// - Uses Rust's default `Display` (Ryu-based) which emits the
///   shortest decimal that round-trips bit-exactly to the same `f32`.
/// - Appends `.0` if no decimal / exponent is present so the literal
///   parses as `float`, not `int`.
/// - Always suffixes `f` so the literal is single-precision.
/// - Handles non-finite values explicitly: NaN becomes
///   `__int_as_float(0x7fc00000)` (the standard quiet NaN bit
///   pattern); ±inf become `__int_as_float(0x7f800000)` and
///   `__int_as_float(0xff800000)`.  These intrinsics are part of
///   CUDA's built-in API and don't require any header.
///
/// `pub(crate)` so future emitters in this module (or sibling
/// modules) can reuse the same formatting and stay bit-identical.
pub(crate) fn format_f32_literal(v: f32) -> String {
    if v.is_nan() {
        // Quiet NaN with the standard payload.  __int_as_float is a
        // CUDA built-in (no header required by NVRTC).
        "__int_as_float(0x7fc00000)".to_string()
    } else if v.is_infinite() {
        if v.is_sign_negative() {
            "__int_as_float(0xff800000)".to_string()
        } else {
            "__int_as_float(0x7f800000)".to_string()
        }
    } else {
        let mut s = format!("{}", v);
        if !s.contains('.') && !s.contains('e') && !s.contains('E') {
            s.push_str(".0");
        }
        s.push('f');
        s
    }
}

// ────────────────────────── tests ──────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use matrix_profile::{RangeClass, ShapeClass, SpecKey};

    fn add_const_key(constant: f32) -> SpecKey {
        SpecKey {
            op_kind: 0x07, // Op::Add
            dtype: DType::F32,
            shape_class: ShapeClass::Dynamic,
            range_class: RangeClass::Constant {
                bytes: constant.to_le_bytes().to_vec(),
            },
            backend_id: 2, // cuda
            folded_slot: Some(1),
        }
    }

    fn binary_const_key_slot(op_kind: u8, constant: f32, folded_slot: u8) -> SpecKey {
        SpecKey {
            op_kind,
            dtype: DType::F32,
            shape_class: ShapeClass::Dynamic,
            range_class: RangeClass::Constant {
                bytes: constant.to_le_bytes().to_vec(),
            },
            backend_id: 2,
            folded_slot: Some(folded_slot),
        }
    }

    fn unary_const_key(op_kind: u8, constant: f32) -> SpecKey {
        SpecKey {
            op_kind,
            dtype: DType::F32,
            shape_class: ShapeClass::Dynamic,
            range_class: RangeClass::Constant {
                bytes: constant.to_le_bytes().to_vec(),
            },
            backend_id: 2,
            folded_slot: Some(0),
        }
    }

    // ── happy-path snapshots ────────────────────────────────────────

    #[test]
    fn add_f32_with_constant_emits_kernel() {
        let k = emit_specialised_kernel(&add_const_key(7.0), 0xDEAD_BEEF_DEAD_BEEF).unwrap();
        assert_eq!(
            k.entry_point,
            "specialised_add_const_f32_0xDEADBEEFDEADBEEF"
        );
        assert_eq!(k.input_buffer_count, 1);
        assert_eq!(k.output_buffer_count, 1);
        assert!(k.source.contains("extern \"C\" __global__ void"));
        assert!(k.source.contains("a[gid] + 7.0f"));
        // No second input buffer — the whole point of folding.
        assert!(
            !k.source.contains("const float* __restrict__ b"),
            "specialised add_const must drop the second input buffer"
        );
        // CUDA kernels do not include MSL headers.
        assert!(!k.source.contains("metal_stdlib"));
    }

    #[test]
    fn sub_f32_lhs_const_uses_correct_operand_order() {
        let key = binary_const_key_slot(0x08, 3.5, 0);
        let k = emit_specialised_kernel(&key, 0xABCDEF12).unwrap();
        assert!(k.entry_point.contains("_sub_lhs_const_f32_"));
        // LHS constant: K - a[gid].
        assert!(k.source.contains("3.5f - a[gid]"));
    }

    #[test]
    fn sub_f32_rhs_const_uses_correct_operand_order() {
        let key = binary_const_key_slot(0x08, 3.5, 1);
        let k = emit_specialised_kernel(&key, 0xABCDEF12).unwrap();
        assert!(k.entry_point.contains("_sub_rhs_const_f32_"));
        assert!(k.source.contains("a[gid] - 3.5f"));
    }

    #[test]
    fn div_f32_rhs_const_emits_div() {
        let key = binary_const_key_slot(0x0A, 2.0, 1);
        let k = emit_specialised_kernel(&key, 0xCAFE).unwrap();
        assert!(k.source.contains("a[gid] / 2.0f"));
    }

    #[test]
    fn pow_f32_rhs_const_uses_powf() {
        let key = binary_const_key_slot(0x0D, 2.0, 1);
        let k = emit_specialised_kernel(&key, 0xFADE).unwrap();
        assert!(k.source.contains("powf(a[gid], 2.0f)"));
    }

    #[test]
    fn max_f32_emits_fmaxf() {
        let k = emit_specialised_kernel(&binary_const_key_slot(0x0B, 1.0, 1), 0).unwrap();
        assert!(k.source.contains("fmaxf(a[gid], 1.0f)"));
    }

    #[test]
    fn min_f32_emits_fminf() {
        let k = emit_specialised_kernel(&binary_const_key_slot(0x0C, 1.0, 1), 0).unwrap();
        assert!(k.source.contains("fminf(a[gid], 1.0f)"));
    }

    #[test]
    fn mul_f32_commutative_no_slot_required() {
        // Commutative ops shouldn't require folded_slot to be set
        // sensibly — we emit the canonical RHS form regardless.
        let key = binary_const_key_slot(0x09, 4.0, 0);
        let k = emit_specialised_kernel(&key, 0).unwrap();
        assert!(k.source.contains("a[gid] * 4.0f"));
        assert_eq!(k.entry_point, "specialised_mul_const_f32_0x0000000000000000");
    }

    // ── unary folded-input precomputed memset ────────────────────────

    #[test]
    fn unary_neg_f32_emits_memset_of_precomputed_value() {
        let k = emit_specialised_kernel(&unary_const_key(0x00, 2.5), 1).unwrap();
        assert_eq!(k.input_buffer_count, 0);
        assert_eq!(k.output_buffer_count, 1);
        assert!(k.entry_point.contains("_neg_input_const_f32_"));
        // out[gid] = -2.5f (precomputed)
        assert!(k.source.contains("out[gid] = -2.5f"));
    }

    #[test]
    fn unary_abs_f32_precomputes_abs() {
        let k = emit_specialised_kernel(&unary_const_key(0x01, -3.0), 1).unwrap();
        assert!(k.source.contains("out[gid] = 3.0f"));
    }

    #[test]
    fn unary_sqrt_f32_precomputes_sqrt() {
        let k = emit_specialised_kernel(&unary_const_key(0x02, 4.0), 1).unwrap();
        assert!(k.source.contains("out[gid] = 2.0f"));
    }

    #[test]
    fn unary_recip_f32_precomputes_one_over() {
        let k = emit_specialised_kernel(&unary_const_key(0x06, 4.0), 1).unwrap();
        assert!(k.source.contains("out[gid] = 0.25f"));
    }

    // ── matmul with folded RHS matrix ───────────────────────────────

    #[test]
    fn matmul_2x2_rhs_const_emits_per_column_dotproducts() {
        // RHS = [[1, 2], [3, 4]] in row-major.
        let rhs: [f32; 4] = [1.0, 2.0, 3.0, 4.0];
        let bytes: Vec<u8> = rhs.iter().flat_map(|f| f.to_le_bytes()).collect();
        let key = SpecKey {
            op_kind: 0x15,
            dtype: DType::F32,
            shape_class: ShapeClass::Dynamic,
            range_class: RangeClass::Constant { bytes },
            backend_id: 2,
            folded_slot: Some(1),
        };
        let k = emit_specialised_kernel(&key, 0x42).unwrap();
        assert!(k.entry_point.contains("specialised_matmul_2x2_rhs_const_f32_"));
        // Column 0 should pick rhs[0]=1, rhs[2]=3: a[r*2+0]*1.0f + a[r*2+1]*3.0f.
        assert!(k.source.contains("a[r * 2 + 0] * 1.0f + a[r * 2 + 1] * 3.0f"));
        // Column 1: a[r*2+0]*2.0f + a[r*2+1]*4.0f.
        assert!(k.source.contains("a[r * 2 + 0] * 2.0f + a[r * 2 + 1] * 4.0f"));
    }

    #[test]
    fn matmul_4x4_rhs_const_supported() {
        let rhs: [f32; 16] = [0.0; 16];
        let bytes: Vec<u8> = rhs.iter().flat_map(|f| f.to_le_bytes()).collect();
        let key = SpecKey {
            op_kind: 0x15,
            dtype: DType::F32,
            shape_class: ShapeClass::Dynamic,
            range_class: RangeClass::Constant { bytes },
            backend_id: 2,
            folded_slot: Some(1),
        };
        let k = emit_specialised_kernel(&key, 0).unwrap();
        assert!(k.entry_point.contains("matmul_4x4_rhs_const"));
    }

    #[test]
    fn matmul_unsupported_dim_returns_none() {
        // 3×3 (9 floats) is not in the supported set.
        let bytes = vec![0u8; 36];
        let key = SpecKey {
            op_kind: 0x15,
            dtype: DType::F32,
            shape_class: ShapeClass::Dynamic,
            range_class: RangeClass::Constant { bytes },
            backend_id: 2,
            folded_slot: Some(1),
        };
        assert!(emit_specialised_kernel(&key, 0).is_none());
    }

    #[test]
    fn matmul_lhs_folded_falls_back_to_none() {
        let bytes = vec![0u8; 16];
        let key = SpecKey {
            op_kind: 0x15,
            dtype: DType::F32,
            shape_class: ShapeClass::Dynamic,
            range_class: RangeClass::Constant { bytes },
            backend_id: 2,
            folded_slot: Some(0),
        };
        // LHS-folded matmul is intentionally Phase 4 not-supported.
        assert!(emit_specialised_kernel(&key, 0).is_none());
    }

    // ── fall-through cases ──────────────────────────────────────────

    #[test]
    fn non_f32_returns_none() {
        let key = SpecKey {
            op_kind: 0x07,
            dtype: DType::U8,
            shape_class: ShapeClass::Dynamic,
            range_class: RangeClass::Constant {
                bytes: 7.0f32.to_le_bytes().to_vec(),
            },
            backend_id: 2,
            folded_slot: Some(1),
        };
        assert!(emit_specialised_kernel(&key, 0).is_none());
    }

    #[test]
    fn non_constant_range_class_returns_none() {
        let key = SpecKey {
            op_kind: 0x07,
            dtype: DType::F32,
            shape_class: ShapeClass::Dynamic,
            range_class: RangeClass::FloatBits {
                min_bits: 0,
                max_bits: 0,
            },
            backend_id: 2,
            folded_slot: Some(1),
        };
        assert!(emit_specialised_kernel(&key, 0).is_none());
    }

    #[test]
    fn unsupported_op_kind_returns_none() {
        // 0x0E is ReduceSum — not in the Phase 4 set.
        let key = binary_const_key_slot(0x0E, 0.0, 1);
        assert!(emit_specialised_kernel(&key, 0).is_none());
    }

    #[test]
    fn non_commutative_without_folded_slot_returns_none() {
        let key = SpecKey {
            op_kind: 0x08, // Sub
            dtype: DType::F32,
            shape_class: ShapeClass::Dynamic,
            range_class: RangeClass::Constant {
                bytes: 1.0_f32.to_le_bytes().to_vec(),
            },
            backend_id: 2,
            folded_slot: None,
        };
        assert!(emit_specialised_kernel(&key, 0).is_none());
    }

    #[test]
    fn three_byte_constant_returns_none() {
        // Constant payload must be exactly 4 bytes (one f32) for the
        // binary / unary paths.  3 bytes is malformed.
        let key = SpecKey {
            op_kind: 0x07,
            dtype: DType::F32,
            shape_class: ShapeClass::Dynamic,
            range_class: RangeClass::Constant { bytes: vec![0; 3] },
            backend_id: 2,
            folded_slot: Some(1),
        };
        assert!(emit_specialised_kernel(&key, 0).is_none());
    }

    // ── float-literal formatting ────────────────────────────────────

    #[test]
    fn format_f32_literal_round_trips_with_f_suffix() {
        assert_eq!(format_f32_literal(7.0), "7.0f");
        assert_eq!(format_f32_literal(2.5), "2.5f");
        assert_eq!(format_f32_literal(-0.0), "-0.0f");
        // Rust's default Display picks the shortest round-trip
        // decimal, which is "0.1" for 0.1_f32 (the closest f32 to
        // 1/10 prints as "0.1" via Ryu, not as e.g. 0.10000000149...).
        assert_eq!(format_f32_literal(0.1_f32), "0.1f");
    }

    #[test]
    fn format_f32_literal_handles_nan_and_inf() {
        assert_eq!(format_f32_literal(f32::NAN), "__int_as_float(0x7fc00000)");
        assert_eq!(
            format_f32_literal(f32::INFINITY),
            "__int_as_float(0x7f800000)"
        );
        assert_eq!(
            format_f32_literal(f32::NEG_INFINITY),
            "__int_as_float(0xff800000)"
        );
    }

    // ── entry-point name handle encoding ────────────────────────────

    #[test]
    fn handle_encoded_zero_padded_uppercase_hex() {
        let k = emit_specialised_kernel(&add_const_key(1.0), 0x1).unwrap();
        // 0x1 should pad out to 16 hex digits, all uppercase.
        assert!(
            k.entry_point.ends_with("0x0000000000000001"),
            "got entry_point = {}",
            k.entry_point
        );
    }
}
