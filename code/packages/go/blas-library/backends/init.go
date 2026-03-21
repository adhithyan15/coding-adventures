// Registration -- connect backends to the global registry.
//
// This file registers all seven BLAS backends with the global registry.
// When the backends package is imported, all backends become available
// for auto-detection via registry.GetBest() or explicit selection via
// registry.Get("cuda").
//
// The registration uses factory functions (not instances) because GPU
// backends allocate device resources during creation, and we do not want
// to waste GPU memory on backends that are not being used.
package backends

import (
	blas "github.com/adhithyan15/coding-adventures/code/packages/go/blas-library"
)

func init() {
	// CPU -- the universal fallback that always works.
	blas.GlobalRegistry.Register("cpu", func() (blas.BlasBackend, error) {
		return &CpuBlas{}, nil
	})

	// CUDA -- NVIDIA GPU acceleration.
	blas.GlobalRegistry.Register("cuda", func() (blas.BlasBackend, error) {
		return NewCudaBlas()
	})

	// Metal -- Apple Silicon unified memory.
	blas.GlobalRegistry.Register("metal", func() (blas.BlasBackend, error) {
		return NewMetalBlas()
	})

	// Vulkan -- explicit maximum-control GPU.
	blas.GlobalRegistry.Register("vulkan", func() (blas.BlasBackend, error) {
		return NewVulkanBlas()
	})

	// OpenCL -- portable cross-vendor GPU.
	blas.GlobalRegistry.Register("opencl", func() (blas.BlasBackend, error) {
		return NewOpenClBlas()
	})

	// WebGPU -- safe browser-first GPU.
	blas.GlobalRegistry.Register("webgpu", func() (blas.BlasBackend, error) {
		return NewWebGpuBlas()
	})

	// OpenGL -- legacy state machine GPU.
	blas.GlobalRegistry.Register("opengl", func() (blas.BlasBackend, error) {
		return NewOpenGlBlas()
	})
}
