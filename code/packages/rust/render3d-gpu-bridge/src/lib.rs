//! # render3d-gpu-bridge
//!
//! `GAME44` (Era 4, Tier 1): Hardware 3D render-bridge uniting [`gpu_runtime`]
//! (`G09`), backend capability descriptors, and **bit-exact deterministic CPU
//! fallback** to [`raster3d_soft`] (`GAME33`).
//!
//! ## Architecture & Pipeline
//!
//! ```text
//!  RenderPass<'a> (RenderTargetSpec + CameraUniforms + LightRig + Meshes + Materials + DrawCalls)
//!         │
//!         ▼  1. Upfront Atomic Validation (CWE-20 / CWE-125 / CWE-190 / CWE-369 / CWE-400)
//!         │     (framebuffer, depth, target, camera, light, meshes, materials, draw calls)
//!         │     ──► Err(...) leaves framebuffer, depth, and scratch byte-for-byte untouched
//!         ▼  2. Backend Selection (BackendPreference + optional &gpu_runtime::Runtime)
//!         │     ├─► ForceSoftware / AutoHardwareWithSoftwareFallback ──► raster3d-soft (zero-alloc)
//!         │     ├─► ForceGpuRuntimeCpu ──► raster3d-soft + gpu-runtime CPU compute stage
//!         │     └─► RequireHardware(kind) ──► hardware runtime or RequestedBackendUnavailable
//!         ▼  3. Compute Post-Process Stage (BRIDGE_POSTPROCESS_SHADERS: CPU / MSL / CUDA)
//!         │
//!         ▼  4. Canonical FNV-1a Frame & Depth Digests
//!  RenderExecutionReport { active_backend, fell_back, fallback_reason, stats, frame_digest, depth_digest }
//! ```
//!
//! ## Safety, Determinism, and Zero-Allocation Guarantees
//!
//! * `#![forbid(unsafe_code)]` — zero unsafe blocks or raw pointer arithmetic.
//! * `capabilities: []` — pure in-memory rendering bridge; never calls
//!   `gpu_runtime::Runtime::detect()` or `gpu_runtime::Runtime::global()`
//!   implicitly, keeping seccomp `RCG001` sandboxes free of driver `dlopen` probes.
//! * **Multi-draw atomicity**: every target, mesh, material, index range, and
//!   total-triangle budget across all draw calls is validated before clearing or
//!   mutating `framebuffer`, `depth`, or `scratch`.
//! * **Zero-alloc steady-state hot path (`VER07`)**: [`execute_render_pass_into`]
//!   with a pre-sized [`RenderBridgeScratch`] executes directly onto caller-owned
//!   [`PixelContainer`] and [`DepthBufferMut`] with `0` heap allocations.
//! * **Bit-exact cross-backend parity**: `ForceSoftware`,
//!   `AutoHardwareWithSoftwareFallback`, and `ForceGpuRuntimeCpu` (both zero-alloc
//!   and `gpu_runtime::Runtime::cpu_only()` dispatch) produce identical 64-bit
//!   FNV-1a frame and depth digests.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

/// Generated capability verification witnesses.
pub mod capabilities;

pub use capabilities::PureCapabilities;
pub use gpu_runtime::{
    BackendKind as GpuRuntimeBackendKind, GpuError, Runtime as GpuRuntime, Shaders,
};
pub use math3d::{Mat3, Mat4, Vec3, Vec4};
pub use pixel_container::PixelContainer;
pub use raster3d_soft::{
    clear_depth_f32, clear_depth_f64, clear_framebuffer, depth_digest_f32_fnv1a,
    depth_digest_f64_fnv1a, draw_indexed_mesh_into, frame_digest_fnv1a, ClipSpace, CullMode,
    DepthBufferMut, DepthFunc, DirectionalLight, FrontFace, LightRig, RasterConfig, RasterError,
    RasterScratch, RasterStats, RgbColor, Rgba8, ShadingMode, Texture, TextureWrap, UvCoord,
    Vertex,
};

// ---------------------------------------------------------------------------
// Resource & Numeric Bounds (CWE-400 / CWE-190 / CWE-20)
// ---------------------------------------------------------------------------

/// Maximum supported framebuffer width in pixels (CWE-400).
pub const MAX_FRAMEBUFFER_WIDTH: u32 = raster3d_soft::MAX_FRAMEBUFFER_WIDTH;

/// Maximum supported framebuffer height in pixels (CWE-400).
pub const MAX_FRAMEBUFFER_HEIGHT: u32 = raster3d_soft::MAX_FRAMEBUFFER_HEIGHT;

/// Maximum number of draw calls permitted in a single [`RenderPass`] (CWE-400).
pub const MAX_DRAW_CALLS: usize = 4096;

/// Maximum number of mesh slices permitted in a single [`RenderPass`] (CWE-400).
pub const MAX_MESHES: usize = 4096;

/// Maximum number of material bindings permitted in a single [`RenderPass`] (CWE-400).
pub const MAX_MATERIALS: usize = 4096;

/// Maximum total triangles permitted across all draw calls in a [`RenderPass`] (CWE-400).
pub const MAX_TOTAL_TRIANGLES: usize = raster3d_soft::MAX_TRIANGLES;

/// Maximum vertices permitted in a single [`MeshSlice`] (CWE-400).
pub const MAX_VERTICES_PER_MESH: usize = raster3d_soft::MAX_VERTICES;

/// Maximum coordinate / uniform scalar magnitude accepted on input (CWE-20 / CWE-190).
pub const MAX_COORD: f64 = raster3d_soft::MAX_COORD;

/// Serialized uniform payload size (in bytes) passed to `gpu-runtime` post-process shaders.
pub const POSTPROCESS_UNIFORM_BYTES: usize = 12;

/// Serialized camera+model uniform payload size (in bytes) for GPU uniform staging (`2 * 16 * 4`).
pub const CAMERA_UNIFORM_BYTES: usize = 128;

/// Fixed-size uniform byte array for post-process kernel dispatch.
pub type PostProcessUniformBytes = [u8; POSTPROCESS_UNIFORM_BYTES];

/// Fixed-size serialized `f32` uniform byte array (`view_proj` + `model`).
pub type CameraUniformBytes = [u8; CAMERA_UNIFORM_BYTES];

// ---------------------------------------------------------------------------
// Error Type
// ---------------------------------------------------------------------------

/// Failure modes returned by `render3d-gpu-bridge` operations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BridgeError {
    /// An input float (`clear_depth`, matrix, vector, vertex, light) is `NaN` or `+/-Inf` (CWE-20).
    NonFiniteInput,
    /// A coordinate, color channel, or intensity lies outside safe bounded ranges (CWE-20 / CWE-190).
    ValueOutOfBounds,
    /// Framebuffer, Z-buffer, or texture dimensions are zero, exceed limits, or mismatch buffer length (CWE-400 / CWE-125 / CWE-787).
    InvalidDimensions,
    /// RenderPass exceeds `MAX_DRAW_CALLS`, `MAX_MESHES`, `MAX_MATERIALS`, `MAX_VERTICES_PER_MESH`, or `MAX_TOTAL_TRIANGLES` (CWE-400).
    BudgetExceeded,
    /// A `DrawCall` references an out-of-bounds `mesh_index` or `material_index` (CWE-125).
    InvalidHandleIndex,
    /// A `DrawCall` index slice overflows, exceeds mesh indices, or is not a multiple of 3 (CWE-125 / CWE-190).
    InvalidIndexRange,
    /// A mesh vertex index refers past `vertices.len()` (CWE-125).
    VertexIndexOutOfBounds,
    /// A textured shading mode was requested without a bound texture.
    MissingTexture,
    /// Model matrix upper 3x3 is singular and cannot yield an inverse-transpose normal matrix (CWE-369).
    SingularTransform,
    /// Caller specified `BackendPreference::RequireHardware(kind)` and that hardware backend is unavailable.
    RequestedBackendUnavailable(RenderBackendKind),
    /// Dispatch failure reported by `gpu-runtime`.
    GpuRuntimeFailure,
    /// Underlying error from `raster3d-soft` or `math3d`.
    Raster(RasterError),
}

