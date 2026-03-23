# frozen_string_literal: true

require_relative "test_helper"

# ================================================================
# Cross-backend tests -- verify all backends produce identical results.
#
# The GPU backends exercise the full GPU memory pipeline but delegate
# arithmetic to the CPU reference. These tests verify that the round-trip
# through GPU memory doesn't corrupt the data.
# ================================================================

class TestCrossBackendConsistency < Minitest::Test
  Vector = CodingAdventures::BlasLibrary::Vector
  Matrix = CodingAdventures::BlasLibrary::Matrix
  Transpose = CodingAdventures::BlasLibrary::Transpose
  Side = CodingAdventures::BlasLibrary::Side

  BACKEND_NAMES = %w[cpu cuda metal opencl opengl vulkan webgpu].freeze

  def setup
    @backends = BACKEND_NAMES.map { |name| CodingAdventures::BlasLibrary.create_blas(name) }
  end

  def test_all_backends_saxpy_consistent
    x = Vector.new(data: [1.0, 2.0, 3.0, 4.0], size: 4)
    y = Vector.new(data: [5.0, 6.0, 7.0, 8.0], size: 4)
    ref = @backends[0].saxpy(2.5, x, y)

    @backends[1..].each do |blas|
      result = blas.saxpy(2.5, x, y)
      ref.data.zip(result.data).each do |expected, actual|
        assert_in_delta expected, actual, 1e-4,
          "#{blas.name} SAXPY differs from CPU"
      end
    end
  end

  def test_all_backends_sdot_consistent
    x = Vector.new(data: [1.0, 2.0, 3.0], size: 3)
    y = Vector.new(data: [4.0, 5.0, 6.0], size: 3)
    ref = @backends[0].sdot(x, y)

    @backends[1..].each do |blas|
      result = blas.sdot(x, y)
      assert_in_delta ref, result, 1e-4,
        "#{blas.name} SDOT differs from CPU"
    end
  end

  def test_all_backends_sgemm_consistent
    a = Matrix.new(data: [1.0, 2.0, 3.0, 4.0], rows: 2, cols: 2)
    b = Matrix.new(data: [5.0, 6.0, 7.0, 8.0], rows: 2, cols: 2)
    c = Matrix.new(data: [0.0, 0.0, 0.0, 0.0], rows: 2, cols: 2)
    ref = @backends[0].sgemm(Transpose::NO_TRANS, Transpose::NO_TRANS, 1.0, a, b, 0.0, c)

    @backends[1..].each do |blas|
      result = blas.sgemm(Transpose::NO_TRANS, Transpose::NO_TRANS, 1.0, a, b, 0.0, c)
      ref.data.zip(result.data).each do |expected, actual|
        assert_in_delta expected, actual, 1e-4,
          "#{blas.name} SGEMM differs from CPU"
      end
    end
  end

  def test_all_backends_sgemv_consistent
    a = Matrix.new(data: [1.0, 2.0, 3.0, 4.0, 5.0, 6.0], rows: 2, cols: 3)
    x = Vector.new(data: [1.0, 2.0, 3.0], size: 3)
    y = Vector.new(data: [0.0, 0.0], size: 2)
    ref = @backends[0].sgemv(Transpose::NO_TRANS, 1.0, a, x, 0.0, y)

    @backends[1..].each do |blas|
      result = blas.sgemv(Transpose::NO_TRANS, 1.0, a, x, 0.0, y)
      ref.data.zip(result.data).each do |expected, actual|
        assert_in_delta expected, actual, 1e-4,
          "#{blas.name} SGEMV differs from CPU"
      end
    end
  end

  def test_all_backends_snrm2_consistent
    x = Vector.new(data: [3.0, 4.0, 5.0], size: 3)
    ref = @backends[0].snrm2(x)

    @backends[1..].each do |blas|
      result = blas.snrm2(x)
      assert_in_delta ref, result, 1e-4,
        "#{blas.name} SNRM2 differs from CPU"
    end
  end

  def test_all_backends_sscal_consistent
    x = Vector.new(data: [1.0, 2.0, 3.0], size: 3)
    ref = @backends[0].sscal(3.0, x)

    @backends[1..].each do |blas|
      result = blas.sscal(3.0, x)
      ref.data.zip(result.data).each do |expected, actual|
        assert_in_delta expected, actual, 1e-4,
          "#{blas.name} SSCAL differs from CPU"
      end
    end
  end

  def test_all_backends_sasum_consistent
    x = Vector.new(data: [1.0, -2.0, 3.0, -4.0], size: 4)
    ref = @backends[0].sasum(x)

    @backends[1..].each do |blas|
      result = blas.sasum(x)
      assert_in_delta ref, result, 1e-4,
        "#{blas.name} SASUM differs from CPU"
    end
  end

  def test_all_backends_isamax_consistent
    x = Vector.new(data: [1.0, -5.0, 3.0], size: 3)
    ref = @backends[0].isamax(x)

    @backends[1..].each do |blas|
      result = blas.isamax(x)
      assert_equal ref, result, "#{blas.name} ISAMAX differs from CPU"
    end
  end

  def test_all_backends_sger_consistent
    x = Vector.new(data: [1.0, 2.0], size: 2)
    y = Vector.new(data: [3.0, 4.0, 5.0], size: 3)
    a = Matrix.new(data: [1.0, 2.0, 3.0, 4.0, 5.0, 6.0], rows: 2, cols: 3)
    ref = @backends[0].sger(2.0, x, y, a)

    @backends[1..].each do |blas|
      result = blas.sger(2.0, x, y, a)
      ref.data.zip(result.data).each do |expected, actual|
        assert_in_delta expected, actual, 1e-4,
          "#{blas.name} SGER differs from CPU"
      end
    end
  end

  def test_all_backends_ssymm_consistent
    a = Matrix.new(data: [2.0, 1.0, 1.0, 3.0], rows: 2, cols: 2)
    b = Matrix.new(data: [1.0, 2.0, 3.0, 4.0], rows: 2, cols: 2)
    c = Matrix.new(data: [0.0, 0.0, 0.0, 0.0], rows: 2, cols: 2)
    ref = @backends[0].ssymm(Side::LEFT, 1.0, a, b, 0.0, c)

    @backends[1..].each do |blas|
      result = blas.ssymm(Side::LEFT, 1.0, a, b, 0.0, c)
      ref.data.zip(result.data).each do |expected, actual|
        assert_in_delta expected, actual, 1e-4,
          "#{blas.name} SSYMM differs from CPU"
      end
    end
  end
end
