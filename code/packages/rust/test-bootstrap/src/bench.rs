//! # VER07 — Microbenchmark Harness and Deterministic Cost Counters
//!
//! This module implements the zero-third-party-dependency microbenchmark harness
//! specified in [`VER07`](../../specs/VER07-microbenchmarks.md).
//!
//! ## Architectural Principles (VER00 §7.4)
//!
//! 1. **Proof vs. Measurement**: Mathematical bounds (WIR steps, memory safety,
//!    constant-time noninterference) are proved via SMT/SAT and symbolic execution.
//!    Microbenchmarks measure physical performance, calibrate timing assumptions
//!    ($t_{\text{step}}$ on platform $P$), and check for regression.
//! 2. **Shared CI Gating**: Shared CI virtual machines exhibit substantial scheduling
//!    and thermal jitter. **CI runners never gate on noisy wall-clock duration.**
//!    Instead, hard gating enforces deterministic cost counters:
//!    - Abstract step counts;
//!    - Memory allocation bytes and counts;
//!    - Hardware instructions retired (when PMU access is permitted), falling back
//!      honestly to `CounterSample::NotLive`.
//! 3. **Robust Statistics**: Avoids fragile arithmetic mean and standard deviation on
//!    asymmetric distributions. Uses Median, Median Absolute Deviation (MAD),
//!    normalized MAD (nMAD), linear-interpolated percentiles, and deterministic
//!    SplitMix64 bootstrap confidence intervals.
//!
//! ```text
//! ┌──────────────┐     ┌──────────────┐     ┌──────────────┐     ┌──────────────┐
//! │  1. Warmup   │ ──► │2. Calibrate  │ ──► │  3. Sample   │ ──► │ 4. Analysis  │
//! │  Iterations  │     │  Batch Size  │     │ Collection   │     │ & Statistics │
//! └──────────────┘     └──────────────┘     └──────────────┘     └──────────────┘
//! ```

use std::cell::Cell;
use std::hint::black_box;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use crate::counting_alloc::{is_installed, thread_stats, AllocStats};
use crate::prng::SplitMix64;

/// Multiplier for normalizing Median Absolute Deviation (MAD) to estimate
/// the standard deviation of an asymptotic normal distribution:
///
/// $$\text{nMAD} = \frac{1}{\Phi^{-1}(0.75)} \times \text{MAD} \approx 1.482602218505602 \times \text{MAD}$$
pub const MAD_NORMAL_FACTOR: f64 = 1.482_602_218_505_602;

/// A counter sample observed during benchmark execution.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CounterSample {
    /// The counter is supported and active, returning an exact integer metric.
    Live(u64),
    /// The counter is recognized on this target, but inactive or unprivileged
    /// on this host environment (e.g. PMU access blocked in CI container).
    NotLive,
    /// The metric is not supported on this operating system architecture.
    Unsupported,
}

impl CounterSample {
    /// Whether the counter produced an active live reading.
    pub const fn is_live(&self) -> bool {
        matches!(self, Self::Live(_))
    }

    /// Return the inner value if live, or `None` if not live or unsupported.
    pub const fn value(&self) -> Option<u64> {
        match *self {
            Self::Live(val) => Some(val),
            Self::NotLive | Self::Unsupported => None,
        }
    }
}

/// Abstract cost counter interface for separating deterministic execution work
/// from non-deterministic physical clocks.
pub trait CostCounter {
    /// Unique identifier for this counter (e.g. `"steps"`, `"alloc_bytes"`).
    fn name(&self) -> &'static str;
    /// Reset and arm the counter immediately prior to a sample batch.
    fn start(&mut self);
    /// Sample cumulative count since `start`.
    fn sample(&mut self) -> CounterSample;
}

impl<C: CostCounter + ?Sized> CostCounter for &mut C {
    fn name(&self) -> &'static str {
        (**self).name()
    }
    fn start(&mut self) {
        (**self).start();
    }
    fn sample(&mut self) -> CounterSample {
        (**self).sample()
    }
}

/// An in-memory deterministic step counter for abstract operations (e.g. WIR steps).
#[derive(Debug, Default)]
pub struct StepCounter {
    current: AtomicU64,
    snapshot: AtomicU64,
}

impl StepCounter {
    /// Create a new step counter initialized to zero.
    pub const fn new() -> Self {
        Self {
            current: AtomicU64::new(0),
            snapshot: AtomicU64::new(0),
        }
    }

    /// Advance the counter by `n` steps.
    pub fn advance(&self, n: u64) {
        self.current.fetch_add(n, Ordering::Relaxed);
    }

    /// Total cumulative steps since creation.
    pub fn total_steps(&self) -> u64 {
        self.current.load(Ordering::Relaxed)
    }
}

impl CostCounter for StepCounter {
    fn name(&self) -> &'static str {
        "steps"
    }

    fn start(&mut self) {
        self.snapshot
            .store(self.current.load(Ordering::Relaxed), Ordering::Relaxed);
    }

    fn sample(&mut self) -> CounterSample {
        let cur = self.current.load(Ordering::Relaxed);
        let snap = self.snapshot.load(Ordering::Relaxed);
        CounterSample::Live(cur.saturating_sub(snap))
    }
}

impl CostCounter for &StepCounter {
    fn name(&self) -> &'static str {
        "steps"
    }

    fn start(&mut self) {
        self.snapshot
            .store(self.current.load(Ordering::Relaxed), Ordering::Relaxed);
    }

    fn sample(&mut self) -> CounterSample {
        let cur = self.current.load(Ordering::Relaxed);
        let snap = self.snapshot.load(Ordering::Relaxed);
        CounterSample::Live(cur.saturating_sub(snap))
    }
}

