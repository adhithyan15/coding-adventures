//! MSL kernels for the V1 op set.
//!
//! Every supported op has one MSL kernel function per supported dtype.
//! V1 supports F32 only — the canonical "GPU-accelerated tensor math"
//! workload.  Integer dtypes, casts, reductions, shape ops, and
//! comparisons are V2 work; the planner falls back to CPU for those.
//!
//! The kernels are bundled into a single MSL string compiled once at
//! executor startup (via `metal_compute::MetalDevice::compile`).
//! Pipelines are looked up by entry-point name.
//!
//! ## Threadgroup sizing
//!
//! Each kernel uses `[[thread_position_in_grid]]` and fans out to one
//! thread per output element.  matrix-metal's dispatch picks the
//! threadgroup size from `MetalComputePipeline::thread_width`.
//!
//! ## Out-of-bounds protection
//!
//! Elementwise kernels compare `gid` to a passed-in `count` constant
//! and early-return if out of bounds.  This handles the case where the
//! dispatched grid is rounded up past the actual element count.

/// The bundled MSL source — all kernel functions in one library so a
/// single compile yields every pipeline we need.
pub const KERNELS_MSL: &str = r#"
#include <metal_stdlib>
using namespace metal;

// ──────────────── elementwise unary (F32) ────────────────

#define UNARY_F32(NAME, EXPR) \
kernel void NAME ( \
    device const float* in [[buffer(0)]], \
    device float* out [[buffer(1)]], \
    constant uint& n [[buffer(2)]], \
    uint gid [[thread_position_in_grid]] \
) { \
    if (gid >= n) return; \
    float x = in[gid]; \
    out[gid] = EXPR; \
}

UNARY_F32(neg_f32,   -x)
UNARY_F32(abs_f32,   fabs(x))
UNARY_F32(sqrt_f32,  sqrt(x))
UNARY_F32(exp_f32,   exp(x))
UNARY_F32(log_f32,   log(x))
UNARY_F32(tanh_f32,  tanh(x))
UNARY_F32(recip_f32, 1.0f / x)

// ──────────────── elementwise binary (F32) ────────────────

#define BINARY_F32(NAME, EXPR) \
kernel void NAME ( \
    device const float* a [[buffer(0)]], \
    device const float* b [[buffer(1)]], \
    device float* out [[buffer(2)]], \
    constant uint& n [[buffer(3)]], \
    uint gid [[thread_position_in_grid]] \
) { \
    if (gid >= n) return; \
    float x = a[gid]; \
    float y = b[gid]; \
    out[gid] = EXPR; \
}

BINARY_F32(add_f32, x + y)
BINARY_F32(sub_f32, x - y)
BINARY_F32(mul_f32, x * y)
BINARY_F32(div_f32, x / y)
BINARY_F32(max_f32, max(x, y))
BINARY_F32(min_f32, min(x, y))
BINARY_F32(pow_f32, pow(x, y))

// ──────────────── matmul (F32, rank-2, row-major) ────────────────

kernel void matmul_f32(
    device const float* a    [[buffer(0)]],   // [m, k]
    device const float* b    [[buffer(1)]],   // [k, n]
    device float*       c    [[buffer(2)]],   // [m, n]
    constant uint3&     dims [[buffer(3)]],   // (m, k, n)
    uint2               gid  [[thread_position_in_grid]]
) {
    uint i = gid.x;
    uint j = gid.y;
    uint m = dims.x;
    uint k = dims.y;
    uint n = dims.z;
    if (i >= m || j >= n) return;
    float acc = 0.0f;
    for (uint kk = 0; kk < k; ++kk) {
        acc += a[i * k + kk] * b[kk * n + j];
    }
    c[i * n + j] = acc;
}

// ──────────────── transpose (F32, general N-D, max rank 4) ────────────────
//
// Permutation-driven transpose for tensors up to rank 4.  Bigger
// ranks fall back to CPU via the planner's capability filter (matrix-
// metal advertises max_tensor_rank = 4 in its BackendProfile).
//
// Walks output linearly: for each output element, decompose its
// linear index into a multi-index using the **output** dims, then
// reconstruct the **input** multi-index by reversing the permutation,
// then re-flatten to a linear index using the **input** dims.
//
// The args struct carries rank, output numel, and three rank-4
// arrays (perm, in_dims, out_dims).  Unused trailing dims at lower
// ranks are zero-padded.
//
// Cost per element: O(rank) divides + O(rank) multiplies.  Memory
// access pattern is non-coalesced for non-trivial permutations —
// that's the price of generality.  V2 could special-case the rank-2
// matrix-transpose path with a tiled shared-memory kernel; V1 keeps
// the kernel small.

struct TransposeArgs {
    uint rank;
    uint numel;
    uint perm[4];
    uint in_dims[4];
    uint out_dims[4];
};

kernel void transpose_f32(
    device const float* in [[buffer(0)]],
    device float* out [[buffer(1)]],
    constant TransposeArgs& args [[buffer(2)]],
    uint gid [[thread_position_in_grid]]
) {
    if (gid >= args.numel) return;

    // 1. Decompose `gid` (output linear index) into output multi-index.
    uint out_idx[4] = {0u, 0u, 0u, 0u};
    uint linear = gid;
    for (int d = (int)args.rank - 1; d >= 0; --d) {
        uint extent = args.out_dims[d];
        out_idx[d] = linear % extent;
        linear /= extent;
    }

    // 2. Reverse the permutation to get the input multi-index.
    //    out_dims[d] == in_dims[perm[d]], and out[i_out] reads
    //    from in[i_in] where i_in[perm[d]] = i_out[d].
    uint in_idx[4] = {0u, 0u, 0u, 0u};
    for (uint d = 0; d < args.rank; ++d) {
        in_idx[args.perm[d]] = out_idx[d];
    }

    // 3. Re-flatten the input multi-index using row-major input strides.
    uint in_linear = 0u;
    uint stride = 1u;
    for (int d = (int)args.rank - 1; d >= 0; --d) {
        in_linear += in_idx[d] * stride;
        stride *= args.in_dims[d];
    }

    out[gid] = in[in_linear];
}

