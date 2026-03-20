# frozen_string_literal: true

# ---------------------------------------------------------------------------
# BaseVendorSimulator -- the shared foundation for all six vendor API simulators.
# ---------------------------------------------------------------------------
#
# === Why a Base Class? ===
#
# Every GPU API, no matter how different its surface looks, needs to do the same
# things underneath:
#
#     1. Find a GPU                  --> RuntimeInstance
#     2. Create a usable handle      --> LogicalDevice
#     3. Get a queue for submission   --> CommandQueue
#     4. Manage memory               --> MemoryManager
#
# This base class sets all that up. Each simulator subclass then adds its
# vendor-specific vocabulary on top.
#
# Think of it like building six different restaurant fronts (CUDA Grill, Metal
# Bistro, Vulkan Steakhouse...) that all share the same kitchen in the back.
# The kitchen is our compute runtime (Layer 5). The restaurant menus look
# completely different, but the same chefs cook the same food.
#
# === Device Selection ===
#
# Different APIs have different preferences for which device to use:
#
#     - CUDA always wants an NVIDIA GPU (vendor_hint: "nvidia")
#     - Metal always wants an Apple device (vendor_hint: "apple")
#     - OpenCL, Vulkan, WebGPU, OpenGL are cross-vendor
#
# The _select_device method handles this: it picks the best matching device
# from the runtime's enumerated physical devices, preferring the vendor hint
# if given, then falling back to any GPU.
#
# === The _create_and_submit_cb Helper ===
#
# CUDA and OpenGL hide command buffers from the user. When you call
# cuda.malloc or gl.dispatch_compute, those APIs internally:
#
#     1. Create a command buffer
#     2. Begin recording
#     3. Record the command(s) via a block
#     4. End recording
#     5. Submit to the compute queue with a fence
#     6. Wait for the fence
#
# This helper encapsulates that pattern. Pass a block that records commands
# into a CB, and this method handles the rest.

module CodingAdventures
  module VendorApiSimulators
    class BaseVendorSimulator
      # These are the core objects every simulator gets access to.
      # Subclasses use these to implement vendor-specific APIs.
      attr_reader :_instance, :_physical_devices, :_physical_device,
        :_logical_device, :_compute_queue, :_memory_manager

      # Initialize the simulator with device discovery and setup.
      #
      # @param device_type [Symbol, nil] Preferred device type (:gpu, :tpu, :npu).
      #     If nil, any type is acceptable.
      # @param vendor_hint [String, nil] Preferred vendor string (e.g., "nvidia",
      #     "apple"). If the preferred vendor isn't found, falls back to any
      #     available device.
      def initialize(device_type: nil, vendor_hint: nil)
        # Step 1: Create the runtime instance (discovers all hardware)
        @_instance = ComputeRuntime::RuntimeInstance.new

        # Step 2: Enumerate all physical devices
        @_physical_devices = @_instance.enumerate_physical_devices

        # Step 3: Select the best matching device
        @_physical_device = _select_device(device_type, vendor_hint)

        # Step 4: Create a logical device (the usable handle)
        @_logical_device = @_instance.create_logical_device(@_physical_device)

        # Step 5: Get a compute queue for submitting work
        @_compute_queue = @_logical_device.queues["compute"][0]

        # Step 6: Get the memory manager for allocations
        @_memory_manager = @_logical_device.memory_manager
      end

      private

      # Pick the best matching device from enumerated physical devices.
      #
      # === Selection Strategy ===
      #
      # The strategy is a two-pass filter:
      #
      #     Pass 1: Try to match both vendor_hint AND device_type (if given).
      #     Pass 2: Try vendor_hint only.
      #     Pass 3: Try device_type only.
      #     Pass 4: Take the first device (any will do).
      #
      # This ensures that:
      #     - CUDARuntime(vendor_hint: "nvidia") gets an NVIDIA GPU
      #     - MTLDevice(vendor_hint: "apple") gets an Apple device
      #     - VkInstance() gets whatever is available
      #
      # @param device_type [Symbol, nil] Preferred device type, or nil for any.
      # @param vendor_hint [String, nil] Preferred vendor string, or nil for any.
      # @return [ComputeRuntime::PhysicalDevice] The best matching device.
      # @raise [RuntimeError] If no devices are available at all.
      def _select_device(device_type, vendor_hint)
        raise RuntimeError, "No physical devices available" if @_physical_devices.empty?

        # Pass 1: Match both vendor and type
        if vendor_hint && device_type
          @_physical_devices.each do |dev|
            return dev if dev.vendor == vendor_hint && dev.device_type == device_type
          end
        end

        # Pass 2: Match vendor only
        if vendor_hint
          @_physical_devices.each do |dev|
            return dev if dev.vendor == vendor_hint
          end
        end

        # Pass 3: Match device type only
        if device_type
          @_physical_devices.each do |dev|
            return dev if dev.device_type == device_type
          end
        end

        # Pass 4: Take whatever is available
        @_physical_devices[0]
      end

      # Create a command buffer, record commands, submit, and wait.
      #
      # === The "Immediate Execution" Pattern ===
      #
      # APIs like CUDA and OpenGL present an "immediate" execution model
      # where each API call appears to execute right away. Under the hood,
      # they still use command buffers -- they just hide them from you.
      #
      # This method implements that pattern:
      #
      #     1. Create a new command buffer
      #     2. Begin recording
      #     3. Yield the CB to the caller's block for recording
      #     4. End recording
      #     5. Submit to the queue with a fence
      #     6. Wait for the fence to signal (synchronous completion)
      #     7. Return the command buffer (for inspection/debugging)
      #
      # @param queue [ComputeRuntime::CommandQueue, nil] Which queue to submit to.
      #     Defaults to @_compute_queue.
      # @yield [ComputeRuntime::CommandBuffer] The CB in RECORDING state.
      # @return [ComputeRuntime::CommandBuffer] The completed CB.
      def _create_and_submit_cb(queue: nil, &block)
        target_queue = queue || @_compute_queue

        # Create and begin recording
        cb = @_logical_device.create_command_buffer
        cb.begin

        # Let the caller record whatever commands they need
        block.call(cb)

        # End recording and submit
        cb.end_recording
        fence = @_logical_device.create_fence
        target_queue.submit([cb], fence: fence)
        fence.wait

        cb
      end
    end
  end
end
