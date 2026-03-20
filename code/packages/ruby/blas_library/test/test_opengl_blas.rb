# frozen_string_literal: true

require_relative "test_helper"
require_relative "gpu_backend_tests"

class TestOpenGlBlas < Minitest::Test
  include GpuBackendTests

  def setup
    @blas = CodingAdventures::BlasLibrary::Backends::OpenGlBlas.new
  end

  def test_opengl_name
    assert_equal "opengl", @blas.name
  end

  def test_opengl_device_name
    assert_equal "OpenGL Device", @blas.device_name
  end
end
