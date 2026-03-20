# Changelog

All notable changes to the `@coding-adventures/vendor-api-simulators` package.

## [0.1.0] - 2026-03-20

### Added

- **BaseVendorSimulator** -- shared base class with device discovery, logical device creation, queue setup, and `_createAndSubmitCb()` helper.
- **CUDA simulator** -- `CUDARuntime` with streams, events, malloc/free/memcpy/memset, kernel launch with dim3 grid/block dimensions, device properties.
- **OpenCL simulator** -- `CLPlatform`/`CLDevice`/`CLContext`/`CLCommandQueue` hierarchy with event-based dependencies, program build lifecycle, NDRange kernel dispatch.
- **Metal simulator** -- `MTLDevice` with unified memory model, command encoder pattern (compute + blit), `MTLBuffer.writeBytes()`/`contents()` for direct CPU access.
- **Vulkan simulator** -- Thin wrapper with `VkInstance`/`VkDevice`/`VkQueue`/`VkCommandPool`/`VkCommandBuffer`, create-info structs, `VkResult` return codes, fence/semaphore sync.
- **WebGPU simulator** -- `GPU`/`GPUAdapter`/`GPUDevice` with single queue, bind groups, command encoder -> frozen command buffer pattern, buffer map/unmap.
- **OpenGL simulator** -- `GLContext` global state machine with integer handles, `glBind*`/`glDispatch*` pattern, SSBO bindings, sync objects, uniforms.
- **Cross-API tests** verifying all six simulators coexist and share the underlying compute runtime.
- 268+ unit tests across all simulators targeting 95%+ coverage.
