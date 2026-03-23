# frozen_string_literal: true

require_relative "test_helper"

# ================================================================
# Tests for the CPU BLAS reference implementation.
#
# This is the most comprehensive test suite because CpuBlas is the
# reference. Every other backend is tested against it.
# ================================================================

class TestCpuBlasLevel1 < Minitest::Test
  include CodingAdventures::BlasLibrary

  def setup
    @blas = Backends::CpuBlas.new
  end

  # --- SAXPY ---

  def test_saxpy_basic
    x = Vector.new(data: [1.0, 2.0, 3.0], size: 3)
    y = Vector.new(data: [4.0, 5.0, 6.0], size: 3)
    result = @blas.saxpy(2.0, x, y)
    assert_equal [6.0, 9.0, 12.0], result.data
  end

  def test_saxpy_zero_alpha
    x = Vector.new(data: [1.0, 2.0], size: 2)
    y = Vector.new(data: [3.0, 4.0], size: 2)
    result = @blas.saxpy(0.0, x, y)
    assert_equal [3.0, 4.0], result.data
  end

  def test_saxpy_negative_alpha
    x = Vector.new(data: [1.0, 2.0], size: 2)
    y = Vector.new(data: [3.0, 4.0], size: 2)
    result = @blas.saxpy(-1.0, x, y)
    assert_equal [2.0, 2.0], result.data
  end

  def test_saxpy_dimension_mismatch
    x = Vector.new(data: [1.0], size: 1)
    y = Vector.new(data: [1.0, 2.0], size: 2)
    assert_raises(ArgumentError) { @blas.saxpy(1.0, x, y) }
  end

  def test_saxpy_single_element
    x = Vector.new(data: [5.0], size: 1)
    y = Vector.new(data: [3.0], size: 1)
    result = @blas.saxpy(2.0, x, y)
    assert_equal [13.0], result.data
  end

  # --- SDOT ---

  def test_sdot_basic
    x = Vector.new(data: [1.0, 2.0, 3.0], size: 3)
    y = Vector.new(data: [4.0, 5.0, 6.0], size: 3)
    assert_in_delta 32.0, @blas.sdot(x, y), 1e-6
  end

  def test_sdot_orthogonal
    x = Vector.new(data: [1.0, 0.0], size: 2)
    y = Vector.new(data: [0.0, 1.0], size: 2)
    assert_in_delta 0.0, @blas.sdot(x, y), 1e-6
  end

  def test_sdot_dimension_mismatch
    x = Vector.new(data: [1.0], size: 1)
    y = Vector.new(data: [1.0, 2.0], size: 2)
    assert_raises(ArgumentError) { @blas.sdot(x, y) }
  end

  # --- SNRM2 ---

  def test_snrm2_basic
    x = Vector.new(data: [3.0, 4.0], size: 2)
    assert_in_delta 5.0, @blas.snrm2(x), 1e-6
  end

  def test_snrm2_single
    x = Vector.new(data: [7.0], size: 1)
    assert_in_delta 7.0, @blas.snrm2(x), 1e-6
  end

  def test_snrm2_zero
    x = Vector.new(data: [0.0, 0.0], size: 2)
    assert_in_delta 0.0, @blas.snrm2(x), 1e-6
  end

  # --- SSCAL ---

  def test_sscal_basic
    x = Vector.new(data: [1.0, 2.0, 3.0], size: 3)
    result = @blas.sscal(2.0, x)
    assert_equal [2.0, 4.0, 6.0], result.data
  end

  def test_sscal_zero
    x = Vector.new(data: [1.0, 2.0], size: 2)
    result = @blas.sscal(0.0, x)
    assert_equal [0.0, 0.0], result.data
  end

  # --- SASUM ---

  def test_sasum_basic
    x = Vector.new(data: [1.0, -2.0, 3.0], size: 3)
    assert_in_delta 6.0, @blas.sasum(x), 1e-6
  end

  def test_sasum_all_positive
    x = Vector.new(data: [1.0, 2.0, 3.0], size: 3)
    assert_in_delta 6.0, @blas.sasum(x), 1e-6
  end

  def test_sasum_all_negative
    x = Vector.new(data: [-1.0, -2.0, -3.0], size: 3)
    assert_in_delta 6.0, @blas.sasum(x), 1e-6
  end

  # --- ISAMAX ---

  def test_isamax_basic
    x = Vector.new(data: [1.0, -5.0, 3.0], size: 3)
    assert_equal 1, @blas.isamax(x)
  end

  def test_isamax_first_element
    x = Vector.new(data: [10.0, 2.0, 3.0], size: 3)
    assert_equal 0, @blas.isamax(x)
  end

  def test_isamax_last_element
    x = Vector.new(data: [1.0, 2.0, 10.0], size: 3)
    assert_equal 2, @blas.isamax(x)
  end

  def test_isamax_empty
    x = Vector.new(data: [], size: 0)
    assert_equal 0, @blas.isamax(x)
  end

  # --- SCOPY ---

  def test_scopy_basic
    x = Vector.new(data: [1.0, 2.0, 3.0], size: 3)
    result = @blas.scopy(x)
    assert_equal [1.0, 2.0, 3.0], result.data
    refute_same x.data, result.data
  end

  # --- SSWAP ---

  def test_sswap_basic
    x = Vector.new(data: [1.0, 2.0], size: 2)
    y = Vector.new(data: [3.0, 4.0], size: 2)
    new_x, new_y = @blas.sswap(x, y)
    assert_equal [3.0, 4.0], new_x.data
    assert_equal [1.0, 2.0], new_y.data
  end

  def test_sswap_dimension_mismatch
    x = Vector.new(data: [1.0], size: 1)
    y = Vector.new(data: [1.0, 2.0], size: 2)
    assert_raises(ArgumentError) { @blas.sswap(x, y) }
  end
