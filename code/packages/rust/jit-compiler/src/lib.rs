//! # JIT Compiler — shell scaffolding for future native code generation.
//!
//! A real JIT compiler does two distinct jobs:
//!
//! 1. decide which bytecode regions are worth compiling
//! 2. manage the native blocks that replace interpretation
//!
//! This crate intentionally implements only those first management layers.
//! It does not yet lower bytecode to ARM, RISC-V, or x86 machine code.
//! Instead, it provides:
//!
//! - hot-path execution profiling
//! - threshold-based "this is hot now" detection
//! - shell native-block registration
//! - deoptimization hooks
//!
//! This keeps the package honest: we have a real Rust port for the JIT layer,
//! but we do not pretend the hard code-generation work is already done.

use std::collections::BTreeMap;

/// Target architecture the future JIT would emit native code for.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TargetIsa {
    RiscV,
    Arm,
    X86,
}

/// Configuration for a JIT compiler instance.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JitCompilerConfig {
    hot_threshold: u64,
    target: TargetIsa,
}

impl JitCompilerConfig {
    /// Create a new configuration.
    pub fn new(target: TargetIsa, hot_threshold: u64) -> Self {
        assert!(hot_threshold > 0, "hot_threshold must be > 0");
        Self {
            hot_threshold,
            target,
        }
    }

    pub fn hot_threshold(&self) -> u64 {
        self.hot_threshold
    }

    pub fn target(&self) -> TargetIsa {
        self.target
    }
}

/// Snapshot of profiling information for one bytecode offset.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HotPathProfile {
    pub bytecode_offset: usize,
    pub execution_count: u64,
    pub is_hot: bool,
}

/// Shell representation of a compiled native block.
///
/// The `machine_code` buffer remains empty in the current implementation. It
/// exists so the rest of the API already reflects the shape of a future JIT.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeBlock {
    pub bytecode_offset: usize,
    pub target: TargetIsa,
    pub machine_code: Vec<u8>,
    pub assumptions: Vec<String>,
}

/// Threshold-based JIT profiler and block registry.
#[derive(Clone, Debug)]
pub struct JitCompiler {
    config: JitCompilerConfig,
    execution_counts: BTreeMap<usize, u64>,
    native_blocks: BTreeMap<usize, NativeBlock>,
}

impl JitCompiler {
    /// Create a new shell JIT compiler.
    pub fn new(config: JitCompilerConfig) -> Self {
        Self {
            config,
            execution_counts: BTreeMap::new(),
            native_blocks: BTreeMap::new(),
        }
    }

    /// Record one execution of the bytecode instruction at `bytecode_offset`.
    ///
    /// Returns `true` exactly when the path transitions to hot on this call.
    pub fn observe_execution(&mut self, bytecode_offset: usize) -> bool {
        let count = self.execution_counts.entry(bytecode_offset).or_insert(0);
        *count += 1;
        *count == self.config.hot_threshold
    }

    /// Return the profiling snapshot for one bytecode offset, if it has ever executed.
    pub fn profile(&self, bytecode_offset: usize) -> Option<HotPathProfile> {
        self.execution_counts
            .get(&bytecode_offset)
            .copied()
            .map(|execution_count| HotPathProfile {
                bytecode_offset,
                execution_count,
                is_hot: execution_count >= self.config.hot_threshold,
            })
    }

    /// Install a shell native block for a bytecode offset.
    ///
    /// This mimics the moment where a future code generator would hand a
    /// compiled block back to the VM. For now, the machine code buffer is
    /// intentionally empty.
    pub fn install_shell_block(
        &mut self,
        bytecode_offset: usize,
        assumptions: Vec<String>,
    ) -> &NativeBlock {
        let block = NativeBlock {
            bytecode_offset,
            target: self.config.target,
            machine_code: Vec::new(),
            assumptions,
        };
        self.native_blocks.insert(bytecode_offset, block);
        self.native_blocks
            .get(&bytecode_offset)
            .expect("native block should exist immediately after insertion")
    }

    /// Check whether a native block is registered for this bytecode offset.
    pub fn has_native_block(&self, bytecode_offset: usize) -> bool {
        self.native_blocks.contains_key(&bytecode_offset)
    }

    /// Borrow the registered native block for this bytecode offset.
    pub fn native_block(&self, bytecode_offset: usize) -> Option<&NativeBlock> {
        self.native_blocks.get(&bytecode_offset)
    }

    /// Remove a native block and fall back to interpreted execution.
    pub fn deoptimize(&mut self, bytecode_offset: usize) -> Option<NativeBlock> {
        self.native_blocks.remove(&bytecode_offset)
    }

    pub fn config(&self) -> &JitCompilerConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::{JitCompiler, JitCompilerConfig, TargetIsa};

    #[test]
    fn path_becomes_hot_exactly_at_threshold() {
        let mut jit = JitCompiler::new(JitCompilerConfig::new(TargetIsa::RiscV, 3));

        assert!(!jit.observe_execution(24));
        assert!(!jit.observe_execution(24));
        assert!(jit.observe_execution(24));
        assert!(!jit.observe_execution(24));
    }

    #[test]
    fn profile_reports_execution_count_and_hotness() {
        let mut jit = JitCompiler::new(JitCompilerConfig::new(TargetIsa::Arm, 2));

        jit.observe_execution(8);
        let profile = jit.profile(8).expect("profile should exist after execution");
        assert_eq!(profile.execution_count, 1);
        assert!(!profile.is_hot);

        jit.observe_execution(8);
        let hot_profile = jit.profile(8).expect("profile should still exist");
        assert_eq!(hot_profile.execution_count, 2);
        assert!(hot_profile.is_hot);
    }

    #[test]
    fn shell_block_installation_uses_configured_target() {
        let mut jit = JitCompiler::new(JitCompilerConfig::new(TargetIsa::X86, 5));

        let block = jit.install_shell_block(32, vec!["locals stay integers".to_string()]);
        assert_eq!(block.bytecode_offset, 32);
        assert_eq!(block.target, TargetIsa::X86);
        assert!(block.machine_code.is_empty());
        assert_eq!(block.assumptions.len(), 1);
        assert!(jit.has_native_block(32));
    }

    #[test]
    fn deoptimize_removes_native_block() {
        let mut jit = JitCompiler::new(JitCompilerConfig::new(TargetIsa::RiscV, 10));
        jit.install_shell_block(99, vec!["shape stays stable".to_string()]);

        let block = jit.deoptimize(99).expect("block should be removed");
        assert_eq!(block.bytecode_offset, 99);
        assert!(!jit.has_native_block(99));
        assert!(jit.deoptimize(99).is_none());
    }
}
