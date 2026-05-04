//! Executor registry — a list of `(ExecutorId, BackendProfile)` plus
//! a transport handle for each.
//!
//! V1 registry is a simple `Vec`-backed structure indexed by
//! `ExecutorId`.  The CPU executor (`CPU_EXECUTOR = ExecutorId(0)`)
//! is always present.

use compute_ir::{ExecutorId, CPU_EXECUTOR};
use executor_protocol::BackendProfile;

/// Public summary of a registered executor.  The transport handle
/// itself is held internally by the [`Registry`] but not exposed here
/// because it requires the `Transport` trait that some callers don't
/// want to depend on.
#[derive(Clone, Debug)]
pub struct RegisteredExecutor {
    /// The id assigned at registration time.
    pub id: ExecutorId,
    /// Free-form kind tag ("cpu", "metal", "cuda", …).
    pub kind: String,
    /// Capability + cost profile.
    pub profile: BackendProfile,
    /// Whether the executor passed its most recent heartbeat.  V1
    /// only updates this through [`Registry::set_healthy`]; future
    /// versions will wire it to event streams.
    pub healthy: bool,
}

impl RegisteredExecutor {
    /// True iff the executor supports the given matrix-ir op tag.
    pub fn supports_op(&self, op_tag: u8) -> bool {
        if op_tag >= 32 {
            return false;
        }
        self.profile.supported_ops & (1u32 << op_tag) != 0
    }

    /// True iff the executor supports the given dtype tag.
    pub fn supports_dtype(&self, dtype_tag: u8) -> bool {
        if dtype_tag >= 8 {
            return false;
        }
        self.profile.supported_dtypes & (1u8 << dtype_tag) != 0
    }
}

/// The registry.
///
/// V1 owns just `RegisteredExecutor` records; transport wiring lives
/// at the runtime layer (see [`Runtime`](crate::Runtime)) so the
/// planner can be tested without the async machinery.
#[derive(Default)]
pub struct Registry {
    executors: Vec<RegisteredExecutor>,
}

impl Registry {
    /// Construct an empty registry.  Callers typically register the
    /// CPU executor first via [`Self::register`].
    pub fn new() -> Self {
        Registry {
            executors: Vec::new(),
        }
    }

    /// Convenience constructor that pre-registers a CPU executor with
    /// the supplied profile under [`CPU_EXECUTOR`].  Returns the new
    /// registry plus the assigned id (which is always `CPU_EXECUTOR`).
    pub fn with_cpu(cpu_profile: BackendProfile) -> (Self, ExecutorId) {
        let mut r = Registry::new();
        let id = r.register("cpu".to_string(), cpu_profile);
        debug_assert_eq!(id, CPU_EXECUTOR);
        (r, id)
    }

    /// Register a new executor.  Returns the assigned [`ExecutorId`].
    /// Ids are assigned monotonically starting from 0; the first call
    /// should always be the CPU executor so it lands on
    /// [`CPU_EXECUTOR`].
    pub fn register(&mut self, kind: String, profile: BackendProfile) -> ExecutorId {
        let id = ExecutorId(self.executors.len() as u32);
        self.executors.push(RegisteredExecutor {
            id,
            kind,
            profile,
            healthy: true,
        });
        id
    }

    /// Mark an executor as healthy or unhealthy.  Unhealthy executors
    /// are filtered out by the planner.
    pub fn set_healthy(&mut self, id: ExecutorId, healthy: bool) {
        if let Some(e) = self.executors.get_mut(id.0 as usize) {
            e.healthy = healthy;
        }
    }

    /// Replace an executor's profile (e.g. after a `ProfileUpdated`
    /// event).
    pub fn update_profile(&mut self, id: ExecutorId, profile: BackendProfile) {
        if let Some(e) = self.executors.get_mut(id.0 as usize) {
            e.profile = profile;
        }
    }

    /// Look up an executor by id.
    pub fn get(&self, id: ExecutorId) -> Option<&RegisteredExecutor> {
        self.executors.get(id.0 as usize)
    }

    /// All registered executors, healthy or not.
    pub fn all(&self) -> &[RegisteredExecutor] {
        &self.executors
    }

    /// Healthy executors only — what the planner sees.
    pub fn healthy(&self) -> Vec<&RegisteredExecutor> {
        self.executors.iter().filter(|e| e.healthy).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(kind: &str, ops: u32, dtypes: u8) -> BackendProfile {
        BackendProfile {
            kind: kind.to_string(),
            supported_ops: ops,
            supported_dtypes: dtypes,
            gflops_f32: 100,
            gflops_u8: 100,
            gflops_i32: 100,
            host_to_device_bw: 10,
            device_to_host_bw: 10,
            device_internal_bw: 100,
            launch_overhead_ns: 0,
            transport_latency_ns: 0,
            on_device_mib: 1024,
            max_tensor_rank: 8,
            max_dim: 65535,
        }
    }

    #[test]
    fn cpu_registers_at_zero() {
        let (_r, id) = Registry::with_cpu(p("cpu", 0xFFFF_FFFF, 0x07));
        assert_eq!(id, CPU_EXECUTOR);
    }

    #[test]
    fn supports_op_bitset() {
        let executor = RegisteredExecutor {
            id: ExecutorId(0),
            kind: "test".to_string(),
            profile: p("test", 0b101, 0xFF),
            healthy: true,
        };
        assert!(executor.supports_op(0));
        assert!(!executor.supports_op(1));
        assert!(executor.supports_op(2));
        assert!(!executor.supports_op(3));
        // Out-of-range tag returns false.
        assert!(!executor.supports_op(32));
    }

    #[test]
    fn supports_dtype_bitset() {
        let executor = RegisteredExecutor {
            id: ExecutorId(0),
            kind: "test".to_string(),
            profile: p("test", 0xFFFF_FFFF, 0b011),
            healthy: true,
        };
        assert!(executor.supports_dtype(0)); // F32
        assert!(executor.supports_dtype(1)); // U8
        assert!(!executor.supports_dtype(2)); // I32
        assert!(!executor.supports_dtype(8));
    }

    #[test]
    fn unhealthy_filtered_from_healthy_list() {
        let mut r = Registry::new();
        let id = r.register("cpu".to_string(), p("cpu", 0xFFFF_FFFF, 0x07));
        assert_eq!(r.healthy().len(), 1);
        r.set_healthy(id, false);
        assert_eq!(r.healthy().len(), 0);
        assert_eq!(r.all().len(), 1);
    }

    #[test]
    fn update_profile_changes_capabilities() {
        let mut r = Registry::new();
        let id = r.register("cpu".to_string(), p("cpu", 0, 0));
        assert!(!r.get(id).unwrap().supports_op(0));
        r.update_profile(id, p("cpu", 0xFFFF_FFFF, 0x07));
        assert!(r.get(id).unwrap().supports_op(0));
    }
}