/// A heap allocation counter backed by the real allocator.
///
/// It reads the calling thread's tally from
/// [`crate::counting_alloc::CountingAllocator`], so it sees every allocation
/// the closure makes whether or not the closure cooperates. (The previous
/// version only counted what closures *recorded*, which made every
/// `max_alloc_bytes = 0` budget pass vacuously; audit finding LH-003a.)
///
/// The binary using it must install the allocator with
/// [`crate::install_counting_allocator!`]. If it did not, [`sample`] returns
/// [`CounterSample::NotLive`] and the harness leaves `alloc_bytes` in
/// [`BenchmarkResult::unchecked_hard_budgets`] rather than reporting a
/// meaningless zero.
///
/// [`sample`]: CostCounter::sample
#[derive(Debug, Default)]
pub struct AllocCounter {
    snapshot: Cell<AllocStats>,
}

impl AllocCounter {
    /// Create an allocation counter, armed at the thread's current tally.
    pub fn new() -> Self {
        Self {
            snapshot: Cell::new(thread_stats()),
        }
    }

    /// Heap activity on this thread since the last `start` (or since `new`),
    /// or `None` when the counting allocator is not installed.
    pub fn stats(&self) -> Option<AllocStats> {
        // Read the tally BEFORE probing: the probe allocates one box itself.
        let now = thread_stats();
        if is_installed() {
            Some(now.since(self.snapshot.get()))
        } else {
            None
        }
    }

    fn arm(&self) {
        self.snapshot.set(thread_stats());
    }

    fn read(&self) -> CounterSample {
        match self.stats() {
            Some(stats) => CounterSample::Live(stats.bytes_allocated),
            None => CounterSample::NotLive,
        }
    }
}

impl CostCounter for AllocCounter {
    fn name(&self) -> &'static str {
        "alloc_bytes"
    }

    fn start(&mut self) {
        self.arm();
    }

    fn sample(&mut self) -> CounterSample {
        self.read()
    }
}

impl CostCounter for &AllocCounter {
    fn name(&self) -> &'static str {
        "alloc_bytes"
    }

    fn start(&mut self) {
        self.arm();
    }

    fn sample(&mut self) -> CounterSample {
        self.read()
    }
}

/// Hardware performance monitoring counter abstraction (instructions retired, cycles).
///
/// On Linux hosts with available `perf_event_open` subsystem access, queries
/// hardware PMUs. On other operating systems or restricted sandboxes, safely
/// returns [`CounterSample::NotLive`].
#[derive(Debug)]
pub struct HardwarePerfCounter {
    name: &'static str,
    #[cfg(target_os = "linux")]
    fd: Option<i32>,
}

impl HardwarePerfCounter {
    /// Create an instruction counter (`"instructions"`).
    pub fn instructions() -> Self {
        Self::create("instructions", 0 /* PERF_COUNT_HW_INSTRUCTIONS */)
    }

    /// Create a cycle counter (`"cycles"`).
    pub fn cycles() -> Self {
        Self::create("cycles", 1 /* PERF_COUNT_HW_CPU_CYCLES */)
    }

    #[cfg(target_os = "linux")]
    fn create(name: &'static str, event_config: u64) -> Self {
        // Attempt perf_event_open syscall safely
        // In unprivileged or virtualized CI, perf_event_open returns -1
        let fd = open_perf_event_linux(event_config);
        Self { name, fd }
    }

    #[cfg(not(target_os = "linux"))]
    fn create(name: &'static str, _event_config: u64) -> Self {
        Self { name }
    }
}

#[cfg(target_os = "linux")]
mod linux_ffi {
    use core::ffi::c_void;

    extern "C" {
        pub fn syscall(number: i64, ...) -> i64;
        pub fn ioctl(fd: i32, request: u64, ...) -> i32;
        pub fn read(fd: i32, buf: *mut c_void, count: usize) -> isize;
        pub fn close(fd: i32) -> i32;
    }

    pub const PERF_EVENT_IOC_ENABLE: u64 = 0x2400;
    pub const PERF_EVENT_IOC_DISABLE: u64 = 0x2401;
    pub const PERF_EVENT_IOC_RESET: u64 = 0x2403;
}

#[cfg(target_os = "linux")]
fn open_perf_event_linux(event_config: u64) -> Option<i32> {
    // Linux perf_event_attr struct header (64 bytes baseline)
    #[repr(C)]
    struct PerfEventAttr {
        type_: u32,
        size: u32,
        config: u64,
        sample_period: u64,
        sample_type: u64,
        read_format: u64,
        flags: u64,
        wakeup_events: u32,
        bp_type: u32,
        bp_addr: u64,
        bp_len: u64,
        branch_sample_type: u64,
        sample_regs_user: u64,
        sample_stack_user: u32,
        clockid: i32,
        sample_regs_intr: u64,
        aux_watermark: u32,
        sample_max_stack: u16,
        reserved: u16,
    }

    let mut attr: PerfEventAttr = unsafe { std::mem::zeroed() };
    attr.type_ = 0; // PERF_TYPE_HARDWARE
    attr.size = std::mem::size_of::<PerfEventAttr>() as u32;
    attr.config = event_config;
    attr.flags = (1 << 0)   // disabled = 1
        | (1 << 16)         // exclude_kernel = 1
        | (1 << 17); // exclude_hv = 1

    // __NR_perf_event_open on x86_64 is 298, on aarch64 is 241
    #[cfg(target_arch = "x86_64")]
    let syscall_nr = 298;
    #[cfg(target_arch = "aarch64")]
    let syscall_nr = 241;
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    let syscall_nr = 0;

    if syscall_nr == 0 {
        return None;
    }

    let ret = unsafe {
        linux_ffi::syscall(
            syscall_nr,
            &attr as *const PerfEventAttr,
            0i32,  // pid = 0 (current thread)
            -1i32, // cpu = -1 (any cpu)
            -1i32, // group_fd = -1
            0u64,  // flags = 0
        )
    };

    if ret < 0 {
        None
    } else {
        Some(ret as i32)
    }
}

