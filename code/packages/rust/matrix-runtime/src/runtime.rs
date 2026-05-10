//! `Runtime` — the public API surface domain libraries call.
//!
//! V1 ships the planner and registry surface.  The execution loop
//! (driving `ComputeGraph`s through transports and collecting outputs)
//! is left as a seam: callers can call `runtime.plan(graph)` to get a
//! `ComputeGraph` and inspect or hand-execute it.  The `run()` end-to-end
//! method lands when the first executor crate (`matrix-cpu`) is in.

use crate::planner::{plan, PlanError};
use crate::registry::{RegisteredExecutor, Registry};
use compute_ir::{ComputeGraph, ExecutorId};
use executor_protocol::BackendProfile;
use matrix_ir::Graph;

/// Errors produced by the runtime API.  Wraps both planner and
/// registry errors plus future execution-path errors.
#[derive(Clone, Debug)]
pub enum RuntimeError {
    /// No executor available for the operation.
    NoExecutorAvailable,
    /// Planner refused the graph.
    Plan(PlanError),
    /// Underlying matrix-ir validation failure.
    InvalidGraph(matrix_ir::IrError),
}

impl From<PlanError> for RuntimeError {
    fn from(e: PlanError) -> Self {
        RuntimeError::Plan(e)
    }
}

impl From<matrix_ir::IrError> for RuntimeError {
    fn from(e: matrix_ir::IrError) -> Self {
        RuntimeError::InvalidGraph(e)
    }
}

/// The runtime — owns a [`Registry`] and exposes [`plan`] and
/// [`executors`] to callers.
pub struct Runtime {
    registry: Registry,
}

impl Runtime {
    /// Construct a runtime with the always-available CPU executor.
    /// The CPU profile must be supplied by the caller (typically a
    /// short host-machine calibration; spec MX04 §"CPU executor").
    pub fn new(cpu_profile: BackendProfile) -> Self {
        let (registry, _) = Registry::with_cpu(cpu_profile);
        Runtime { registry }
    }

    /// Construct a runtime with no executors (not even CPU).  Mostly
    /// useful for testing the empty-registry code path.
    pub fn empty() -> Self {
        Runtime {
            registry: Registry::new(),
        }
    }

    /// Register an additional executor.  Returns its assigned id.
    pub fn register(&mut self, kind: impl Into<String>, profile: BackendProfile) -> ExecutorId {
        self.registry.register(kind.into(), profile)
    }

    /// Mark an executor as healthy/unhealthy.  Unhealthy executors
    /// are skipped by the planner.
    pub fn set_healthy(&mut self, id: ExecutorId, healthy: bool) {
        self.registry.set_healthy(id, healthy);
    }

    /// Replace an executor's profile (e.g. on a `ProfileUpdated` event).
    pub fn update_profile(&mut self, id: ExecutorId, profile: BackendProfile) {
        self.registry.update_profile(id, profile);
    }

    /// All registered executors.
    pub fn executors(&self) -> &[RegisteredExecutor] {
        self.registry.all()
    }

    /// Plan a `matrix_ir::Graph` to a `ComputeGraph`.  Pure — no
    /// execution.
    pub fn plan(&self, graph: &Graph) -> Result<ComputeGraph, RuntimeError> {
        Ok(plan(graph, &self.registry)?)
    }

    /// Borrow the underlying registry for advanced inspection.
    pub fn registry(&self) -> &Registry {
        &self.registry
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use matrix_ir::{DType, GraphBuilder, Shape};

    fn cpu_profile() -> BackendProfile {
        BackendProfile {
            kind: "cpu".to_string(),
            supported_ops: 0xFFFF_FFFF,
            supported_dtypes: 0x07,
            gflops_f32: 40,
            gflops_u8: 40,
            gflops_i32: 40,
            host_to_device_bw: 100,
            device_to_host_bw: 100,
            device_internal_bw: 100,
            launch_overhead_ns: 0,
            transport_latency_ns: 0,
            on_device_mib: 8 * 1024,
            max_tensor_rank: 16,
            max_dim: u32::MAX,
        }
    }

    #[test]
    fn runtime_with_cpu_plans_simple_graph() {
        let rt = Runtime::new(cpu_profile());
        assert_eq!(rt.executors().len(), 1);

        let mut g = GraphBuilder::new();
        let a = g.input(DType::F32, Shape::from(&[3]));
        let b = g.input(DType::F32, Shape::from(&[3]));
        let c = g.add(&a, &b);
        g.output(&c);
        let g = g.build().unwrap();

        let placed = rt.plan(&g).expect("plan");
        placed.validate().expect("placed validates");
    }

    #[test]
    fn empty_runtime_fails_to_plan() {
        let rt = Runtime::empty();
        let mut g = GraphBuilder::new();
        let a = g.input(DType::F32, Shape::from(&[3]));
        g.output(&a);
        let g = g.build().unwrap();

        assert!(matches!(
            rt.plan(&g),
            Err(RuntimeError::Plan(PlanError::EmptyRegistry))
        ));
    }
}
