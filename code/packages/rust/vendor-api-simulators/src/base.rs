//! BaseSimulator -- the shared foundation for all six vendor API simulators.
//!
//! # Why a Base?
//!
//! Every GPU API, no matter how different its surface looks, needs to do the
//! same things underneath:
//!
//! 1. Find a GPU                  --> RuntimeInstance
//! 2. Create a usable handle      --> LogicalDevice
//! 3. Get a queue for submission   --> CommandQueue
//! 4. Manage memory                --> MemoryManager
//!
//! This module provides `BaseSimulator` which sets all that up. Each vendor
//! simulator wraps it and adds vendor-specific vocabulary on top.
//!
//! Think of it like building six different restaurant fronts (CUDA Grill,
//! Metal Bistro, Vulkan Steakhouse...) that all share the same kitchen in
//! the back. The kitchen is our compute runtime (Layer 5). The restaurant
//! menus look completely different, but the same chefs cook the same food.
//!
//! # Device Selection
//!
//! Different APIs have different preferences for which device to use:
//!
//! - CUDA always wants an NVIDIA GPU (`vendor_hint = "nvidia"`)
//! - Metal always wants an Apple device (`vendor_hint = "apple"`)
//! - OpenCL, Vulkan, WebGPU, OpenGL are cross-vendor
//!
//! The `select_device()` function handles this: it picks the best matching
//! device from the runtime's enumerated physical devices, preferring the
//! vendor hint if given, then falling back to any GPU.
//!
//! # The `create_and_submit_cb()` Helper
//!
//! CUDA and OpenGL hide command buffers from the user. When you call
//! `cuda.memcpy()` or `gl.dispatch_compute()`, those APIs internally:
//!
//! 1. Create a command buffer
//! 2. Begin recording
//! 3. Record the command(s) via a callback
//! 4. End recording
//! 5. Submit to the compute queue with a fence
//! 6. Wait for the fence
//!
//! This helper encapsulates that pattern.

use compute_runtime::instance::{LogicalDevice, RuntimeInstance};
use compute_runtime::protocols::{
    BufferUsage, DescriptorBinding, DeviceType, MemoryType,
};

/// Shared foundation for all vendor API simulators.
///
/// Every vendor simulator owns a `BaseSimulator` that holds:
/// - `instance`:        RuntimeInstance (Layer 5 entry point)
/// - `device`:          LogicalDevice (usable device handle)
/// - `device_index`:    Which physical device was selected
///
/// This is the Rust equivalent of Python's `BaseVendorSimulator` class.
/// Instead of inheritance, Rust uses composition: each simulator struct
/// contains a `BaseSimulator` field and delegates to it.
pub struct BaseSimulator {
    pub instance: RuntimeInstance,
    pub device: LogicalDevice,
    pub device_index: usize,
}

impl BaseSimulator {
    /// Create a new base simulator with optional device selection hints.
    ///
    /// # Device Selection Strategy
    ///
    /// The strategy is a multi-pass filter:
    ///
    /// 1. Try to match both `vendor_hint` AND `device_type` (if given).
    /// 2. Try `vendor_hint` only.
    /// 3. Try `device_type` only.
    /// 4. Take the first device (any will do).
    ///
    /// This ensures that:
    /// - `CUDARuntime` (vendor_hint="nvidia") gets an NVIDIA GPU
    /// - `MTLDevice` (vendor_hint="apple") gets an Apple device
    /// - `VkInstance` gets whatever is available
    pub fn new(
        device_type: Option<DeviceType>,
        vendor_hint: Option<&str>,
    ) -> Result<Self, String> {
        let instance = RuntimeInstance::new(None);
        let physical_devices = instance.enumerate_physical_devices();

        if physical_devices.is_empty() {
            return Err("No physical devices available".to_string());
        }

        let device_index = select_device(&physical_devices, device_type, vendor_hint);

        let device = instance.create_logical_device(device_index, None)?;

        Ok(Self {
            instance,
            device,
            device_index,
        })
    }

