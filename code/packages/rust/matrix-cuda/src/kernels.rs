//! # CUDA C kernels for the V1 op set
//!
//! MX06 Phase 3.  Mirrors `matrix-metal::kernels` but in CUDA C
//! compiled at runtime through NVRTC (via `cuda-compute`'s
//! `CudaDevice::compile`).  V1 ops are F32-only:
//!
//! - Elementwise unary (op tags `0x00..=0x06`): `Neg`, `Abs`, `Sqrt`,
//!   `Exp`, `Log`, `Tanh`, `Recip`.
//! - Elementwise binary (op tags `0x07..=0x0D`): `Add`, `Sub`, `Mul`,
//!   `Div`, `Max`, `Min`, `Pow`.
//! - `MatMul` (op tag `0x15`, rank-2, row-major).
//! - `Const` (op tag `0x1B`) is handled by buffer upload, not by a
//!   kernel — see `BufferStore::write`.
//!
//! Everything else (reductions, casts, shape ops, integer dtypes)
//! is V2 work; the planner's capability filter falls back to CPU.
//!
//! ## Why compile at executor startup
//!
//! We compile the entire CUDA C source once on first
//! [`Kernels::new`] call.  NVRTC compilation is ~100 ms on a modern
//! card — paying it once at startup keeps dispatches latency-free.
//! Matches `matrix-metal`'s startup-compile strategy.
//!
//! ## Threadblock sizing
//!
//! Elementwise kernels launch with a 1-D grid of 256-thread blocks.
//! MatMul launches with a 2-D grid of 16×16 blocks.  These are
//! conservative defaults; Phase 7 (planner integration) will revisit
//! once we have a calibration workload.

use cuda_compute::{CudaBuffer, CudaDevice, CudaFunction, CudaModule};
use std::collections::HashMap;
use std::ffi::c_void;

/// The bundled CUDA C source for every V1 kernel.  Compiled once at
/// executor startup; the resulting [`CudaModule`] is cached for the
/// lifetime of the executor.
pub const KERNELS_CUDA_C: &str = r#"
// ──────────────── elementwise unary (F32) ────────────────

#define UNARY_F32(NAME, EXPR) \
extern "C" __global__ void NAME ( \
    const float* __restrict__ in, \
    float* __restrict__ out, \
    unsigned int n \
) { \
    unsigned int gid = blockIdx.x * blockDim.x + threadIdx.x; \
    if (gid >= n) return; \
    float x = in[gid]; \
    out[gid] = (EXPR); \
}

UNARY_F32(neg_f32,   -x)
UNARY_F32(abs_f32,   fabsf(x))
UNARY_F32(sqrt_f32,  sqrtf(x))
UNARY_F32(exp_f32,   expf(x))
UNARY_F32(log_f32,   logf(x))
UNARY_F32(tanh_f32,  tanhf(x))
UNARY_F32(recip_f32, 1.0f / x)

// ──────────────── elementwise binary (F32) ────────────────

#define BINARY_F32(NAME, EXPR) \
extern "C" __global__ void NAME ( \
    const float* __restrict__ a, \
    const float* __restrict__ b, \
    float* __restrict__ out, \
    unsigned int n \
) { \
    unsigned int gid = blockIdx.x * blockDim.x + threadIdx.x; \
    if (gid >= n) return; \
    float x = a[gid]; \
    float y = b[gid]; \
    out[gid] = (EXPR); \
}

BINARY_F32(add_f32, x + y)
BINARY_F32(sub_f32, x - y)
BINARY_F32(mul_f32, x * y)
BINARY_F32(div_f32, x / y)
BINARY_F32(max_f32, fmaxf(x, y))
BINARY_F32(min_f32, fminf(x, y))
BINARY_F32(pow_f32, powf(x, y))

// ──────────────── matmul (F32, rank-2, row-major) ────────────────
//
// One thread per output element.  c[i,j] = sum_kk a[i,kk] * b[kk,j].
// 2-D grid; 16×16 block by convention.  Phase 7 may tune.

extern "C" __global__ void matmul_f32(
    const float* __restrict__ a,
    const float* __restrict__ b,
    float* __restrict__ c,
    unsigned int m,
    unsigned int k,
    unsigned int n
) {
    unsigned int i = blockIdx.y * blockDim.y + threadIdx.y;
    unsigned int j = blockIdx.x * blockDim.x + threadIdx.x;
    if (i >= m || j >= n) return;
    float acc = 0.0f;
    for (unsigned int kk = 0; kk < k; ++kk) {
        acc += a[i * k + kk] * b[kk * n + j];
    }
    c[i * n + j] = acc;
}
"#;

