# frozen_string_literal: true

require_relative "test_helper"
require_relative "gpu_backend_tests"

class TestWebGpuBlas < Minitest::Test
  include GpuBackendTests

  def setup
    @blas = CodingAdventures::BlasLibrary::Backends::WebGpuBlas.new
  end

  def test_webgpu_name
    assert_equal "webgpu", @blas.name
  end

  def test_webgpu_device_name
    assert_equal "WebGPU Device", @blas.device_name
  end
end
