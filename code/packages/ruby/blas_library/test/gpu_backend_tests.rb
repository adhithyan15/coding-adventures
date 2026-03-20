# frozen_string_literal: true

# ================================================================
# Shared test module for all GPU BLAS backends.
#
# Every GPU backend must produce the same results as the CPU reference.
# This module provides the shared test definitions. Each backend test
# file includes this module and only needs to provide the backend class.
# ================================================================

module GpuBackendTests
  include CodingAdventures::BlasLibrary

  # --- Properties ---

  def test_name
    refute_nil @blas.name
    refute_empty @blas.name
  end

  def test_device_name
    refute_nil @blas.device_name
    refute_empty @blas.device_name
  end

  # --- Level 1: SAXPY ---

  def test_saxpy_basic
    x = Vector.new(data: [1.0, 2.0, 3.0], size: 3)
    y = Vector.new(data: [4.0, 5.0, 6.0], size: 3)
    result = @blas.saxpy(2.0, x, y)
    assert_in_delta 6.0, result.data[0], 1e-4
    assert_in_delta 9.0, result.data[1], 1e-4
    assert_in_delta 12.0, result.data[2], 1e-4
  end

  def test_saxpy_zero_alpha
    x = Vector.new(data: [1.0, 2.0], size: 2)
    y = Vector.new(data: [3.0, 4.0], size: 2)
    result = @blas.saxpy(0.0, x, y)
    assert_in_delta 3.0, result.data[0], 1e-4
    assert_in_delta 4.0, result.data[1], 1e-4
  end

  # --- Level 1: SDOT ---

  def test_sdot_basic
    x = Vector.new(data: [1.0, 2.0, 3.0], size: 3)
    y = Vector.new(data: [4.0, 5.0, 6.0], size: 3)
    assert_in_delta 32.0, @blas.sdot(x, y), 1e-4
  end

  # --- Level 1: SNRM2 ---

  def test_snrm2_basic
    x = Vector.new(data: [3.0, 4.0], size: 2)
    assert_in_delta 5.0, @blas.snrm2(x), 1e-4
  end

  # --- Level 1: SSCAL ---

  def test_sscal_basic
    x = Vector.new(data: [1.0, 2.0, 3.0], size: 3)
    result = @blas.sscal(2.0, x)
    assert_in_delta 2.0, result.data[0], 1e-4
    assert_in_delta 4.0, result.data[1], 1e-4
    assert_in_delta 6.0, result.data[2], 1e-4
  end

  # --- Level 1: SASUM ---

  def test_sasum_basic
    x = Vector.new(data: [1.0, -2.0, 3.0], size: 3)
    assert_in_delta 6.0, @blas.sasum(x), 1e-4
  end

  # --- Level 1: ISAMAX ---

  def test_isamax_basic
    x = Vector.new(data: [1.0, -5.0, 3.0], size: 3)
    assert_equal 1, @blas.isamax(x)
  end

  # --- Level 1: SCOPY ---

  def test_scopy_basic
    x = Vector.new(data: [1.0, 2.0, 3.0], size: 3)
    result = @blas.scopy(x)
    assert_in_delta 1.0, result.data[0], 1e-4
    assert_in_delta 2.0, result.data[1], 1e-4
    assert_in_delta 3.0, result.data[2], 1e-4
  end

  # --- Level 1: SSWAP ---

  def test_sswap_basic
    x = Vector.new(data: [1.0, 2.0], size: 2)
    y = Vector.new(data: [3.0, 4.0], size: 2)
    new_x, new_y = @blas.sswap(x, y)
    assert_in_delta 3.0, new_x.data[0], 1e-4
    assert_in_delta 4.0, new_x.data[1], 1e-4
    assert_in_delta 1.0, new_y.data[0], 1e-4
    assert_in_delta 2.0, new_y.data[1], 1e-4
  end

  # --- Level 2: SGEMV ---

  def test_sgemv_no_trans
    a = Matrix.new(data: [1.0, 2.0, 3.0, 4.0], rows: 2, cols: 2)
    x = Vector.new(data: [1.0, 1.0], size: 2)
    y = Vector.new(data: [0.0, 0.0], size: 2)
    result = @blas.sgemv(Transpose::NO_TRANS, 1.0, a, x, 0.0, y)
    assert_in_delta 3.0, result.data[0], 1e-4
    assert_in_delta 7.0, result.data[1], 1e-4
  end

  # --- Level 2: SGER ---

  def test_sger_basic
    x = Vector.new(data: [1.0, 2.0], size: 2)
    y = Vector.new(data: [3.0, 4.0], size: 2)
    a = Matrix.new(data: [0.0, 0.0, 0.0, 0.0], rows: 2, cols: 2)
    result = @blas.sger(1.0, x, y, a)
    assert_in_delta 3.0, result.data[0], 1e-4
    assert_in_delta 4.0, result.data[1], 1e-4
    assert_in_delta 6.0, result.data[2], 1e-4
    assert_in_delta 8.0, result.data[3], 1e-4
  end

  # --- Level 3: SGEMM ---

  def test_sgemm_2x2
    a = Matrix.new(data: [1.0, 2.0, 3.0, 4.0], rows: 2, cols: 2)
    b = Matrix.new(data: [5.0, 6.0, 7.0, 8.0], rows: 2, cols: 2)
    c = Matrix.new(data: [0.0, 0.0, 0.0, 0.0], rows: 2, cols: 2)
    result = @blas.sgemm(Transpose::NO_TRANS, Transpose::NO_TRANS, 1.0, a, b, 0.0, c)
    assert_in_delta 19.0, result.data[0], 1e-4
    assert_in_delta 22.0, result.data[1], 1e-4
    assert_in_delta 43.0, result.data[2], 1e-4
    assert_in_delta 50.0, result.data[3], 1e-4
  end

  def test_sgemm_identity
    a = Matrix.new(data: [1.0, 0.0, 0.0, 1.0], rows: 2, cols: 2)
    b = Matrix.new(data: [5.0, 6.0, 7.0, 8.0], rows: 2, cols: 2)
    c = Matrix.new(data: [0.0, 0.0, 0.0, 0.0], rows: 2, cols: 2)
    result = @blas.sgemm(Transpose::NO_TRANS, Transpose::NO_TRANS, 1.0, a, b, 0.0, c)
    assert_in_delta 5.0, result.data[0], 1e-4
    assert_in_delta 6.0, result.data[1], 1e-4
    assert_in_delta 7.0, result.data[2], 1e-4
    assert_in_delta 8.0, result.data[3], 1e-4
  end

  # --- Level 3: SSYMM ---

  def test_ssymm_left
    a = Matrix.new(data: [1.0, 2.0, 2.0, 3.0], rows: 2, cols: 2)
    b = Matrix.new(data: [1.0, 0.0, 0.0, 1.0], rows: 2, cols: 2)
    c = Matrix.new(data: [0.0, 0.0, 0.0, 0.0], rows: 2, cols: 2)
    result = @blas.ssymm(Side::LEFT, 1.0, a, b, 0.0, c)
    assert_in_delta 1.0, result.data[0], 1e-4
    assert_in_delta 2.0, result.data[1], 1e-4
    assert_in_delta 2.0, result.data[2], 1e-4
    assert_in_delta 3.0, result.data[3], 1e-4
  end

  # --- Level 3: SGEMM_BATCHED ---

  def test_sgemm_batched
    a1 = Matrix.new(data: [1.0, 0.0, 0.0, 1.0], rows: 2, cols: 2)
    b1 = Matrix.new(data: [5.0, 6.0, 7.0, 8.0], rows: 2, cols: 2)
    c1 = Matrix.new(data: [0.0, 0.0, 0.0, 0.0], rows: 2, cols: 2)

    results = @blas.sgemm_batched(
      Transpose::NO_TRANS, Transpose::NO_TRANS,
      1.0, [a1], [b1], 0.0, [c1]
    )
    assert_equal 1, results.length
    assert_in_delta 5.0, results[0].data[0], 1e-4
  end
end