/// Every kernel entry-point name we expect to find in
/// [`KERNELS_CUDA_C`].  Adding a kernel?  Add to both this list and
/// the source string above — [`Kernels::new`] returns an error if any
/// name fails to resolve.
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

/// Compiled-and-cached kernel set for one [`CudaDevice`].
///
/// Owned by `CudaExecutor::State`.  Construction is the slow part
/// (NVRTC compile); subsequent kernel lookups are `HashMap` reads.
pub struct Kernels {
    /// The compiled module is kept alive so the functions remain
    /// valid for the executor's lifetime.  `CudaFunction` internally
    /// shares an `Arc<CudaModuleInner>`, so this field is partly
    /// redundant — but explicit ownership reads better.
    _module: CudaModule,
    /// Lookup table from kernel entry-point name → cached function
    /// handle.  Populated once at construction.
    fns: HashMap<&'static str, CudaFunction>,
}

impl Kernels {
    /// Compile [`KERNELS_CUDA_C`] on the given device and cache one
    /// [`CudaFunction`] per entry in [`KERNEL_ENTRY_POINTS`].
    ///
    /// Returns `Err(String)` if compilation or function lookup fails.
    /// The error message includes the entry-point name so failures
    /// to add a new kernel to one of the two lists surface clearly.
    pub fn new(device: &CudaDevice) -> Result<Self, String> {
        let module = device
            .compile(KERNELS_CUDA_C)
            .map_err(|e| format!("matrix-cuda: NVRTC compile: {:?}", e))?;
        let mut fns: HashMap<&'static str, CudaFunction> =
            HashMap::with_capacity(KERNEL_ENTRY_POINTS.len());
        for &name in KERNEL_ENTRY_POINTS {
            let f = module
                .function(name)
                .map_err(|e| format!("matrix-cuda: function {}: {:?}", name, e))?;
            fns.insert(name, f);
        }
        Ok(Kernels {
            _module: module,
            fns,
        })
    }

    /// Look up a cached function by entry-point name.  Returns
    /// `Err(String)` if the name isn't in [`KERNEL_ENTRY_POINTS`] —
    /// callers must use one of the documented kernel names.
    pub fn get(&self, name: &str) -> Result<&CudaFunction, String> {
        self.fns
            .get(name)
            .ok_or_else(|| format!("matrix-cuda: unknown kernel '{}'", name))
    }

    /// Dispatch one of the elementwise-unary kernels.  `n` is the
    /// element count; both buffers must hold at least `n * 4` bytes.
    ///
    /// Used by Phase 3+ dispatch code and by the device-gated
    /// tests in this module.
    pub fn launch_unary(
        &self,
        device: &CudaDevice,
        name: &str,
        input: &mut CudaBuffer,
        output: &mut CudaBuffer,
        n: u32,
    ) -> Result<(), String> {
        let func = self.get(name)?;
        let block: [u32; 3] = [256, 1, 1];
        let grid: [u32; 3] = [n.div_ceil(block[0]).max(1), 1, 1];
        // SAFETY: pointers below remain valid until launch returns.
        // We assemble the args array and call launch in the same
        // expression — no moves of `input`/`output`/`n` in between.
        unsafe {
            let mut n_local = n;
            let mut args: [*mut c_void; 3] = [
                input.as_kernel_arg(),
                output.as_kernel_arg(),
                &mut n_local as *mut u32 as *mut c_void,
            ];
            device
                .launch(func, grid, block, &mut args)
                .map_err(|e| format!("launch {}: {:?}", name, e))?;
        }
        device
            .synchronize()
            .map_err(|e| format!("synchronize after {}: {:?}", name, e))
    }