impl CostCounter for HardwarePerfCounter {
    fn name(&self) -> &'static str {
        self.name
    }

    #[cfg(target_os = "linux")]
    fn start(&mut self) {
        if let Some(fd) = self.fd {
            unsafe {
                // Reset counter then enable counting
                linux_ffi::ioctl(fd, linux_ffi::PERF_EVENT_IOC_RESET, 0);
                linux_ffi::ioctl(fd, linux_ffi::PERF_EVENT_IOC_ENABLE, 0);
            }
        }
    }

    #[cfg(not(target_os = "linux"))]
    fn start(&mut self) {}

    #[cfg(target_os = "linux")]
    fn sample(&mut self) -> CounterSample {
        if let Some(fd) = self.fd {
            let mut val: u64 = 0;
            let read_bytes = unsafe {
                linux_ffi::read(
                    fd,
                    &mut val as *mut u64 as *mut core::ffi::c_void,
                    std::mem::size_of::<u64>(),
                )
            };
            if read_bytes == std::mem::size_of::<u64>() as isize {
                CounterSample::Live(val)
            } else {
                CounterSample::NotLive
            }
        } else {
            CounterSample::NotLive
        }
    }

    #[cfg(not(target_os = "linux"))]
    fn sample(&mut self) -> CounterSample {
        CounterSample::Unsupported
    }
}

#[cfg(target_os = "linux")]
impl Drop for HardwarePerfCounter {
    fn drop(&mut self) {
        if let Some(fd) = self.fd {
            unsafe {
                linux_ffi::ioctl(fd, linux_ffi::PERF_EVENT_IOC_DISABLE, 0);
                linux_ffi::close(fd);
            }
        }
    }
}

/// Configuration parameters for calibrating and running a benchmark.
#[derive(Clone, Debug)]
pub struct BenchmarkConfig {
    /// Iterations executed in the initial warmup phase.
    pub warmup_iterations: u64,
    /// Maximum wall-clock duration allocated to warmup.
    pub warmup_max_duration: Duration,
    /// Target minimum duration for a sample batch to minimize clock read overhead.
    pub min_batch_duration: Duration,
    /// Safety ceiling for the automatically calibrated batch size.
    pub max_batch_size: u64,
    /// Total number of independent sample batches collected.
    pub sample_count: usize,
    /// Random seed for deterministic bootstrap confidence interval calculation.
    pub seed: u64,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            warmup_iterations: 100,
            warmup_max_duration: Duration::from_millis(10),
            min_batch_duration: Duration::from_micros(100),
            max_batch_size: 10_000_000,
            sample_count: 50,
            seed: 0x9e37_79b9_7f4a_7c15,
        }
    }
}

/// Performance budget specification defining limits on timing and deterministic costs.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Budget {
    /// Maximum permissible median duration in nanoseconds per iteration.
    /// Advisory / soft in CI environments to avoid noise-induced flakes.
    pub max_median_duration_ns: Option<u64>,
    /// Maximum permissible deterministic steps per iteration.
    pub max_steps: Option<u64>,
    /// Maximum permissible heap allocated bytes per iteration.
    pub max_alloc_bytes: Option<u64>,
    /// Maximum permissible hardware retired instructions per iteration.
    /// Checked strictly when counter is `Live`; logs `NotLive` otherwise.
    pub max_instructions: Option<u64>,
}

/// Outcome of evaluating a benchmark against its stated [`Budget`].
#[derive(Clone, Debug, PartialEq)]
pub enum BudgetVerdict {
    /// All specified hard budgets passed.
    Pass,
    /// A hard deterministic cost counter exceeded its maximum limit.
    Failed {
        reason: &'static str,
        observed: u64,
        limit: u64,
    },
    /// A soft advisory budget (e.g. wall-clock median) was exceeded in CI mode.
    PassWithWarning {
        reason: &'static str,
        observed_ns: f64,
        limit_ns: u64,
    },
}

impl BudgetVerdict {
    /// Returns true if the verdict is acceptable for gating (Pass or PassWithWarning).
    pub const fn is_acceptable(&self) -> bool {
        match self {
            Self::Pass | Self::PassWithWarning { .. } => true,
            Self::Failed { .. } => false,
        }
    }
}

/// Comprehensive statistical summary of per-iteration benchmark measurements.
#[derive(Clone, Debug)]
pub struct BenchmarkResult {
    /// Name of the benchmark.
    pub name: String,
    /// Automatically calibrated batch size $B$.
    pub batch_size: u64,
    /// Number of collected sample batches.
    pub sample_count: usize,
    /// Median per-iteration duration in nanoseconds.
    pub median_ns: f64,
    /// Median Absolute Deviation (MAD) in nanoseconds.
    pub mad_ns: f64,
    /// Normalized Median Absolute Deviation (nMAD = 1.4826 * MAD).
    pub nmad_ns: f64,
    /// 25th percentile (1st quartile) in nanoseconds.
    pub p25_ns: f64,
    /// 75th percentile (3rd quartile) in nanoseconds.
    pub p75_ns: f64,
    /// 95th percentile in nanoseconds.
    pub p95_ns: f64,
    /// 99th percentile in nanoseconds.
    pub p99_ns: f64,
    /// Minimum observed per-iteration duration in nanoseconds.
    pub min_ns: f64,
    /// Maximum observed per-iteration duration in nanoseconds.
    pub max_ns: f64,
    /// 95% bootstrap confidence interval for the median [lower, upper].
    pub ci_95_ns: (f64, f64),
    /// Average deterministic steps per iteration (if tracked).
    pub steps_per_iter: Option<f64>,
    /// Average allocated bytes per iteration (if tracked).
    pub alloc_bytes_per_iter: Option<f64>,
    /// Average hardware instructions retired per iteration (if live).
    pub instructions_per_iter: CounterSample,
    /// Outcome of comparing metrics against stated budget.
    pub budget_verdict: BudgetVerdict,
    /// Hard budgets (`"steps"`, `"alloc_bytes"`, `"instructions"`) the
    /// [`Budget`] declared but this run could not check: no counter of that
    /// kind was attached, or it was not `Live`. A `Pass` verdict with a
    /// non-empty list means "nothing was found", not "the budget held".
    /// Before the audit this was invisible: `bench()` with `max_steps: Some(1)`
    /// reported a plain pass.
    pub unchecked_hard_budgets: Vec<&'static str>,
}

