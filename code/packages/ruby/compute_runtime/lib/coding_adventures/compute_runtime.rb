# frozen_string_literal: true

# ---------------------------------------------------------------------------
# Compute Runtime -- Layer 5 of the accelerator computing stack.
# ---------------------------------------------------------------------------
#
# A low-level Vulkan-inspired compute runtime that provides the software
# infrastructure between user-facing APIs (CUDA, OpenCL, Metal, Vulkan)
# and the hardware device simulators (Layer 6).
#
# === Quick Start ===
#
#     require "coding_adventures_compute_runtime"
#     include CodingAdventures
#
#     # 1. Discover devices
#     instance = ComputeRuntime::RuntimeInstance.new
#     devices = instance.enumerate_physical_devices
#     nvidia = devices.find { |d| d.vendor == "nvidia" }
#
#     # 2. Create logical device
#     device = instance.create_logical_device(nvidia)
#     queue = device.queues["compute"][0]
#     mm = device.memory_manager
#
#     # 3. Allocate buffers
#     buf = mm.allocate(256,
#       ComputeRuntime::MemoryType::DEVICE_LOCAL | ComputeRuntime::MemoryType::HOST_VISIBLE,
#       usage: ComputeRuntime::BufferUsage::STORAGE)
#
#     # 4. Create pipeline
#     shader = device.create_shader_module(code: [GpuCore.limm(0, 42.0), GpuCore.halt])
#     ds_layout = device.create_descriptor_set_layout([])
#     pl_layout = device.create_pipeline_layout([ds_layout])
#     pipeline = device.create_compute_pipeline(shader, pl_layout)
#
#     # 5. Record and submit commands
#     cb = device.create_command_buffer
#     cb.begin
#     cb.cmd_bind_pipeline(pipeline)
#     cb.cmd_dispatch(1, 1, 1)
#     cb.end_recording
#
#     fence = device.create_fence
#     queue.submit([cb], fence: fence)
#     fence.wait
#
# === Architecture ===
#
#     RuntimeInstance
#     +-- enumerate_physical_devices -> PhysicalDevice[]
#     +-- create_logical_device -> LogicalDevice
#         +-- queues: CommandQueue[]
#         +-- memory_manager: MemoryManager
#         +-- create_command_buffer -> CommandBuffer
#         +-- create_compute_pipeline -> Pipeline
#         +-- create_fence -> Fence
#         +-- create_semaphore -> Semaphore

require "set"
require "coding_adventures_device_simulator"

require_relative "compute_runtime/version"
require_relative "compute_runtime/protocols"
require_relative "compute_runtime/sync"
require_relative "compute_runtime/memory"
require_relative "compute_runtime/command_buffer"
require_relative "compute_runtime/pipeline"
require_relative "compute_runtime/command_queue"
require_relative "compute_runtime/instance"
require_relative "compute_runtime/validation"
