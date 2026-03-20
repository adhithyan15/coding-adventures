"""Vendor API Simulators — six GPU programming APIs over one runtime.

This package provides six vendor API simulators, each wrapping the same
Vulkan-inspired compute runtime (Layer 5) with different programming models:

    CUDA    — NVIDIA's implicit, "just launch it" model
    OpenCL  — Khronos cross-platform, event-based dependencies
    Metal   — Apple's unified memory, command encoder model
    Vulkan  — Ultra-explicit, maximum control
    WebGPU  — Safe, browser-first, single queue
    OpenGL  — Legacy global state machine

=== Quick Start ===

    # CUDA style (simplest)
    from vendor_api_simulators.cuda import CUDARuntime, CUDAKernel, dim3

    cuda = CUDARuntime()
    d_x = cuda.malloc(256)
    cuda.launch_kernel(kernel, dim3(1,1,1), dim3(32,1,1), [d_x])
    cuda.device_synchronize()
    cuda.free(d_x)

    # Metal style (unified memory)
    from vendor_api_simulators.metal import MTLDevice

    device = MTLDevice()
    buf = device.make_buffer(256)
    buf.write_bytes(data)
    result = bytes(buf.contents())

    # OpenGL style (state machine)
    from vendor_api_simulators.opengl import GLContext, GL_COMPUTE_SHADER

    gl = GLContext()
    shader = gl.create_shader(GL_COMPUTE_SHADER)
"""

# Base class
from ._base import BaseVendorSimulator

# CUDA
from .cuda import (
    CUDADeviceProperties,
    CUDADevicePtr,
    CUDAEvent,
    CUDAKernel,
    CUDAMemcpyKind,
    CUDARuntime,
    CUDAStream,
    dim3,
)

# OpenCL
from .opencl import (
    CLBuffer,
    CLBuildStatus,
    CLCommandQueue,
    CLContext,
    CLDevice,
    CLDeviceInfo,
    CLDeviceType,
    CLEvent,
    CLKernel,
    CLMemFlags,
    CLPlatform,
    CLProgram,
    CLEventStatus,
)

# Metal
from .metal import (
    MTLBlitCommandEncoder,
    MTLBuffer,
    MTLCommandBuffer,
    MTLCommandBufferStatus,
    MTLCommandQueue,
    MTLComputeCommandEncoder,
    MTLComputePipelineState,
    MTLDevice,
    MTLFunction,
    MTLLibrary,
    MTLResourceOptions,
    MTLSize,
)

# Vulkan
from .vulkan import (
    VkBuffer,
    VkBufferCopy,
    VkBufferCreateInfo,
    VkBufferUsageFlagBits,
    VkCommandBuffer,
    VkCommandPool,
    VkCommandPoolCreateInfo,
    VkComputePipelineCreateInfo,
    VkDescriptorBufferInfo,
    VkDescriptorSet,
    VkDescriptorSetAllocateInfo,
    VkDescriptorSetLayout,
    VkDescriptorSetLayoutBinding,
    VkDescriptorSetLayoutCreateInfo,
    VkDevice,
    VkDeviceMemory,
    VkFence,
    VkInstance,
    VkMemoryAllocateInfo,
    VkMemoryPropertyFlagBits,
    VkPhysicalDevice,
    VkPipeline,
    VkPipelineBindPoint,
    VkPipelineLayout,
    VkPipelineLayoutCreateInfo,
    VkPipelineShaderStageCreateInfo,
    VkQueue,
    VkResult,
    VkSemaphore,
    VkShaderModule,
    VkShaderModuleCreateInfo,
    VkSharingMode,
    VkSubmitInfo,
    VkWriteDescriptorSet,
)

# WebGPU
from .webgpu import (
    GPU,
    GPUAdapter,
    GPUAdapterLimits,
    GPUBindGroup,
    GPUBindGroupDescriptor,
    GPUBindGroupEntry,
    GPUBindGroupLayout,
    GPUBindGroupLayoutDescriptor,
    GPUBindGroupLayoutEntry,
    GPUBuffer,
    GPUBufferBindingLayout,
    GPUBufferDescriptor,
    GPUBufferUsage,
    GPUCommandBuffer,
    GPUCommandEncoder,
    GPUCommandEncoderDescriptor,
    GPUComputePassDescriptor,
    GPUComputePassEncoder,
    GPUComputePipeline,
    GPUComputePipelineDescriptor,
    GPUDevice,
    GPUDeviceDescriptor,
    GPUDeviceLimits,
    GPUMapMode,
    GPUPipelineLayout,
    GPUPipelineLayoutDescriptor,
    GPUProgrammableStage,
    GPUQueue,
    GPURequestAdapterOptions,
    GPUShaderModule,
    GPUShaderModuleDescriptor,
)