impl std::fmt::Display for BridgeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NonFiniteInput => write!(f, "input contains NaN or Infinity"),
            Self::ValueOutOfBounds => write!(f, "input value exceeds safe bounds"),
            Self::InvalidDimensions => {
                write!(
                    f,
                    "framebuffer, Z-buffer, or texture dimensions are invalid"
                )
            }
            Self::BudgetExceeded => write!(f, "render pass exceeds resource budget caps"),
            Self::InvalidHandleIndex => {
                write!(f, "draw call mesh or material index is out of bounds")
            }
            Self::InvalidIndexRange => write!(f, "draw call index slice range is invalid"),
            Self::VertexIndexOutOfBounds => write!(f, "mesh vertex index is out of bounds"),
            Self::MissingTexture => write!(f, "textured shading mode requires a texture"),
            Self::SingularTransform => {
                write!(f, "model transform upper 3x3 matrix is singular")
            }
            Self::RequestedBackendUnavailable(kind) => {
                write!(f, "requested hardware backend {kind:?} is unavailable")
            }
            Self::GpuRuntimeFailure => write!(f, "gpu-runtime shader dispatch failed"),
            Self::Raster(err) => write!(f, "raster3d-soft error: {err}"),
        }
    }
}

impl std::error::Error for BridgeError {}

impl From<RasterError> for BridgeError {
    #[inline]
    fn from(err: RasterError) -> Self {
        match err {
            RasterError::NonFiniteInput => Self::NonFiniteInput,
            RasterError::ValueOutOfBounds => Self::ValueOutOfBounds,
            RasterError::InvalidDimensions => Self::InvalidDimensions,
            RasterError::MeshTooLarge => Self::BudgetExceeded,
            RasterError::InvalidIndexCount => Self::InvalidIndexRange,
            RasterError::IndexOutOfBounds => Self::VertexIndexOutOfBounds,
            RasterError::MissingTexture => Self::MissingTexture,
            other => Self::Raster(other),
        }
    }
}

impl From<math3d::Math3dError> for BridgeError {
    #[inline]
    fn from(err: math3d::Math3dError) -> Self {
        match err {
            math3d::Math3dError::NonFiniteInput => Self::NonFiniteInput,
            math3d::Math3dError::ValueOutOfBounds => Self::ValueOutOfBounds,
            math3d::Math3dError::SingularMatrix => Self::SingularTransform,
            other => Self::Raster(RasterError::Math(other)),
        }
    }
}

// ---------------------------------------------------------------------------
// Warrant Contract Validation Helpers
// ---------------------------------------------------------------------------

/// Validate framebuffer width or height against `[1, 4096]` (`render3d-gpu-bridge.warrant`).
#[inline]
pub fn validate_framebuffer_dim(n: u32) -> Result<bool, BridgeError> {
    if n == 0 || n > MAX_FRAMEBUFFER_WIDTH || n > MAX_FRAMEBUFFER_HEIGHT {
        return Err(BridgeError::InvalidDimensions);
    }
    Ok(true)
}

/// Validate draw-call, mesh, or material table length against `[0, MAX_DRAW_CALLS]` (`render3d-gpu-bridge.warrant`).
#[inline]
pub fn validate_draw_call_count(n: usize) -> Result<bool, BridgeError> {
    if n > MAX_DRAW_CALLS {
        return Err(BridgeError::BudgetExceeded);
    }
    Ok(true)
}

/// Validate total submitted triangles across a `RenderPass` against `MAX_TOTAL_TRIANGLES` (`render3d-gpu-bridge.warrant`).
#[inline]
pub fn validate_total_triangles(n: usize) -> Result<bool, BridgeError> {
    if n > MAX_TOTAL_TRIANGLES {
        return Err(BridgeError::BudgetExceeded);
    }
    Ok(true)
}

/// Validate a mesh or material handle index against `table_len` (`render3d-gpu-bridge.warrant`).
#[inline]
pub fn validate_handle_index(idx: usize, table_len: usize) -> Result<bool, BridgeError> {
    if idx >= table_len {
        return Err(BridgeError::InvalidHandleIndex);
    }
    Ok(true)
}

/// Validate a draw-call index slice `[offset .. offset + index_count]` against `total_indices` (`render3d-gpu-bridge.warrant`).
#[inline]
pub fn validate_index_range(
    offset: usize,
    index_count: usize,
    total_indices: usize,
) -> Result<bool, BridgeError> {
    if !index_count.is_multiple_of(3) {
        return Err(BridgeError::InvalidIndexRange);
    }
    let Some(end) = offset.checked_add(index_count) else {
        return Err(BridgeError::InvalidIndexRange);
    };
    if end > total_indices {
        return Err(BridgeError::InvalidIndexRange);
    }
    Ok(true)
}

// ---------------------------------------------------------------------------
// Internal Validation Helpers
// ---------------------------------------------------------------------------

#[inline]
fn check_finite_scalar(v: f64) -> Result<f64, BridgeError> {
    if !v.is_finite() {
        return Err(BridgeError::NonFiniteInput);
    }
    Ok(v)
}

#[inline]
fn check_bounded_scalar(v: f64) -> Result<f64, BridgeError> {
    if !v.is_finite() {
        return Err(BridgeError::NonFiniteInput);
    }
    if v.abs() > MAX_COORD {
        return Err(BridgeError::ValueOutOfBounds);
    }
    Ok(v)
}

#[inline]
fn check_mat4_finite(m: &Mat4) -> Result<(), BridgeError> {
    for col in m.cols {
        for v in col {
            check_finite_scalar(v)?;
        }
    }
    Ok(())
}

#[inline]
fn check_mat3_finite(m: &Mat3) -> Result<(), BridgeError> {
    for col in m.cols {
        for v in col {
            check_finite_scalar(v)?;
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Backend Capability Descriptors & Selection Policy
// ---------------------------------------------------------------------------

/// Rendering backend descriptor supported by `render3d-gpu-bridge`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RenderBackendKind {
    /// Deterministic pure-Rust software rasterizer (`raster3d-soft`).
    Software,
    /// `gpu-runtime` pure-Rust CPU compute backend (`Runtime::cpu_only()`).
    GpuRuntimeCpu,
    /// `gpu-runtime` Apple Metal compute backend (requires `metal` feature on Apple targets).
    GpuRuntimeMetal,
    /// `gpu-runtime` NVIDIA CUDA compute backend (`libcuda.so` dynamic dispatch).
    GpuRuntimeCuda,
    /// WGPU cross-platform hardware capability descriptor.
    Wgpu,
    /// Native Apple Metal capability descriptor.
    Metal,
    /// Native Vulkan capability descriptor.
    Vulkan,
}

impl RenderBackendKind {
    /// Returns `true` if this descriptor represents a hardware GPU backend rather than CPU/software execution.
    #[inline]
    pub const fn backend_is_hardware(self) -> bool {
        matches!(
            self,
            Self::GpuRuntimeMetal | Self::GpuRuntimeCuda | Self::Wgpu | Self::Metal | Self::Vulkan
        )
    }

    /// Stable human-readable identifier for this backend descriptor.
    #[inline]
    pub const fn backend_name(self) -> &'static str {
        match self {
            Self::Software => "software-raster3d-soft",
            Self::GpuRuntimeCpu => "gpu-runtime-cpu",
            Self::GpuRuntimeMetal => "gpu-runtime-metal",
            Self::GpuRuntimeCuda => "gpu-runtime-cuda",
            Self::Wgpu => "wgpu",
            Self::Metal => "metal",
            Self::Vulkan => "vulkan",
        }
    }
}

/// Capability metadata describing a single [`RenderBackendKind`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BackendCapabilities {
    /// Backend kind described by this record.
    pub kind: RenderBackendKind,
    /// Whether this backend is usable with the current build/runtime configuration.
    pub available: bool,
    /// Whether this backend executes on dedicated GPU hardware.
    pub hardware_accelerated: bool,
    /// Whether output is bit-exact with the deterministic `raster3d-soft` CPU reference.
    pub deterministic_software_parity: bool,
    /// Maximum supported 2D texture dimension.
    pub max_texture_dim: u32,
    /// Maximum supported framebuffer dimension.
    pub max_framebuffer_dim: u32,
    /// Whether 64-bit floating-point depth buffers (`f64`) are natively supported.
    pub supports_f64_depth: bool,
    /// Whether the `gpu-runtime` compute post-process kernel stage is supported.
    pub supports_compute_post_process: bool,
}

