//! Pipeline -- compiled kernels, descriptor sets, shader modules.
//!
//! # What is a Pipeline?
//!
//! A pipeline is a **compiled kernel ready to execute**. In Vulkan terms, it
//! packages three things together:
//!
//! 1. `ShaderModule` -- the compiled program (instructions)
//! 2. `PipelineLayout` -- what data the kernel expects (descriptor set layout)
//! 3. `Pipeline` -- the combined, ready-to-dispatch object
//!
//! Think of it like a function call:
//! - ShaderModule = the function body (code)
//! - DescriptorSetLayout = the function signature (parameter types)
//! - DescriptorSet = the actual arguments (concrete buffers)
//! - Pipeline = the compiled function ready to call

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};

use gpu_core::Instruction;

use crate::protocols::DescriptorBinding;

// =========================================================================
// ID generators
// =========================================================================

static NEXT_SHADER_ID: AtomicUsize = AtomicUsize::new(0);
static NEXT_DS_LAYOUT_ID: AtomicUsize = AtomicUsize::new(0);
static NEXT_PL_LAYOUT_ID: AtomicUsize = AtomicUsize::new(0);
static NEXT_PIPELINE_ID: AtomicUsize = AtomicUsize::new(0);
static NEXT_DS_ID: AtomicUsize = AtomicUsize::new(0);

/// Reset all pipeline-related ID counters (for test isolation).
pub fn reset_pipeline_ids() {
    NEXT_SHADER_ID.store(0, Ordering::SeqCst);
    NEXT_DS_LAYOUT_ID.store(0, Ordering::SeqCst);
    NEXT_PL_LAYOUT_ID.store(0, Ordering::SeqCst);
    NEXT_PIPELINE_ID.store(0, Ordering::SeqCst);
    NEXT_DS_ID.store(0, Ordering::SeqCst);
}

// =========================================================================
// ShaderModule -- compiled program
// =========================================================================

/// A compiled program ready to be used in a pipeline.
///
/// # GPU vs Dataflow
///
/// For GPU-style devices (NVIDIA, AMD, Intel), the code is a list of
/// instructions from our GenericISA (gpu-core package).
///
/// For dataflow-style devices (TPU, ANE), the code is an operation
/// descriptor -- just the operation name and parameters.
///
/// The shader module doesn't care which -- it stores whatever code was
/// given. The pipeline compilation step adapts it to the target device.
#[derive(Debug, Clone)]
pub struct ShaderModule {
    id: usize,
    code: Option<Vec<Instruction>>,
    operation: String,
    entry_point: String,
    local_size: (usize, usize, usize),
}

impl ShaderModule {
    pub fn new(
        code: Option<Vec<Instruction>>,
        operation: &str,
        entry_point: &str,
        local_size: (usize, usize, usize),
    ) -> Self {
        Self {
            id: NEXT_SHADER_ID.fetch_add(1, Ordering::SeqCst),
            code,
            operation: operation.to_string(),
            entry_point: entry_point.to_string(),
            local_size,
        }
    }

    pub fn module_id(&self) -> usize {
        self.id
    }

    /// GPU-style: list of instructions. None for dataflow.
    pub fn code(&self) -> Option<&[Instruction]> {
        self.code.as_deref()
    }

    /// Dataflow-style: operation name. Empty for GPU.
    pub fn operation(&self) -> &str {
        &self.operation
    }

    pub fn entry_point(&self) -> &str {
        &self.entry_point
    }

    /// Workgroup dimensions declared in the shader.
    pub fn local_size(&self) -> (usize, usize, usize) {
        self.local_size
    }

    /// True if this is a GPU-style shader (has instruction code).
    pub fn is_gpu_style(&self) -> bool {
        self.code.is_some()
    }

    /// True if this is a dataflow-style shader (has operation name).
    pub fn is_dataflow_style(&self) -> bool {
        !self.operation.is_empty()
    }
}

// =========================================================================
// DescriptorSetLayout -- describes the shape of data bindings
// =========================================================================

/// Describes what data a kernel expects.
///
/// A layout is like a function signature -- it says "this kernel takes
/// 3 storage buffers." It doesn't say WHICH buffers, just how many
/// and what type.
#[derive(Debug, Clone)]
pub struct DescriptorSetLayout {
    id: usize,
    bindings: Vec<DescriptorBinding>,
}

impl DescriptorSetLayout {
    pub fn new(bindings: Vec<DescriptorBinding>) -> Self {
        Self {
            id: NEXT_DS_LAYOUT_ID.fetch_add(1, Ordering::SeqCst),
            bindings,
        }
    }

    pub fn layout_id(&self) -> usize {
        self.id
    }

    pub fn bindings(&self) -> &[DescriptorBinding] {
        &self.bindings
    }
}

// =========================================================================
// PipelineLayout -- shader + descriptor layout + push constants
// =========================================================================

/// Describes the complete interface of a pipeline.
///
/// Combines:
/// - Descriptor set layouts (what buffers the kernel reads/writes)
/// - Push constant size (small inline data like alpha in SAXPY)
#[derive(Debug, Clone)]
pub struct PipelineLayout {
    id: usize,
    set_layouts: Vec<DescriptorSetLayout>,
    push_constant_size: usize,
}

impl PipelineLayout {
    pub fn new(set_layouts: Vec<DescriptorSetLayout>, push_constant_size: usize) -> Self {
        Self {
            id: NEXT_PL_LAYOUT_ID.fetch_add(1, Ordering::SeqCst),
            set_layouts,
            push_constant_size,
        }
    }

    pub fn layout_id(&self) -> usize {
        self.id
    }

    pub fn set_layouts(&self) -> &[DescriptorSetLayout] {
        &self.set_layouts
    }

