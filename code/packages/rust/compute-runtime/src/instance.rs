//! Instance -- device discovery, physical/logical device management.
//!
//! # The Entry Point
//!
//! The RuntimeInstance is how everything starts. It's the first object you
//! create, and it gives you access to all available hardware:
//!
//! ```text
//! let instance = RuntimeInstance::new(None);
//! let devices = instance.enumerate_physical_devices();
//! // -> [PhysicalDevice("NVIDIA H100"), PhysicalDevice("Apple M3 Max ANE"), ...]
//! ```
//!
//! # Physical vs Logical Device
//!
//! A `PhysicalDevice` is a read-only description of hardware. You can query
//! its name, type, memory, and capabilities, but you can't use it directly.
//!
//! A `LogicalDevice` is a usable handle. It wraps a PhysicalDevice and provides:
//! - Command queues for submitting work
//! - Memory manager for allocating buffers
//! - Factory methods for pipelines, sync objects, etc.
//!
//! Why the separation?
//! - A system may have multiple GPUs. You query all of them, compare, and pick.
//! - Multiple logical devices can share one physical device.
//! - The physical device never changes. The logical device owns mutable state.
//!
//! This pattern comes directly from Vulkan (VkPhysicalDevice vs VkDevice).

use std::collections::HashMap;

use device_simulator::{
    AcceleratorDevice, AmdGPU, AppleANE, GoogleTPU, IntelGPU, NvidiaGPU,
};

use crate::command_buffer::CommandBuffer;
use crate::command_queue::CommandQueue;
use crate::memory::MemoryManager;
use crate::pipeline::{
    DescriptorSet, DescriptorSetLayout, Pipeline, PipelineLayout, ShaderModule,
};
use crate::protocols::{
    DescriptorBinding, DeviceLimits, DeviceType, MemoryHeap, MemoryProperties, MemoryType,
    QueueFamily, QueueType, RuntimeStats, RuntimeTrace,
};
use crate::sync::{Event, Fence, Semaphore};

use gpu_core::Instruction;

// =========================================================================
// PhysicalDevice -- read-only hardware description
// =========================================================================

/// Read-only description of a physical accelerator.
///
/// You can learn:
/// - name, device_type, vendor
/// - memory_properties: what memory types are available, how much
/// - queue_families: what kinds of queues the device supports
/// - limits: hardware constraints
///
/// You can't execute anything on a PhysicalDevice. Create a LogicalDevice
/// for that.
pub struct PhysicalDevice {
    pub(crate) device_id: usize,
    name: String,
    device_type: DeviceType,
    vendor: String,
    memory_properties: MemoryProperties,
    queue_families: Vec<QueueFamily>,
    limits: DeviceLimits,
    /// Stored for creating the LogicalDevice later.
    accelerator_factory: AcceleratorFactory,
}

/// Factory that can produce a fresh AcceleratorDevice for a LogicalDevice.
///
/// We store the parameters needed to recreate the device, because
/// AcceleratorDevice is not Clone and we may want multiple LogicalDevices.
enum AcceleratorFactory {
    Nvidia { num_sms: usize },
    Amd { num_cus: usize },
    Google { mxu_size: usize },
    Intel { num_cores: usize },
    Apple { num_cores: usize },
}

impl AcceleratorFactory {
    fn create(&self) -> Box<dyn AcceleratorDevice> {
        match self {
            AcceleratorFactory::Nvidia { num_sms } => Box::new(NvidiaGPU::new(None, *num_sms)),
            AcceleratorFactory::Amd { num_cus } => Box::new(AmdGPU::new(None, *num_cus)),
            AcceleratorFactory::Google { mxu_size } => Box::new(GoogleTPU::new(None, *mxu_size)),
            AcceleratorFactory::Intel { num_cores } => Box::new(IntelGPU::new(None, *num_cores)),
            AcceleratorFactory::Apple { num_cores } => Box::new(AppleANE::new(None, *num_cores)),
        }
    }
}