end

class TestCpuBlasLevel2 < Minitest::Test
  include CodingAdventures::BlasLibrary

  def setup
    @blas = Backends::CpuBlas.new
  end

  # --- SGEMV ---

  def test_sgemv_no_trans
    a = Matrix.new(data: [1.0, 2.0, 3.0, 4.0], rows: 2, cols: 2)
    x = Vector.new(data: [1.0, 1.0], size: 2)
    y = Vector.new(data: [0.0, 0.0], size: 2)
    result = @blas.sgemv(Transpose::NO_TRANS, 1.0, a, x, 0.0, y)
    assert_in_delta 3.0, result.data[0], 1e-6
    assert_in_delta 7.0, result.data[1], 1e-6
  end

  def test_sgemv_trans
    a = Matrix.new(data: [1.0, 2.0, 3.0, 4.0], rows: 2, cols: 2)
    x = Vector.new(data: [1.0, 1.0], size: 2)
    y = Vector.new(data: [0.0, 0.0], size: 2)
    result = @blas.sgemv(Transpose::TRANS, 1.0, a, x, 0.0, y)
    assert_in_delta 4.0, result.data[0], 1e-6
    assert_in_delta 6.0, result.data[1], 1e-6
  end

  def test_sgemv_with_beta
    a = Matrix.new(data: [1.0, 0.0, 0.0, 1.0], rows: 2, cols: 2)
    x = Vector.new(data: [2.0, 3.0], size: 2)
    y = Vector.new(data: [10.0, 20.0], size: 2)
    result = @blas.sgemv(Transpose::NO_TRANS, 1.0, a, x, 1.0, y)
    assert_in_delta 12.0, result.data[0], 1e-6
    assert_in_delta 23.0, result.data[1], 1e-6
  end

  def test_sgemv_rectangular
    a = Matrix.new(data: [1.0, 2.0, 3.0, 4.0, 5.0, 6.0], rows: 2, cols: 3)
    x = Vector.new(data: [1.0, 1.0, 1.0], size: 3)
    y = Vector.new(data: [0.0, 0.0], size: 2)
    result = @blas.sgemv(Transpose::NO_TRANS, 1.0, a, x, 0.0, y)
    assert_in_delta 6.0, result.data[0], 1e-6
    assert_in_delta 15.0, result.data[1], 1e-6
  end

  def test_sgemv_dimension_mismatch_x
    a = Matrix.new(data: [1.0, 2.0, 3.0, 4.0], rows: 2, cols: 2)
    x = Vector.new(data: [1.0], size: 1)
    y = Vector.new(data: [0.0, 0.0], size: 2)
    assert_raises(ArgumentError) { @blas.sgemv(Transpose::NO_TRANS, 1.0, a, x, 0.0, y) }
  end

  def test_sgemv_dimension_mismatch_y
    a = Matrix.new(data: [1.0, 2.0, 3.0, 4.0], rows: 2, cols: 2)
    x = Vector.new(data: [1.0, 1.0], size: 2)
    y = Vector.new(data: [0.0], size: 1)
    assert_raises(ArgumentError) { @blas.sgemv(Transpose::NO_TRANS, 1.0, a, x, 0.0, y) }
  end

  # --- SGER ---

  def test_sger_basic
    x = Vector.new(data: [1.0, 2.0], size: 2)
    y = Vector.new(data: [3.0, 4.0], size: 2)
    a = Matrix.new(data: [0.0, 0.0, 0.0, 0.0], rows: 2, cols: 2)
    result = @blas.sger(1.0, x, y, a)
    assert_in_delta 3.0, result.data[0], 1e-6
    assert_in_delta 4.0, result.data[1], 1e-6
    assert_in_delta 6.0, result.data[2], 1e-6
    assert_in_delta 8.0, result.data[3], 1e-6
  end

  def test_sger_with_existing_matrix
    x = Vector.new(data: [1.0, 2.0], size: 2)
    y = Vector.new(data: [1.0, 1.0], size: 2)
    a = Matrix.new(data: [10.0, 20.0, 30.0, 40.0], rows: 2, cols: 2)
    result = @blas.sger(1.0, x, y, a)
    assert_in_delta 11.0, result.data[0], 1e-6
    assert_in_delta 21.0, result.data[1], 1e-6
    assert_in_delta 32.0, result.data[2], 1e-6
    assert_in_delta 42.0, result.data[3], 1e-6
  end

  def test_sger_dimension_mismatch
    x = Vector.new(data: [1.0], size: 1)
    y = Vector.new(data: [1.0], size: 1)
    a = Matrix.new(data: [1.0, 2.0, 3.0, 4.0], rows: 2, cols: 2)
    assert_raises(ArgumentError) { @blas.sger(1.0, x, y, a) }
  end