impl BackendCapabilities {
    /// Return static build-time capability metadata for `kind` without probing OS GPU drivers.
    ///
    /// Note: hardware descriptors (`GpuRuntimeMetal`, `GpuRuntimeCuda`, `Wgpu`, `Metal`, `Vulkan`)
    /// report `available = false` statically until a live hardware [`GpuRuntime`] is supplied to
    /// [`BackendCapabilities::backend_caps_detect_safe`].
    pub const fn backend_caps_for(kind: RenderBackendKind) -> Self {
        match kind {
            RenderBackendKind::Software => Self {
                kind,
                available: true,
                hardware_accelerated: false,
                deterministic_software_parity: true,
                max_texture_dim: raster3d_soft::MAX_TEXTURE_DIM,
                max_framebuffer_dim: MAX_FRAMEBUFFER_WIDTH,
                supports_f64_depth: true,
                supports_compute_post_process: true,
            },
            RenderBackendKind::GpuRuntimeCpu => Self {
                kind,
                available: true,
                hardware_accelerated: false,
                deterministic_software_parity: true,
                max_texture_dim: raster3d_soft::MAX_TEXTURE_DIM,
                max_framebuffer_dim: MAX_FRAMEBUFFER_WIDTH,
                supports_f64_depth: true,
                supports_compute_post_process: true,
            },
            RenderBackendKind::GpuRuntimeMetal => Self {
                kind,
                available: false,
                hardware_accelerated: true,
                deterministic_software_parity: false,
                max_texture_dim: MAX_FRAMEBUFFER_WIDTH,
                max_framebuffer_dim: MAX_FRAMEBUFFER_WIDTH,
                supports_f64_depth: false,
                supports_compute_post_process: cfg!(all(
                    target_vendor = "apple",
                    feature = "metal"
                )),
            },
            RenderBackendKind::GpuRuntimeCuda => Self {
                kind,
                available: false,
                hardware_accelerated: true,
                deterministic_software_parity: false,
                max_texture_dim: MAX_FRAMEBUFFER_WIDTH,
                max_framebuffer_dim: MAX_FRAMEBUFFER_WIDTH,
                supports_f64_depth: true,
                supports_compute_post_process: true,
            },
            RenderBackendKind::Wgpu | RenderBackendKind::Metal | RenderBackendKind::Vulkan => {
                Self {
                    kind,
                    available: false,
                    hardware_accelerated: true,
                    deterministic_software_parity: false,
                    max_texture_dim: MAX_FRAMEBUFFER_WIDTH,
                    max_framebuffer_dim: MAX_FRAMEBUFFER_WIDTH,
                    supports_f64_depth: false,
                    supports_compute_post_process: false,
                }
            }
        }
    }

    /// Inspect capability availability using an explicit caller-supplied [`GpuRuntime`],
    /// never invoking `Runtime::detect()` or `Runtime::global()`.
    pub fn backend_caps_detect_safe(runtime: Option<&GpuRuntime>) -> BackendCapabilitySet {
        let mut set = BackendCapabilitySet {
            software: Self::backend_caps_for(RenderBackendKind::Software),
            gpu_runtime_cpu: Self::backend_caps_for(RenderBackendKind::GpuRuntimeCpu),
            gpu_runtime_metal: Self::backend_caps_for(RenderBackendKind::GpuRuntimeMetal),
            gpu_runtime_cuda: Self::backend_caps_for(RenderBackendKind::GpuRuntimeCuda),
            wgpu: Self::backend_caps_for(RenderBackendKind::Wgpu),
            metal: Self::backend_caps_for(RenderBackendKind::Metal),
            vulkan: Self::backend_caps_for(RenderBackendKind::Vulkan),
        };
        if let Some(rt) = runtime {
            match rt.backend() {
                GpuRuntimeBackendKind::Metal => {
                    set.gpu_runtime_metal.available = true;
                    set.metal.available = true;
                }
                GpuRuntimeBackendKind::Cuda => {
                    set.gpu_runtime_cuda.available = true;
                }
                GpuRuntimeBackendKind::Cpu => {
                    set.gpu_runtime_cpu.available = true;
                }
            }
        }
        set
    }
}

/// Complete capability table across all seven backend descriptors.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BackendCapabilitySet {
    /// Pure-Rust `raster3d-soft` capability record.
    pub software: BackendCapabilities,
    /// `gpu-runtime` CPU compute capability record.
    pub gpu_runtime_cpu: BackendCapabilities,
    /// `gpu-runtime` Metal compute capability record.
    pub gpu_runtime_metal: BackendCapabilities,
    /// `gpu-runtime` CUDA compute capability record.
    pub gpu_runtime_cuda: BackendCapabilities,
    /// WGPU descriptor capability record.
    pub wgpu: BackendCapabilities,
    /// Native Metal descriptor capability record.
    pub metal: BackendCapabilities,
    /// Native Vulkan descriptor capability record.
    pub vulkan: BackendCapabilities,
}

impl BackendCapabilitySet {
    /// Retrieve the capability record matching `kind`.
    #[inline]
    pub const fn cap_set_get(&self, kind: RenderBackendKind) -> BackendCapabilities {
        match kind {
            RenderBackendKind::Software => self.software,
            RenderBackendKind::GpuRuntimeCpu => self.gpu_runtime_cpu,
            RenderBackendKind::GpuRuntimeMetal => self.gpu_runtime_metal,
            RenderBackendKind::GpuRuntimeCuda => self.gpu_runtime_cuda,
            RenderBackendKind::Wgpu => self.wgpu,
            RenderBackendKind::Metal => self.metal,
            RenderBackendKind::Vulkan => self.vulkan,
        }
    }
}

/// Caller policy controlling backend selection and software fallback behavior.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum BackendPreference {
    /// Prefer an available hardware backend when explicitly supplied, falling back deterministically to `Software`.
    #[default]
    AutoHardwareWithSoftwareFallback,
    /// Always execute directly on the deterministic `Software` (`raster3d-soft`) backend.
    ForceSoftware,
    /// Execute via the `gpu-runtime` CPU compute stage paired with `raster3d-soft`.
    ForceGpuRuntimeCpu,
    /// Require the specified hardware backend and return `Err(BridgeError::RequestedBackendUnavailable)` if unavailable.
    RequireHardware(RenderBackendKind),
}

/// Reason why execution fell back to deterministic CPU software rendering.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum FallbackReason {
    /// No fallback occurred; the primary requested backend executed directly.
    #[default]
    NoFallback,
    /// Caller supplied no hardware `gpu_runtime::Runtime` instance (`AutoHardwareWithSoftwareFallback`).
    NoHardwareRuntimeProvided,
    /// The platform build disabled optional hardware backend support (e.g., `metal` feature off).
    HardwareFeatureDisabled,
    /// Hardware driver or supplied runtime did not provide an active hardware GPU backend.
    HardwareDriverUnavailable,
    /// Compute post-process dispatch failed or lacked parity, falling back to `Software`.
    PostProcessParityFallback,
}

/// Execution telemetry and canonical digests returned by render-pass execution.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RenderExecutionReport {
    /// Backend that actually produced the final framebuffer and Z-buffer.
    pub active_backend: RenderBackendKind,
    /// `true` when execution fell back from a hardware preference to deterministic CPU fallback.
    pub fell_back: bool,
    /// Structured reason describing any fallback that occurred.
    pub fallback_reason: FallbackReason,
    /// Aggregated rasterization counters summed across all draw calls in the pass.
    pub stats: RasterStats,
    /// Canonical 64-bit FNV-1a digest of the output [`PixelContainer`].
    pub frame_digest: u64,
    /// Canonical 64-bit FNV-1a digest of the output [`DepthBufferMut`].
    pub depth_digest: u64,
}

// ---------------------------------------------------------------------------
// gpu-runtime Shaders Bundle (CPU + MSL + CUDA C)
// ---------------------------------------------------------------------------

const POSTPROCESS_MODE_IDENTITY: u32 = 0;
const POSTPROCESS_MODE_GAIN_BIAS: u32 = 1;
const POSTPROCESS_MODE_TINT: u32 = 2;

/// Metal Shading Language (MSL) compute kernel for RGBA8 post-processing.
pub const POSTPROCESS_MSL_SOURCE: &str = r#"
#include <metal_stdlib>
using namespace metal;

kernel void bridge_postprocess(
    device const uchar* src [[buffer(0)]],
    device uchar* dst [[buffer(1)]],
    constant uint* uniforms [[buffer(2)]],
    uint gid [[thread_position_in_grid]]
) {
    uint mode = uniforms[0];
    uint param = uniforms[1];
    uint pixel_count = uniforms[2];
    if (gid >= pixel_count) {
        return;
    }
    uint base = gid * 4;
    uchar r = src[base + 0];
    uchar g = src[base + 1];
    uchar b = src[base + 2];
    uchar a = src[base + 3];
    if (mode == 1u) {
        int gain_q8 = int(param & 0xffffu);
        int bias = int(short(param >> 16));
        int nr = clamp(((int(r) * gain_q8 + 128) >> 8) + bias, 0, 255);
        int ng = clamp(((int(g) * gain_q8 + 128) >> 8) + bias, 0, 255);
        int nb = clamp(((int(b) * gain_q8 + 128) >> 8) + bias, 0, 255);
        dst[base + 0] = uchar(nr);
        dst[base + 1] = uchar(ng);
        dst[base + 2] = uchar(nb);
        dst[base + 3] = a;
    } else if (mode == 2u) {
        uint tr = (param) & 0xffu;
        uint tg = (param >> 8) & 0xffu;
        uint tb = (param >> 16) & 0xffu;
        uint ta = (param >> 24) & 0xffu;
        dst[base + 0] = uchar((uint(r) * tr + 127u) / 255u);
        dst[base + 1] = uchar((uint(g) * tg + 127u) / 255u);
        dst[base + 2] = uchar((uint(b) * tb + 127u) / 255u);
        dst[base + 3] = uchar((uint(a) * ta + 127u) / 255u);
    } else {
        dst[base + 0] = r;
        dst[base + 1] = g;
        dst[base + 2] = b;
        dst[base + 3] = a;
    }
}
"#;

