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
/// | `op_kind`  | `dtype` | `shape_class` | `range_class`              | Notes                          |
/// |------------|---------|---------------|----------------------------|--------------------------------|
/// | `0x07` Add | `F32`   | any           | `Constant { bytes: 4 B }`  | Commutative; slot irrelevant   |
/// | `0x09` Mul | `F32`   | any           | `Constant { bytes: 4 B }`  | Commutative; slot irrelevant   |
/// | `0x0B` Max | `F32`   | any           | `Constant { bytes: 4 B }`  | Commutative; slot irrelevant   |
/// | `0x0C` Min | `F32`   | any           | `Constant { bytes: 4 B }`  | Commutative; slot irrelevant   |
///
/// All other combinations return `None` for now.  Adding a shape is
/// a small change — add a match arm and a unit test.
///
/// ## Why only commutative binary ops in V0.7.0
///
/// `Op::Sub` and `Op::Div` are mathematically non-commutative:
/// `LHS - K` differs from `K - LHS`, and `LHS / K` differs from
/// `K / LHS`.  Today's `SpecKey` doesn't encode which input slot
/// the policy folded — it just records the constant bytes — so the
/// emitter can't safely generate one of the two non-commutative
/// variants without risking wrong output if the policy happened to
/// pick the slot opposite the emitter's assumption.
///
/// Phase 4.6 will extend `SpecKey` with a `folded_slot: u8` field
/// (or equivalent) and unlock Sub / Div / Pow.  Until then, the
/// runtime falls back to the generic dispatch for these ops.
pub fn emit_specialised_kernel(key: &SpecKey, handle: u64) -> Option<EmittedKernel> {
    if key.dtype != DType::F32 {
        return None;
    }
    let RangeClass::Constant { bytes } = &key.range_class else {
        return None;
    };
    if bytes.len() != 4 {
        return None;
    }
    let arr: [u8; 4] = bytes.as_slice().try_into().ok()?;
    let constant = f32::from_le_bytes(arr);

    // Map op_kind to (kernel-name fragment, MSL expression template).
    // The MSL expression is what goes after `out[gid] = ` in the
    // emitted body; `{a}` substitutes the variable input, `{k}`
    // substitutes the constant.
    let (op_name, expr_template): (&str, &str) = match key.op_kind {
        0x07 => ("add", "{a} + {k}"),        // Op::Add (commutative)
        0x09 => ("mul", "{a} * {k}"),        // Op::Mul (commutative)
        0x0B => ("max", "max({a}, {k})"),    // Op::Max (commutative)
        0x0C => ("min", "min({a}, {k})"),    // Op::Min (commutative)
        // Sub (0x08), Div (0x0A), Pow (0x0D), reductions, shape ops,
        // unary, matmul — Phase 4.6+ work.  Each needs either
        // `folded_slot` in SpecKey or a separate emitter design.
        _ => return None,
    };
    Some(emit_binary_f32_with_rhs_const(
        handle,
        op_name,
        expr_template,
        constant,
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
    /// Convenience wrapper for the Phase 4.5 tests below.
    fn binary_const_key(op_kind: u8, constant: f32) -> SpecKey {
        SpecKey {
            op_kind,
            dtype: DType::F32,
            shape_class: ShapeClass::Dynamic,
            range_class: RangeClass::Constant {
                bytes: constant.to_le_bytes().to_vec(),
            },
            backend_id: 1,
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

    /// **MX05 Phase 4.5.**  Op::Sub (0x08), Op::Div (0x0A), and
    /// Op::Pow (0x0D) are NOT yet supported.  The reason — see the
    /// rustdoc on `emit_specialised_kernel` — is that they're
    /// non-commutative and SpecKey doesn't yet encode which slot the
    /// policy folded.  Phase 4.6+ will add a `folded_slot` field and
    /// unlock these.
    #[test]
    fn sub_div_pow_return_none_until_folded_slot_lands() {
        for op_kind in [0x08u8, 0x0A, 0x0D] {
            let key = binary_const_key(op_kind, 7.0);
            assert!(
                emit_specialised_kernel(&key, 0).is_none(),
                "expected None for non-commutative op_kind 0x{:02X} \
                 (Sub/Div/Pow await SpecKey::folded_slot)",
                op_kind
            );
        }
    }

    #[test]
    fn returns_none_for_unsupported_op_kind() {
        // Sub instead of Add.
        let mut key = add_const_key(7.0);
        key.op_kind = 0x08;
        assert!(emit_specialised_kernel(&key, 0).is_none());
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