    pub fn push_constant_size(&self) -> usize {
        self.push_constant_size
    }
}

// =========================================================================
// Pipeline -- compiled, ready to dispatch
// =========================================================================

/// A compiled kernel bound to a pipeline layout.
///
/// Once created, bind it in a command buffer:
/// ```text
/// cb.cmd_bind_pipeline(pipeline_id)
/// cb.cmd_dispatch(grid_x, grid_y, grid_z)
/// ```
#[derive(Debug, Clone)]
pub struct Pipeline {
    id: usize,
    shader: ShaderModule,
    layout: PipelineLayout,
}

impl Pipeline {
    pub fn new(shader: ShaderModule, layout: PipelineLayout) -> Self {
        Self {
            id: NEXT_PIPELINE_ID.fetch_add(1, Ordering::SeqCst),
            shader,
            layout,
        }
    }

    pub fn pipeline_id(&self) -> usize {
        self.id
    }

    pub fn shader(&self) -> &ShaderModule {
        &self.shader
    }

    pub fn layout(&self) -> &PipelineLayout {
        &self.layout
    }

    pub fn workgroup_size(&self) -> (usize, usize, usize) {
        self.shader.local_size()
    }
}

// =========================================================================
// DescriptorSet -- concrete buffer bindings
// =========================================================================

/// Concrete buffer assignments for a descriptor set layout.
///
/// # Layout vs Set
///
/// Layout says: "binding 0 is a storage buffer"
/// Set says:    "binding 0 is buf#3 (address 0x1000, 4096 bytes)"
///
/// You create a set from a layout, then `write()` buffer IDs into it.
/// Multiple sets can share the same layout with different buffers.
#[derive(Debug, Clone)]
pub struct DescriptorSet {
    id: usize,
    layout: DescriptorSetLayout,
    bindings: HashMap<usize, usize>, // binding slot -> buffer_id
}

impl DescriptorSet {
    pub fn new(layout: DescriptorSetLayout) -> Self {
        Self {
            id: NEXT_DS_ID.fetch_add(1, Ordering::SeqCst),
            layout,
            bindings: HashMap::new(),
        }
    }

    pub fn set_id(&self) -> usize {
        self.id
    }

    pub fn layout(&self) -> &DescriptorSetLayout {
        &self.layout
    }

    pub fn bindings(&self) -> &HashMap<usize, usize> {
        &self.bindings
    }

    /// Bind a buffer (by ID) to a slot.
    ///
    /// Returns an error if the binding doesn't exist in the layout.
    pub fn write(&mut self, binding: usize, buffer_id: usize) -> Result<(), String> {
        let valid_bindings: Vec<usize> = self.layout.bindings().iter().map(|b| b.binding).collect();
        if !valid_bindings.contains(&binding) {
            return Err(format!(
                "Binding {} not in layout (valid: {:?})",
                binding, valid_bindings
            ));
        }
        self.bindings.insert(binding, buffer_id);
        Ok(())
    }

    /// Get the buffer ID at a binding slot, or None if not bound.
    pub fn get_buffer_id(&self, binding: usize) -> Option<usize> {
        self.bindings.get(&binding).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpu_core::opcodes::{halt, limm};

    #[test]
    fn test_shader_module_gpu_style() {
        let shader = ShaderModule::new(
            Some(vec![limm(0, 42.0), halt()]),
            "",
            "main",
            (32, 1, 1),
        );
        assert!(shader.is_gpu_style());
        assert!(!shader.is_dataflow_style());
        assert!(shader.code().is_some());
        assert_eq!(shader.local_size(), (32, 1, 1));
    }

    #[test]
    fn test_shader_module_dataflow_style() {
        let shader = ShaderModule::new(None, "matmul", "main", (1, 1, 1));
        assert!(!shader.is_gpu_style());
        assert!(shader.is_dataflow_style());
        assert!(shader.code().is_none());
        assert_eq!(shader.operation(), "matmul");
    }

    #[test]
    fn test_descriptor_set_layout() {
        let layout = DescriptorSetLayout::new(vec![
            DescriptorBinding::new(0),
            DescriptorBinding::new(1).with_type("uniform"),
        ]);
        assert_eq!(layout.bindings().len(), 2);
    }

    #[test]
    fn test_pipeline_layout() {
        let ds_layout = DescriptorSetLayout::new(vec![]);
        let pl_layout = PipelineLayout::new(vec![ds_layout], 64);
        assert_eq!(pl_layout.push_constant_size(), 64);
        assert_eq!(pl_layout.set_layouts().len(), 1);
    }

    #[test]
    fn test_pipeline() {
        let shader = ShaderModule::new(
            Some(vec![limm(0, 1.0), halt()]),
            "",
            "main",
            (256, 1, 1),
        );
        let layout = PipelineLayout::new(vec![], 0);
        let pipeline = Pipeline::new(shader, layout);
        assert_eq!(pipeline.workgroup_size(), (256, 1, 1));
    }

    #[test]
    fn test_descriptor_set_write_and_read() {
        let layout = DescriptorSetLayout::new(vec![
            DescriptorBinding::new(0),
            DescriptorBinding::new(1),
        ]);
        let mut ds = DescriptorSet::new(layout);

        ds.write(0, 42).unwrap();
        ds.write(1, 99).unwrap();

        assert_eq!(ds.get_buffer_id(0), Some(42));
        assert_eq!(ds.get_buffer_id(1), Some(99));
        assert_eq!(ds.get_buffer_id(2), None);
    }

    #[test]
    fn test_descriptor_set_invalid_binding() {
        let layout = DescriptorSetLayout::new(vec![DescriptorBinding::new(0)]);
        let mut ds = DescriptorSet::new(layout);
        assert!(ds.write(5, 10).is_err());
    }
}