    /// Helper: create a command buffer, record commands, submit, and wait.
    ///
    /// This implements the "immediate execution" pattern used by CUDA and
    /// OpenGL, where each API call appears to execute right away. Under
    /// the hood, we still use command buffers.
    ///
    /// The `record_fn` receives a mutable reference to a command buffer
    /// in RECORDING state and should record whatever commands are needed.
    pub fn create_and_submit_cb<F>(&mut self, record_fn: F) -> Result<(), String>
    where
        F: FnOnce(&mut compute_runtime::command_buffer::CommandBuffer) -> Result<(), String>,
    {
        let mut cb = self.device.create_command_buffer();
        cb.begin()?;
        record_fn(&mut cb)?;
        cb.end()?;
        let mut fence = self.device.create_fence(false);
        self.device.submit(
            "compute",
            0,
            &mut [&mut cb],
            &mut [],
            &mut [],
            Some(&mut fence),
        )?;
        Ok(())
    }

    /// Allocate a buffer with standard flags for simulation.
    ///
    /// Most vendor APIs need HOST_VISIBLE | HOST_COHERENT | DEVICE_LOCAL
    /// so the CPU can write data and the GPU can read it. This helper
    /// avoids repeating those flags everywhere.
    pub fn allocate_buffer(&mut self, size: usize) -> Result<usize, String> {
        self.device.memory_manager_mut().allocate(
            size,
            MemoryType::DEVICE_LOCAL | MemoryType::HOST_VISIBLE | MemoryType::HOST_COHERENT,
            BufferUsage::STORAGE | BufferUsage::TRANSFER_SRC | BufferUsage::TRANSFER_DST,
        )
    }

    /// Create a compute pipeline from optional code and local size.
    ///
    /// Returns the pipeline ID, which can be used with `cmd_bind_pipeline`.
    pub fn create_pipeline(
        &mut self,
        code: Option<Vec<gpu_core::Instruction>>,
        local_size: (usize, usize, usize),
        bindings: &[usize],
    ) -> Result<(usize, usize), String> {
        let shader = self.device.create_shader_module(
            code,
            "",
            "main",
            local_size,
        );
        let descriptor_bindings: Vec<DescriptorBinding> = bindings
            .iter()
            .map(|&b| DescriptorBinding::new(b))
            .collect();
        let ds_layout = self
            .device
            .create_descriptor_set_layout(descriptor_bindings);
        let pl_layout = self
            .device
            .create_pipeline_layout(vec![ds_layout.clone()], 0);
        let pipeline_id = self.device.create_compute_pipeline(shader, pl_layout);
        let ds = self.device.create_descriptor_set(ds_layout);
        Ok((pipeline_id, ds.set_id()))
    }
}

/// Select the best matching device from a list of physical devices.
///
/// Uses a 4-pass fallback strategy:
/// 1. Match both vendor AND device type
/// 2. Match vendor only
/// 3. Match device type only
/// 4. Take the first available device
fn select_device(
    devices: &[&compute_runtime::instance::PhysicalDevice],
    device_type: Option<DeviceType>,
    vendor_hint: Option<&str>,
) -> usize {
    // Pass 1: Match both vendor and type
    if let (Some(vendor), Some(dtype)) = (vendor_hint, device_type) {
        for (i, dev) in devices.iter().enumerate() {
            if dev.vendor() == vendor && dev.device_type() == dtype {
                return i;
            }
        }
    }

    // Pass 2: Match vendor only
    if let Some(vendor) = vendor_hint {
        for (i, dev) in devices.iter().enumerate() {
            if dev.vendor() == vendor {
                return i;
            }
        }
    }

    // Pass 3: Match device type only
    if let Some(dtype) = device_type {
        for (i, dev) in devices.iter().enumerate() {
            if dev.device_type() == dtype {
                return i;
            }
        }
    }

    // Pass 4: Take whatever is available
    0
}