/// CUDA C compute kernel for RGBA8 post-processing.
pub const POSTPROCESS_CUDA_SOURCE: &str = r#"
extern "C" __global__ void bridge_postprocess(
    const unsigned char* src,
    unsigned char* dst,
    const unsigned char* uni,
    unsigned int n
) {
    unsigned int gid = blockIdx.x * blockDim.x + threadIdx.x;
    if (gid >= n) {
        return;
    }
    const unsigned int* u = reinterpret_cast<const unsigned int*>(uni);
    unsigned int mode = u[0];
    unsigned int param = u[1];
    unsigned int base = gid * 4u;
    unsigned char r = src[base + 0u];
    unsigned char g = src[base + 1u];
    unsigned char b = src[base + 2u];
    unsigned char a = src[base + 3u];
    if (mode == 1u) {
        int gain_q8 = static_cast<int>(param & 0xffffu);
        int bias = static_cast<int>(static_cast<short>(param >> 16));
        int nr = ((static_cast<int>(r) * gain_q8 + 128) >> 8) + bias;
        int ng = ((static_cast<int>(g) * gain_q8 + 128) >> 8) + bias;
        int nb = ((static_cast<int>(b) * gain_q8 + 128) >> 8) + bias;
        dst[base + 0u] = static_cast<unsigned char>(nr < 0 ? 0 : (nr > 255 ? 255 : nr));
        dst[base + 1u] = static_cast<unsigned char>(ng < 0 ? 0 : (ng > 255 ? 255 : ng));
        dst[base + 2u] = static_cast<unsigned char>(nb < 0 ? 0 : (nb > 255 ? 255 : nb));
        dst[base + 3u] = a;
    } else if (mode == 2u) {
        unsigned int tr = (param) & 0xffu;
        unsigned int tg = (param >> 8) & 0xffu;
        unsigned int tb = (param >> 16) & 0xffu;
        unsigned int ta = (param >> 24) & 0xffu;
        dst[base + 0u] = static_cast<unsigned char>((static_cast<unsigned int>(r) * tr + 127u) / 255u);
        dst[base + 1u] = static_cast<unsigned char>((static_cast<unsigned int>(g) * tg + 127u) / 255u);
        dst[base + 2u] = static_cast<unsigned char>((static_cast<unsigned int>(b) * tb + 127u) / 255u);
        dst[base + 3u] = static_cast<unsigned char>((static_cast<unsigned int>(a) * ta + 127u) / 255u);
    } else {
        dst[base + 0u] = r;
        dst[base + 1u] = g;
        dst[base + 2u] = b;
        dst[base + 3u] = a;
    }
}
"#;

/// Metal Shading Language (MSL) compute kernel resolving `f32` depth to grayscale RGBA8.
pub const DEPTH_RESOLVE_MSL_SOURCE: &str = r#"
#include <metal_stdlib>
using namespace metal;

kernel void bridge_depth_resolve(
    device const float* src [[buffer(0)]],
    device uchar* dst [[buffer(1)]],
    constant uint* uniforms [[buffer(2)]],
    uint gid [[thread_position_in_grid]]
) {
    uint pixel_count = uniforms[0];
    if (gid >= pixel_count) {
        return;
    }
    float d = src[gid];
    float c = isfinite(d) ? clamp(d, 0.0f, 1.0f) : 1.0f;
    uchar g = uchar(clamp(int(c * 255.0f + 0.5f), 0, 255));
    uint base = gid * 4u;
    dst[base + 0u] = g;
    dst[base + 1u] = g;
    dst[base + 2u] = g;
    dst[base + 3u] = 255u;
}
"#;

/// CUDA C compute kernel resolving `f32` depth to grayscale RGBA8.
pub const DEPTH_RESOLVE_CUDA_SOURCE: &str = r#"
extern "C" __global__ void bridge_depth_resolve(
    const unsigned char* src_bytes,
    unsigned char* dst,
    const unsigned char* uni,
    unsigned int n
) {
    unsigned int gid = blockIdx.x * blockDim.x + threadIdx.x;
    if (gid >= n) {
        return;
    }
    const float* src = reinterpret_cast<const float*>(src_bytes);
    float d = src[gid];
    float c = isfinite(d) ? (d < 0.0f ? 0.0f : (d > 1.0f ? 1.0f : d)) : 1.0f;
    int q = static_cast<int>(c * 255.0f + 0.5f);
    unsigned char g = static_cast<unsigned char>(q < 0 ? 0 : (q > 255 ? 255 : q));
    unsigned int base = gid * 4u;
    dst[base + 0u] = g;
    dst[base + 1u] = g;
    dst[base + 2u] = g;
    dst[base + 3u] = 255u;
}
"#;

fn decode_u32_le(bytes: &[u8], start: usize) -> u32 {
    match bytes.get(start..start + 4) {
        Some(s) => u32::from_le_bytes([s[0], s[1], s[2], s[3]]),
        None => 0,
    }
}

fn postprocess_cpu_kernel(src: &[u8], dst: &mut [u8], uniforms: &[u8]) {
    let mode = decode_u32_le(uniforms, 0);
    let param = decode_u32_le(uniforms, 4);

    if mode == POSTPROCESS_MODE_IDENTITY {
        let len = src.len().min(dst.len());
        dst[..len].copy_from_slice(&src[..len]);
        return;
    }

    let (src_chunks, _) = src.as_chunks::<4>();
    let (dst_chunks, _) = dst.as_chunks_mut::<4>();

    if mode == POSTPROCESS_MODE_GAIN_BIAS {
        let gain_q8 = i32::from((param & 0xffff) as u16);
        let bias = i32::from(((param >> 16) as u16) as i16);
        for (s, d) in src_chunks.iter().zip(dst_chunks.iter_mut()) {
            let nr = ((i32::from(s[0]) * gain_q8 + 128) >> 8) + bias;
            let ng = ((i32::from(s[1]) * gain_q8 + 128) >> 8) + bias;
            let nb = ((i32::from(s[2]) * gain_q8 + 128) >> 8) + bias;
            *d = [
                nr.clamp(0, 255) as u8,
                ng.clamp(0, 255) as u8,
                nb.clamp(0, 255) as u8,
                s[3],
            ];
        }
    } else if mode == POSTPROCESS_MODE_TINT {
        let tr = param & 0xff;
        let tg = (param >> 8) & 0xff;
        let tb = (param >> 16) & 0xff;
        let ta = (param >> 24) & 0xff;
        for (s, d) in src_chunks.iter().zip(dst_chunks.iter_mut()) {
            *d = [
                ((u32::from(s[0]) * tr + 127) / 255) as u8,
                ((u32::from(s[1]) * tg + 127) / 255) as u8,
                ((u32::from(s[2]) * tb + 127) / 255) as u8,
                ((u32::from(s[3]) * ta + 127) / 255) as u8,
            ];
        }
    } else {
        let len = src.len().min(dst.len());
        dst[..len].copy_from_slice(&src[..len]);
    }
}

fn depth_resolve_cpu_kernel(src: &[u8], dst: &mut [u8], _uniforms: &[u8]) {
    let (src_chunks, _) = src.as_chunks::<4>();
    let (dst_chunks, _) = dst.as_chunks_mut::<4>();
    for (s, d) in src_chunks.iter().zip(dst_chunks.iter_mut()) {
        let val = f32::from_le_bytes(*s);
        let clamped = if val.is_finite() {
            val.clamp(0.0, 1.0)
        } else {
            1.0
        };
        let q = ((clamped * 255.0 + 0.5) as i32).clamp(0, 255) as u8;
        *d = [q, q, q, 255];
    }
}