impl BenchmarkResult {
    /// Format result as a structured TAP 14 YAML diagnostic block.
    pub fn to_tap_yaml(&self) -> String {
        let mut yaml = String::new();
        yaml.push_str("  ---\n");
        yaml.push_str(&format!(
            "  benchmark: \"{}\"\n",
            escape_quoted(&self.name)
        ));
        yaml.push_str(&format!("  batch_size: {}\n", self.batch_size));
        yaml.push_str(&format!("  sample_count: {}\n", self.sample_count));
        yaml.push_str("  timing_ns:\n");
        yaml.push_str(&format!("    median: {:.3}\n", self.median_ns));
        yaml.push_str(&format!("    mad: {:.3}\n", self.mad_ns));
        yaml.push_str(&format!("    nmad: {:.3}\n", self.nmad_ns));
        yaml.push_str(&format!("    p25: {:.3}\n", self.p25_ns));
        yaml.push_str(&format!("    p75: {:.3}\n", self.p75_ns));
        yaml.push_str(&format!("    p95: {:.3}\n", self.p95_ns));
        yaml.push_str(&format!("    p99: {:.3}\n", self.p99_ns));
        yaml.push_str(&format!("    min: {:.3}\n", self.min_ns));
        yaml.push_str(&format!("    max: {:.3}\n", self.max_ns));
        yaml.push_str(&format!(
            "    ci_95: [{:.3}, {:.3}]\n",
            self.ci_95_ns.0, self.ci_95_ns.1
        ));

        yaml.push_str("  counters:\n");
        if let Some(steps) = self.steps_per_iter {
            yaml.push_str(&format!("    steps_per_iter: {:.2}\n", steps));
        }
        if let Some(alloc) = self.alloc_bytes_per_iter {
            yaml.push_str(&format!("    alloc_bytes_per_iter: {:.2}\n", alloc));
        }
        match self.instructions_per_iter {
            CounterSample::Live(val) => {
                yaml.push_str(&format!("    instructions_per_iter: {}\n", val));
            }
            CounterSample::NotLive => {
                yaml.push_str("    instructions: not-live\n");
            }
            CounterSample::Unsupported => {
                yaml.push_str("    instructions: unsupported\n");
            }
        }

        yaml.push_str("  budget:\n");
        match &self.budget_verdict {
            BudgetVerdict::Pass => {
                yaml.push_str("    verdict: pass\n");
            }
            BudgetVerdict::Failed {
                reason,
                observed,
                limit,
            } => {
                yaml.push_str("    verdict: fail\n");
                yaml.push_str(&format!("    reason: \"{}\"\n", reason));
                yaml.push_str(&format!("    observed: {}\n", observed));
                yaml.push_str(&format!("    limit: {}\n", limit));
            }
            BudgetVerdict::PassWithWarning {
                reason,
                observed_ns,
                limit_ns,
            } => {
                yaml.push_str("    verdict: pass_with_warning\n");
                yaml.push_str(&format!("    reason: \"{}\"\n", reason));
                yaml.push_str(&format!("    observed_ns: {:.3}\n", observed_ns));
                yaml.push_str(&format!("    limit_ns: {}\n", limit_ns));
            }
        }
        if !self.unchecked_hard_budgets.is_empty() {
            yaml.push_str(&format!(
                "    unchecked_hard_budgets: [{}]\n",
                self.unchecked_hard_budgets.join(", ")
            ));
        }
        yaml.push_str("  ...\n");
        yaml
    }

    /// Serialize result as clean structured JSON.
    pub fn to_json(&self) -> String {
        let instr_str = match self.instructions_per_iter {
            CounterSample::Live(val) => val.to_string(),
            _ => "null".to_string(),
        };

        let verdict_str = match &self.budget_verdict {
            BudgetVerdict::Pass => "\"pass\"".to_string(),
            BudgetVerdict::Failed { reason, .. } => format!("\"failed: {}\"", reason),
            BudgetVerdict::PassWithWarning { reason, .. } => {
                format!("\"warning: {}\"", reason)
            }
        };

        format!(
            "{{\n  \
              \"name\": \"{}\",\n  \
              \"batch_size\": {},\n  \
              \"sample_count\": {},\n  \
              \"median_ns\": {:.3},\n  \
              \"mad_ns\": {:.3},\n  \
              \"nmad_ns\": {:.3},\n  \
              \"p25_ns\": {:.3},\n  \
              \"p75_ns\": {:.3},\n  \
              \"p95_ns\": {:.3},\n  \
              \"p99_ns\": {:.3},\n  \
              \"min_ns\": {:.3},\n  \
              \"max_ns\": {:.3},\n  \
              \"ci_95_lower_ns\": {:.3},\n  \
              \"ci_95_upper_ns\": {:.3},\n  \
              \"steps_per_iter\": {},\n  \
              \"alloc_bytes_per_iter\": {},\n  \
              \"instructions_per_iter\": {},\n  \
              \"unchecked_hard_budgets\": [{}],\n  \
              \"budget_verdict\": {}\n\
            }}",
            escape_quoted(&self.name),
            self.batch_size,
            self.sample_count,
            self.median_ns,
            self.mad_ns,
            self.nmad_ns,
            self.p25_ns,
            self.p75_ns,
            self.p95_ns,
            self.p99_ns,
            self.min_ns,
            self.max_ns,
            self.ci_95_ns.0,
            self.ci_95_ns.1,
            self.steps_per_iter
                .map(|v| format!("{:.2}", v))
                .unwrap_or_else(|| "null".into()),
            self.alloc_bytes_per_iter
                .map(|v| format!("{:.2}", v))
                .unwrap_or_else(|| "null".into()),
            instr_str,
            self.unchecked_hard_budgets
                .iter()
                .map(|name| format!("\"{name}\""))
                .collect::<Vec<_>>()
                .join(", "),
            verdict_str
        )
    }
}

