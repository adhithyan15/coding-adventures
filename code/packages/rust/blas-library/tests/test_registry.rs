//! Tests for the BackendRegistry: registration, lookup, auto-detect, priority.

use blas_library::traits::BlasBackend;
use blas_library::{BackendRegistry, CpuBlas, Vector};

// =========================================================================
// Registry creation and listing
// =========================================================================

#[test]
fn test_registry_new_empty() {
    let registry = BackendRegistry::new();
    assert!(registry.list_available().is_empty());
}

#[test]
fn test_registry_with_defaults() {
    let registry = BackendRegistry::with_defaults();
    let available = registry.list_available();
    assert!(available.contains(&"cpu".to_string()));
    assert!(available.contains(&"cuda".to_string()));
    assert!(available.contains(&"metal".to_string()));
    assert!(available.contains(&"vulkan".to_string()));
    assert!(available.contains(&"opencl".to_string()));
    assert!(available.contains(&"webgpu".to_string()));
    assert!(available.contains(&"opengl".to_string()));
    assert_eq!(available.len(), 7);
}

#[test]
fn test_registry_default_trait() {
    let registry = BackendRegistry::default();
    assert_eq!(registry.list_available().len(), 7);
}

// =========================================================================
// Getting specific backends
// =========================================================================

#[test]
fn test_registry_get_cpu() {
    let registry = BackendRegistry::with_defaults();
    let backend = registry.get("cpu").unwrap();
    assert_eq!(backend.name(), "cpu");
}

#[test]
fn test_registry_get_cuda() {
    let registry = BackendRegistry::with_defaults();
    let backend = registry.get("cuda").unwrap();
    assert_eq!(backend.name(), "cuda");
}

#[test]
fn test_registry_get_metal() {
    let registry = BackendRegistry::with_defaults();
    let backend = registry.get("metal").unwrap();
    assert_eq!(backend.name(), "metal");
}

#[test]
fn test_registry_get_vulkan() {
    let registry = BackendRegistry::with_defaults();
    let backend = registry.get("vulkan").unwrap();
    assert_eq!(backend.name(), "vulkan");
}

#[test]
fn test_registry_get_opencl() {
    let registry = BackendRegistry::with_defaults();
    let backend = registry.get("opencl").unwrap();
    assert_eq!(backend.name(), "opencl");
}

#[test]
fn test_registry_get_webgpu() {
    let registry = BackendRegistry::with_defaults();
    let backend = registry.get("webgpu").unwrap();
    assert_eq!(backend.name(), "webgpu");
}

#[test]
fn test_registry_get_opengl() {
    let registry = BackendRegistry::with_defaults();
    let backend = registry.get("opengl").unwrap();
    assert_eq!(backend.name(), "opengl");
}

#[test]
fn test_registry_get_nonexistent() {
    let registry = BackendRegistry::with_defaults();
    let result = registry.get("nonexistent");
    assert!(result.is_err());
    let err = match result {
        Err(e) => e,
        Ok(_) => unreachable!(),
    };
    assert!(err.contains("not registered"));
}

#[test]
fn test_registry_get_empty() {
    let registry = BackendRegistry::new();
    assert!(registry.get("cpu").is_err());
}

// =========================================================================
// Auto-detect (get_best)
// =========================================================================

#[test]
fn test_registry_get_best() {
    let registry = BackendRegistry::with_defaults();
    let backend = registry.get_best().unwrap();
    // Should get some backend -- the first in priority that initializes
    assert!(!backend.name().is_empty());
}

#[test]
fn test_registry_get_best_empty() {
    let registry = BackendRegistry::new();
    assert!(registry.get_best().is_err());
}

#[test]
fn test_registry_get_best_cpu_only() {
    let mut registry = BackendRegistry::new();
    registry.register(
        "cpu",
        Box::new(|| Ok(Box::new(CpuBlas) as Box<dyn BlasBackend>)),
    );
    registry.set_priority(vec!["cpu".to_string()]);
    let backend = registry.get_best().unwrap();
    assert_eq!(backend.name(), "cpu");
}

// =========================================================================
// Priority management
// =========================================================================

#[test]
fn test_registry_default_priority() {
    let registry = BackendRegistry::with_defaults();
    let priority = registry.priority();
    assert_eq!(priority[0], "cuda");
    assert_eq!(*priority.last().unwrap(), "cpu");
}

#[test]
fn test_registry_set_priority() {
    let mut registry = BackendRegistry::with_defaults();
    registry.set_priority(vec!["cpu".to_string(), "cuda".to_string()]);
    let priority = registry.priority();
    assert_eq!(priority[0], "cpu");
    assert_eq!(priority[1], "cuda");
}

#[test]
fn test_registry_set_priority_cpu_first() {
    let mut registry = BackendRegistry::with_defaults();
    registry.set_priority(vec!["cpu".to_string()]);
    let backend = registry.get_best().unwrap();
    assert_eq!(backend.name(), "cpu");
}

// =========================================================================
// Custom backend registration
// =========================================================================

#[test]
fn test_registry_register_custom() {
    let mut registry = BackendRegistry::new();
    registry.register(
        "my_custom",
        Box::new(|| Ok(Box::new(CpuBlas) as Box<dyn BlasBackend>)),
    );
    let available = registry.list_available();
    assert!(available.contains(&"my_custom".to_string()));
}

#[test]
fn test_registry_register_overwrites() {
    let mut registry = BackendRegistry::new();
    registry.register(
        "test",
        Box::new(|| Ok(Box::new(CpuBlas) as Box<dyn BlasBackend>)),
    );
    registry.register(
        "test",
        Box::new(|| Ok(Box::new(CpuBlas) as Box<dyn BlasBackend>)),
    );
    assert_eq!(
        registry
            .list_available()
            .iter()
            .filter(|n| *n == "test")
            .count(),
        1
    );
}

#[test]
fn test_registry_register_failing_factory() {
    let mut registry = BackendRegistry::new();
    registry.register(
        "broken",
        Box::new(|| Err("This backend always fails".to_string())),
    );
    assert!(registry.get("broken").is_err());
}

// =========================================================================
// Backend functionality through registry
// =========================================================================

#[test]
fn test_registry_backend_saxpy() {
    let registry = BackendRegistry::with_defaults();
    let backend = registry.get("cpu").unwrap();
    let x = Vector::new(vec![1.0, 2.0, 3.0]);
    let y = Vector::new(vec![4.0, 5.0, 6.0]);
    let result = backend.saxpy(2.0, &x, &y).unwrap();
    assert_eq!(result.data(), &[6.0, 9.0, 12.0]);
}

#[test]
fn test_registry_gpu_backend_saxpy() {
    let registry = BackendRegistry::with_defaults();
    // All GPU backends delegate to CPU reference, so results should match
    for name in &["cuda", "metal", "vulkan", "opencl", "webgpu", "opengl"] {
        let backend = registry.get(name).unwrap();
        let x = Vector::new(vec![1.0, 2.0, 3.0]);
        let y = Vector::new(vec![4.0, 5.0, 6.0]);
        let result = backend.saxpy(2.0, &x, &y).unwrap();
        assert_eq!(
            result.data(),
            &[6.0, 9.0, 12.0],
            "Backend {} produced wrong SAXPY result",
            name
        );
    }
}

#[test]
fn test_all_backends_device_name() {
    let registry = BackendRegistry::with_defaults();
    for name in registry.list_available() {
        let backend = registry.get(&name).unwrap();
        assert!(!backend.device_name().is_empty(), "Backend {} has empty device name", name);
    }
}
