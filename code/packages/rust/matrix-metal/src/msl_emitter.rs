//! `msl_emitter` — the **MSL string generator** for specialised kernels.
//!
//! # MX05 Phase 4.2 — what this module is
//!
//! Matrix-cpu's Phase 4.1 closed the loop on the CPU side by routing
//! `DispatchSpecialised { handle, .. }` to an installed closure.  That
//! works because Rust closures are first-class — you build one
//! in-process, hand it to the executor, and the executor calls it.
//!
//! For GPU backends the loop is more interesting: there's no Rust
//! closure that runs on the GPU.  A specialised GPU "kernel" *is*
//! the source code (MSL on Apple, PTX on NVIDIA, SPIR-V on Vulkan)
//! that the device driver compiles and runs.  Specialisation
//! becomes a **code-generation** problem: take a [`SpecKey`] —
//! which carries everything the profiler observed about the op
//! (op kind, dtype, shape class, **range class including constants**)
//! — and emit a tailored kernel string where the observed
//! information has been baked in.
//!
//! This module is the code generator.  It runs on every platform
//! (including the Ubuntu and Windows CI runners that have no Metal
//! device at all) because emitting a string requires no device.
//! The string-handling tests in this module are the **only**
//! Phase 4.2 verification that runs everywhere; everything else
//! (compile, dispatch) is gated on `cfg(target_vendor = "apple")`.
//!
//! # What "constants folded in" means
//!
//! The generic add_f32 kernel ships in [`crate::kernels`] as:
//!
//! ```msl
//! kernel void add_f32(
//!     device const float* a,
//!     device const float* b,
//!     device float* out,
//!     constant uint& n,
//!     uint gid)
//! {
//!     if (gid >= n) return;
//!     out[gid] = a[gid] + b[gid];
//! }
//! ```
//!
//! When the profiler reports that the right-hand operand has been
//! the same `f32` value `K` across enough invocations to satisfy
//! the policy, the cache emits a specialised key.  This emitter
//! produces:
//!
//! ```msl
//! kernel void specialised_add_const_f32_0xDEADBEEF(
//!     device const float* a,
//!     device float* out,
//!     constant uint& n,
//!     uint gid)
//! {
//!     if (gid >= n) return;
//!     out[gid] = a[gid] + 7.000000f;   // ← K baked in
//! }
//! ```
//!
//! Notice:
//! - One fewer buffer argument (no `b`).  The runtime, when routing
//!   through `DispatchSpecialised`, only uploads `a`.
//! - The constant is a literal float in the source.  When the
//!   Metal driver compiles this it can fold the addition into other
//!   ALU work and skip the second memory read entirely.
//! - The entry-point name embeds the **handle** so distinct
//!   specialisations can coexist in their own compiled libraries
//!   without name collisions.
//!
//! # V0.6.0 minimum-viable scope
//!
//! Supports exactly one specialisation pattern:
//!
//! - **F32 elementwise binary `Add` with a 4-byte (`f32`) RHS constant.**
//!   (op_kind = `0x07`, dtype = F32, range_class = `Constant { bytes:
//!   [b0, b1, b2, b3] }`.)
//!
//! All other [`SpecKey`] shapes return `None` so the runtime falls
//! back to either matrix-cpu's specialised path (different backend
//! handle) or the generic `Dispatch` path.
//!
//! Phase 4.3 will extend the emitter to `Sub`/`Mul`/`Div`, then to
//! unary ops with folded constants (rare but possible — e.g. an
//! op whose only input is a known constant becomes a pure write),
//! then to `MatMul` with one matrix folded into a literal table.
//!
//! # Why a string, not a syntax tree
//!
//! MSL has no convenient Rust crate.  Writing an AST type would be
//! a multi-thousand-line investment for the marginal benefit over
//! `format!` — and we'd still emit strings at the end.  The string
//! approach lets us snapshot-test the emitter output exactly and
//! audit the MSL by reading the source.

use matrix_ir::DType;
use matrix_profile::{RangeClass, SpecKey};

/// A single emitted MSL kernel, ready to hand to
/// [`metal_compute::MetalDevice::compile`].
///
/// Held as plain data so the emitter is a pure function: caller
/// receives the string and decides when (or whether) to compile it.
/// This is what lets the emitter work on non-Apple targets — the
/// string is just bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EmittedKernel {
    /// Full MSL source.  Self-contained: includes
    /// `#include <metal_stdlib>` and `using namespace metal;` so the
    /// caller can compile it directly without prepending anything.
    pub source: String,

    /// Entry-point name to pass to `MetalDevice::function(name)`.
    /// Always embeds the 64-bit `handle` in hex so different
    /// specialisations of the same op coexist.
    pub entry_point: String,

    /// Number of `device const T*` input buffers the kernel expects.
    /// The runtime, when routing `DispatchSpecialised`, must pass
    /// exactly this many `BufferId`s in `inputs`.
    pub input_buffer_count: usize,

    /// Number of `device T*` output buffers the kernel produces.
    /// The runtime must pass exactly this many `BufferId`s in
    /// `outputs`.
    pub output_buffer_count: usize,
}

