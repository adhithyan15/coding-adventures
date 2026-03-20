# frozen_string_literal: true

require_relative "test_helper"

# ================================================================
# Tests for the convenience API (create_blas, use_backend).
# ================================================================

class TestConvenience < Minitest::Test
  def test_create_blas_cpu
    blas = CodingAdventures::BlasLibrary.create_blas("cpu")
    assert_equal "cpu", blas.name
  end

  def test_create_blas_cuda
    blas = CodingAdventures::BlasLibrary.create_blas("cuda")
    assert_equal "cuda", blas.name
  end

  def test_create_blas_metal
    blas = CodingAdventures::BlasLibrary.create_blas("metal")
    assert_equal "metal", blas.name
  end

  def test_create_blas_opencl
    blas = CodingAdventures::BlasLibrary.create_blas("opencl")
    assert_equal "opencl", blas.name
  end

  def test_create_blas_opengl
    blas = CodingAdventures::BlasLibrary.create_blas("opengl")
    assert_equal "opengl", blas.name
  end

  def test_create_blas_vulkan
    blas = CodingAdventures::BlasLibrary.create_blas("vulkan")
    assert_equal "vulkan", blas.name
  end

  def test_create_blas_webgpu
    blas = CodingAdventures::BlasLibrary.create_blas("webgpu")
    assert_equal "webgpu", blas.name
  end

  def test_create_blas_auto
    blas = CodingAdventures::BlasLibrary.create_blas("auto")
    refute_nil blas.name
  end

  def test_create_blas_default_is_auto
    blas = CodingAdventures::BlasLibrary.create_blas
    refute_nil blas.name
  end

  def test_create_blas_unknown_raises
    assert_raises(RuntimeError) do
      CodingAdventures::BlasLibrary.create_blas("nonexistent")
    end
  end

  def test_use_backend_block
    called = false
    CodingAdventures::BlasLibrary.use_backend("cpu") do |blas|
      assert_equal "cpu", blas.name
      called = true
    end
    assert called, "Block was not called"
  end

  def test_use_backend_yields_working_blas
    CodingAdventures::BlasLibrary.use_backend("cpu") do |blas|
      x = CodingAdventures::BlasLibrary::Vector.new(data: [1.0, 2.0], size: 2)
      y = CodingAdventures::BlasLibrary::Vector.new(data: [3.0, 4.0], size: 2)
      result = blas.saxpy(1.0, x, y)
      assert_in_delta 4.0, result.data[0], 1e-6
    end
  end
end
