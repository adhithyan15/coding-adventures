package blaslibrary

import (
	"fmt"
	"sync"
)

// =========================================================================
// BackendRegistry -- find and select BLAS backends
// =========================================================================

// BackendRegistry is a central catalog of available BLAS backends. It provides
// three modes of selection:
//
//  1. EXPLICIT:    registry.Get("cuda")     -- give me CUDA specifically
//  2. AUTO-DETECT: registry.GetBest()       -- give me the best available
//  3. CUSTOM:      registry.Register(...)   -- add my own backend
//
// # Auto-Detection Priority
//
// When you ask for "the best available backend," the registry tries each
// backend in priority order and returns the first one that successfully
// initializes:
//
//	cuda > metal > vulkan > opencl > webgpu > opengl > cpu
//
// CUDA is first because it's the most optimized for ML (and most GPUs are
// NVIDIA in data centers). CPU is always last -- it's the universal fallback
// that works everywhere.
//
// # Factory Functions
//
// The registry stores factory functions (not instances). When you call
// Get("cuda"), it calls the factory to create a new CudaBlas on the spot.
// This is because GPU backends allocate device resources during creation,
// and we don't want to waste GPU memory on backends that aren't being used.
type BackendRegistry struct {
	mu       sync.RWMutex
	backends map[string]func() (BlasBackend, error)
	priority []string
}

// defaultPriority is the auto-detection order. CUDA first (ML standard),
// CPU last (universal fallback).
var defaultPriority = []string{
	"cuda",
	"metal",
	"vulkan",
	"opencl",
	"webgpu",
	"opengl",
	"cpu",
}

// NewBackendRegistry creates a new empty registry with default priority order.
func NewBackendRegistry() *BackendRegistry {
	return &BackendRegistry{
		backends: make(map[string]func() (BlasBackend, error)),
		priority: append([]string{}, defaultPriority...),
	}
}

// Register adds a backend factory to the registry.
//
// The factory is stored but NOT called yet. Instantiation happens
// when Get() or GetBest() is called.
func (r *BackendRegistry) Register(name string, factory func() (BlasBackend, error)) {
	r.mu.Lock()
	defer r.mu.Unlock()
	r.backends[name] = factory
}

// Get returns a specific backend by name, creating it on demand.
//
// Returns an error if the backend name is not registered or if
// the factory function fails.
func (r *BackendRegistry) Get(name string) (BlasBackend, error) {
	r.mu.RLock()
	factory, ok := r.backends[name]
	r.mu.RUnlock()

	if !ok {
		r.mu.RLock()
		available := make([]string, 0, len(r.backends))
		for k := range r.backends {
			available = append(available, k)
		}
		r.mu.RUnlock()
		return nil, fmt.Errorf("backend %q not registered. Available: %v", name, available)
	}
	return factory()
}

// GetBest tries each backend in priority order, returning the first that works.
//
// Each factory is called inside a recover block. If creation fails (e.g., no
// GPU available), we skip to the next one. CPU always works, so this never
// fails (as long as CPU is registered).
func (r *BackendRegistry) GetBest() (BlasBackend, error) {
	r.mu.RLock()
	defer r.mu.RUnlock()

	var tried []string
	for _, name := range r.priority {
		factory, ok := r.backends[name]
		if !ok {
			continue
		}
		tried = append(tried, name)
		backend, err := factory()
		if err == nil {
			return backend, nil
		}
		// This backend failed to initialize -- try the next one.
		// Common reasons: no GPU driver, wrong platform, etc.
	}

	return nil, fmt.Errorf(
		"no BLAS backend could be initialized. Tried: %v", tried,
	)
}

// ListAvailable returns the names of all registered backends.
func (r *BackendRegistry) ListAvailable() []string {
	r.mu.RLock()
	defer r.mu.RUnlock()
	result := make([]string, 0, len(r.backends))
	for k := range r.backends {
		result = append(result, k)
	}
	return result
}

// SetPriority changes the auto-detection priority order.
func (r *BackendRegistry) SetPriority(priority []string) {
	r.mu.Lock()
	defer r.mu.Unlock()
	r.priority = append([]string{}, priority...)
}

// =========================================================================
// Global registry instance -- shared across the whole application
// =========================================================================

// GlobalRegistry is the single global registry. It's populated by the init()
// function when the package is imported. Users can also register custom
// backends here.
var GlobalRegistry = NewBackendRegistry()

// CreateBlas creates a BLAS instance with the specified backend.
//
//	"auto"   -- selects the best available backend by priority
//	"cuda"   -- NVIDIA GPU
//	"metal"  -- Apple Silicon
//	"vulkan" -- any Vulkan-capable GPU
//	"opencl" -- any OpenCL device
//	"webgpu" -- WebGPU-capable device
//	"opengl" -- OpenGL 4.3+ device
//	"cpu"    -- pure Go fallback (always works)
func CreateBlas(backendName string) (BlasBackend, error) {
	if backendName == "auto" {
		return GlobalRegistry.GetBest()
	}
	return GlobalRegistry.Get(backendName)
}
