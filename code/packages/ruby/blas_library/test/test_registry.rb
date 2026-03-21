# frozen_string_literal: true

require_relative "test_helper"

# ================================================================
# Tests for BackendRegistry -- backend discovery and selection.
# ================================================================

class TestBackendRegistry < Minitest::Test
  include CodingAdventures::BlasLibrary

  def setup
    @registry = BackendRegistry.new
  end

  def test_register_and_get
    @registry.register("cpu", Backends::CpuBlas)
    blas = @registry.get("cpu")
    assert_equal "cpu", blas.name
  end

  def test_get_unknown_raises
    assert_raises(RuntimeError) { @registry.get("nonexistent") }
  end

  def test_list_available_empty
    assert_empty @registry.list_available
  end

  def test_list_available_after_register
    @registry.register("cpu", Backends::CpuBlas)
    @registry.register("cuda", Backends::CudaBlas)
    available = @registry.list_available
    assert_includes available, "cpu"
    assert_includes available, "cuda"
  end

  def test_get_best_returns_highest_priority
    @registry.register("cpu", Backends::CpuBlas)
    @registry.register("cuda", Backends::CudaBlas)
    blas = @registry.get_best
    assert_equal "cuda", blas.name
  end

  def test_get_best_cpu_only
    @registry.register("cpu", Backends::CpuBlas)
    blas = @registry.get_best
    assert_equal "cpu", blas.name
  end

  def test_get_best_empty_raises
    assert_raises(RuntimeError) { @registry.get_best }
  end

  def test_set_priority
    @registry.register("cpu", Backends::CpuBlas)
    @registry.register("cuda", Backends::CudaBlas)
    @registry.set_priority(%w[cpu cuda])
    blas = @registry.get_best
    assert_equal "cpu", blas.name
  end

  def test_default_priority_order
    assert_equal %w[cuda metal vulkan opencl webgpu opengl cpu],
      BackendRegistry::DEFAULT_PRIORITY
  end

  def test_register_all_backends
    @registry.register("cpu", Backends::CpuBlas)
    @registry.register("cuda", Backends::CudaBlas)
    @registry.register("metal", Backends::MetalBlas)
    @registry.register("opencl", Backends::OpenClBlas)
    @registry.register("opengl", Backends::OpenGlBlas)
    @registry.register("vulkan", Backends::VulkanBlas)
    @registry.register("webgpu", Backends::WebGpuBlas)
    assert_equal 7, @registry.list_available.length
  end

  def test_get_each_backend
    backends = {
      "cpu" => Backends::CpuBlas,
      "cuda" => Backends::CudaBlas,
      "metal" => Backends::MetalBlas,
      "opencl" => Backends::OpenClBlas,
      "opengl" => Backends::OpenGlBlas,
      "vulkan" => Backends::VulkanBlas,
      "webgpu" => Backends::WebGpuBlas
    }
    backends.each do |name, klass|
      @registry.register(name, klass)
    end
    backends.each_key do |name|
      blas = @registry.get(name)
      assert_equal name, blas.name
    end
  end
end

class TestGlobalRegistry < Minitest::Test
  def test_global_registry_exists
    refute_nil CodingAdventures::BlasLibrary::GLOBAL_REGISTRY
  end

  def test_global_registry_has_all_backends
    registry = CodingAdventures::BlasLibrary::GLOBAL_REGISTRY
    available = registry.list_available
    %w[cpu cuda metal opencl opengl vulkan webgpu].each do |name|
      assert_includes available, name, "Global registry missing '#{name}'"
    end
  end

  def test_global_registry_get_cpu
    blas = CodingAdventures::BlasLibrary::GLOBAL_REGISTRY.get("cpu")
    assert_equal "cpu", blas.name
  end

  def test_global_registry_get_best
    blas = CodingAdventures::BlasLibrary::GLOBAL_REGISTRY.get_best
    refute_nil blas.name
  end
end