    /// Dispatch one of the elementwise-binary kernels.  `n` is the
    /// element count; all three buffers must hold at least `n * 4`
    /// bytes.
    pub fn launch_binary(
        &self,
        device: &CudaDevice,
        name: &str,
        a: &mut CudaBuffer,
        b: &mut CudaBuffer,
        output: &mut CudaBuffer,
        n: u32,
    ) -> Result<(), String> {
        let func = self.get(name)?;
        let block: [u32; 3] = [256, 1, 1];
        let grid: [u32; 3] = [n.div_ceil(block[0]).max(1), 1, 1];
        unsafe {
            let mut n_local = n;
            let mut args: [*mut c_void; 4] = [
                a.as_kernel_arg(),
                b.as_kernel_arg(),
                output.as_kernel_arg(),
                &mut n_local as *mut u32 as *mut c_void,
            ];
            device
                .launch(func, grid, block, &mut args)
                .map_err(|e| format!("launch {}: {:?}", name, e))?;
        }
        device
            .synchronize()
            .map_err(|e| format!("synchronize after {}: {:?}", name, e))
    }

    /// Dispatch the rank-2 row-major MatMul: `c = a * b` where
    /// `a` is `[m, k]`, `b` is `[k, n]`, and `c` is `[m, n]`.
    ///
    /// Buffers must be F32 of the correct sizes; this method doesn't
    /// validate (the executor's pre-dispatch validation is upstream).
    pub fn launch_matmul(
        &self,
        device: &CudaDevice,
        a: &mut CudaBuffer,
        b: &mut CudaBuffer,
        c: &mut CudaBuffer,
        m: u32,
        k: u32,
        n: u32,
    ) -> Result<(), String> {
        let func = self.get("matmul_f32")?;
        let block: [u32; 3] = [16, 16, 1];
        let grid: [u32; 3] = [n.div_ceil(block[0]).max(1), m.div_ceil(block[1]).max(1), 1];
        unsafe {
            let mut m_local = m;
            let mut k_local = k;
            let mut n_local = n;
            let mut args: [*mut c_void; 6] = [
                a.as_kernel_arg(),
                b.as_kernel_arg(),
                c.as_kernel_arg(),
                &mut m_local as *mut u32 as *mut c_void,
                &mut k_local as *mut u32 as *mut c_void,
                &mut n_local as *mut u32 as *mut c_void,
            ];
            device
                .launch(func, grid, block, &mut args)
                .map_err(|e| format!("launch matmul_f32: {:?}", e))?;
        }
        device
            .synchronize()
            .map_err(|e| format!("synchronize after matmul_f32: {:?}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn device_or_skip() -> Option<CudaDevice> {
        CudaDevice::new(0).ok()
    }

    /// Round-trip a single elementwise kernel through the CUDA
    /// device, comparing to a CPU oracle.  Used by every unary /
    /// binary kernel test.
    fn round_trip_unary(name: &str, input: &[f32], expect_fn: impl Fn(f32) -> f32) {
        let Some(device) = device_or_skip() else {
            return;
        };
        let kernels = Kernels::new(&device).expect("compile kernels");

        let n = input.len() as u32;
        let bytes = (n as usize) * 4;
        let mut in_buf = device.alloc(bytes).unwrap();
        let mut out_buf = device.alloc(bytes).unwrap();
        device
            .upload(&in_buf, bytemuck_like(input))
            .unwrap();

        kernels
            .launch_unary(&device, name, &mut in_buf, &mut out_buf, n)
            .unwrap();

        let got_bytes = device.download(&out_buf).unwrap();
        let got = bytes_to_f32(&got_bytes);

        for (i, (g, x)) in got.iter().zip(input.iter()).enumerate() {
            let expected = expect_fn(*x);
            assert!(
                approx_eq(*g, expected),
                "{} mismatch at {}: got {}, expected {} (input {})",
                name,
                i,
                g,
                expected,
                x
            );
        }
    }

    fn round_trip_binary(
        name: &str,
        a: &[f32],
        b: &[f32],
        expect_fn: impl Fn(f32, f32) -> f32,
    ) {
        let Some(device) = device_or_skip() else {
            return;
        };
        let kernels = Kernels::new(&device).expect("compile kernels");

        let n = a.len() as u32;
        let bytes = (n as usize) * 4;
        let mut a_buf = device.alloc(bytes).unwrap();
        let mut b_buf = device.alloc(bytes).unwrap();
        let mut out_buf = device.alloc(bytes).unwrap();
        device.upload(&a_buf, bytemuck_like(a)).unwrap();
        device.upload(&b_buf, bytemuck_like(b)).unwrap();

        kernels
            .launch_binary(&device, name, &mut a_buf, &mut b_buf, &mut out_buf, n)
            .unwrap();

        let got_bytes = device.download(&out_buf).unwrap();
        let got = bytes_to_f32(&got_bytes);

        for (i, ((g, x), y)) in got.iter().zip(a.iter()).zip(b.iter()).enumerate() {
            let expected = expect_fn(*x, *y);
            assert!(
                approx_eq(*g, expected),
                "{} mismatch at {}: got {}, expected {} (a={}, b={})",
                name,
                i,
                g,
                expected,
                x,
                y
            );
        }
    }

    /// Hand-rolled bytemuck — we don't want to pull in the bytemuck
    /// dep just for tests.  Same memory layout, no transmute fancy.
    fn bytemuck_like(xs: &[f32]) -> &[u8] {
        // SAFETY: f32 has size 4 and alignment 4; reading its bytes
        // is well-defined.  Lifetime tied to input slice.
        unsafe { std::slice::from_raw_parts(xs.as_ptr() as *const u8, xs.len() * 4) }
    }

    fn bytes_to_f32(bytes: &[u8]) -> Vec<f32> {
        assert_eq!(bytes.len() % 4, 0);
        let mut out = Vec::with_capacity(bytes.len() / 4);
        for chunk in bytes.chunks_exact(4) {
            out.push(f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
        }
        out
    }

    /// Loose float equality — kernels run in single precision and
    /// transcendentals can vary by a few ULPs vs the host's libm.
    fn approx_eq(a: f32, b: f32) -> bool {
        if a.is_nan() && b.is_nan() {
            return true;
        }
        if a.is_infinite() && b.is_infinite() && a.signum() == b.signum() {
            return true;
        }
        let diff = (a - b).abs();
        let scale = a.abs().max(b.abs()).max(1.0);
        diff <= scale * 1.0e-4
    }

    /// **MX06 Phase 5a invariant.**  `Kernels` must be `Send + Sync`
    /// so it can live in `CudaExecutor::State` behind a `Mutex<State>`.
    /// Verified via cuda-compute 0.1.2's Send + Sync impls on
    /// `CudaModuleInner` and `CudaFunction`.
    ///
    /// Compile-only — no runtime cost; if Kernels ever drops Send or
    /// Sync this test breaks the build, which is exactly what we want
    /// before Phase 5b tries to wire Dispatch.
    #[test]
    fn kernels_is_send_and_sync() {
        fn require_send_sync<T: Send + Sync>() {}
        require_send_sync::<Kernels>();
    }

    // ── kernels compile ─────────────────────────────────────────────

    #[test]
    fn kernels_new_compiles_all_entry_points() {
        let Some(device) = device_or_skip() else {
            return;
        };
        let kernels = Kernels::new(&device).unwrap();
        for &name in KERNEL_ENTRY_POINTS {
            assert!(kernels.get(name).is_ok(), "missing {}", name);
        }
    }

    #[test]
    fn unknown_kernel_name_errors() {
        let Some(device) = device_or_skip() else {
            return;
        };
        let kernels = Kernels::new(&device).unwrap();
        assert!(kernels.get("not_a_real_kernel").is_err());
    }

    // ── unary kernels ───────────────────────────────────────────────

    #[test]
    fn neg_f32_matches_cpu() {
        round_trip_unary("neg_f32", &[1.0, -2.0, 3.5, 0.0], |x| -x);
    }

    #[test]
    fn abs_f32_matches_cpu() {
        round_trip_unary("abs_f32", &[1.0, -2.0, -3.5, 0.0, -0.0], |x| x.abs());
    }

    #[test]
    fn sqrt_f32_matches_cpu() {
        round_trip_unary("sqrt_f32", &[1.0, 4.0, 9.0, 16.25], |x| x.sqrt());
    }

    #[test]
    fn exp_f32_matches_cpu() {
        round_trip_unary("exp_f32", &[0.0, 1.0, -1.0, 2.5], |x| x.exp());
    }

    #[test]
    fn log_f32_matches_cpu() {
        round_trip_unary("log_f32", &[1.0, std::f32::consts::E, 10.0, 100.0], |x| x.ln());
    }

    #[test]
    fn tanh_f32_matches_cpu() {
        round_trip_unary("tanh_f32", &[-2.0, -0.5, 0.0, 0.5, 2.0], |x| x.tanh());
    }

    #[test]
    fn recip_f32_matches_cpu() {
        round_trip_unary("recip_f32", &[1.0, 2.0, -4.0, 0.5], |x| 1.0 / x);
    }

    // ── binary kernels ──────────────────────────────────────────────

    #[test]
    fn add_f32_matches_cpu() {
        round_trip_binary("add_f32", &[1.0, 2.0, 3.0], &[10.0, 20.0, 30.0], |a, b| a + b);
    }

    #[test]
    fn sub_f32_matches_cpu() {
        round_trip_binary("sub_f32", &[5.0, 7.0, 9.0], &[1.0, 2.0, 3.0], |a, b| a - b);
    }

    #[test]
    fn mul_f32_matches_cpu() {
        round_trip_binary("mul_f32", &[2.0, 3.0, 4.0], &[5.0, 6.0, 7.0], |a, b| a * b);
    }

    #[test]
    fn div_f32_matches_cpu() {
        round_trip_binary("div_f32", &[10.0, 20.0, 30.0], &[2.0, 4.0, 5.0], |a, b| a / b);
    }

    #[test]
    fn max_f32_matches_cpu() {
        round_trip_binary("max_f32", &[1.0, 5.0, -3.0], &[2.0, 4.0, -1.0], |a, b| a.max(b));
    }

    #[test]
    fn min_f32_matches_cpu() {
        round_trip_binary("min_f32", &[1.0, 5.0, -3.0], &[2.0, 4.0, -1.0], |a, b| a.min(b));
    }

    #[test]
    fn pow_f32_matches_cpu() {
        round_trip_binary("pow_f32", &[2.0, 3.0, 4.0], &[2.0, 0.5, 3.0], |a, b| a.powf(b));
    }

    // ── matmul ──────────────────────────────────────────────────────

    #[test]
    fn matmul_f32_2x2_matches_cpu() {
        let Some(device) = device_or_skip() else {
            return;
        };
        let kernels = Kernels::new(&device).expect("compile kernels");

        // A = [[1, 2], [3, 4]], B = [[5, 6], [7, 8]] → C = [[19,22],[43,50]]
        let a: Vec<f32> = vec![1.0, 2.0, 3.0, 4.0];
        let b: Vec<f32> = vec![5.0, 6.0, 7.0, 8.0];

        let mut a_buf = device.alloc(16).unwrap();
        let mut b_buf = device.alloc(16).unwrap();
        let mut c_buf = device.alloc(16).unwrap();
        device.upload(&a_buf, bytemuck_like(&a)).unwrap();
        device.upload(&b_buf, bytemuck_like(&b)).unwrap();

        kernels
            .launch_matmul(&device, &mut a_buf, &mut b_buf, &mut c_buf, 2, 2, 2)
            .unwrap();

        let got = bytes_to_f32(&device.download(&c_buf).unwrap());
        assert!(approx_eq(got[0], 19.0));
        assert!(approx_eq(got[1], 22.0));
        assert!(approx_eq(got[2], 43.0));
        assert!(approx_eq(got[3], 50.0));
    }

    #[test]
    fn matmul_f32_3x4_4x2_matches_cpu_oracle() {
        let Some(device) = device_or_skip() else {
            return;
        };
        let kernels = Kernels::new(&device).expect("compile kernels");

        // m=3, k=4, n=2
        let a: Vec<f32> = (1..=12).map(|x| x as f32).collect(); // 3x4
        let b: Vec<f32> = (1..=8).map(|x| x as f32 * 0.5).collect(); // 4x2

        let mut a_buf = device.alloc(48).unwrap();
        let mut b_buf = device.alloc(32).unwrap();
        let mut c_buf = device.alloc(24).unwrap();
        device.upload(&a_buf, bytemuck_like(&a)).unwrap();
        device.upload(&b_buf, bytemuck_like(&b)).unwrap();

        kernels
            .launch_matmul(&device, &mut a_buf, &mut b_buf, &mut c_buf, 3, 4, 2)
            .unwrap();

        let got = bytes_to_f32(&device.download(&c_buf).unwrap());

        // CPU oracle:
        for i in 0..3 {
            for j in 0..2 {
                let mut acc = 0.0f32;
                for kk in 0..4 {
                    acc += a[i * 4 + kk] * b[kk * 2 + j];
                }
                assert!(
                    approx_eq(got[i * 2 + j], acc),
                    "matmul mismatch at ({}, {}): got {}, expected {}",
                    i,
                    j,
                    got[i * 2 + j],
                    acc
                );
            }
        }
    }
}
