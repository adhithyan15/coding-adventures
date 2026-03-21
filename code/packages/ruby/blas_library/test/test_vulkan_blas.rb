# frozen_string_literal: true

require_relative "test_helper"
require_relative "gpu_backend_tests"

class TestVulkanBlas < Minitest::Test
  include GpuBackendTests

  def setup
    @blas = CodingAdventures::BlasLibrary::Backends::VulkanBlas.new
  end

  def test_vulkan_name
    assert_equal "vulkan", @blas.name
  end

  def test_vulkan_device_name
    assert_equal "Vulkan Device", @blas.device_name
  end
end
