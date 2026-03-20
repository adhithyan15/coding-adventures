# frozen_string_literal: true

# ---------------------------------------------------------------------------
# Backend Registry -- find and select BLAS backends.
# ---------------------------------------------------------------------------
#
# === What is the Registry? ===
#
# The registry is a central catalog of available BLAS backends. It provides
# three modes of selection:
#
#     1. EXPLICIT:    registry.get("cuda")     -- give me CUDA specifically
#     2. AUTO-DETECT: registry.get_best        -- give me the best available
#     3. CUSTOM:      registry.register(...)   -- add my own backend
#
# === Auto-Detection Priority ===
#
# When you ask for "the best available backend," the registry tries each
# backend in priority order and returns the first one that successfully
# initializes:
#
#     cuda > metal > vulkan > opencl > webgpu > opengl > cpu
#
# CUDA is first because it's the most optimized for ML (and most GPUs are
# NVIDIA in data centers). CPU is always last -- it's the universal fallback
# that works everywhere.
#
# === How It Works Internally ===
#
# The registry stores *classes* (not instances). When you call get("cuda"),
# it instantiates CudaBlas.new on the spot. This is because GPU backends
# allocate device resources in initialize, and we don't want to waste GPU
# memory on backends that aren't being used.

module CodingAdventures
  module BlasLibrary
    class BackendRegistry
      # ================================================================
      # BACKEND REGISTRY -- FIND AND SELECT BLAS BACKENDS
      # ================================================================
      #
      # The registry keeps track of which backends are available and
      # helps the caller pick one. Three modes of selection:
      #
      # 1. EXPLICIT:    registry.get("cuda")
      # 2. AUTO-DETECT: registry.get_best
      # 3. CUSTOM:      registry.register("my_backend", MyBlas)
      #
      # Auto-detection priority (customizable):
      #     cuda > metal > vulkan > opencl > webgpu > opengl > cpu
      #
      # CUDA is first because it's the most optimized for ML.
      # Metal is second because Apple silicon has unified memory.
      # CPU is always last -- it's the universal fallback.
      # ================================================================

      # The default auto-detection order. CUDA first (ML standard),
      # CPU last (universal fallback).
      DEFAULT_PRIORITY = %w[
        cuda
        metal
        vulkan
        opencl
        webgpu
        opengl
        cpu
      ].freeze

      def initialize
        @backends = {}
        @priority = DEFAULT_PRIORITY.dup
      end

      # Register a backend class by name.
      #
      # The class is stored but NOT instantiated yet. Instantiation happens
      # when get() or get_best() is called.
      #
      # @param name [String] Backend identifier (e.g., "cuda", "cpu").
      # @param backend_class [Class] The backend class to register.
      def register(name, backend_class)
        @backends[name] = backend_class
      end

      # Get a specific backend by name, instantiating it on demand.
      #
      # @param name [String] Backend identifier.
      # @return [Object] An instantiated backend.
      # @raise [RuntimeError] If the backend name is not registered.
      def get(name)
        unless @backends.key?(name)
          available = @backends.keys.sort.join(", ")
          raise RuntimeError,
            "Backend '#{name}' not registered. Available: #{available}"
        end
        @backends[name].new
      end

      # Try each backend in priority order, return the first that works.
      #
      # Each backend is instantiated inside a begin/rescue. If initialization
      # fails (e.g., no GPU available), we skip to the next one. CPU always
      # works, so this never fails (as long as CPU is registered).
      #
      # @return [Object] The highest-priority backend that successfully initializes.
      # @raise [RuntimeError] If no backend could be initialized.
      def get_best
        @priority.each do |name|
          next unless @backends.key?(name)

          begin
            return @backends[name].new
          rescue => _e
            # This backend failed to initialize -- try the next one.
            # Common reasons: no GPU driver, wrong platform, etc.
            next
          end
        end

        tried = @priority.select { |n| @backends.key?(n) }
        raise RuntimeError,
          "No BLAS backend could be initialized. Tried: #{tried}"
      end

      # List names of all registered backends.
      #
      # @return [Array<String>] A list of registered backend names.
      def list_available
        @backends.keys
      end

      # Change the auto-detection priority order.
      #
      # @param priority [Array<String>] New priority list (first = highest).
      def set_priority(priority)
        @priority = priority.dup
      end
    end

    # =====================================================================
    # Global registry instance -- shared across the whole application
    # =====================================================================

    # This is the single global registry. It's populated by the main
    # require file when the package is imported. Users can also register
    # custom backends here.
    GLOBAL_REGISTRY = BackendRegistry.new
  end
end