/// Registered `gpu-runtime` shader bundle for RGBA8 post-process / blit dispatch.
pub static BRIDGE_POSTPROCESS_SHADERS: Shaders = Shaders {
    metal: Some(POSTPROCESS_MSL_SOURCE),
    cuda: Some(POSTPROCESS_CUDA_SOURCE),
    cpu: Some(postprocess_cpu_kernel),
};

/// Registered `gpu-runtime` shader bundle for `f32` Z-buffer depth visualization resolve.
pub static BRIDGE_DEPTH_RESOLVE_SHADERS: Shaders = Shaders {
    metal: Some(DEPTH_RESOLVE_MSL_SOURCE),
    cuda: Some(DEPTH_RESOLVE_CUDA_SOURCE),
    cpu: Some(depth_resolve_cpu_kernel),
};

/// Return the registered [`Shaders`] bundle for the post-process compute stage.
#[inline]
pub const fn bridge_postprocess_shaders() -> &'static Shaders {
    &BRIDGE_POSTPROCESS_SHADERS
}

/// Return the registered [`Shaders`] bundle for the depth-resolve compute stage.
#[inline]
pub const fn bridge_depth_resolve_shaders() -> &'static Shaders {
    &BRIDGE_DEPTH_RESOLVE_SHADERS
}

// ---------------------------------------------------------------------------
// Validated Scene, Target, Camera, Mesh, Material, DrawCall, RenderPass
// ---------------------------------------------------------------------------

/// Compute post-process / blit mode executed after 3D rasterization.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PostProcessMode {
    /// Bit-exact identity blit preserving raw rasterizer pixels untouched.
    #[default]
    Identity,
    /// Fixed-point Q8 multiplicative exposure gain (`gain_q8 / 256`) plus signed bias.
    ExposureGainBias {
        /// Multiplicative gain in Q8 fixed point (`256 == 1.0x`).
        gain_q8: u16,
        /// Signed additive offset applied after scaling.
        bias: i16,
    },
    /// Per-channel RGBA8 modulation tint (`[r, g, b, a]`).
    ChannelTint {
        /// RGBA8 multiplicative tint color.
        tint: Rgba8,
    },
}

impl PostProcessMode {
    fn encode_uniforms(self, pixel_count: u32) -> PostProcessUniformBytes {
        let (mode, param) = match self {
            Self::Identity => (POSTPROCESS_MODE_IDENTITY, 0u32),
            Self::ExposureGainBias { gain_q8, bias } => {
                let packed = u32::from(gain_q8) | (u32::from(bias as u16) << 16);
                (POSTPROCESS_MODE_GAIN_BIAS, packed)
            }
            Self::ChannelTint { tint } => {
                let packed = u32::from_le_bytes(tint);
                (POSTPROCESS_MODE_TINT, packed)
            }
        };
        let m = mode.to_le_bytes();
        let p = param.to_le_bytes();
        let c = pixel_count.to_le_bytes();
        [
            m[0], m[1], m[2], m[3], p[0], p[1], p[2], p[3], c[0], c[1], c[2], c[3],
        ]
    }
}

/// Validated framebuffer target specification for a [`RenderPass`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RenderTargetSpec {
    /// Target width in pixels (`1..=MAX_FRAMEBUFFER_WIDTH`).
    pub width: u32,
    /// Target height in pixels (`1..=MAX_FRAMEBUFFER_HEIGHT`).
    pub height: u32,
    /// RGBA8 clear color written at pass start.
    pub clear_color: Rgba8,
    /// Initial depth value written across the Z-buffer (typically `1.0`).
    pub clear_depth: f64,
    /// Homogeneous NDC depth convention (`OpenGl` or `ZeroToOne`).
    pub clip_space: ClipSpace,
    /// Post-process compute kernel applied after rasterization.
    pub post_process: PostProcessMode,
}

impl RenderTargetSpec {
    /// Construct and validate a [`RenderTargetSpec`].
    pub fn target_spec_new(
        width: u32,
        height: u32,
        clear_color: Rgba8,
        clear_depth: f64,
    ) -> Result<Self, BridgeError> {
        let spec = Self {
            width,
            height,
            clear_color,
            clear_depth,
            clip_space: ClipSpace::OpenGl,
            post_process: PostProcessMode::Identity,
        };
        spec.target_spec_validate()?;
        Ok(spec)
    }

    /// Validate target dimensions and `clear_depth` finiteness (CWE-20 / CWE-400).
    pub fn target_spec_validate(&self) -> Result<(), BridgeError> {
        validate_framebuffer_dim(self.width)?;
        validate_framebuffer_dim(self.height)?;
        check_finite_scalar(self.clear_depth)?;
        Ok(())
    }
}

/// Camera view, projection, and combined view-projection uniforms (`math3d::Mat4`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CameraUniforms {
    /// World-to-camera view matrix.
    pub view: Mat4,
    /// Camera-to-clip projection matrix.
    pub projection: Mat4,
    /// Pre-multiplied `projection * view` matrix.
    pub view_projection: Mat4,
    /// Camera world-space position.
    pub camera_position: Vec3,
}

impl CameraUniforms {
    /// Build validated [`CameraUniforms`] from `view`, `projection`, and `camera_position`.
    pub fn camera_from_view_proj(
        view: Mat4,
        projection: Mat4,
        camera_position: Vec3,
    ) -> Result<Self, BridgeError> {
        check_mat4_finite(&view)?;
        check_mat4_finite(&projection)?;
        if !camera_position.vec3_is_finite() {
            return Err(BridgeError::NonFiniteInput);
        }
        check_bounded_scalar(camera_position.x)?;
        check_bounded_scalar(camera_position.y)?;
        check_bounded_scalar(camera_position.z)?;
        let view_projection = projection.mul(&view)?;
        check_mat4_finite(&view_projection)?;
        Ok(Self {
            view,
            projection,
            view_projection,
            camera_position,
        })
    }

    /// Build validated [`CameraUniforms`] from a single combined `view_projection` matrix.
    pub fn camera_from_combined(
        view_projection: Mat4,
        camera_position: Vec3,
    ) -> Result<Self, BridgeError> {
        Self::camera_from_view_proj(Mat4::mat4_identity(), view_projection, camera_position)
    }

    /// Validate all matrix entries and `camera_position` for finiteness and consistency.
    pub fn camera_validate(&self) -> Result<(), BridgeError> {
        check_mat4_finite(&self.view)?;
        check_mat4_finite(&self.projection)?;
        check_mat4_finite(&self.view_projection)?;
        if !self.camera_position.vec3_is_finite() {
            return Err(BridgeError::NonFiniteInput);
        }
        check_bounded_scalar(self.camera_position.x)?;
        check_bounded_scalar(self.camera_position.y)?;
        check_bounded_scalar(self.camera_position.z)?;
        Ok(())
    }

    /// Serialize `view_projection` and `model` into a deterministic 128-byte `f32` GPU uniform buffer.
    pub fn camera_to_uniform_bytes(&self, model: &Mat4) -> Result<CameraUniformBytes, BridgeError> {
        self.camera_validate()?;
        check_mat4_finite(model)?;
        let vp_f32 = self.view_projection.mat4_to_f32_flat()?;
        let model_f32 = model.mat4_to_f32_flat()?;
        let mut out = [0u8; CAMERA_UNIFORM_BYTES];
        for (i, &v) in vp_f32.iter().enumerate() {
            let b = v.to_le_bytes();
            let base = i * 4;
            out[base] = b[0];
            out[base + 1] = b[1];
            out[base + 2] = b[2];
            out[base + 3] = b[3];
        }
        for (i, &v) in model_f32.iter().enumerate() {
            let b = v.to_le_bytes();
            let base = 64 + i * 4;
            out[base] = b[0];
            out[base + 1] = b[1];
            out[base + 2] = b[2];
            out[base + 3] = b[3];
        }
        Ok(out)
    }
}

/// Borrowed immutable indexed triangle mesh slice (`raster3d_soft::Vertex` + `&[u32]`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MeshSlice<'a> {
    /// Vertex buffer slice.
    pub vertices: &'a [Vertex],
    /// Triangle index buffer (`3 * num_triangles` indices).
    pub indices: &'a [u32],
}

impl<'a> MeshSlice<'a> {
    /// Construct and validate a [`MeshSlice`].
    pub fn mesh_slice_new(vertices: &'a [Vertex], indices: &'a [u32]) -> Result<Self, BridgeError> {
        let slice = Self { vertices, indices };
        slice.mesh_slice_validate()?;
        Ok(slice)
    }