/// Escape `text` for use inside a double-quoted JSON string, which is also a
/// valid YAML double-quoted scalar. A benchmark name is caller-chosen; written
/// raw, a quote or newline in it would break the JSON or add forged lines to
/// the TAP diagnostic block.
fn escape_quoted(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() || matches!(c, '\u{2028}' | '\u{2029}') => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out
}

/// Compute the non-parametric percentile from an already-sorted slice of floats
/// using continuous linear interpolation.
///
/// For rank $r = p \times (n - 1) + 1$:
/// - If $r$ is integral, pick element at rank $r$;
/// - Otherwise linearly interpolate between $\lfloor r \rfloor$ and $\lceil r \rceil$.
pub fn percentile_sorted(sorted: &[f64], p: f64) -> f64 {
    assert!(!sorted.is_empty(), "percentile requires non-empty slice");
    let p = p.clamp(0.0, 1.0);
    let n = sorted.len();
    if n == 1 {
        return sorted[0];
    }
    let index = p * (n - 1) as f64;
    let lower_idx = index.floor() as usize;
    let upper_idx = (lower_idx + 1).min(n - 1);
    let weight = index - lower_idx as f64;
    // Interpolate as `lo + (hi - lo) * w`, not `lo * (1 - w) + hi * w`.
    //
    // The two are equal on paper but not in floating point. With lo == hi
    // the second form adds two separately rounded products, and the sum can
    // land one ulp ABOVE lo (e.g. 236322.0 at w = 0.225 gives
    // 236322.00000000003). This form returns lo exactly when hi - lo == 0.
    // The final clamp then guarantees, for every weight, that a percentile
    // never leaves the closed interval of its two neighbouring samples, so
    // the quantile chain stays ordered and inside [min, max]. That is the
    // invariant `bootstrap_median_ci_95` and the result shape rely on.
    let lo = sorted[lower_idx];
    let hi = sorted[upper_idx];
    let value = lo + (hi - lo) * weight;
    // `f64::clamp` panics when lo > hi or either bound is NaN. The harness
    // never produces either (finite timings, sorted with `total_cmp`), but
    // this function is public, so a caller's unsorted or NaN input degrades to
    // the unclamped value instead of panicking. `lo <= hi` is false for NaN,
    // so this one comparison covers both cases.
    if lo <= hi {
        value.clamp(lo, hi)
    } else {
        value
    }
}

/// Compute the median of an already-sorted slice of floats.
pub fn median_sorted(sorted: &[f64]) -> f64 {
    percentile_sorted(sorted, 0.5)
}

/// Compute the Median Absolute Deviation (MAD) of a sample:
///
/// $$\text{MAD} = \text{median}\left(\left| x_i - \tilde{x} \right|\right)$$
pub fn mad_from_samples(samples: &[f64], median: f64) -> f64 {
    if samples.is_empty() {
        return 0.0;
    }
    let mut deviations: Vec<f64> = samples.iter().map(|&x| (x - median).abs()).collect();
    deviations.sort_by(|a, b| a.total_cmp(b));
    median_sorted(&deviations)
}

/// Compute a two-sided 95% bootstrap confidence interval for the sample median
/// using deterministic resampling via [`Prng`].
pub fn bootstrap_median_ci_95(samples: &[f64], resample_count: usize, seed: u64) -> (f64, f64) {
    let n = samples.len();
    if n == 0 {
        return (0.0, 0.0);
    }
    if n == 1 || resample_count == 0 {
        return (samples[0], samples[0]);
    }

    let mut prng = SplitMix64::new(seed);
    let mut medians: Vec<f64> = Vec::with_capacity(resample_count);
    let mut resample_buf: Vec<f64> = vec![0.0; n];

    for _ in 0..resample_count {
        for item in resample_buf.iter_mut() {
            let idx = prng.bounded(n as u64) as usize;
            *item = samples[idx];
        }
        resample_buf.sort_by(|a, b| a.total_cmp(b));
        medians.push(median_sorted(&resample_buf));
    }

    medians.sort_by(|a, b| a.total_cmp(b));
    let lower = percentile_sorted(&medians, 0.025);
    let upper = percentile_sorted(&medians, 0.975);
    (lower, upper)
}

/// Benchmark runner managing warmup, batch calibration, sampling, and budget audit.
pub struct BenchmarkHarness {
    config: BenchmarkConfig,
}

impl BenchmarkHarness {
    /// Create a benchmark harness with the provided configuration.
    pub fn new(config: BenchmarkConfig) -> Self {
        Self { config }
    }

