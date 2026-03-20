# Changelog

## [0.1.0] - 2026-03-20

### Added
- Initial release with six vendor API simulators
- `_base.py`: BaseVendorSimulator shared foundation for all simulators
- `cuda.py`: CUDARuntime with malloc, memcpy, launch_kernel, streams, events
- `opencl.py`: CLPlatform, CLContext, CLCommandQueue with event-based dependencies
- `metal.py`: MTLDevice with unified memory, command encoders, blit encoder
- `vulkan.py`: VkInstance, VkDevice with create-info structures and VkResult codes
- `webgpu.py`: GPU, GPUAdapter, GPUDevice with single queue and auto sync
- `opengl.py`: GLContext global state machine with GL_* constants
- Cross-API equivalence tests verifying same computation through all 6 APIs
- 95%+ test coverage across all simulators