// ──────────────── broadcast (F32, general N-D, max rank 4) ────────────────
//
// Replicate input across size-1 axes to match `target_shape`.  Each
// input axis must equal the corresponding target axis or be 1 (the
// matrix-ir validator enforces this; we trust the validated graph).
//
// Walks the output linearly: for each output element, decompose its
// linear index into a multi-index using the **target** dims, then
// build the input multi-index by clamping each size-1 axis to index 0
// and copying through every non-broadcast axis.  Re-flatten with the
// input dims and read.
//
// The args struct mirrors the Transpose layout (rank, output numel,
// in_dims[4], out_dims[4]) minus the perm field.  Memory access is
// **read-fan-in**: many output threads can read the same input
// element when broadcasting along a hot axis, which Metal handles
// well via its texture cache on Apple Silicon.

struct BroadcastArgs {
    uint rank;
    uint numel;
    uint in_dims[4];
    uint out_dims[4];
};

kernel void broadcast_f32(
    device const float* in [[buffer(0)]],
    device float* out [[buffer(1)]],
    constant BroadcastArgs& args [[buffer(2)]],
    uint gid [[thread_position_in_grid]]
) {
    if (gid >= args.numel) return;

    // 1. Decompose `gid` (output linear index) into output multi-index.
    uint out_idx[4] = {0u, 0u, 0u, 0u};
    uint linear = gid;
    for (int d = (int)args.rank - 1; d >= 0; --d) {
        uint extent = args.out_dims[d];
        out_idx[d] = linear % extent;
        linear /= extent;
    }

    // 2. Build the input multi-index: clamp size-1 axes to index 0;
    //    otherwise copy the output index across.  The validator
    //    guarantees `in_dims[d] == 1 || in_dims[d] == out_dims[d]`.
    uint in_linear = 0u;
    uint stride = 1u;
    for (int d = (int)args.rank - 1; d >= 0; --d) {
        uint i = (args.in_dims[d] == 1u) ? 0u : out_idx[d];
        in_linear += i * stride;
        stride *= args.in_dims[d];
    }

    out[gid] = in[in_linear];
}

// ──────────────── cast (F32 output) ────────────────
//
// Op::Cast specifies an output dtype.  matrix-metal advertises
// `supported_dtypes = F32`, which constrains the planner's capability
// filter to route only Casts whose **output** dtype is F32 to us.
// That leaves three input-dtype paths to support:
//
//   - F32 → F32 (degenerate identity cast; rare but legal)
//   - U8  → F32 (widening conversion)
//   - I32 → F32 (widening conversion)
//
// The other three directions (anything → U8 / I32) need
// `supported_dtypes` to advertise U8 / I32 — and that would also
// let the planner route U8/I32 elementwise ops to us, which we
// don't yet implement.  Keeping the dtype bitset at F32 means
// those casts stay on CPU, and we can ship the F32-output ones
// today.
//
// Each kernel is a one-line element-wise scalar cast.  MSL's
// implicit conversions match Rust's `as` semantics for these
// widening paths (no rounding mode ambiguity, no clamping to
// worry about — every `u8` and `i32` value fits in `f32` exactly
// or with at most one rounding step).

kernel void cast_u8_to_f32(
    device const uchar* in [[buffer(0)]],
    device float* out [[buffer(1)]],
    constant uint& n [[buffer(2)]],
    uint gid [[thread_position_in_grid]]
) {
    if (gid >= n) return;
    out[gid] = (float)in[gid];
}

kernel void cast_i32_to_f32(
    device const int* in [[buffer(0)]],
    device float* out [[buffer(1)]],
    constant uint& n [[buffer(2)]],
    uint gid [[thread_position_in_grid]]
) {
    if (gid >= n) return;
    out[gid] = (float)in[gid];
}

kernel void cast_f32_to_f32(
    device const float* in [[buffer(0)]],
    device float* out [[buffer(1)]],
    constant uint& n [[buffer(2)]],
    uint gid [[thread_position_in_grid]]
) {
    if (gid >= n) return;
    out[gid] = in[gid];
}
"#;

/// Names of every kernel entry point in [`KERNELS_MSL`].  Used at
/// startup to walk the library and build a pipeline cache keyed by
/// these strings.  Adding a new kernel?  Add to both [`KERNELS_MSL`]
/// and this list.
pub const KERNEL_ENTRY_POINTS: &[&str] = &[
    // unary
    "neg_f32",
    "abs_f32",
    "sqrt_f32",
    "exp_f32",
    "log_f32",
    "tanh_f32",
    "recip_f32",
    // binary
    "add_f32",
    "sub_f32",
    "mul_f32",
    "div_f32",
    "max_f32",
    "min_f32",
    "pow_f32",
    // matmul
    "matmul_f32",
    // transpose
    "transpose_f32",
    // broadcast
    "broadcast_f32",
    // cast (F32 output paths only — see KERNELS_MSL comment)
    "cast_u8_to_f32",
    "cast_i32_to_f32",
    "cast_f32_to_f32",
];