/// Emit an MSL kernel for the given [`SpecKey`] under the given
/// 64-bit handle.  Returns `None` if Phase 4.2 doesn't know how to
/// specialise this shape.
///
/// The handle is woven into the entry-point name so multiple
/// specialisations don't collide if a backend compiles them into the
/// same library — though in practice each emitted kernel is compiled
/// into its own one-function library, and the runtime tracks which
/// `MetalComputePipeline` belongs to which handle in a separate
/// `HashMap`.
///
/// # Returns
///
/// - `Some(EmittedKernel)` — Phase 4.2 understands this shape.
/// - `None` — fall back to the generic dispatch path.
///
/// # Currently supported shapes
///
/// | `op_kind`    | `dtype` | `shape_class` | `range_class`              | Notes                                            |
/// |--------------|---------|---------------|----------------------------|--------------------------------------------------|
/// | `0x00` Neg   | `F32`   | any           | `Constant { bytes: 4 B }`  | Unary; output precomputed (memset of `-K`)       |
/// | `0x01` Abs   | `F32`   | any           | `Constant { bytes: 4 B }`  | Unary; output precomputed (memset of `\|K\|`)      |
/// | `0x02` Sqrt  | `F32`   | any           | `Constant { bytes: 4 B }`  | Unary; output precomputed (memset of `√K`)       |
/// | `0x03` Exp   | `F32`   | any           | `Constant { bytes: 4 B }`  | Unary; output precomputed (memset of `e^K`)      |
/// | `0x04` Log   | `F32`   | any           | `Constant { bytes: 4 B }`  | Unary; output precomputed (memset of `ln K`)     |
/// | `0x05` Tanh  | `F32`   | any           | `Constant { bytes: 4 B }`  | Unary; output precomputed (memset of `tanh K`)   |
/// | `0x06` Recip | `F32`   | any           | `Constant { bytes: 4 B }`  | Unary; output precomputed (memset of `1/K`)      |
/// | `0x07` Add   | `F32`   | any           | `Constant { bytes: 4 B }`  | Commutative; slot irrelevant                     |
/// | `0x08` Sub   | `F32`   | any           | `Constant { bytes: 4 B }`  | Non-commutative; LHS- and RHS-folded variants    |
/// | `0x09` Mul   | `F32`   | any           | `Constant { bytes: 4 B }`  | Commutative; slot irrelevant                     |
/// | `0x0A` Div   | `F32`   | any           | `Constant { bytes: 4 B }`  | Non-commutative; LHS- and RHS-folded variants    |
/// | `0x0B` Max   | `F32`   | any           | `Constant { bytes: 4 B }`  | Commutative; slot irrelevant                     |
/// | `0x0C` Min   | `F32`   | any           | `Constant { bytes: 4 B }`  | Commutative; slot irrelevant                     |
/// | `0x0D` Pow   | `F32`   | any           | `Constant { bytes: 4 B }`  | Non-commutative; LHS- and RHS-folded variants    |
///
/// ## Unary ops with folded input constant (Phase 4.7)
///
/// When a unary op's single input is itself observed as a stable
/// constant `K`, the entire output is precomputed at emit time:
/// every element is `f(K)`.  The kernel collapses to a memset —
/// `input_buffer_count = 0`, only the output buffer is bound.
/// The dispatcher passes an empty `inputs` vector to
/// `DispatchSpecialised`.
///
/// All other combinations return `None` for now.  Adding a shape is
/// a small change — add a match arm and a unit test.
///
/// ## Non-commutative ops and `folded_slot`
///
/// `Op::Sub`, `Op::Div`, and `Op::Pow` are mathematically
/// non-commutative.  For each we emit one of two variants based on
/// `key.folded_slot`:
///
/// - `Some(0)` → **LHS** was the constant.  Kernel reads `b[gid]`
///   (the RHS input) and computes `K op b[gid]`.
/// - `Some(1)` → **RHS** was the constant.  Kernel reads `a[gid]`
///   (the LHS input) and computes `a[gid] op K`.
/// - `None` → emitter returns `None` (the policy didn't tell us
///   which side; we can't safely guess).
///
/// The entry-point names embed the variant: `specialised_sub_lhs_const_f32_…`
/// vs `specialised_sub_rhs_const_f32_…`.  The dispatcher in
/// image-gpu-core consults the SpecKey's `folded_slot` to pick which
/// IR input buffer (`inputs()[0]` or `inputs()[1]`) to pass.
pub fn emit_specialised_kernel(key: &SpecKey, handle: u64) -> Option<EmittedKernel> {
    if key.dtype != DType::F32 {
        return None;
    }
    let RangeClass::Constant { bytes } = &key.range_class else {
        return None;
    };

    // **MX05 Phase 4.10.**  Op::MatMul (wire tag 0x15) with a
    // folded **matrix** constant.  Branches off here because the
    // constant is more than 4 bytes — a flat [m_b, n_b] f32 matrix.
    // V1 supports only 2x2 (16 bytes) with `folded_slot = Some(1)`
    // (RHS folded — the common case: variable input times a stable
    // transform).
    if key.op_kind == 0x15 {
        return emit_matmul_with_folded_matrix(key, handle, bytes);
    }

    if bytes.len() != 4 {
        return None;
    }
    let arr: [u8; 4] = bytes.as_slice().try_into().ok()?;
    let constant = f32::from_le_bytes(arr);

    // Commutative ops: the same kernel works no matter which slot
    // was folded.  We use the canonical `a[gid] OP K` form.
    let commutative_template: Option<(&str, &str)> = match key.op_kind {
        0x07 => Some(("add", "{a} + {k}")),
        0x09 => Some(("mul", "{a} * {k}")),
        0x0B => Some(("max", "max({a}, {k})")),
        0x0C => Some(("min", "min({a}, {k})")),
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

    // Non-commutative ops: we **must** know which slot the policy
    // folded, otherwise we'd guess wrong half the time.  No
    // `folded_slot` → fall back to generic dispatch.
    let folded_slot = key.folded_slot?;

    // **MX05 Phase 4.7.**  Unary f32 ops with a folded input constant.
    //
    // When the single input of a unary op is itself observed as a
    // stable constant `K`, the entire output tensor is precomputed:
    // every element is `f(K)`.  The kernel collapses to a memset.
    // The kernel takes **zero** input buffers — the value is baked
    // into the source as a literal.  Output buffer count is still 1.
    //
    // For `folded_slot` we require `Some(0)` since unary ops have
    // exactly one input slot.
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

    // Non-commutative binary ops: we **must** know which slot the
    // policy folded, otherwise we'd guess wrong half the time.
    let (op_name, lhs_template, rhs_template) = match key.op_kind {
        0x08 => ("sub", "{k} - {a}", "{a} - {k}"),
        0x0A => ("div", "{k} / {a}", "{a} / {k}"),
        0x0D => ("pow", "pow({k}, {a})", "pow({a}, {k})"),
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

// ────────────────────── binary f32 with folded constant ──────────────────────

/// Emit a commutative-binary-f32-with-folded-constant kernel.  Pure
/// helper — no I/O, no globals, deterministic on
/// `(handle, op_name, expr_template, constant)`.
///
/// The emitted entry-point name follows the pattern:
///
/// ```text
/// specialised_<op_name>_const_f32_0xHHHHHHHHHHHHHHHH
/// ```
///
/// where the 16 hex digits are `handle` in big-endian zero-padded
/// hex.  This naming is documented and stable: tests assert on the
/// exact entry name.
///
/// `expr_template` is the MSL fragment that goes after `out[gid] = ` in
/// the body.  `{a}` substitutes the per-element load from input 0
/// (i.e. `a[gid]`); `{k}` substitutes the formatted constant literal.
/// For example:
///   - Add: `"{a} + {k}"`        →  `a[gid] + 7.000000000f`
///   - Mul: `"{a} * {k}"`        →  `a[gid] * 2.500000000f`
///   - Max: `"max({a}, {k})"`    →  `max(a[gid], 1.000000000f)`
///   - Min: `"min({a}, {k})"`    →  `min(a[gid], 1.000000000f)`
fn emit_binary_f32_with_rhs_const(
    handle: u64,
    op_name: &str,
    expr_template: &str,
    constant: f32,
) -> EmittedKernel {
    let entry = format!("specialised_{op_name}_const_f32_0x{handle:016X}");

    // Format the constant as a float literal with `f` suffix.  9
    // significant decimal digits is the f32 round-trip minimum per
    // IEEE-754.
    let literal = format_f32_literal(constant);

    // Build the body expression by substituting `{a}` / `{k}` in the
    // template.
    let body_expr = expr_template
        .replace("{a}", "a[gid]")
        .replace("{k}", &literal);

    let source = format!(
        "#include <metal_stdlib>\n\
         using namespace metal;\n\
         \n\
         // MX05 — specialised {op_name}_f32 with folded constant {literal}.\n\
         // handle = 0x{handle:016X}\n\
         kernel void {entry}(\n\
         \x20   device const float* a   [[buffer(0)]],\n\
         \x20   device float*       out [[buffer(1)]],\n\
         \x20   constant uint&      n   [[buffer(2)]],\n\
         \x20   uint gid [[thread_position_in_grid]]\n\
         ) {{\n\
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

/// **MX05 Phase 4.6.**  Emit a non-commutative-binary-f32 kernel where
/// the folded constant lives on either the LHS or the RHS of the
/// operator, chosen by `folded_slot`.
///
/// The two templates encode the two slot variants:
/// - `lhs_template`: used when `folded_slot == 0` (constant is LHS).
///   Example for Sub: `"{k} - {a}"`.
/// - `rhs_template`: used when `folded_slot == 1` (constant is RHS).
///   Example for Sub: `"{a} - {k}"`.
///
/// The emitted entry-point name encodes the variant:
///
/// ```text
/// specialised_<op_name>_lhs_const_f32_0xHHHHHHHHHHHHHHHH
/// specialised_<op_name>_rhs_const_f32_0xHHHHHHHHHHHHHHHH
/// ```
///
/// Two SpecKeys that differ only in `folded_slot` produce different
/// entry-point names, so they coexist in the executor's
/// `SpecialisedTable` under different handles (the CpuSpecialiser
/// hash now feeds on `folded_slot`).
///
/// Returns `None` if `folded_slot` is anything other than 0 or 1
/// (no current binary op has more than two input slots).
fn emit_binary_f32_with_const_at_slot(
    handle: u64,
    op_name: &str,
    lhs_template: &str,
    rhs_template: &str,
    constant: f32,
    folded_slot: u8,
) -> EmittedKernel {
    // Pick the right template based on which slot the policy
    // observed as constant.  `folded_slot == 0` → constant is LHS,
    // kernel computes `K op a[gid]` (we use `lhs_template`).
    // `folded_slot == 1` → constant is RHS, kernel computes
    // `a[gid] op K` (we use `rhs_template`).
    let (variant_name, expr_template): (&str, &str) = match folded_slot {
        0 => ("lhs", lhs_template),
        1 => ("rhs", rhs_template),
        // Out-of-range slot: caller's bug; produce a kernel that
        // mirrors the RHS variant to avoid panicking, but use a
        // distinct name so reviewers spot the anomaly in any
        // compiled-kernel listing.  The dispatcher's metadata
        // bookkeeping will also flag this via a count mismatch
        // before any invocation.
        _ => ("unknown", rhs_template),
    };

    let entry = format!("specialised_{op_name}_{variant_name}_const_f32_0x{handle:016X}");
    let literal = format_f32_literal(constant);
    let body_expr = expr_template
        .replace("{a}", "a[gid]")
        .replace("{k}", &literal);

    let source = format!(
        "#include <metal_stdlib>\n\
         using namespace metal;\n\
         \n\
         // MX05 Phase 4.6 — specialised {op_name}_f32 with folded {variant_name} constant {literal}.\n\
         // handle = 0x{handle:016X}\n\
         kernel void {entry}(\n\
         \x20   device const float* a   [[buffer(0)]],\n\
         \x20   device float*       out [[buffer(1)]],\n\
         \x20   constant uint&      n   [[buffer(2)]],\n\
         \x20   uint gid [[thread_position_in_grid]]\n\
         ) {{\n\
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

/// **MX05 Phase 4.10.**  Emit an MSL kernel for
/// `Op::MatMul(A[m, k] × B[k, n] = C[m, n])` where **one** of the
/// two input matrices is a stable constant baked into the kernel
/// source as literal values.
///
/// V1 supports a single shape: **2×2** with `folded_slot = Some(1)`
/// (RHS folded — the variable input `A` is `[m, 2]`, the constant
/// `B` is `[2, 2]`, and the output `C` is `[m, 2]`).  The kernel
/// fans out one thread per output element.
///
/// `m` is determined at dispatch time from the output buffer's
/// byte length (`n_elems = bytes / 4`; the kernel uses `gid >> 1`
/// for the row and `gid & 1` for the column).
///
/// Why folded RHS first: the typical workload is "multiply an
/// image's pixel matrix by a constant colour transform", where
/// the transform matrix is the RHS and is the constant.  LHS-folded
/// MatMul is a future phase — it'd need a different kernel shape
/// because `n` (the runtime dim) would be on a different axis.
///
/// Why 2×2 specifically: the bake-in approach (each constant
/// element as a float literal in the kernel) is only realistic
/// for small matrices.  16-element (4×4) is doable; bigger
/// matrices would generate huge kernels.  We hard-cap at 16
/// elements and let larger matrices fall back to generic dispatch.
fn emit_matmul_with_folded_matrix(
    key: &SpecKey,
    handle: u64,
    bytes: &[u8],
) -> Option<EmittedKernel> {
    // Only RHS-folded in V1.  LHS-folded support will land in a
    // later phase with its own kernel shape.
    if key.folded_slot != Some(1) {
        return None;
    }

    // Decode the constant matrix.  Bytes must be a multiple of 4
    // (f32) and the count must match a supported square shape.
    if bytes.len() % 4 != 0 {
        return None;
    }
    let n_floats = bytes.len() / 4;
    let dim = match n_floats {
        4 => 2usize,  // 2x2
        16 => 4usize, // 4x4
        _ => return None,
    };
    let matrix: Vec<f32> = bytes
        .chunks(4)
        .map(|c| {
            let arr: [u8; 4] = c.try_into().unwrap();
            f32::from_le_bytes(arr)
        })
        .collect();

    // Build the dot-product expression for each output column.
    // For 2×2: out[r*2 + c] = a[r*2 + 0] * B[0,c] + a[r*2 + 1] * B[1,c].
    // For 4×4: same shape, just larger sum.
    //
    // We emit a flat `if-else` chain on `c` to pick the right
    // column's worth of constants.  Could use `select(...)` for a
    // branch-free form, but for 2×2 / 4×4 the branch predictor
    // handles this trivially and the source is more readable.
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
        "#include <metal_stdlib>\n\
         using namespace metal;\n\
         \n\
         // MX05 Phase 4.10 — specialised {dim}x{dim} matmul with RHS matrix folded.\n\
         // handle = 0x{handle:016X}\n\
         // Output element count `n = m * {dim}` — passed in as a uniform.\n\
         kernel void {entry}(\n\
         \x20   device const float* a   [[buffer(0)]],\n\
         \x20   device float*       out [[buffer(1)]],\n\
         \x20   constant uint&      n   [[buffer(2)]],\n\
         \x20   uint gid [[thread_position_in_grid]]\n\
         ) {{\n\
         \x20   if (gid >= n) return;\n\
         \x20   uint r = gid / {dim};\n\
         \x20   uint c = gid % {dim};\n\
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

/// **MX05 Phase 4.7.**  Emit a unary-f32 kernel whose input is a
/// known constant, producing a memset of the precomputed value.
///
/// The output is `f(K)` for every element, where `f` is the unary
/// op (Neg / Abs / Sqrt / Exp / Log / Tanh / Recip) and `K` is the
/// observed constant input.  Since every output element is the same
/// value, the kernel is a one-line write of the precomputed literal.
///
/// Kernel signature:
/// - **0 inputs** (input was folded away)
/// - 1 output buffer at `buffer(0)`
/// - `n` element count at `buffer(1)`
///
/// Entry-point name: `specialised_<op_name>_input_const_f32_0xHHHHHHHHHHHHHHHH`.
/// The `_input_const_` fragment distinguishes these from the binary
/// `_const_` and `_lhs_const_`/`_rhs_const_` kernels in the same
/// library.
///
/// The dispatcher in image-gpu-core consults `KernelMetadata.n_in`
/// and passes an empty `inputs: vec![]` when `n_in == 0`.
fn emit_unary_f32_folded_constant(handle: u64, op_name: &str, precomputed: f32) -> EmittedKernel {
    let entry = format!("specialised_{op_name}_input_const_f32_0x{handle:016X}");
    let literal = format_f32_literal(precomputed);

    let source = format!(
        "#include <metal_stdlib>\n\
         using namespace metal;\n\
         \n\
         // MX05 Phase 4.7 — specialised {op_name}_f32 with folded input.\n\
         // handle = 0x{handle:016X}\n\
         // precomputed = {literal}\n\
         kernel void {entry}(\n\
         \x20   device float*       out [[buffer(0)]],\n\
         \x20   constant uint&      n   [[buffer(1)]],\n\
         \x20   uint gid [[thread_position_in_grid]]\n\
         ) {{\n\
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

/// Format an `f32` as a Metal-friendly float literal.
///
/// - Always emits 9 significant decimal digits (the f32 round-trip
///   minimum per IEEE-754).
/// - Always emits an `f` suffix so MSL parses as `float`, not `double`.
/// - Handles non-finite values: NaN → `NAN`, infinities → `INFINITY` /
///   `-INFINITY` (MSL exposes these macros via `<metal_stdlib>`).
///
/// Made `pub(crate)` so per-handle emitters that come in later phases
/// (Sub/Mul/Div, MatMul-with-folded-matrix) can reuse exactly the same
/// float formatting and stay bit-identical with this one.
pub(crate) fn format_f32_literal(v: f32) -> String {
    if v.is_nan() {
        // MSL exposes NAN via <metal_stdlib>.  The `f` cast is a
        // belt-and-braces measure in case the surrounding expression
        // is double-typed.
        "((float)NAN)".to_string()
    } else if v.is_infinite() {
        if v.is_sign_negative() {
            "(-INFINITY)".to_string()
        } else {
            "(INFINITY)".to_string()
        }
    } else {
        // Use Rust's default `{}` for `f32` (Ryu-based) which gives
        // the **shortest** decimal that round-trips bit-exactly back
        // to the same `f32`.  `{:.9}` was tempting for snapshot
        // stability, but it truncates very-small magnitudes (e.g.
        // `-1e-30` rounded to 9 fractional digits is plain `0.0`),
        // which breaks the round-trip invariant.  The IEEE-754
        // round-trip minimum for f32 is 9 *significant* digits, and
        // Ryu always emits at least that many when needed.
        //
        // Rust's `Display` for `f32` always includes a decimal point
        // (e.g. `7` is emitted as `7`, not `7.`), but to be 100% sure
        // MSL parses the result as `float` rather than `int` we
        // append a `.0` whenever the formatted form has no decimal
        // point, then suffix `f`.
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

    /// Build a SpecKey for the canonical "F32 add with RHS constant"
    /// specialisation.
    fn add_const_key(constant: f32) -> SpecKey {
        SpecKey {
            op_kind: 0x07, // Op::Add
            dtype: DType::F32,
            shape_class: ShapeClass::Dynamic,
            range_class: RangeClass::Constant {
                bytes: constant.to_le_bytes().to_vec(),
            },
            backend_id: 1, // metal
            // Commutative — slot doesn't matter for the emitted
            // kernel, but the field has to be set on every SpecKey
            // now (Phase 4.6).  `Some(1)` mirrors what the policy
            // would record for the canonical "RHS is the constant"
            // arrangement.
            folded_slot: Some(1),
        }
    }

    #[test]
    fn add_f32_with_constant_emits_kernel() {
        let k = emit_specialised_kernel(&add_const_key(7.0), 0xDEAD_BEEF_DEAD_BEEF).unwrap();
        assert_eq!(
            k.entry_point,
            "specialised_add_const_f32_0xDEADBEEFDEADBEEF"
        );
        assert_eq!(k.input_buffer_count, 1);
        assert_eq!(k.output_buffer_count, 1);
        assert!(k.source.contains("#include <metal_stdlib>"));
        assert!(k.source.contains("kernel void specialised_add_const_f32_"));
        // The constant must literally appear in the kernel body.
        // Ryu emits the shortest round-trip form so 7.0_f32 → "7.0".
        assert!(
            k.source.contains("7.0f"),
            "kernel must contain folded constant; got source:\n{}",
            k.source
        );
        // The kernel must NOT take a second input buffer — that's the
        // whole point of folding the constant in.
        assert!(
            !k.source.contains("device const float* b"),
            "specialised add_const must drop the second input buffer"
        );
    }

    /// Build a SpecKey for any of the commutative binary f32 ops.
    /// Convenience wrapper for the Phase 4.5 tests below.  Phase 4.6
    /// added `folded_slot`; the wrapper defaults to `Some(1)` since
    /// most policy traces fold the RHS, but tests that exercise
    /// non-commutative ops override this via [`binary_const_key_slot`].
    fn binary_const_key(op_kind: u8, constant: f32) -> SpecKey {
        binary_const_key_slot(op_kind, constant, 1)
    }

    /// **MX05 Phase 4.6.**  Build a SpecKey with an explicit
    /// `folded_slot` so tests can exercise the LHS-folded vs
    /// RHS-folded variants of non-commutative ops (Sub/Div/Pow).
    fn binary_const_key_slot(op_kind: u8, constant: f32, folded_slot: u8) -> SpecKey {
        SpecKey {
            op_kind,
            dtype: DType::F32,
            shape_class: ShapeClass::Dynamic,
            range_class: RangeClass::Constant {
                bytes: constant.to_le_bytes().to_vec(),
            },
            backend_id: 1,
            folded_slot: Some(folded_slot),
        }
    }

    /// **MX05 Phase 4.5.**  Op::Mul (wire tag 0x09) with folded RHS
    /// constant must emit a kernel whose body multiplies the input
    /// by the literal.
    #[test]
    fn mul_f32_with_constant_emits_kernel() {
        let k = emit_specialised_kernel(&binary_const_key(0x09, 2.5), 0xAA).unwrap();
        assert_eq!(k.entry_point, "specialised_mul_const_f32_0x00000000000000AA");
        assert_eq!(k.input_buffer_count, 1);
        assert_eq!(k.output_buffer_count, 1);
        assert!(k.source.contains("a[gid] * "));
        assert!(k.source.contains("2.5f"), "expected '2.5f' in:\n{}", k.source);
        assert!(!k.source.contains("device const float* b"));
    }

    /// **MX05 Phase 4.5.**  Op::Max (wire tag 0x0B) with folded RHS
    /// constant must emit `max(a[gid], K)` — MSL's `max` is the
    /// element-wise maximum on `float`.
    #[test]
    fn max_f32_with_constant_emits_kernel() {
        let k = emit_specialised_kernel(&binary_const_key(0x0B, -1.0), 0xBB).unwrap();
        assert_eq!(k.entry_point, "specialised_max_const_f32_0x00000000000000BB");
        assert_eq!(k.input_buffer_count, 1);
        assert!(k.source.contains("max(a[gid], "));
        // `-1.0` formats via Ryu as `-1` → emitter appends `.0` and
        // suffix `f`, giving `-1.0f`.
        assert!(k.source.contains("-1.0f"));
        assert!(!k.source.contains("device const float* b"));
    }

    /// **MX05 Phase 4.5.**  Op::Min (wire tag 0x0C) with folded RHS
    /// constant must emit `min(a[gid], K)`.  Common use case: clamp
    /// an unbounded value to a known ceiling.
    #[test]
    fn min_f32_with_constant_emits_kernel() {
        let k = emit_specialised_kernel(&binary_const_key(0x0C, 255.0), 0xCC).unwrap();
        assert_eq!(k.entry_point, "specialised_min_const_f32_0x00000000000000CC");
        assert_eq!(k.input_buffer_count, 1);
        assert!(k.source.contains("min(a[gid], "));
        assert!(k.source.contains("255.0f"));
    }

    /// **MX05 Phase 4.5.**  Distinct op kinds must produce distinct
    /// entry-point names so multiple specialised kernels can coexist
    /// in their own compiled libraries without collision.  The
    /// "op-name fragment" in the entry name is the differentiator.
    #[test]
    fn distinct_ops_produce_distinct_entry_point_prefixes() {
        let same_handle = 0xDEAD_BEEFu64;
        let add = emit_specialised_kernel(&binary_const_key(0x07, 1.0), same_handle).unwrap();
        let mul = emit_specialised_kernel(&binary_const_key(0x09, 1.0), same_handle).unwrap();
        let max = emit_specialised_kernel(&binary_const_key(0x0B, 1.0), same_handle).unwrap();
        let min = emit_specialised_kernel(&binary_const_key(0x0C, 1.0), same_handle).unwrap();
        // Entry points start with `specialised_<op_name>_`, so they
        // must all differ in the op_name slot even with identical
        // handle.
        let names = [
            &add.entry_point,
            &mul.entry_point,
            &max.entry_point,
            &min.entry_point,
        ];
        for i in 0..names.len() {
            for j in (i + 1)..names.len() {
                assert_ne!(
                    names[i], names[j],
                    "entry-point collision between op_kind kernels"
                );
            }
        }
        // And the bodies must reflect the different MSL operators.
        assert!(add.source.contains("a[gid] + "));
        assert!(mul.source.contains("a[gid] * "));
        assert!(max.source.contains("max(a[gid], "));
        assert!(min.source.contains("min(a[gid], "));
    }

    /// **MX05 Phase 4.6 regression.**  Non-commutative ops still
    /// return `None` if `folded_slot` is `None` — the emitter
    /// refuses to guess which side carries the constant.
    #[test]
    fn sub_div_pow_return_none_without_folded_slot() {
        for op_kind in [0x08u8, 0x0A, 0x0D] {
            let mut key = binary_const_key(op_kind, 7.0);
            key.folded_slot = None;
            assert!(
                emit_specialised_kernel(&key, 0).is_none(),
                "expected None for non-commutative op_kind 0x{:02X} \
                 with folded_slot=None",
                op_kind
            );
        }
    }

    /// **MX05 Phase 4.6.**  `Op::Sub` with folded RHS constant
    /// (slot 1) emits `a[gid] - K`.  Entry-point name contains
    /// `_rhs_const_`.
    #[test]
    fn sub_f32_rhs_folded_emits_kernel() {
        let k = emit_specialised_kernel(&binary_const_key_slot(0x08, 3.0, 1), 0x55).unwrap();
        assert_eq!(k.entry_point, "specialised_sub_rhs_const_f32_0x0000000000000055");
        assert_eq!(k.input_buffer_count, 1);
        assert!(k.source.contains("a[gid] - "));
        assert!(k.source.contains("3.0f"));
    }

    /// **MX05 Phase 4.6.**  `Op::Sub` with folded LHS constant
    /// (slot 0) emits `K - a[gid]` — the *non*-commutative variant.
    /// Entry-point name contains `_lhs_const_` so it doesn't collide
    /// with the RHS variant under the same handle.
    #[test]
    fn sub_f32_lhs_folded_emits_kernel() {
        let k = emit_specialised_kernel(&binary_const_key_slot(0x08, 10.0, 0), 0x66).unwrap();
        assert_eq!(k.entry_point, "specialised_sub_lhs_const_f32_0x0000000000000066");
        assert_eq!(k.input_buffer_count, 1);
        assert!(
            k.source.contains("10.0f - a[gid]"),
            "expected '10.0f - a[gid]' in body; got:\n{}",
            k.source
        );
    }

    /// **MX05 Phase 4.6.**  `Op::Div` with folded RHS emits
    /// `a[gid] / K`.  The common case — dividing a variable input
    /// by a stable constant denominator (e.g. normalising by 255).
    #[test]
    fn div_f32_rhs_folded_emits_kernel() {
        let k = emit_specialised_kernel(&binary_const_key_slot(0x0A, 255.0, 1), 0x77).unwrap();
        assert_eq!(k.entry_point, "specialised_div_rhs_const_f32_0x0000000000000077");
        assert!(k.source.contains("a[gid] / "));
        assert!(k.source.contains("255.0f"));
    }

    /// **MX05 Phase 4.6.**  `Op::Div` with folded LHS — rarer, but
    /// e.g. `1 / x` reciprocal of a variable input with a known
    /// numerator.  Emits `K / a[gid]`.
    #[test]
    fn div_f32_lhs_folded_emits_kernel() {
        let k = emit_specialised_kernel(&binary_const_key_slot(0x0A, 1.0, 0), 0x88).unwrap();
        assert_eq!(k.entry_point, "specialised_div_lhs_const_f32_0x0000000000000088");
        assert!(
            k.source.contains("1.0f / a[gid]"),
            "expected '1.0f / a[gid]' in body; got:\n{}",
            k.source
        );
    }

    /// **MX05 Phase 4.6.**  `Op::Pow` with folded RHS exponent —
    /// the standard "raise variable input to a constant power" case.
    /// MSL emits `pow(a[gid], K)`.
    #[test]
    fn pow_f32_rhs_folded_emits_kernel() {
        let k = emit_specialised_kernel(&binary_const_key_slot(0x0D, 2.0, 1), 0x99).unwrap();
        assert_eq!(k.entry_point, "specialised_pow_rhs_const_f32_0x0000000000000099");
        assert!(k.source.contains("pow(a[gid], "));
        assert!(k.source.contains("2.0f"));
    }

    /// **MX05 Phase 4.6.**  `Op::Pow` with folded LHS base — rarer
    /// but real (e.g. `2^x` exponential with a fixed base).  MSL
    /// emits `pow(K, a[gid])`.
    #[test]
    fn pow_f32_lhs_folded_emits_kernel() {
        let k = emit_specialised_kernel(&binary_const_key_slot(0x0D, 2.0, 0), 0xAB).unwrap();
        assert_eq!(k.entry_point, "specialised_pow_lhs_const_f32_0x00000000000000AB");
        assert!(
            k.source.contains("pow(2.0f, a[gid])"),
            "expected 'pow(2.0f, a[gid])' in body; got:\n{}",
            k.source
        );
    }

    /// **MX05 Phase 4.6.**  Two SpecKeys that differ only in
    /// `folded_slot` must produce different emitted entry-point
    /// names — otherwise the executor's per-handle table would
    /// collide.  We assert on the variant fragment in the entry name.
    #[test]
    fn lhs_and_rhs_variants_have_distinct_entry_points() {
        let same_handle = 0xCDCDCDCDCDCDCDCDu64;
        let lhs = emit_specialised_kernel(&binary_const_key_slot(0x08, 4.0, 0), same_handle)
            .unwrap();
        let rhs = emit_specialised_kernel(&binary_const_key_slot(0x08, 4.0, 1), same_handle)
            .unwrap();
        assert_ne!(lhs.entry_point, rhs.entry_point);
        assert!(lhs.entry_point.contains("_lhs_"));
        assert!(rhs.entry_point.contains("_rhs_"));
    }

    #[test]
    fn returns_none_for_unsupported_op_kind() {
        // Op::ReduceSum (0x0E) — reductions aren't yet in the
        // emitter's shape catalogue.  Future phase work could add
        // them (with the reduction axis encoded in the SpecKey),
        // but until then this returns None.
        let mut key = add_const_key(7.0);
        key.op_kind = 0x0E;
        assert!(emit_specialised_kernel(&key, 0).is_none());
    }

    // ───────────── MX05 Phase 4.7 — unary with folded input ─────────────

    /// Build a SpecKey for a unary op with its single input folded
    /// as a constant.
    fn unary_input_const_key(op_kind: u8, constant: f32) -> SpecKey {
        SpecKey {
            op_kind,
            dtype: DType::F32,
            shape_class: ShapeClass::Dynamic,
            range_class: RangeClass::Constant {
                bytes: constant.to_le_bytes().to_vec(),
            },
            backend_id: 1,
            folded_slot: Some(0), // Unary ops have only one input slot.
        }
    }

    /// **MX05 Phase 4.7.**  `Op::Neg` (0x00) with folded input
    /// constant `K` collapses to a memset of `-K`.  The emitted
    /// kernel takes **zero** input buffers — the value is baked in.
    #[test]
    fn neg_f32_with_folded_input_emits_memset_kernel() {
        let k = emit_specialised_kernel(&unary_input_const_key(0x00, 3.0), 0xA1).unwrap();
        assert_eq!(
            k.entry_point,
            "specialised_neg_input_const_f32_0x00000000000000A1"
        );
        assert_eq!(k.input_buffer_count, 0);
        assert_eq!(k.output_buffer_count, 1);
        // -3.0 formats as `-3.0f` after Ryu + suffix.
        assert!(
            k.source.contains("out[gid] = -3.0f"),
            "expected 'out[gid] = -3.0f' in body:\n{}",
            k.source
        );
        // No input buffer in the signature.
        assert!(
            !k.source.contains("device const float* a"),
            "specialised unary kernel must NOT take an input buffer"
        );
    }

    /// **MX05 Phase 4.7.**  `Op::Sqrt` (0x02) with `K = 16.0` is the
    /// canonical example: the kernel writes `4.0` everywhere.
    #[test]
    fn sqrt_f32_with_folded_input_emits_memset_kernel() {
        let k = emit_specialised_kernel(&unary_input_const_key(0x02, 16.0), 0xA2).unwrap();
        assert_eq!(
            k.entry_point,
            "specialised_sqrt_input_const_f32_0x00000000000000A2"
        );
        assert_eq!(k.input_buffer_count, 0);
        assert!(k.source.contains("out[gid] = 4.0f"));
    }

    /// **MX05 Phase 4.7.**  `Op::Abs` (0x01) with `K = -7.5` writes
    /// `7.5` everywhere.
    #[test]
    fn abs_f32_with_folded_input_emits_memset_kernel() {
        let k = emit_specialised_kernel(&unary_input_const_key(0x01, -7.5), 0xA3).unwrap();
        assert_eq!(k.input_buffer_count, 0);
        assert!(k.source.contains("out[gid] = 7.5f"));
    }

    /// **MX05 Phase 4.7.**  `Op::Exp` (0x03) with `K = 0.0` writes
    /// `1.0` (since `exp(0) = 1`).
    #[test]
    fn exp_f32_with_folded_input_emits_memset_kernel() {
        let k = emit_specialised_kernel(&unary_input_const_key(0x03, 0.0), 0xA4).unwrap();
        assert_eq!(k.input_buffer_count, 0);
        assert!(k.source.contains("out[gid] = 1.0f"));
    }

    /// **MX05 Phase 4.7.**  `Op::Log` (0x04) with `K = 1.0` writes
    /// `0.0` (since `ln(1) = 0`).
    #[test]
    fn log_f32_with_folded_input_emits_memset_kernel() {
        let k = emit_specialised_kernel(&unary_input_const_key(0x04, 1.0), 0xA5).unwrap();
        assert_eq!(k.input_buffer_count, 0);
        assert!(k.source.contains("out[gid] = 0.0f"));
    }

    /// **MX05 Phase 4.7.**  `Op::Tanh` (0x05) with `K = 0.0` writes
    /// `0.0` (since `tanh(0) = 0`).
    #[test]
    fn tanh_f32_with_folded_input_emits_memset_kernel() {
        let k = emit_specialised_kernel(&unary_input_const_key(0x05, 0.0), 0xA6).unwrap();
        assert_eq!(k.input_buffer_count, 0);
        assert!(k.source.contains("out[gid] = 0.0f"));
    }

    /// **MX05 Phase 4.7.**  `Op::Recip` (0x06) with `K = 4.0` writes
    /// `0.25` everywhere.
    #[test]
    fn recip_f32_with_folded_input_emits_memset_kernel() {
        let k = emit_specialised_kernel(&unary_input_const_key(0x06, 4.0), 0xA7).unwrap();
        assert_eq!(k.input_buffer_count, 0);
        assert!(k.source.contains("out[gid] = 0.25f"));
    }

    /// **MX05 Phase 4.7.**  Unary kernels' entry-point names embed
    /// the `_input_const_` fragment so they don't collide with the
    /// binary-with-folded-constant kernels (`_const_`,
    /// `_lhs_const_`, `_rhs_const_`).
    #[test]
    fn unary_input_const_entry_names_distinct_from_binary_const() {
        let neg = emit_specialised_kernel(&unary_input_const_key(0x00, 3.0), 0xBB).unwrap();
        let add = emit_specialised_kernel(&binary_const_key(0x07, 3.0), 0xBB).unwrap();
        assert_ne!(neg.entry_point, add.entry_point);
        assert!(neg.entry_point.contains("_input_const_"));
        assert!(add.entry_point.contains("_const_"));
        assert!(!add.entry_point.contains("_input_const_"));
    }

    /// **MX05 Phase 4.7.**  Unary ops still return `None` if the
    /// policy didn't fold the input — the emitter has no other
    /// useful unary specialisation to offer until later phases
    /// (e.g. range narrowing) land.
    #[test]
    fn unary_ops_return_none_without_folded_slot() {
        for op_kind in [0x00u8, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06] {
            let mut key = unary_input_const_key(op_kind, 1.0);
            key.folded_slot = None;
            assert!(
                emit_specialised_kernel(&key, 0).is_none(),
                "expected None for unary op_kind 0x{:02X} with folded_slot=None",
                op_kind
            );
        }
    }

    // ───────────── MX05 Phase 4.10 — MatMul with folded matrix ─────────────

    /// Build a SpecKey for `Op::MatMul(0x15)` with the RHS matrix
    /// folded as a flat f32 byte sequence (`[m, n]` row-major).
    fn matmul_const_key(matrix: &[f32]) -> SpecKey {
        let bytes: Vec<u8> = matrix.iter().flat_map(|v| v.to_le_bytes()).collect();
        SpecKey {
            op_kind: 0x15,
            dtype: DType::F32,
            shape_class: ShapeClass::Dynamic,
            range_class: RangeClass::Constant { bytes },
            backend_id: 1,
            folded_slot: Some(1),
        }
    }

    /// **MX05 Phase 4.10.**  A 2×2 RHS-folded MatMul kernel emits
    /// a per-column dot-product expression with the four constant
    /// values baked in.  Verifies the entry-point convention and
    /// that all four matrix elements appear in the kernel source.
    #[test]
    fn matmul_2x2_rhs_folded_emits_kernel() {
        // B = [[1, 2], [3, 4]] (row-major)
        let b = [1.0_f32, 2.0, 3.0, 4.0];
        let k = emit_specialised_kernel(&matmul_const_key(&b), 0xCAFE).unwrap();
        assert_eq!(
            k.entry_point,
            "specialised_matmul_2x2_rhs_const_f32_0x000000000000CAFE"
        );
        assert_eq!(k.input_buffer_count, 1);
        assert_eq!(k.output_buffer_count, 1);
        // All four matrix elements should appear as literals in the
        // kernel source.  format_f32_literal appends `.0` when the
        // value has no decimal, so 1.0_f32 → "1.0f".
        for v in &b {
            let lit = format_f32_literal(*v);
            assert!(
                k.source.contains(&lit),
                "expected '{}' in:\n{}",
                lit,
                k.source
            );
        }
        // The kernel must have a `c == 0` branch and a `c == 1`
        // branch — one per column.
        assert!(k.source.contains("if (c == 0)"));
        assert!(k.source.contains("if (c == 1)"));
    }

    /// **MX05 Phase 4.10.**  A 4×4 RHS-folded MatMul kernel emits
    /// 16 baked-in constants and 4 column branches.
    #[test]
    fn matmul_4x4_rhs_folded_emits_kernel() {
        // 16 distinct values so each literal is unique in the source.
        let b: Vec<f32> = (1..=16).map(|i| i as f32).collect();
        let k = emit_specialised_kernel(&matmul_const_key(&b), 0xCD).unwrap();
        assert_eq!(
            k.entry_point,
            "specialised_matmul_4x4_rhs_const_f32_0x00000000000000CD"
        );
        for v in &b {
            let lit = format_f32_literal(*v);
            assert!(k.source.contains(&lit), "expected '{}' in kernel", lit);
        }
        // 4 column branches.
        for c in 0..4 {
            assert!(k.source.contains(&format!("if (c == {})", c)));
        }
    }

    /// **MX05 Phase 4.10.**  Unsupported matrix sizes (e.g. 3×3 = 9
    /// elements, or 5×5 = 25) return `None`.  V1 caps at 4×4.
    #[test]
    fn matmul_unsupported_size_returns_none() {
        let three_x_three: Vec<f32> = vec![1.0; 9];
        let five_x_five: Vec<f32> = vec![1.0; 25];
        assert!(emit_specialised_kernel(&matmul_const_key(&three_x_three), 0).is_none());
        assert!(emit_specialised_kernel(&matmul_const_key(&five_x_five), 0).is_none());
    }

    /// **MX05 Phase 4.10.**  LHS-folded MatMul isn't supported in
    /// V1 — emitter returns `None` so the runtime falls back to
    /// generic dispatch.  LHS-folded support would need a
    /// different kernel shape (`n` is on a different axis) and
    /// lands in a later phase.
    #[test]
    fn matmul_lhs_folded_returns_none() {
        let mut k = matmul_const_key(&[1.0_f32, 2.0, 3.0, 4.0]);
        k.folded_slot = Some(0);
        assert!(emit_specialised_kernel(&k, 0).is_none());
    }

    /// **MX05 Phase 4.10.**  Without a `folded_slot`, MatMul is
    /// non-commutative and the emitter has no idea which side is
    /// folded — returns `None`.
    #[test]
    fn matmul_no_folded_slot_returns_none() {
        let mut k = matmul_const_key(&[1.0_f32, 2.0, 3.0, 4.0]);
        k.folded_slot = None;
        assert!(emit_specialised_kernel(&k, 0).is_none());
    }

    #[test]
    fn returns_none_for_unsupported_dtype() {
        let mut key = add_const_key(7.0);
        key.dtype = DType::I32;
        assert!(emit_specialised_kernel(&key, 0).is_none());
    }

    #[test]
    fn returns_none_when_range_class_not_constant() {
        let mut key = add_const_key(7.0);
        key.range_class = RangeClass::Unknown;
        assert!(emit_specialised_kernel(&key, 0).is_none());
    }

    #[test]
    fn returns_none_when_constant_byte_length_wrong() {
        // F32 needs 4 bytes; pass 8 (f64) and expect rejection.
        let mut key = add_const_key(7.0);
        key.range_class = RangeClass::Constant {
            bytes: 7.0_f64.to_le_bytes().to_vec(),
        };
        assert!(emit_specialised_kernel(&key, 0).is_none());
    }

    #[test]
    fn returns_none_when_constant_bytes_empty() {
        let mut key = add_const_key(7.0);
        key.range_class = RangeClass::Constant { bytes: vec![] };
        assert!(emit_specialised_kernel(&key, 0).is_none());
    }

    #[test]
    fn handle_appears_zero_padded_in_entry_point() {
        // Small handle: 0x42 must come out as 0x0000000000000042.
        let k = emit_specialised_kernel(&add_const_key(0.0), 0x42).unwrap();
        assert_eq!(
            k.entry_point,
            "specialised_add_const_f32_0x0000000000000042"
        );
    }

    #[test]
    fn distinct_handles_produce_distinct_entry_points() {
        let k1 = emit_specialised_kernel(&add_const_key(0.0), 0x1).unwrap();
        let k2 = emit_specialised_kernel(&add_const_key(0.0), 0x2).unwrap();
        assert_ne!(k1.entry_point, k2.entry_point);
    }

    #[test]
    fn distinct_constants_produce_distinct_sources_same_handle() {
        let k1 = emit_specialised_kernel(&add_const_key(1.5), 0x42).unwrap();
        let k2 = emit_specialised_kernel(&add_const_key(2.5), 0x42).unwrap();
        // Same entry name (handle drives entry), distinct bodies.
        assert_eq!(k1.entry_point, k2.entry_point);
        assert_ne!(k1.source, k2.source);
    }

    #[test]
    fn emission_is_deterministic() {
        // Same (key, handle) twice → byte-identical output.  Required
        // so the specialiser's cache hit + emitter re-run produces
        // the same kernel.
        let k1 = emit_specialised_kernel(&add_const_key(3.14), 0xCAFEBABE).unwrap();
        let k2 = emit_specialised_kernel(&add_const_key(3.14), 0xCAFEBABE).unwrap();
        assert_eq!(k1, k2);
    }

    #[test]
    fn format_f32_literal_round_trips_normal_values() {
        for v in [0.0_f32, 1.0, -1.0, 3.14159, 1e30, -1e-30, f32::EPSILON] {
            let s = format_f32_literal(v);
            // Strip the trailing 'f' suffix to parse.
            let stripped = s.trim_end_matches('f');
            let parsed: f32 = stripped.parse().expect(&format!(
                "format_f32_literal output must parse as f32; got {}",
                s
            ));
            assert_eq!(parsed.to_bits(), v.to_bits(), "round-trip failed for {}", v);
        }
    }

    #[test]
    fn format_f32_literal_handles_non_finite() {
        assert!(format_f32_literal(f32::NAN).contains("NAN"));
        assert!(format_f32_literal(f32::INFINITY).contains("INFINITY"));
        assert!(format_f32_literal(f32::INFINITY).chars().filter(|&c| c == '-').count() == 0);
        assert!(format_f32_literal(f32::NEG_INFINITY).contains("-INFINITY"));
    }

    #[test]
    fn format_f32_literal_always_has_f_suffix_or_macro() {
        for v in [0.0_f32, -1.0, 7.0, 1e6] {
            assert!(
                format_f32_literal(v).ends_with('f'),
                "finite literal must end in 'f': {}",
                format_f32_literal(v)
            );
        }
    }

    /// Output should be parseable as MSL.  We can't run a real MSL
    /// parser on the CI runners, but we can sanity-check structural
    /// invariants: matched braces, exactly one `kernel void`, the
    /// `[[thread_position_in_grid]]` attribute, an `if (gid >= n)`
    /// bounds check, and a final `}`.
    #[test]
    fn emitted_source_passes_structural_sanity() {
        let k = emit_specialised_kernel(&add_const_key(42.0), 0).unwrap();
        let src = &k.source;

        let opens = src.matches('{').count();
        let closes = src.matches('}').count();
        assert_eq!(opens, closes, "unbalanced braces in:\n{}", src);

        assert_eq!(
            src.matches("kernel void").count(),
            1,
            "exactly one kernel function expected"
        );
        assert!(src.contains("[[thread_position_in_grid]]"));
        assert!(src.contains("if (gid >= n) return;"));
        assert!(src.trim_end().ends_with('}'));
    }
}