# OpenGL
from .opengl import (
    GL_ALL_BARRIER_BITS,
    GL_ALREADY_SIGNALED,
    GL_ARRAY_BUFFER,
    GL_BUFFER_UPDATE_BARRIER_BIT,
    GL_COMPUTE_SHADER,
    GL_CONDITION_SATISFIED,
    GL_DYNAMIC_DRAW,
    GL_MAP_READ_BIT,
    GL_MAP_WRITE_BIT,
    GL_SHADER_STORAGE_BARRIER_BIT,
    GL_SHADER_STORAGE_BUFFER,
    GL_STATIC_DRAW,
    GL_STREAM_DRAW,
    GL_SYNC_FLUSH_COMMANDS_BIT,
    GL_SYNC_GPU_COMMANDS_COMPLETE,
    GL_TIMEOUT_EXPIRED,
    GL_UNIFORM_BUFFER,
    GL_WAIT_FAILED,
    GLContext,
)

__all__ = [
    # Base
    "BaseVendorSimulator",
    # CUDA
    "CUDARuntime", "CUDAKernel", "CUDADevicePtr", "CUDAStream",
    "CUDAEvent", "CUDAMemcpyKind", "CUDADeviceProperties", "dim3",
    # OpenCL
    "CLPlatform", "CLDevice", "CLContext", "CLCommandQueue", "CLProgram",
    "CLKernel", "CLBuffer", "CLEvent", "CLMemFlags", "CLDeviceType",
    "CLBuildStatus", "CLEventStatus", "CLDeviceInfo",
    # Metal
    "MTLDevice", "MTLCommandQueue", "MTLCommandBuffer",
    "MTLComputeCommandEncoder", "MTLBlitCommandEncoder", "MTLBuffer",
    "MTLLibrary", "MTLFunction", "MTLComputePipelineState",
    "MTLSize", "MTLResourceOptions", "MTLCommandBufferStatus",
    # Vulkan
    "VkInstance", "VkPhysicalDevice", "VkDevice", "VkQueue",
    "VkCommandPool", "VkCommandBuffer", "VkBuffer", "VkDeviceMemory",
    "VkShaderModule", "VkPipeline", "VkDescriptorSetLayout",
    "VkPipelineLayout", "VkDescriptorSet", "VkFence", "VkSemaphore",
    "VkResult", "VkPipelineBindPoint", "VkBufferUsageFlagBits",
    "VkMemoryPropertyFlagBits", "VkSharingMode",
    "VkBufferCreateInfo", "VkMemoryAllocateInfo", "VkShaderModuleCreateInfo",
    "VkComputePipelineCreateInfo", "VkPipelineShaderStageCreateInfo",
    "VkSubmitInfo", "VkBufferCopy", "VkWriteDescriptorSet",
    "VkDescriptorBufferInfo", "VkCommandPoolCreateInfo",
    "VkDescriptorSetLayoutCreateInfo", "VkDescriptorSetLayoutBinding",
    "VkPipelineLayoutCreateInfo", "VkDescriptorSetAllocateInfo",
    # WebGPU
    "GPU", "GPUAdapter", "GPUDevice", "GPUQueue",
    "GPUCommandEncoder", "GPUComputePassEncoder", "GPUCommandBuffer",
    "GPUBuffer", "GPUShaderModule", "GPUComputePipeline",
    "GPUBindGroup", "GPUBindGroupLayout", "GPUPipelineLayout",
    "GPUBufferUsage", "GPUMapMode",
    "GPUBufferDescriptor", "GPUShaderModuleDescriptor",
    "GPUComputePipelineDescriptor", "GPUProgrammableStage",
    "GPUBindGroupDescriptor", "GPUBindGroupEntry",
    "GPUBindGroupLayoutDescriptor", "GPUBindGroupLayoutEntry",
    "GPUBufferBindingLayout", "GPUPipelineLayoutDescriptor",
    "GPURequestAdapterOptions", "GPUDeviceDescriptor",
    "GPUAdapterLimits", "GPUDeviceLimits",
    "GPUComputePassDescriptor", "GPUCommandEncoderDescriptor",
    # OpenGL
    "GLContext",
    "GL_COMPUTE_SHADER", "GL_SHADER_STORAGE_BUFFER",
    "GL_ARRAY_BUFFER", "GL_UNIFORM_BUFFER",
    "GL_STATIC_DRAW", "GL_DYNAMIC_DRAW", "GL_STREAM_DRAW",
    "GL_MAP_READ_BIT", "GL_MAP_WRITE_BIT",
    "GL_SHADER_STORAGE_BARRIER_BIT", "GL_BUFFER_UPDATE_BARRIER_BIT",
    "GL_ALL_BARRIER_BITS",
    "GL_ALREADY_SIGNALED", "GL_CONDITION_SATISFIED",
    "GL_TIMEOUT_EXPIRED", "GL_WAIT_FAILED",
    "GL_SYNC_FLUSH_COMMANDS_BIT", "GL_SYNC_GPU_COMMANDS_COMPLETE",
]
