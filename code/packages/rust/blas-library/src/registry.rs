//! Backend Registry -- find and select BLAS backends.
//!
//! # What is the Registry?
//!
//! The registry is a central catalog of available BLAS backends. It provides
//! three modes of selection:
//!
//! 1. **Explicit**:    `registry.get("cuda")`     -- give me CUDA specifically
//! 2. **Auto-detect**: `registry.get_best()`      -- give me the best available
//! 3. **Custom**:      `registry.register(...)`   -- add my own backend
//!
//! # Auto-Detection Priority
//!
//! When you ask for "the best available backend," the registry tries each
//! backend in priority order and returns the first one that successfully
//! initializes:
//!
//! ```text
//!     cuda > metal > vulkan > opencl > webgpu > opengl > cpu
//! ```
//!
//! CUDA is first because it's the most optimized for ML (and most GPUs are
//! NVIDIA in data centers). CPU is always last -- it's the universal fallback
//! that works everywhere.
//!
//! # How It Works Internally
//!
//! The registry stores *factory functions* (not instances). When you call
//! `get("cuda")`, it calls the factory to create a new backend on the spot.
//! This is because GPU backends allocate device resources, and we don't want
//! to waste GPU memory on backends that aren't being used.

use crate::backends::cpu::CpuBlas;
use crate::backends::cuda::CudaBlas;
use crate::backends::gpu_base::GpuBlasWrapper;
use crate::backends::metal::MetalBlas;
use crate::backends::opencl::OpenClBlas;
use crate::backends::opengl::OpenGlBlas;
use crate::backends::vulkan::VulkanBlas;
use crate::backends::webgpu::WebGpuBlas;
use crate::traits::BlasBackend;

// =========================================================================
// BackendEntry -- a named factory for creating backends
// =========================================================================

/// A registered backend: a name and a factory function that creates it.
///
/// We store factories (closures) instead of instances because GPU backends
/// allocate device resources on construction. We only want to allocate when
/// the user actually requests that backend.
struct BackendEntry {
    name: String,
    factory: Box<dyn Fn() -> Result<Box<dyn BlasBackend>, String>>,
}

// =========================================================================
// BackendRegistry -- the central catalog
// =========================================================================

/// Backend registry -- find and select BLAS backends.
///
/// # Backend Registry
///
/// The registry keeps track of which backends are available and helps the
/// caller pick one. Three modes of selection:
///
/// 1. **Explicit**:    `registry.get("cuda")`
/// 2. **Auto-detect**: `registry.get_best()`
/// 3. **Custom**:      `registry.register("my_backend", factory_fn)`
///
/// # Example
///
/// ```
/// use blas_library::BackendRegistry;
///
/// let registry = BackendRegistry::with_defaults();
/// let names = registry.list_available();
/// assert!(names.contains(&"cpu".to_string()));
///
/// // Get the CPU backend (always available)
/// let backend = registry.get("cpu").unwrap();
/// assert_eq!(backend.name(), "cpu");
/// ```
pub struct BackendRegistry {
    /// Registered backends in insertion order.
    backends: Vec<BackendEntry>,
    /// Priority order for auto-detection. First entry = highest priority.
    priority: Vec<String>,
}

/// The default auto-detection priority order.
///
/// CUDA first (ML standard), CPU last (universal fallback).
const DEFAULT_PRIORITY: &[&str] = &[
    "cuda", "metal", "vulkan", "opencl", "webgpu", "opengl", "cpu",
];

impl BackendRegistry {
    /// Create an empty registry with default priority order.
    pub fn new() -> Self {
        Self {
            backends: Vec::new(),
            priority: DEFAULT_PRIORITY.iter().map(|s| s.to_string()).collect(),
        }
    }