impl PhysicalDevice {
    pub fn device_id(&self) -> usize {
        self.device_id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn device_type(&self) -> DeviceType {
        self.device_type
    }

    pub fn vendor(&self) -> &str {
        &self.vendor
    }

    pub fn memory_properties(&self) -> &MemoryProperties {
        &self.memory_properties
    }

    pub fn queue_families(&self) -> &[QueueFamily] {
        &self.queue_families
    }

    pub fn limits(&self) -> &DeviceLimits {
        &self.limits
    }

    /// Check if a feature is supported.
    ///
    /// Currently supported features:
    /// - "fp32": 32-bit float (always true)
    /// - "fp16": 16-bit float (always true)
    /// - "unified_memory": CPU/GPU shared memory
    /// - "transfer_queue": dedicated DMA engine
    pub fn supports_feature(&self, feature: &str) -> bool {
        match feature {
            "fp32" | "fp16" => true,
            "unified_memory" => self.memory_properties.is_unified,
            "transfer_queue" => self
                .queue_families
                .iter()
                .any(|qf| qf.queue_type == QueueType::Transfer),
            _ => false,
        }
    }
}

// =========================================================================
// LogicalDevice -- usable handle with queues and factories
// =========================================================================

/// A usable device handle with command queues and resource factories.
///
/// - Submit work via command queues
/// - Allocate memory via memory_manager
/// - Create command buffers, pipelines, sync objects
/// - Wait for all work to complete
pub struct LogicalDevice {
    physical_device_id: usize,
    physical_device_name: String,
    physical_device_type: DeviceType,
    physical_vendor: String,
    physical_memory_properties: MemoryProperties,
    physical_queue_families: Vec<QueueFamily>,
    physical_limits: DeviceLimits,

    queues: HashMap<String, Vec<CommandQueue>>,
    memory_manager: MemoryManager,
    stats: Box<RuntimeStats>,
    pipelines: HashMap<usize, Pipeline>,
}

impl LogicalDevice {
    pub fn physical_device_id(&self) -> usize {
        self.physical_device_id
    }

    pub fn physical_device_name(&self) -> &str {
        &self.physical_device_name
    }

    pub fn physical_device_type(&self) -> DeviceType {
        self.physical_device_type
    }

    pub fn physical_vendor(&self) -> &str {
        &self.physical_vendor
    }

    pub fn physical_memory_properties(&self) -> &MemoryProperties {
        &self.physical_memory_properties
    }

    pub fn physical_queue_families(&self) -> &[QueueFamily] {
        &self.physical_queue_families
    }

    pub fn physical_limits(&self) -> &DeviceLimits {
        &self.physical_limits
    }

    pub fn queues(&self) -> &HashMap<String, Vec<CommandQueue>> {
        &self.queues
    }

    pub fn queues_mut(&mut self) -> &mut HashMap<String, Vec<CommandQueue>> {
        &mut self.queues
    }

    pub fn memory_manager(&self) -> &MemoryManager {
        &self.memory_manager
    }

    pub fn memory_manager_mut(&mut self) -> &mut MemoryManager {
        &mut self.memory_manager
    }

    pub fn stats(&self) -> &RuntimeStats {
        &self.stats
    }

    pub fn pipelines(&self) -> &HashMap<usize, Pipeline> {
        &self.pipelines
    }

    // --- Factory methods ---

    pub fn create_command_buffer(&self) -> CommandBuffer {
        CommandBuffer::new()
    }

    pub fn create_shader_module(
        &self,
        code: Option<Vec<Instruction>>,
        operation: &str,
        entry_point: &str,
        local_size: (usize, usize, usize),
    ) -> ShaderModule {
        ShaderModule::new(code, operation, entry_point, local_size)
    }

    pub fn create_descriptor_set_layout(
        &self,
        bindings: Vec<DescriptorBinding>,
    ) -> DescriptorSetLayout {
        DescriptorSetLayout::new(bindings)
    }

    pub fn create_pipeline_layout(
        &self,
        set_layouts: Vec<DescriptorSetLayout>,
        push_constant_size: usize,
    ) -> PipelineLayout {
        PipelineLayout::new(set_layouts, push_constant_size)
    }

    /// Create a compute pipeline and register it internally.
    ///
    /// Returns the pipeline ID that can be used with command buffers.
    pub fn create_compute_pipeline(
        &mut self,
        shader: ShaderModule,
        layout: PipelineLayout,
    ) -> usize {
        let pipeline = Pipeline::new(shader, layout);
        let id = pipeline.pipeline_id();
        self.pipelines.insert(id, pipeline);
        id
    }