    /// Run a microbenchmark for the given callable closure.
    pub fn run<F, T>(&self, name: &str, budget: &Budget, mut f: F) -> BenchmarkResult
    where
        F: FnMut() -> T,
    {
        assert!(
            self.config.sample_count >= 1,
            "sample_count must be at least 1"
        );

        // 1. Warmup phase
        let warmup_start = Instant::now();
        for _ in 0..self.config.warmup_iterations {
            black_box(f());
            if warmup_start.elapsed() >= self.config.warmup_max_duration {
                break;
            }
        }

        // 2. Batch calibration phase
        let mut batch_size: u64 = 1;
        while batch_size < self.config.max_batch_size {
            let start = Instant::now();
            for _ in 0..batch_size {
                black_box(f());
            }
            let elapsed = start.elapsed();
            if elapsed >= self.config.min_batch_duration {
                break;
            }

            let elapsed_ns = elapsed.as_nanos().max(1) as f64;
            let target_ns = self.config.min_batch_duration.as_nanos() as f64;
            let scale = (target_ns / elapsed_ns).ceil() as u64;
            let next_batch = batch_size
                .saturating_mul(scale.max(2))
                .min(self.config.max_batch_size);
            if next_batch == batch_size {
                break;
            }
            batch_size = next_batch;
        }

        // 3. Sample collection
        let mut raw_samples_ns: Vec<f64> = Vec::with_capacity(self.config.sample_count);
        for _ in 0..self.config.sample_count {
            let start = Instant::now();
            for _ in 0..batch_size {
                black_box(f());
            }
            let elapsed = start.elapsed();
            let per_iter_ns = elapsed.as_nanos() as f64 / batch_size as f64;
            raw_samples_ns.push(per_iter_ns);
        }

        raw_samples_ns.sort_by(|a, b| a.total_cmp(b));

        // 4. Robust statistics
        let median_ns = median_sorted(&raw_samples_ns);
        let mad_ns = mad_from_samples(&raw_samples_ns, median_ns);
        let nmad_ns = mad_ns * MAD_NORMAL_FACTOR;
        let p25_ns = percentile_sorted(&raw_samples_ns, 0.25);
        let p75_ns = percentile_sorted(&raw_samples_ns, 0.75);
        let p95_ns = percentile_sorted(&raw_samples_ns, 0.95);
        let p99_ns = percentile_sorted(&raw_samples_ns, 0.99);
        let min_ns = raw_samples_ns[0];
        let max_ns = raw_samples_ns[raw_samples_ns.len() - 1];
        let ci_95_ns = bootstrap_median_ci_95(&raw_samples_ns, 1000, self.config.seed);

        // 5. Evaluate budget
        let mut budget_verdict = BudgetVerdict::Pass;
        if let Some(limit_ns) = budget.max_median_duration_ns {
            if median_ns > limit_ns as f64 {
                budget_verdict = BudgetVerdict::PassWithWarning {
                    reason: "median duration exceeded soft timing budget",
                    observed_ns: median_ns,
                    limit_ns,
                };
            }
        }

        BenchmarkResult {
            name: name.to_string(),
            batch_size,
            sample_count: self.config.sample_count,
            median_ns,
            mad_ns,
            nmad_ns,
            p25_ns,
            p75_ns,
            p95_ns,
            p99_ns,
            min_ns,
            max_ns,
            ci_95_ns,
            steps_per_iter: None,
            alloc_bytes_per_iter: None,
            instructions_per_iter: CounterSample::NotLive,
            budget_verdict,
            // Nothing here measures steps, allocations or instructions, so
            // every hard budget the caller declared is, so far, unchecked.
            unchecked_hard_budgets: [
                ("steps", budget.max_steps),
                ("alloc_bytes", budget.max_alloc_bytes),
                ("instructions", budget.max_instructions),
            ]
            .into_iter()
            .filter_map(|(name, limit)| limit.map(|_| name))
            .collect(),
        }
    }

    /// Run a microbenchmark with an explicit [`CostCounter`].
    pub fn run_with_counter<F, T, C>(
        &self,
        name: &str,
        budget: &Budget,
        mut counter: C,
        mut f: F,
    ) -> BenchmarkResult
    where
        F: FnMut() -> T,
        C: CostCounter,
    {
        let mut result = self.run(name, budget, &mut f);

        // Run one dedicated instrumented measurement window for the cost counter
        counter.start();
        for _ in 0..result.batch_size {
            black_box(f());
        }
        let sample = counter.sample();

        match counter.name() {
            "steps" => {
                if let CounterSample::Live(val) = sample {
                    result.unchecked_hard_budgets.retain(|n| *n != "steps");
                    let per_iter = val as f64 / result.batch_size as f64;
                    result.steps_per_iter = Some(per_iter);
                    if let Some(limit) = budget.max_steps {
                        if per_iter.ceil() as u64 > limit {
                            result.budget_verdict = BudgetVerdict::Failed {
                                reason: "steps exceeded hard deterministic budget",
                                observed: per_iter.ceil() as u64,
                                limit,
                            };
                        }
                    }
                }
            }
            "alloc_bytes" => {
                if let CounterSample::Live(val) = sample {
                    result.unchecked_hard_budgets.retain(|n| *n != "alloc_bytes");
                    let per_iter = val as f64 / result.batch_size as f64;
                    result.alloc_bytes_per_iter = Some(per_iter);
                    if let Some(limit) = budget.max_alloc_bytes {
                        if per_iter.ceil() as u64 > limit {
                            result.budget_verdict = BudgetVerdict::Failed {
                                reason: "heap allocations exceeded hard deterministic budget",
                                observed: per_iter.ceil() as u64,
                                limit,
                            };
                        }
                    }
                }
            }
            "instructions" => {
                result.instructions_per_iter = sample;
                if let CounterSample::Live(val) = sample {
                    result.unchecked_hard_budgets.retain(|n| *n != "instructions");
                    let per_iter = (val as f64 / result.batch_size as f64).ceil() as u64;
                    if let Some(limit) = budget.max_instructions {
                        if per_iter > limit {
                            result.budget_verdict = BudgetVerdict::Failed {
                                reason: "retired instructions exceeded hard budget",
                                observed: per_iter,
                                limit,
                            };
                        }
                    }
                }
            }
            _ => {}
        }

        result
    }
}

