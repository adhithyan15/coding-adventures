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
];