end

class TestCpuBlasLevel3 < Minitest::Test
  include CodingAdventures::BlasLibrary

  def setup
    @blas = Backends::CpuBlas.new
  end

  # --- SGEMM ---

  def test_sgemm_identity
    a = Matrix.new(data: [1.0, 0.0, 0.0, 1.0], rows: 2, cols: 2)
    b = Matrix.new(data: [5.0, 6.0, 7.0, 8.0], rows: 2, cols: 2)
    c = Matrix.new(data: [0.0, 0.0, 0.0, 0.0], rows: 2, cols: 2)
    result = @blas.sgemm(Transpose::NO_TRANS, Transpose::NO_TRANS, 1.0, a, b, 0.0, c)
    assert_in_delta 5.0, result.data[0], 1e-6
    assert_in_delta 6.0, result.data[1], 1e-6
    assert_in_delta 7.0, result.data[2], 1e-6
    assert_in_delta 8.0, result.data[3], 1e-6
  end

  def test_sgemm_2x2
    a = Matrix.new(data: [1.0, 2.0, 3.0, 4.0], rows: 2, cols: 2)
    b = Matrix.new(data: [5.0, 6.0, 7.0, 8.0], rows: 2, cols: 2)
    c = Matrix.new(data: [0.0, 0.0, 0.0, 0.0], rows: 2, cols: 2)
    result = @blas.sgemm(Transpose::NO_TRANS, Transpose::NO_TRANS, 1.0, a, b, 0.0, c)
    assert_in_delta 19.0, result.data[0], 1e-6
    assert_in_delta 22.0, result.data[1], 1e-6
    assert_in_delta 43.0, result.data[2], 1e-6
    assert_in_delta 50.0, result.data[3], 1e-6
  end

  def test_sgemm_with_alpha_beta
    a = Matrix.new(data: [1.0, 2.0, 3.0, 4.0], rows: 2, cols: 2)
    b = Matrix.new(data: [1.0, 0.0, 0.0, 1.0], rows: 2, cols: 2)
    c = Matrix.new(data: [10.0, 10.0, 10.0, 10.0], rows: 2, cols: 2)
    result = @blas.sgemm(Transpose::NO_TRANS, Transpose::NO_TRANS, 2.0, a, b, 3.0, c)
    # result = 2 * A * I + 3 * C = 2*A + 3*C
    assert_in_delta 32.0, result.data[0], 1e-6  # 2*1 + 3*10
    assert_in_delta 34.0, result.data[1], 1e-6  # 2*2 + 3*10
    assert_in_delta 36.0, result.data[2], 1e-6  # 2*3 + 3*10
    assert_in_delta 38.0, result.data[3], 1e-6  # 2*4 + 3*10
  end

  def test_sgemm_transpose_a
    a = Matrix.new(data: [1.0, 3.0, 2.0, 4.0], rows: 2, cols: 2)
    b = Matrix.new(data: [5.0, 6.0, 7.0, 8.0], rows: 2, cols: 2)
    c = Matrix.new(data: [0.0, 0.0, 0.0, 0.0], rows: 2, cols: 2)
    result = @blas.sgemm(Transpose::TRANS, Transpose::NO_TRANS, 1.0, a, b, 0.0, c)
    # A^T = [[1,2],[3,4]], B = [[5,6],[7,8]]
    assert_in_delta 19.0, result.data[0], 1e-6
    assert_in_delta 22.0, result.data[1], 1e-6
    assert_in_delta 43.0, result.data[2], 1e-6
    assert_in_delta 50.0, result.data[3], 1e-6
  end

  def test_sgemm_transpose_b
    a = Matrix.new(data: [1.0, 2.0, 3.0, 4.0], rows: 2, cols: 2)
    b = Matrix.new(data: [5.0, 7.0, 6.0, 8.0], rows: 2, cols: 2)
    c = Matrix.new(data: [0.0, 0.0, 0.0, 0.0], rows: 2, cols: 2)
    result = @blas.sgemm(Transpose::NO_TRANS, Transpose::TRANS, 1.0, a, b, 0.0, c)
    # A = [[1,2],[3,4]], B^T = [[5,6],[7,8]]
    assert_in_delta 19.0, result.data[0], 1e-6
    assert_in_delta 22.0, result.data[1], 1e-6
    assert_in_delta 43.0, result.data[2], 1e-6
    assert_in_delta 50.0, result.data[3], 1e-6
  end

  def test_sgemm_rectangular
    a = Matrix.new(data: [1.0, 2.0, 3.0, 4.0, 5.0, 6.0], rows: 2, cols: 3)
    b = Matrix.new(data: [7.0, 8.0, 9.0, 10.0, 11.0, 12.0], rows: 3, cols: 2)
    c = Matrix.new(data: [0.0, 0.0, 0.0, 0.0], rows: 2, cols: 2)
    result = @blas.sgemm(Transpose::NO_TRANS, Transpose::NO_TRANS, 1.0, a, b, 0.0, c)
    assert_in_delta 58.0, result.data[0], 1e-6
    assert_in_delta 64.0, result.data[1], 1e-6
    assert_in_delta 139.0, result.data[2], 1e-6
    assert_in_delta 154.0, result.data[3], 1e-6
  end

  def test_sgemm_dimension_mismatch
    a = Matrix.new(data: [1.0, 2.0, 3.0, 4.0], rows: 2, cols: 2)
    b = Matrix.new(data: [1.0, 2.0, 3.0, 4.0, 5.0, 6.0], rows: 3, cols: 2)
    c = Matrix.new(data: [0.0, 0.0, 0.0, 0.0], rows: 2, cols: 2)
    assert_raises(ArgumentError) do
      @blas.sgemm(Transpose::NO_TRANS, Transpose::NO_TRANS, 1.0, a, b, 0.0, c)
    end
  end

  def test_sgemm_c_dimension_mismatch
    a = Matrix.new(data: [1.0, 2.0, 3.0, 4.0], rows: 2, cols: 2)
    b = Matrix.new(data: [1.0, 2.0, 3.0, 4.0], rows: 2, cols: 2)
    c = Matrix.new(data: [0.0, 0.0, 0.0], rows: 1, cols: 3)
    assert_raises(ArgumentError) do
      @blas.sgemm(Transpose::NO_TRANS, Transpose::NO_TRANS, 1.0, a, b, 0.0, c)
    end
  end

  # --- SSYMM ---

  def test_ssymm_left
    a = Matrix.new(data: [1.0, 2.0, 2.0, 3.0], rows: 2, cols: 2)
    b = Matrix.new(data: [1.0, 0.0, 0.0, 1.0], rows: 2, cols: 2)
    c = Matrix.new(data: [0.0, 0.0, 0.0, 0.0], rows: 2, cols: 2)
    result = @blas.ssymm(Side::LEFT, 1.0, a, b, 0.0, c)
    assert_in_delta 1.0, result.data[0], 1e-6
    assert_in_delta 2.0, result.data[1], 1e-6
    assert_in_delta 2.0, result.data[2], 1e-6
    assert_in_delta 3.0, result.data[3], 1e-6
  end

  def test_ssymm_right
    a = Matrix.new(data: [1.0, 2.0, 2.0, 3.0], rows: 2, cols: 2)
    b = Matrix.new(data: [1.0, 0.0, 0.0, 1.0], rows: 2, cols: 2)
    c = Matrix.new(data: [0.0, 0.0, 0.0, 0.0], rows: 2, cols: 2)
    result = @blas.ssymm(Side::RIGHT, 1.0, a, b, 0.0, c)
    assert_in_delta 1.0, result.data[0], 1e-6
    assert_in_delta 2.0, result.data[1], 1e-6
    assert_in_delta 2.0, result.data[2], 1e-6
    assert_in_delta 3.0, result.data[3], 1e-6
  end

  def test_ssymm_non_square_raises
    a = Matrix.new(data: [1.0, 2.0, 3.0, 4.0, 5.0, 6.0], rows: 2, cols: 3)
    b = Matrix.new(data: [1.0, 2.0, 3.0, 4.0, 5.0, 6.0], rows: 2, cols: 3)
    c = Matrix.new(data: [0.0, 0.0, 0.0, 0.0, 0.0, 0.0], rows: 2, cols: 3)
    assert_raises(ArgumentError) { @blas.ssymm(Side::LEFT, 1.0, a, b, 0.0, c) }
  end

  # --- SGEMM_BATCHED ---

  def test_sgemm_batched
    a1 = Matrix.new(data: [1.0, 0.0, 0.0, 1.0], rows: 2, cols: 2)
    b1 = Matrix.new(data: [5.0, 6.0, 7.0, 8.0], rows: 2, cols: 2)
    c1 = Matrix.new(data: [0.0, 0.0, 0.0, 0.0], rows: 2, cols: 2)
    a2 = Matrix.new(data: [2.0, 0.0, 0.0, 2.0], rows: 2, cols: 2)
    b2 = Matrix.new(data: [1.0, 1.0, 1.0, 1.0], rows: 2, cols: 2)
    c2 = Matrix.new(data: [0.0, 0.0, 0.0, 0.0], rows: 2, cols: 2)

    results = @blas.sgemm_batched(
      Transpose::NO_TRANS, Transpose::NO_TRANS,
      1.0, [a1, a2], [b1, b2], 0.0, [c1, c2]
    )

    assert_equal 2, results.length
    assert_in_delta 5.0, results[0].data[0], 1e-6
    assert_in_delta 2.0, results[1].data[0], 1e-6
  end

  def test_sgemm_batched_mismatch
    assert_raises(ArgumentError) do
      @blas.sgemm_batched(
        Transpose::NO_TRANS, Transpose::NO_TRANS,
        1.0, [Matrix.new(data: [1.0], rows: 1, cols: 1)],
        [Matrix.new(data: [1.0], rows: 1, cols: 1), Matrix.new(data: [1.0], rows: 1, cols: 1)],
        0.0, [Matrix.new(data: [0.0], rows: 1, cols: 1)]
      )
    end
  end
end

class TestCpuBlasProperties < Minitest::Test
  def setup
    @blas = CodingAdventures::BlasLibrary::Backends::CpuBlas.new
  end

  def test_name
    assert_equal "cpu", @blas.name
  end

  def test_device_name
    assert_equal "CPU (pure Ruby)", @blas.device_name
  end
end