    /// Validate vertex bounds, index count divisibility by 3, triangle budget, and every index (CWE-20 / CWE-125 / CWE-400).
    pub fn mesh_slice_validate(&self) -> Result<(), BridgeError> {
        if self.vertices.len() > MAX_VERTICES_PER_MESH {
            return Err(BridgeError::BudgetExceeded);
        }
        let (tri_chunks, rem) = self.indices.as_chunks::<3>();
        if !rem.is_empty() {
            return Err(BridgeError::InvalidIndexRange);
        }
        validate_total_triangles(tri_chunks.len())?;
        for v in self.vertices {
            v.vertex_validate()?;
        }
        for tri in tri_chunks {
            for &idx in tri {
                if (idx as usize) >= self.vertices.len() {
                    return Err(BridgeError::VertexIndexOutOfBounds);
                }
            }
        }
        Ok(())
    }
}

/// Material and rasterizer pipeline state bound to a [`DrawCall`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MaterialBinding<'a> {
    /// Optional RGBA8 texture bound for `TexturedNearest` or `TexturedBilinear` modes.
    pub texture: Option<Texture<'a>>,
    /// Triangle shading and texturing mode.
    pub shading: ShadingMode,
    /// Face culling mode.
    pub cull: CullMode,
    /// Front-facing triangle winding order.
    pub front_face: FrontFace,
    /// Texture UV wrap behavior.
    pub texture_wrap: TextureWrap,
    /// Whether fragments test against the Z-buffer.
    pub depth_test: bool,
    /// Whether passing fragments update the Z-buffer.
    pub depth_write: bool,
    /// Depth comparison predicate.
    pub depth_func: DepthFunc,
}

impl<'a> MaterialBinding<'a> {
    /// Construct a non-textured [`MaterialBinding`] with standard depth-testing defaults.
    pub const fn material_new(shading: ShadingMode) -> Self {
        Self {
            texture: None,
            shading,
            cull: CullMode::Back,
            front_face: FrontFace::CounterClockwise,
            texture_wrap: TextureWrap::Repeat,
            depth_test: true,
            depth_write: true,
            depth_func: DepthFunc::Less,
        }
    }

    /// Construct a textured [`MaterialBinding`] with `texture` bound.
    pub const fn material_with_texture(shading: ShadingMode, texture: Texture<'a>) -> Self {
        Self {
            texture: Some(texture),
            shading,
            cull: CullMode::Back,
            front_face: FrontFace::CounterClockwise,
            texture_wrap: TextureWrap::Repeat,
            depth_test: true,
            depth_write: true,
            depth_func: DepthFunc::Less,
        }
    }

    /// Validate that textured shading modes have a bound texture.
    pub fn material_validate(&self) -> Result<(), BridgeError> {
        if matches!(
            self.shading,
            ShadingMode::TexturedNearest | ShadingMode::TexturedBilinear
        ) && self.texture.is_none()
        {
            return Err(BridgeError::MissingTexture);
        }
        Ok(())
    }
}

/// A single indexed draw command inside a [`RenderPass`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DrawCall {
    /// Index into `RenderPass::meshes`.
    pub mesh_index: usize,
    /// Index into `RenderPass::materials`.
    pub material_index: usize,
    /// Model-to-world 4x4 transform matrix.
    pub model: Mat4,
    /// 3x3 normal matrix (`transpose(inverse(upper_mat3(model)))`).
    pub normal_matrix: Mat3,
    /// Starting index inside the target mesh's `indices` slice.
    pub index_offset: usize,
    /// Number of indices to draw (must be a multiple of 3).
    pub index_count: usize,
}

impl DrawCall {
    /// Construct a validated [`DrawCall`], computing `normal_matrix` as the inverse-transpose of `model`'s upper 3x3.
    pub fn draw_call_new(
        mesh_index: usize,
        material_index: usize,
        model: Mat4,
        index_offset: usize,
        index_count: usize,
    ) -> Result<Self, BridgeError> {
        check_mat4_finite(&model)?;
        let upper = model.upper_mat3()?;
        let normal_matrix = upper.mat3_invert()?.mat3_transpose()?;
        check_mat3_finite(&normal_matrix)?;
        if !index_count.is_multiple_of(3) || index_offset.checked_add(index_count).is_none() {
            return Err(BridgeError::InvalidIndexRange);
        }
        Ok(Self {
            mesh_index,
            material_index,
            model,
            normal_matrix,
            index_offset,
            index_count,
        })
    }

    /// Construct a validated [`DrawCall`] with an explicit `normal_matrix`.
    pub fn draw_call_with_normal_matrix(
        mesh_index: usize,
        material_index: usize,
        model: Mat4,
        normal_matrix: Mat3,
        index_offset: usize,
        index_count: usize,
    ) -> Result<Self, BridgeError> {
        check_mat4_finite(&model)?;
        check_mat3_finite(&normal_matrix)?;
        if !index_count.is_multiple_of(3) || index_offset.checked_add(index_count).is_none() {
            return Err(BridgeError::InvalidIndexRange);
        }
        Ok(Self {
            mesh_index,
            material_index,
            model,
            normal_matrix,
            index_offset,
            index_count,
        })
    }

    /// Validate matrix finiteness and index slice alignment of this draw call.
    pub fn draw_call_validate(&self) -> Result<(), BridgeError> {
        check_mat4_finite(&self.model)?;
        check_mat3_finite(&self.normal_matrix)?;
        if !self.index_count.is_multiple_of(3)
            || self.index_offset.checked_add(self.index_count).is_none()
        {
            return Err(BridgeError::InvalidIndexRange);
        }
        Ok(())
    }
}

/// Complete multi-draw 3D render pass submitted to the bridge.
#[derive(Clone, Copy, Debug)]
pub struct RenderPass<'a> {
    /// Target framebuffer dimensions, clear state, clip convention, and post-process mode.
    pub target: RenderTargetSpec,
    /// Camera view/projection uniforms.
    pub camera: CameraUniforms,
    /// Scene ambient + directional light rig.
    pub light: LightRig,
    /// Mesh table referenced by `draw_calls`.
    pub meshes: &'a [MeshSlice<'a>],
    /// Material table referenced by `draw_calls`.
    pub materials: &'a [MaterialBinding<'a>],
    /// Ordered sequence of draw calls to execute.
    pub draw_calls: &'a [DrawCall],
}

