# frozen_string_literal: true

require_relative "test_helper"
require_relative "gpu_backend_tests"

class TestMetalBlas < Minitest::Test
  include GpuBackendTests

  def setup
    @blas = CodingAdventures::BlasLibrary::Backends::MetalBlas.new
  end

  def test_metal_name
    assert_equal "metal", @blas.name
  end

  def test_metal_device_name
    refute_empty @blas.device_name
  end
end