/// Convenience function executing a benchmark with default configuration.
pub fn bench<F, T>(name: &str, budget: &Budget, f: F) -> BenchmarkResult
where
    F: FnMut() -> T,
{
    BenchmarkHarness::new(BenchmarkConfig::default()).run(name, budget, f)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_percentile_and_median_computation() {
        let sorted = vec![10.0, 20.0, 30.0, 40.0, 50.0];
        assert_eq!(median_sorted(&sorted), 30.0);
        assert_eq!(percentile_sorted(&sorted, 0.0), 10.0);
        assert_eq!(percentile_sorted(&sorted, 1.0), 50.0);
        assert_eq!(percentile_sorted(&sorted, 0.5), 30.0);
        assert_eq!(percentile_sorted(&sorted, 0.25), 20.0);
        assert_eq!(percentile_sorted(&sorted, 0.75), 40.0);

        let even = vec![10.0, 20.0, 30.0, 40.0];
        assert_eq!(median_sorted(&even), 25.0);
    }

    #[test]
    fn test_mad_computation() {
        // [1, 2, 3, 4, 5, 6, 7, 8, 9] -> median = 5
        // absolute deviations from 5: [4, 3, 2, 1, 0, 1, 2, 3, 4]
        // sorted deviations: [0, 1, 1, 2, 2, 3, 3, 4, 4] -> median deviation = 2.0
        let samples = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0];
        let med = median_sorted(&samples);
        let mad = mad_from_samples(&samples, med);
        assert_eq!(mad, 2.0);
    }

    #[test]
    fn test_bootstrap_confidence_interval() {
        let samples = vec![100.0, 102.0, 98.0, 101.0, 99.0, 100.5, 99.5];
        let (lower, upper) = bootstrap_median_ci_95(&samples, 500, 42);
        assert!(lower <= upper);
        assert!(lower >= 97.0 && upper <= 103.0);
    }

    #[test]
    fn test_step_counter() {
        let mut counter = StepCounter::new();
        counter.start();
        counter.advance(42);
        assert_eq!(counter.sample(), CounterSample::Live(42));
        assert_eq!(counter.name(), "steps");
        assert_eq!(counter.total_steps(), 42);
    }

    #[test]
    fn test_alloc_counter() {
        // Real measurement: nothing here "reports" its allocations.
        let mut counter = AllocCounter::new();
        counter.start();
        let a = black_box(Vec::<u8>::with_capacity(1024));
        let b = black_box(Vec::<u8>::with_capacity(512));
        assert_eq!(counter.sample(), CounterSample::Live(1536));
        assert_eq!(counter.name(), "alloc_bytes");
        let stats = counter.stats().expect("allocator installed in test builds");
        assert_eq!(stats.allocations, 2);
        assert_eq!(stats.bytes_allocated, 1536);
        drop((a, b));
        // start() re-arms: the earlier activity no longer counts.
        counter.start();
        assert_eq!(counter.sample(), CounterSample::Live(0));
    }

    #[test]
    fn test_counter_sample_helpers() {
        let live = CounterSample::Live(123);
        let not_live = CounterSample::NotLive;
        let unsupp = CounterSample::Unsupported;

        assert!(live.is_live());
        assert!(!not_live.is_live());
        assert_eq!(live.value(), Some(123));
        assert_eq!(not_live.value(), None);
        assert_eq!(unsupp.value(), None);
    }

    #[test]
    fn test_hardware_perf_counter_fallback() {
        let mut counter = HardwarePerfCounter::instructions();
        counter.start();
        let sample = counter.sample();
        // Regardless of host privilege, must not panic and must return either Live or NotLive/Unsupported
        match sample {
            CounterSample::Live(val) => {
                let _ = val;
            }
            CounterSample::NotLive | CounterSample::Unsupported => {}
        }
    }

    #[test]
    fn test_benchmark_execution_and_calibration() {
        let config = BenchmarkConfig {
            warmup_iterations: 10,
            warmup_max_duration: Duration::from_millis(1),
            min_batch_duration: Duration::from_micros(10),
            max_batch_size: 100_000,
            sample_count: 20,
            seed: 12345,
        };
        let harness = BenchmarkHarness::new(config);
        let budget = Budget::default();

        let mut acc: u64 = 0;
        let result = harness.run("addition_loop", &budget, || {
            acc = acc.wrapping_add(1);
            acc
        });

        assert_eq!(result.name, "addition_loop");
        assert_eq!(result.sample_count, 20);
        assert!(result.batch_size >= 1);
        assert!(result.median_ns > 0.0);
        assert!(result.ci_95_ns.0 <= result.ci_95_ns.1);
        assert_eq!(result.budget_verdict, BudgetVerdict::Pass);

        // Verify TAP YAML serialization
        let yaml = result.to_tap_yaml();
        assert!(yaml.contains("benchmark: \"addition_loop\""));
        assert!(yaml.contains("timing_ns:"));
        assert!(yaml.contains("verdict: pass"));

        // Verify JSON serialization
        let json = result.to_json();
        assert!(json.contains("\"name\": \"addition_loop\""));
        assert!(json.contains("\"budget_verdict\": \"pass\""));
    }

    #[test]
    fn test_benchmark_budget_enforcement() {
        let config = BenchmarkConfig {
            warmup_iterations: 5,
            warmup_max_duration: Duration::from_millis(1),
            min_batch_duration: Duration::from_micros(5),
            max_batch_size: 1000,
            sample_count: 10,
            seed: 7,
        };
        let harness = BenchmarkHarness::new(config);

        // Test failing step budget
        let step_counter = StepCounter::new();
        let failing_budget = Budget {
            max_steps: Some(5),
            ..Default::default()
        };

        let result = harness.run_with_counter(
            "step_exceeded",
            &failing_budget,
            &step_counter,
            || {
                step_counter.advance(10);
            },
        );

        assert!(matches!(
            result.budget_verdict,
            BudgetVerdict::Failed {
                observed: 10,
                limit: 5,
                ..
            }
        ));
        assert!(!result.budget_verdict.is_acceptable());

        // Test passing allocation budget
        let alloc_counter = AllocCounter::new();
        let passing_budget = Budget {
            max_alloc_bytes: Some(0),
            ..Default::default()
        };

        let result_pass = harness.run_with_counter(
            "zero_alloc",
            &passing_budget,
            &alloc_counter,
            || {
                // No heap allocation
                black_box(1 + 1)
            },
        );

        assert_eq!(result_pass.budget_verdict, BudgetVerdict::Pass);
        assert!(result_pass.budget_verdict.is_acceptable());
        assert_eq!(result_pass.alloc_bytes_per_iter, Some(0.0));
    }

    #[test]
    fn test_soft_timing_warning_budget() {
        let config = BenchmarkConfig {
            warmup_iterations: 5,
            warmup_max_duration: Duration::from_millis(1),
            min_batch_duration: Duration::from_micros(5),
            max_batch_size: 1000,
            sample_count: 10,
            seed: 99,
        };
        let harness = BenchmarkHarness::new(config);

        // Budget set to 0 ns will trigger warning without failing gating
        let tight_timing_budget = Budget {
            max_median_duration_ns: Some(0),
            ..Default::default()
        };

        let result = harness.run("tight_timing", &tight_timing_budget, || {
            black_box(42 * 42)
        });

        assert!(matches!(
            result.budget_verdict,
            BudgetVerdict::PassWithWarning { .. }
        ));
        assert!(result.budget_verdict.is_acceptable());
    }

    #[test]
    fn test_alloc_budget_exceeded() {
        let config = BenchmarkConfig {
            warmup_iterations: 1,
            warmup_max_duration: Duration::from_millis(1),
            min_batch_duration: Duration::from_micros(1),
            max_batch_size: 10,
            sample_count: 5,
            seed: 1,
        };
        let harness = BenchmarkHarness::new(config);
        let alloc_counter = AllocCounter::new();
        let budget = Budget {
            max_alloc_bytes: Some(10),
            ..Default::default()
        };

        let result = harness.run_with_counter("alloc_exceeded", &budget, &alloc_counter, || {
            black_box(Vec::<u8>::with_capacity(20))
        });

        assert!(matches!(
            result.budget_verdict,
            BudgetVerdict::Failed {
                observed: 20,
                limit: 10,
                ..
            }
        ));
        assert!(!result.budget_verdict.is_acceptable());
    }

    #[test]
    fn test_convenience_bench_function() {
        let budget = Budget::default();
        let res = bench("simple_add", &budget, || black_box(1 + 2));
        assert_eq!(res.name, "simple_add");
        assert!(res.median_ns > 0.0);
    }

    #[test]
    fn test_cycles_and_instructions_constructors() {
        let mut cycles = HardwarePerfCounter::cycles();
        cycles.start();
        let sample = cycles.sample();
        assert_eq!(cycles.name(), "cycles");
        match sample {
            CounterSample::Live(val) => {
                let _ = val;
            }
            CounterSample::NotLive | CounterSample::Unsupported => {}
        }
    }

    #[test]
    fn test_mut_counter_ref_implementation() {
        let mut step_counter = StepCounter::new();
        let r = &mut step_counter;
        r.start();
        r.advance(15);
        assert_eq!(r.sample(), CounterSample::Live(15));
        assert_eq!(r.name(), "steps");
    }

    #[test]
    fn test_statistics_edge_cases() {
        let empty: [f64; 0] = [];
        assert_eq!(mad_from_samples(&empty, 0.0), 0.0);
        assert_eq!(bootstrap_median_ci_95(&empty, 100, 1), (0.0, 0.0));

        let single = [42.0];
        assert_eq!(percentile_sorted(&single, 0.5), 42.0);
        assert_eq!(bootstrap_median_ci_95(&single, 100, 1), (42.0, 42.0));
    }

    #[test]
    fn test_formatting_with_various_counter_states() {
        let mut res = BenchmarkResult {
            name: "test_fmt".into(),
            batch_size: 100,
            sample_count: 50,
            median_ns: 12.345,
            mad_ns: 1.2,
            nmad_ns: 1.779,
            p25_ns: 11.0,
            p75_ns: 13.0,
            p95_ns: 14.5,
            p99_ns: 15.0,
            min_ns: 10.0,
            max_ns: 16.0,
            ci_95_ns: (11.8, 12.8),
            steps_per_iter: Some(4.0),
            alloc_bytes_per_iter: Some(0.0),
            instructions_per_iter: CounterSample::Live(50),
            budget_verdict: BudgetVerdict::Failed {
                reason: "instructions exceeded",
                observed: 50,
                limit: 40,
            },
            unchecked_hard_budgets: vec![],
        };

        let yaml_live = res.to_tap_yaml();
        assert!(yaml_live.contains("instructions_per_iter: 50"));
        assert!(yaml_live.contains("verdict: fail"));
        assert!(yaml_live.contains("steps_per_iter: 4.00"));

        let json_live = res.to_json();
        assert!(json_live.contains("\"instructions_per_iter\": 50"));
        assert!(json_live.contains("\"budget_verdict\": \"failed: instructions exceeded\""));

        res.instructions_per_iter = CounterSample::Unsupported;
        res.budget_verdict = BudgetVerdict::PassWithWarning {
            reason: "soft threshold",
            observed_ns: 12.345,
            limit_ns: 10,
        };
        let yaml_unsupp = res.to_tap_yaml();
        assert!(yaml_unsupp.contains("instructions: unsupported"));
        assert!(yaml_unsupp.contains("verdict: pass_with_warning"));

        let json_warn = res.to_json();
        assert!(json_warn.contains("\"instructions_per_iter\": null"));
        assert!(json_warn.contains("\"budget_verdict\": \"warning: soft threshold\""));
    }
}