    pub fn create_descriptor_set(&self, layout: DescriptorSetLayout) -> DescriptorSet {
        DescriptorSet::new(layout)
    }

    pub fn create_fence(&self, signaled: bool) -> Fence {
        Fence::new(signaled)
    }

    pub fn create_semaphore(&self) -> Semaphore {
        Semaphore::new()
    }

    pub fn create_event(&self) -> Event {
        Event::new()
    }

    /// Block until all queues finish all pending work.
    pub fn wait_idle(&self) {
        for queue_list in self.queues.values() {
            for queue in queue_list {
                queue.wait_idle();
            }
        }
    }

    /// Submit command buffers to a named queue.
    ///
    /// Convenience method that looks up the queue by type name and index,
    /// then delegates to `CommandQueue::submit`.
    pub fn submit(
        &mut self,
        queue_type_name: &str,
        queue_index: usize,
        command_buffers: &mut [&mut CommandBuffer],
        wait_semaphores: &mut [&mut Semaphore],
        signal_semaphores: &mut [&mut Semaphore],
        fence: Option<&mut Fence>,
    ) -> Result<Vec<RuntimeTrace>, String> {
        // We need to take the queue out temporarily to avoid borrow issues
        let mut queue_list = self
            .queues
            .remove(queue_type_name)
            .ok_or_else(|| format!("No queues of type '{}'", queue_type_name))?;

        if queue_index >= queue_list.len() {
            self.queues
                .insert(queue_type_name.to_string(), queue_list);
            return Err(format!(
                "Queue index {} out of range for '{}'",
                queue_index, queue_type_name
            ));
        }

        let result = queue_list[queue_index].submit(
            command_buffers,
            wait_semaphores,
            signal_semaphores,
            fence,
            &self.pipelines,
            &mut self.memory_manager,
        );

        self.queues
            .insert(queue_type_name.to_string(), queue_list);

        result
    }
}

// =========================================================================
// RuntimeInstance -- the entry point
// =========================================================================

/// Helper to build memory properties from device config.
fn make_memory_properties(is_unified: bool, global_memory_size: u64) -> MemoryProperties {
    if is_unified {
        MemoryProperties {
            heaps: vec![MemoryHeap {
                size: global_memory_size,
                flags: MemoryType::DEVICE_LOCAL | MemoryType::HOST_VISIBLE | MemoryType::HOST_COHERENT,
            }],
            is_unified: true,
        }
    } else {
        MemoryProperties {
            heaps: vec![
                MemoryHeap {
                    size: global_memory_size,
                    flags: MemoryType::DEVICE_LOCAL,
                },
                MemoryHeap {
                    size: std::cmp::min(global_memory_size / 4, 256 * 1024 * 1024),
                    flags: MemoryType::HOST_VISIBLE | MemoryType::HOST_COHERENT,
                },
            ],
            is_unified: false,
        }
    }
}

/// Helper to build queue families.
fn make_queue_families(is_unified: bool) -> Vec<QueueFamily> {
    let mut families = vec![QueueFamily {
        queue_type: QueueType::Compute,
        count: 4,
    }];
    if !is_unified {
        families.push(QueueFamily {
            queue_type: QueueType::Transfer,
            count: 2,
        });
    }
    families
}

/// The runtime entry point -- discovers devices and creates handles.
///
/// # Usage
///
/// ```text
/// let instance = RuntimeInstance::new(None);
/// let devices = instance.enumerate_physical_devices();
/// let device = instance.create_logical_device(0, None);
/// ```
pub struct RuntimeInstance {
    version: String,
    physical_devices: Vec<PhysicalDevice>,
}

impl RuntimeInstance {
    /// Create a runtime instance.
    ///
    /// If `devices` is None, creates default test devices (small configs).
    pub fn new(
        devices: Option<Vec<(Box<dyn AcceleratorDevice>, DeviceType, String)>>,
    ) -> Self {
        let physical_devices = if let Some(devs) = devices {
            devs.into_iter()
                .enumerate()
                .map(|(i, (dev, dtype, vendor))| {
                    let config = dev.config().clone();
                    let is_unified = config.unified_memory;
                    let mem_props = make_memory_properties(is_unified, config.global_memory_size);
                    let queue_families = make_queue_families(is_unified);

                    // Determine factory from vendor
                    let factory = match vendor.as_str() {
                        "nvidia" => AcceleratorFactory::Nvidia {
                            num_sms: config.num_compute_units,
                        },
                        "amd" => AcceleratorFactory::Amd {
                            num_cus: config.num_compute_units,
                        },
                        "google" => AcceleratorFactory::Google {
                            mxu_size: config.num_compute_units,
                        },
                        "intel" => AcceleratorFactory::Intel {
                            num_cores: config.num_compute_units,
                        },
                        "apple" => AcceleratorFactory::Apple {
                            num_cores: config.num_compute_units,
                        },
                        _ => AcceleratorFactory::Nvidia {
                            num_sms: config.num_compute_units,
                        },
                    };

                    PhysicalDevice {
                        device_id: i,
                        name: dev.name().to_string(),
                        device_type: dtype,
                        vendor,
                        memory_properties: mem_props,
                        queue_families,
                        limits: DeviceLimits::default(),
                        accelerator_factory: factory,
                    }
                })
                .collect()
        } else {
            Self::create_default_devices()
        };

        Self {
            version: "0.1.0".to_string(),
            physical_devices,
        }
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    /// Return all available physical devices.
    pub fn enumerate_physical_devices(&self) -> Vec<&PhysicalDevice> {
        self.physical_devices.iter().collect()
    }

    /// Create a logical device from a physical device by index.
    ///
    /// `queue_requests` is a list of (type_name, count) pairs.
    /// Default: one compute queue.
    pub fn create_logical_device(
        &self,
        physical_device_index: usize,
        queue_requests: Option<Vec<(&str, usize)>>,
    ) -> Result<LogicalDevice, String> {
        let pd = self
            .physical_devices
            .get(physical_device_index)
            .ok_or_else(|| format!("Physical device {} not found", physical_device_index))?;

        let queue_requests = queue_requests.unwrap_or_else(|| vec![("compute", 1)]);

        let mut stats = Box::new(RuntimeStats::default());
        let stats_ptr: *mut RuntimeStats = &mut *stats;

        let device = pd.accelerator_factory.create();
        let memory_manager = MemoryManager::new(
            device,
            pd.memory_properties.clone(),
            stats_ptr,
        );

        let mut queues: HashMap<String, Vec<CommandQueue>> = HashMap::new();
        for (qt_str, count) in queue_requests {
            let qt = QueueType::from_str_tag(qt_str);
            let queue_list: Vec<CommandQueue> = (0..count)
                .map(|i| CommandQueue::new(qt, i))
                .collect();
            queues.insert(qt_str.to_string(), queue_list);
        }

        Ok(LogicalDevice {
            physical_device_id: pd.device_id,
            physical_device_name: pd.name.clone(),
            physical_device_type: pd.device_type,
            physical_vendor: pd.vendor.clone(),
            physical_memory_properties: pd.memory_properties.clone(),
            physical_queue_families: pd.queue_families.clone(),
            physical_limits: pd.limits.clone(),
            queues,
            memory_manager,
            stats,
            pipelines: HashMap::new(),
        })
    }

    fn create_default_devices() -> Vec<PhysicalDevice> {
        let defaults: Vec<(Box<dyn AcceleratorDevice>, DeviceType, &str, AcceleratorFactory)> = vec![
            (
                Box::new(NvidiaGPU::new(None, 2)),
                DeviceType::GPU,
                "nvidia",
                AcceleratorFactory::Nvidia { num_sms: 2 },
            ),
            (
                Box::new(AmdGPU::new(None, 2)),
                DeviceType::GPU,
                "amd",
                AcceleratorFactory::Amd { num_cus: 2 },
            ),
            (
                Box::new(GoogleTPU::new(None, 2)),
                DeviceType::TPU,
                "google",
                AcceleratorFactory::Google { mxu_size: 2 },
            ),
            (
                Box::new(IntelGPU::new(None, 2)),
                DeviceType::GPU,
                "intel",
                AcceleratorFactory::Intel { num_cores: 2 },
            ),
            (
                Box::new(AppleANE::new(None, 2)),
                DeviceType::NPU,
                "apple",
                AcceleratorFactory::Apple { num_cores: 2 },
            ),
        ];

        defaults
            .into_iter()
            .enumerate()
            .map(|(i, (dev, dtype, vendor, factory))| {
                let config = dev.config().clone();
                let is_unified = config.unified_memory;
                let mem_props = make_memory_properties(is_unified, config.global_memory_size);
                let queue_families = make_queue_families(is_unified);

                PhysicalDevice {
                    device_id: i,
                    name: dev.name().to_string(),
                    device_type: dtype,
                    vendor: vendor.to_string(),
                    memory_properties: mem_props,
                    queue_families,
                    limits: DeviceLimits::default(),
                    accelerator_factory: factory,
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enumerate_default_devices() {
        let instance = RuntimeInstance::new(None);
        let devices = instance.enumerate_physical_devices();
        assert_eq!(devices.len(), 5);
    }

    #[test]
    fn test_device_names() {
        let instance = RuntimeInstance::new(None);
        let devices = instance.enumerate_physical_devices();
        let names: Vec<&str> = devices.iter().map(|d| d.name()).collect();
        assert!(names.iter().any(|n| n.contains("NVIDIA") || n.contains("Nvidia") || n.to_lowercase().contains("nvidia")));
    }

    #[test]
    fn test_device_types() {
        let instance = RuntimeInstance::new(None);
        let devices = instance.enumerate_physical_devices();
        let types: Vec<DeviceType> = devices.iter().map(|d| d.device_type()).collect();
        assert!(types.contains(&DeviceType::GPU));
        assert!(types.contains(&DeviceType::TPU));
        assert!(types.contains(&DeviceType::NPU));
    }

    #[test]
    fn test_feature_query() {
        let instance = RuntimeInstance::new(None);
        let devices = instance.enumerate_physical_devices();
        // All devices support fp32
        for d in &devices {
            assert!(d.supports_feature("fp32"));
        }
        // Apple has unified memory
        let apple = devices.iter().find(|d| d.vendor() == "apple").unwrap();
        assert!(apple.supports_feature("unified_memory"));
    }

    #[test]
    fn test_create_logical_device() {
        let instance = RuntimeInstance::new(None);
        let device = instance.create_logical_device(0, None).unwrap();
        assert!(device.queues().contains_key("compute"));
        assert_eq!(device.queues()["compute"].len(), 1);
    }

    #[test]
    fn test_create_logical_device_with_queues() {
        let instance = RuntimeInstance::new(None);
        let device = instance
            .create_logical_device(0, Some(vec![("compute", 2), ("transfer", 1)]))
            .unwrap();
        assert_eq!(device.queues()["compute"].len(), 2);
        assert_eq!(device.queues()["transfer"].len(), 1);
    }

    #[test]
    fn test_invalid_device_index() {
        let instance = RuntimeInstance::new(None);
        assert!(instance.create_logical_device(99, None).is_err());
    }

    #[test]
    fn test_logical_device_factories() {
        let instance = RuntimeInstance::new(None);
        let mut device = instance.create_logical_device(0, None).unwrap();

        let cb = device.create_command_buffer();
        assert_eq!(cb.state(), crate::protocols::CommandBufferState::Initial);

        let fence = device.create_fence(false);
        assert!(!fence.signaled());

        let sem = device.create_semaphore();
        assert!(!sem.signaled());

        let event = device.create_event();
        assert!(!event.signaled());

        let shader = device.create_shader_module(None, "matmul", "main", (1, 1, 1));
        assert!(shader.is_dataflow_style());

        let ds_layout = device.create_descriptor_set_layout(vec![]);
        let pl_layout = device.create_pipeline_layout(vec![ds_layout.clone()], 0);
        let pipeline_id = device.create_compute_pipeline(shader, pl_layout);
        assert!(device.pipelines().contains_key(&pipeline_id));
    }

    #[test]
    fn test_version() {
        let instance = RuntimeInstance::new(None);
        assert_eq!(instance.version(), "0.1.0");
    }
}
