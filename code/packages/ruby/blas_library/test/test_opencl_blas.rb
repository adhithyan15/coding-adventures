# frozen_string_literal: true

require_relative "test_helper"
require_relative "gpu_backend_tests"

class TestOpenClBlas < Minitest::Test
  include GpuBackendTests

  def setup
    @blas = CodingAdventures::BlasLibrary::Backends::OpenClBlas.new
  end

  def test_opencl_name
    assert_equal "opencl", @blas.name
  end

  def test_opencl_device_name
    refute_empty @blas.device_name
  end
end
