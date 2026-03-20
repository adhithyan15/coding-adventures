# frozen_string_literal: true

# ---------------------------------------------------------------------------
# Convenience API -- simple module-level functions for common usage.
# ---------------------------------------------------------------------------
#
# === The Simplest Way to Use BLAS ===
#
# Instead of manually creating backends and calling methods:
#
#     require "coding_adventures_blas_library"
#     blas = CodingAdventures::BlasLibrary::Backends::CpuBlas.new
#     result = blas.sgemm(...)
#
# You can use the convenience API:
#
#     include CodingAdventures::BlasLibrary
#
#     blas = create_blas("auto")     # Best available backend
#     blas = create_blas("cuda")     # Specific backend
#     blas = create_blas("cpu")      # CPU fallback

module CodingAdventures
  module BlasLibrary
    # Create a BLAS instance with the specified backend.
    #
    # ================================================================
    # CREATE A BLAS BACKEND INSTANCE
    # ================================================================
    #
    # This is the main entry point for the BLAS library. It creates
    # and returns a backend instance:
    #
    #     "auto"   -- selects the best available backend by priority
    #     "cuda"   -- NVIDIA GPU
    #     "metal"  -- Apple Silicon
    #     "vulkan" -- any Vulkan-capable GPU
    #     "opencl" -- any OpenCL device
    #     "webgpu" -- WebGPU-capable device
    #     "opengl" -- OpenGL 4.3+ device
    #     "cpu"    -- pure Ruby fallback (always works)
    #
    # @param backend_name [String] Which backend to use. Default "auto".
    # @return [Object] An instantiated BLAS backend.
    # @raise [RuntimeError] If the requested backend is not available.
    # ================================================================
    def self.create_blas(backend_name = "auto")
      if backend_name == "auto"
        GLOBAL_REGISTRY.get_best
      else
        GLOBAL_REGISTRY.get(backend_name)
      end
    end

    # Context manager pattern for temporary backend selection.
    #
    # ================================================================
    # TEMPORARY BACKEND SWITCHING
    # ================================================================
    #
    # Use this when you want to temporarily switch backends:
    #
    #     BlasLibrary.use_backend("cpu") do |blas|
    #       result = blas.sgemm(...)
    #     end
    #
    # The backend is created on entry and goes out of scope on exit.
    # This is useful for testing (compare results across backends)
    # or for fallback handling (try GPU, fall back to CPU).
    # ================================================================
    #
    # @param name [String] Backend name.
    # @yield [Object] The instantiated backend.
    def self.use_backend(name)
      blas = create_blas(name)
      yield blas
    end
  end
end