impl<'a> RenderPass<'a> {
    /// Validate the entire [`RenderPass`] atomically before any framebuffer or Z-buffer mutation.
    pub fn render_pass_validate(&self) -> Result<(), BridgeError> {
        self.target.target_spec_validate()?;
        self.camera.camera_validate()?;
        self.light.light_rig_validate()?;

        validate_draw_call_count(self.meshes.len())?;
        validate_draw_call_count(self.materials.len())?;
        validate_draw_call_count(self.draw_calls.len())?;

        for mesh in self.meshes {
            mesh.mesh_slice_validate()?;
        }
        for mat in self.materials {
            mat.material_validate()?;
        }

        let mut total_triangles = 0usize;
        for dc in self.draw_calls {
            dc.draw_call_validate()?;
            validate_handle_index(dc.mesh_index, self.meshes.len())?;
            validate_handle_index(dc.material_index, self.materials.len())?;
            let Some(mesh) = self.meshes.get(dc.mesh_index) else {
                return Err(BridgeError::InvalidHandleIndex);
            };
            validate_index_range(dc.index_offset, dc.index_count, mesh.indices.len())?;
            let mvp = self.camera.view_projection.mul(&dc.model)?;
            check_mat4_finite(&mvp)?;
            let tris = dc.index_count / 3;
            total_triangles = total_triangles
                .checked_add(tris)
                .ok_or(BridgeError::BudgetExceeded)?;
            validate_total_triangles(total_triangles)?;
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Pre-Allocated Scratch Buffer (`VER07` Zero-Allocation Steady State)
// ---------------------------------------------------------------------------

/// Reusable caller-owned scratch workspace for zero-allocation steady-state pass execution.
#[derive(Clone, Debug, Default)]
pub struct RenderBridgeScratch {
    raster: RasterScratch,
    post_rgba: Vec<u8>,
}

impl RenderBridgeScratch {
    /// Create an empty [`RenderBridgeScratch`].
    #[inline]
    pub fn bridge_scratch_new() -> Self {
        Self {
            raster: RasterScratch::scratch_new(),
            post_rgba: Vec::new(),
        }
    }

    /// Create a [`RenderBridgeScratch`] pre-allocated for `vertex_capacity` vertices and
    /// `max_width * max_height * 4` post-process staging bytes (CWE-400 bounded).
    pub fn bridge_scratch_with_capacity(
        vertex_capacity: usize,
        max_width: u32,
        max_height: u32,
    ) -> Result<Self, BridgeError> {
        validate_framebuffer_dim(max_width)?;
        validate_framebuffer_dim(max_height)?;
        let pixel_bytes = (max_width as usize)
            .checked_mul(max_height as usize)
            .and_then(|n| n.checked_mul(4))
            .ok_or(BridgeError::InvalidDimensions)?;
        let raster = RasterScratch::scratch_with_capacity(vertex_capacity)?;
        Ok(Self {
            raster,
            post_rgba: Vec::with_capacity(pixel_bytes),
        })
    }

    /// Current vertex capacity before any rasterizer scratch reallocation would occur.
    #[inline]
    pub fn bridge_scratch_vertex_capacity(&self) -> usize {
        self.raster.scratch_capacity()
    }

    /// Current post-process staging byte capacity before reallocation would occur.
    #[inline]
    pub fn bridge_scratch_pixel_byte_capacity(&self) -> usize {
        self.post_rgba.capacity()
    }

    /// Clear transient contents while retaining pre-allocated heap capacities.
    #[inline]
    pub fn bridge_scratch_reset(&mut self) {
        self.raster.scratch_clear();
        self.post_rgba.clear();
    }
}

// ---------------------------------------------------------------------------
// Execution & Canonical Digest Helpers
// ---------------------------------------------------------------------------

/// Compute a canonical 64-bit FNV-1a digest over a [`PixelContainer`].
#[inline]
pub fn bridge_frame_digest_fnv1a(framebuffer: &PixelContainer) -> u64 {
    frame_digest_fnv1a(framebuffer)
}

/// Compute a canonical 64-bit FNV-1a digest over a [`DepthBufferMut`].
#[inline]
pub fn bridge_depth_digest_fnv1a(depth: &DepthBufferMut<'_>) -> u64 {
    match depth {
        DepthBufferMut::F64(slice) => depth_digest_f64_fnv1a(slice),
        DepthBufferMut::F32(slice) => depth_digest_f32_fnv1a(slice),
    }
}

fn validate_targets_and_pass(
    pass: &RenderPass<'_>,
    framebuffer: &PixelContainer,
    depth: &DepthBufferMut<'_>,
) -> Result<(u32, u32, usize), BridgeError> {
    pass.render_pass_validate()?;
    if framebuffer.width != pass.target.width || framebuffer.height != pass.target.height {
        return Err(BridgeError::InvalidDimensions);
    }
    let pixel_count = (pass.target.width as usize)
        .checked_mul(pass.target.height as usize)
        .ok_or(BridgeError::InvalidDimensions)?;
    let expected_bytes = pixel_count
        .checked_mul(4)
        .ok_or(BridgeError::InvalidDimensions)?;
    if framebuffer.data.len() != expected_bytes || depth.depth_mut_len() != pixel_count {
        return Err(BridgeError::InvalidDimensions);
    }
    if matches!(depth, DepthBufferMut::F32(_)) {
        let d32 = pass.target.clear_depth as f32;
        if !d32.is_finite() {
            return Err(BridgeError::ValueOutOfBounds);
        }
    }
    Ok((pass.target.width, pass.target.height, pixel_count))
}

fn resolve_backend_plan(
    preference: BackendPreference,
    runtime: Option<&GpuRuntime>,
) -> Result<(RenderBackendKind, bool, FallbackReason), BridgeError> {
    match preference {
        BackendPreference::ForceSoftware => Ok((
            RenderBackendKind::Software,
            false,
            FallbackReason::NoFallback,
        )),
        BackendPreference::ForceGpuRuntimeCpu => Ok((
            RenderBackendKind::GpuRuntimeCpu,
            false,
            FallbackReason::NoFallback,
        )),
        BackendPreference::AutoHardwareWithSoftwareFallback => {
            let Some(rt) = runtime else {
                return Ok((
                    RenderBackendKind::Software,
                    true,
                    FallbackReason::NoHardwareRuntimeProvided,
                ));
            };
            match rt.backend() {
                GpuRuntimeBackendKind::Metal => Ok((
                    RenderBackendKind::GpuRuntimeMetal,
                    false,
                    FallbackReason::NoFallback,
                )),
                GpuRuntimeBackendKind::Cuda => Ok((
                    RenderBackendKind::GpuRuntimeCuda,
                    false,
                    FallbackReason::NoFallback,
                )),
                GpuRuntimeBackendKind::Cpu => Ok((
                    RenderBackendKind::Software,
                    true,
                    FallbackReason::HardwareDriverUnavailable,
                )),
            }
        }
        BackendPreference::RequireHardware(kind) => {
            if !kind.backend_is_hardware() {
                return Err(BridgeError::RequestedBackendUnavailable(kind));
            }
            let Some(rt) = runtime else {
                return Err(BridgeError::RequestedBackendUnavailable(kind));
            };
            let matches_rt = matches!(
                (kind, rt.backend()),
                (
                    RenderBackendKind::GpuRuntimeMetal | RenderBackendKind::Metal,
                    GpuRuntimeBackendKind::Metal
                ) | (
                    RenderBackendKind::GpuRuntimeCuda,
                    GpuRuntimeBackendKind::Cuda
                )
            );
            if matches_rt {
                Ok((kind, false, FallbackReason::NoFallback))
            } else {
                Err(BridgeError::RequestedBackendUnavailable(kind))
            }
        }
    }
}

fn rasterize_validated_pass(
    pass: &RenderPass<'_>,
    framebuffer: &mut PixelContainer,
    depth: &mut DepthBufferMut<'_>,
    scratch: &mut RenderBridgeScratch,
) -> Result<RasterStats, BridgeError> {
    clear_framebuffer(framebuffer, pass.target.clear_color)?;
    depth.depth_mut_clear(pass.target.clear_depth)?;

    let mut total_stats = RasterStats::default();
    for dc in pass.draw_calls {
        let Some(mesh) = pass.meshes.get(dc.mesh_index) else {
            return Err(BridgeError::InvalidHandleIndex);
        };
        let Some(mat) = pass.materials.get(dc.material_index) else {
            return Err(BridgeError::InvalidHandleIndex);
        };
        let end = dc
            .index_offset
            .checked_add(dc.index_count)
            .ok_or(BridgeError::InvalidIndexRange)?;
        let Some(sub_indices) = mesh.indices.get(dc.index_offset..end) else {
            return Err(BridgeError::InvalidIndexRange);
        };

        let mvp = pass.camera.view_projection.mul(&dc.model)?;
        let cfg = RasterConfig {
            mvp,
            normal_matrix: dc.normal_matrix,
            shading: mat.shading,
            cull: mat.cull,
            front_face: mat.front_face,
            texture_wrap: mat.texture_wrap,
            clip_space: pass.target.clip_space,
            depth_test: mat.depth_test,
            depth_write: mat.depth_write,
            depth_func: mat.depth_func,
            light: pass.light,
        };

        let sub_depth = match depth {
            DepthBufferMut::F64(slice) => DepthBufferMut::F64(slice),
            DepthBufferMut::F32(slice) => DepthBufferMut::F32(slice),
        };
        let stats = draw_indexed_mesh_into(
            framebuffer,
            sub_depth,
            &cfg,
            mat.texture.as_ref(),
            mesh.vertices,
            sub_indices,
            &mut scratch.raster,
        )?;
        total_stats.triangles_submitted = total_stats
            .triangles_submitted
            .saturating_add(stats.triangles_submitted);
        total_stats.triangles_culled = total_stats
            .triangles_culled
            .saturating_add(stats.triangles_culled);
        total_stats.triangles_clipped_out = total_stats
            .triangles_clipped_out
            .saturating_add(stats.triangles_clipped_out);
        total_stats.triangles_rasterized = total_stats
            .triangles_rasterized
            .saturating_add(stats.triangles_rasterized);
        total_stats.pixels_written = total_stats
            .pixels_written
            .saturating_add(stats.pixels_written);
    }

    Ok(total_stats)
}

fn apply_postprocess_zero_alloc(
    mode: PostProcessMode,
    pixel_count: usize,
    framebuffer: &mut PixelContainer,
    scratch: &mut RenderBridgeScratch,
) {
    if mode == PostProcessMode::Identity {
        return;
    }
    let byte_len = framebuffer.data.len();
    if scratch.post_rgba.capacity() < byte_len {
        scratch
            .post_rgba
            .reserve(byte_len - scratch.post_rgba.len());
    }
    scratch.post_rgba.resize(byte_len, 0);
    scratch.post_rgba.copy_from_slice(&framebuffer.data);
    let uniforms = mode.encode_uniforms(pixel_count as u32);
    postprocess_cpu_kernel(&scratch.post_rgba, &mut framebuffer.data, &uniforms);
}

/// Execute a [`RenderPass`] directly into caller-owned `framebuffer`, `depth`, and `scratch`.
///
/// ## Guarantees
/// - **Multi-draw atomicity**: validates the entire `RenderPass`, all `DrawCall`s,
///   buffer dimensions, and `BackendPreference` **before** clearing or writing
///   `framebuffer`, `depth`, or `scratch`.
/// - **Zero-allocation steady state (`VER07`)**: when `scratch` is pre-sized via
///   [`RenderBridgeScratch::bridge_scratch_with_capacity`], this function performs
///   `0` heap allocations for `ForceSoftware`, `AutoHardwareWithSoftwareFallback`,
///   and `ForceGpuRuntimeCpu`.
pub fn execute_render_pass_into(
    pass: &RenderPass<'_>,
    preference: BackendPreference,
    framebuffer: &mut PixelContainer,
    mut depth: DepthBufferMut<'_>,
    scratch: &mut RenderBridgeScratch,
) -> Result<RenderExecutionReport, BridgeError> {
    let (_w, _h, pixel_count) = validate_targets_and_pass(pass, framebuffer, &depth)?;
    let (active_backend, fell_back, fallback_reason) = resolve_backend_plan(preference, None)?;

    let stats = rasterize_validated_pass(pass, framebuffer, &mut depth, scratch)?;

    match active_backend {
        RenderBackendKind::Software => {
            apply_postprocess_zero_alloc(
                pass.target.post_process,
                pixel_count,
                framebuffer,
                scratch,
            );
        }
        RenderBackendKind::GpuRuntimeCpu => {
            let byte_len = framebuffer.data.len();
            if scratch.post_rgba.capacity() < byte_len {
                scratch
                    .post_rgba
                    .reserve(byte_len - scratch.post_rgba.len());
            }
            scratch.post_rgba.resize(byte_len, 0);
            scratch.post_rgba.copy_from_slice(&framebuffer.data);
            let uniforms = pass.target.post_process.encode_uniforms(pixel_count as u32);
            if let Some(cpu_fn) = BRIDGE_POSTPROCESS_SHADERS.cpu {
                cpu_fn(&scratch.post_rgba, &mut framebuffer.data, &uniforms);
            }
        }
        _ => {
            apply_postprocess_zero_alloc(
                pass.target.post_process,
                pixel_count,
                framebuffer,
                scratch,
            );
        }
    }

    let frame_digest = bridge_frame_digest_fnv1a(framebuffer);
    let depth_digest = bridge_depth_digest_fnv1a(&depth);

    Ok(RenderExecutionReport {
        active_backend,
        fell_back,
        fallback_reason,
        stats,
        frame_digest,
        depth_digest,
    })
}

/// Execute a [`RenderPass`] using an explicit caller-supplied [`GpuRuntime`] for the compute post-process stage.
///
/// Prefer passing `&GpuRuntime::cpu_only()` in sandboxed/headless environments to avoid
/// GPU driver `dlopen` probes inside seccomp `RCG001`.
pub fn execute_render_pass_with_runtime(
    pass: &RenderPass<'_>,
    preference: BackendPreference,
    runtime: Option<&GpuRuntime>,
    framebuffer: &mut PixelContainer,
    mut depth: DepthBufferMut<'_>,
    scratch: &mut RenderBridgeScratch,
) -> Result<RenderExecutionReport, BridgeError> {
    let (_w, _h, pixel_count) = validate_targets_and_pass(pass, framebuffer, &depth)?;
    let (mut active_backend, mut fell_back, mut fallback_reason) =
        resolve_backend_plan(preference, runtime)?;

    let stats = rasterize_validated_pass(pass, framebuffer, &mut depth, scratch)?;

    let uniforms = pass.target.post_process.encode_uniforms(pixel_count as u32);
    match active_backend {
        RenderBackendKind::Software => {
            apply_postprocess_zero_alloc(
                pass.target.post_process,
                pixel_count,
                framebuffer,
                scratch,
            );
        }
        RenderBackendKind::GpuRuntimeCpu => {
            if let Some(rt) = runtime {
                let resolved = rt
                    .run_pixels(
                        &BRIDGE_POSTPROCESS_SHADERS,
                        "bridge_postprocess",
                        &framebuffer.data,
                        &uniforms,
                        pixel_count,
                    )
                    .map_err(|_| BridgeError::GpuRuntimeFailure)?;
                if resolved.len() != framebuffer.data.len() {
                    return Err(BridgeError::InvalidDimensions);
                }
                framebuffer.data.copy_from_slice(&resolved);
            } else {
                apply_postprocess_zero_alloc(
                    pass.target.post_process,
                    pixel_count,
                    framebuffer,
                    scratch,
                );
            }
        }
        RenderBackendKind::GpuRuntimeMetal
        | RenderBackendKind::GpuRuntimeCuda
        | RenderBackendKind::Metal => {
            if let Some(rt) = runtime {
                match rt.run_pixels(
                    &BRIDGE_POSTPROCESS_SHADERS,
                    "bridge_postprocess",
                    &framebuffer.data,
                    &uniforms,
                    pixel_count,
                ) {
                    Ok(resolved) if resolved.len() == framebuffer.data.len() => {
                        framebuffer.data.copy_from_slice(&resolved);
                    }
                    _ => {
                        if matches!(preference, BackendPreference::RequireHardware(_)) {
                            return Err(BridgeError::GpuRuntimeFailure);
                        }
                        active_backend = RenderBackendKind::Software;
                        fell_back = true;
                        fallback_reason = FallbackReason::PostProcessParityFallback;
                        apply_postprocess_zero_alloc(
                            pass.target.post_process,
                            pixel_count,
                            framebuffer,
                            scratch,
                        );
                    }
                }
            }
        }
        RenderBackendKind::Wgpu | RenderBackendKind::Vulkan => {
            if matches!(preference, BackendPreference::RequireHardware(_)) {
                return Err(BridgeError::RequestedBackendUnavailable(active_backend));
            }
            active_backend = RenderBackendKind::Software;
            fell_back = true;
            fallback_reason = FallbackReason::HardwareDriverUnavailable;
            apply_postprocess_zero_alloc(
                pass.target.post_process,
                pixel_count,
                framebuffer,
                scratch,
            );
        }
    }

    let frame_digest = bridge_frame_digest_fnv1a(framebuffer);
    let depth_digest = bridge_depth_digest_fnv1a(&depth);

    Ok(RenderExecutionReport {
        active_backend,
        fell_back,
        fallback_reason,
        stats,
        frame_digest,
        depth_digest,
    })
}

/// Resolve an `f32` Z-buffer slice into a grayscale RGBA8 [`PixelContainer`] via [`GpuRuntime`].
///
/// Validates target dimensions and buffer lengths before invoking `runtime.run_pixels(...)`.
pub fn resolve_depth_to_rgba_with_runtime(
    runtime: &GpuRuntime,
    width: u32,
    height: u32,
    depth_f32: &[f32],
    out_framebuffer: &mut PixelContainer,
) -> Result<(), BridgeError> {
    validate_framebuffer_dim(width)?;
    validate_framebuffer_dim(height)?;
    let pixel_count = (width as usize)
        .checked_mul(height as usize)
        .ok_or(BridgeError::InvalidDimensions)?;
    let expected_bytes = pixel_count
        .checked_mul(4)
        .ok_or(BridgeError::InvalidDimensions)?;
    if depth_f32.len() != pixel_count
        || out_framebuffer.width != width
        || out_framebuffer.height != height
        || out_framebuffer.data.len() != expected_bytes
    {
        return Err(BridgeError::InvalidDimensions);
    }

    let mut depth_bytes = vec![0u8; expected_bytes];
    for (i, &d) in depth_f32.iter().enumerate() {
        let b = d.to_le_bytes();
        let base = i * 4;
        depth_bytes[base] = b[0];
        depth_bytes[base + 1] = b[1];
        depth_bytes[base + 2] = b[2];
        depth_bytes[base + 3] = b[3];
    }
    let uniforms = (pixel_count as u32).to_le_bytes();
    let resolved = runtime
        .run_pixels(
            &BRIDGE_DEPTH_RESOLVE_SHADERS,
            "bridge_depth_resolve",
            &depth_bytes,
            &uniforms,
            pixel_count,
        )
        .map_err(|_| BridgeError::GpuRuntimeFailure)?;
    if resolved.len() != expected_bytes {
        return Err(BridgeError::InvalidDimensions);
    }
    out_framebuffer.data.copy_from_slice(&resolved);
    Ok(())
}
