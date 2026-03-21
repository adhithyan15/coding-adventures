# frozen_string_literal: true

require_relative "test_helper"
require_relative "gpu_backend_tests"

class TestCudaBlas < Minitest::Test
  include GpuBackendTests

  def setup
    @blas = CodingAdventures::BlasLibrary::Backends::CudaBlas.new
  end

  def test_cuda_name
    assert_equal "cuda", @blas.name
  end

  def test_cuda_device_name
    refute_empty @blas.device_name
  end
end