    /// Create a registry pre-populated with all seven built-in backends.
    ///
    /// This is the most common way to create a registry. It registers all
    /// backends, but none are instantiated until you call `get()` or
    /// `get_best()`.
    pub fn with_defaults() -> Self {
        let mut registry = Self::new();

        // CPU -- always available, no hardware requirements
        registry.register(
            "cpu",
            Box::new(|| Ok(Box::new(CpuBlas) as Box<dyn BlasBackend>)),
        );

        // CUDA -- NVIDIA GPUs
        registry.register(
            "cuda",
            Box::new(|| {
                let gpu = CudaBlas::new()?;
                Ok(Box::new(GpuBlasWrapper::new(gpu)) as Box<dyn BlasBackend>)
            }),
        );

        // Metal -- Apple Silicon
        registry.register(
            "metal",
            Box::new(|| {
                let gpu = MetalBlas::new()?;
                Ok(Box::new(GpuBlasWrapper::new(gpu)) as Box<dyn BlasBackend>)
            }),
        );

        // Vulkan -- cross-platform, maximum control
        registry.register(
            "vulkan",
            Box::new(|| {
                let gpu = VulkanBlas::new()?;
                Ok(Box::new(GpuBlasWrapper::new(gpu)) as Box<dyn BlasBackend>)
            }),
        );

        // OpenCL -- portable, event-driven
        registry.register(
            "opencl",
            Box::new(|| {
                let gpu = OpenClBlas::new()?;
                Ok(Box::new(GpuBlasWrapper::new(gpu)) as Box<dyn BlasBackend>)
            }),
        );

        // WebGPU -- browser-friendly
        registry.register(
            "webgpu",
            Box::new(|| {
                let gpu = WebGpuBlas::new()?;
                Ok(Box::new(GpuBlasWrapper::new(gpu)) as Box<dyn BlasBackend>)
            }),
        );

        // OpenGL -- legacy state machine
        registry.register(
            "opengl",
            Box::new(|| {
                let gpu = OpenGlBlas::new()?;
                Ok(Box::new(GpuBlasWrapper::new(gpu)) as Box<dyn BlasBackend>)
            }),
        );

        registry
    }

    /// Register a backend factory by name.
    ///
    /// The factory is stored but NOT called yet. Instantiation happens
    /// when `get()` or `get_best()` is called.
    pub fn register(
        &mut self,
        name: &str,
        factory: Box<dyn Fn() -> Result<Box<dyn BlasBackend>, String>>,
    ) {
        // Remove any existing entry with the same name
        self.backends.retain(|e| e.name != name);
        self.backends.push(BackendEntry {
            name: name.to_string(),
            factory,
        });
    }

    /// Get a specific backend by name, instantiating it on demand.
    ///
    /// # Errors
    ///
    /// Returns an error if the backend name is not registered, or if the
    /// factory function fails (e.g., no GPU driver installed).
    pub fn get(&self, name: &str) -> Result<Box<dyn BlasBackend>, String> {
        let entry = self
            .backends
            .iter()
            .find(|e| e.name == name)
            .ok_or_else(|| {
                let available: Vec<&str> = self.backends.iter().map(|e| e.name.as_str()).collect();
                format!(
                    "Backend '{}' not registered. Available: {}",
                    name,
                    available.join(", ")
                )
            })?;
        (entry.factory)()
    }

    /// Try each backend in priority order, return the first that works.
    ///
    /// Each backend is instantiated inside a catch. If initialization
    /// fails (e.g., no GPU available), we skip to the next one. CPU always
    /// works, so this never fails (as long as CPU is registered).
    ///
    /// # Errors
    ///
    /// Returns an error if no backend could be initialized.
    pub fn get_best(&self) -> Result<Box<dyn BlasBackend>, String> {
        for name in &self.priority {
            if let Some(entry) = self.backends.iter().find(|e| &e.name == name) {
                match (entry.factory)() {
                    Ok(backend) => return Ok(backend),
                    Err(_) => continue,
                }
            }
        }
        Err(format!(
            "No BLAS backend could be initialized. Tried: {:?}",
            self.priority
        ))
    }

    /// List names of all registered backends.
    pub fn list_available(&self) -> Vec<String> {
        self.backends.iter().map(|e| e.name.clone()).collect()
    }

    /// Change the auto-detection priority order.
    ///
    /// The first entry has the highest priority.
    pub fn set_priority(&mut self, priority: Vec<String>) {
        self.priority = priority;
    }

    /// Get the current priority order.
    pub fn priority(&self) -> &[String] {
        &self.priority
    }
}

impl Default for BackendRegistry {
    fn default() -> Self {
        Self::with_defaults()
    }
}
