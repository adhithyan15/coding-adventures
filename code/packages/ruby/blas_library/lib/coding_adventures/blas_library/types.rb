# frozen_string_literal: true

# ---------------------------------------------------------------------------
# BLAS Data Types -- Matrix, Vector, and enumeration types.
# ---------------------------------------------------------------------------
#
# === What Lives Here ===
#
# This module defines the core data types used throughout the BLAS library:
#
#     1. StorageOrder  -- how matrix elements are laid out in memory
#     2. Transpose     -- whether to logically transpose a matrix
#     3. Side          -- which side the special matrix is on (for SYMM)
#     4. Vector        -- a 1-D array of floats
#     5. Matrix        -- a 2-D array of floats stored as a flat Array
#
# === Why Flat Storage? ===
#
# GPUs need contiguous memory. A Ruby Array of Arrays (the nested 2D approach)
# has each inner array allocated separately in memory. A flat Array of floats
# is one contiguous block -- when we upload it to GPU memory, it's a single
# memcpy.
#
#     Nested (2D):
#         data = [[1, 2, 3],
#                 [4, 5, 6]]
#         # Each inner Array is a separate Ruby object
#
#     Flat (BLAS library):
#         data = [1, 2, 3, 4, 5, 6]
#         # One contiguous Array. A[i][j] = data[i * cols + j]
#
# === Enumerations ===
#
# Ruby doesn't have a built-in Enum type like Python. We use modules with
# frozen string constants to simulate enums. This gives us:
#   - Namespacing (StorageOrder::ROW_MAJOR)
#   - Type safety through string comparison
#   - Immutability via freeze

module CodingAdventures
  module BlasLibrary
    # =====================================================================
    # Enumerations -- small types that control BLAS operation behavior
    # =====================================================================

    # How matrix elements are laid out in memory.
    #
    # ================================================================
    # HOW MATRICES ARE STORED IN MEMORY
    # ================================================================
    #
    # A 2x3 matrix:
    #     [ 1  2  3 ]
    #     [ 4  5  6 ]
    #
    # Row-major (C convention):    [1, 2, 3, 4, 5, 6]
    #     A[i][j] = data[i * cols + j]
    #
    # Column-major (Fortran/BLAS): [1, 4, 2, 5, 3, 6]
    #     A[i][j] = data[j * rows + i]
    #
    # We default to row-major because Ruby, C, and most ML frameworks
    # use row-major. Traditional BLAS uses column-major (Fortran heritage).
    # ================================================================
    module StorageOrder
      ROW_MAJOR = "row_major"
      COLUMN_MAJOR = "column_major"
    end

    # Transpose flags for GEMM and GEMV.
    #
    # ================================================================
    # TRANSPOSE FLAGS FOR GEMM AND GEMV
    # ================================================================
    #
    # When computing C = alpha * A * B + beta * C, you often want to use
    # A^T or B^T without physically transposing the matrix. The Transpose
    # flag tells the backend to "pretend" the matrix is transposed.
    #
    # This is a classic BLAS optimization: instead of allocating a new
    # matrix and copying transposed data, you just change the access
    # pattern. For a row-major matrix with shape (M, N):
    #   - NO_TRANS: access as (M, N), stride = N
    #   - TRANS:    access as (N, M), stride = M
    # ================================================================
    module Transpose
      NO_TRANS = "no_trans"
      TRANS = "trans"
    end

    # Which side the special matrix is on (for SYMM, TRMM).
    #
    # ================================================================
    # WHICH SIDE THE SPECIAL MATRIX IS ON (FOR SYMM, TRMM)
    # ================================================================
    #
    # SYMM computes C = alpha * A * B + beta * C where A is symmetric.
    # If Side::LEFT:  A is on the left  -> C = alpha * (A) * B + beta * C
    # If Side::RIGHT: A is on the right -> C = alpha * B * (A) + beta * C
    # ================================================================
    module Side
      LEFT = "left"
      RIGHT = "right"
    end

    # =====================================================================
    # Vector -- a 1-D array of single-precision floats
    # =====================================================================

    # A 1-D array of single-precision floats.
    #
    # ================================================================
    # A 1-D ARRAY OF SINGLE-PRECISION FLOATS
    # ================================================================
    #
    # This is the simplest possible vector type. It holds:
    # - data: a flat Array of Float values
    # - size: how many elements
    #
    # It is NOT a tensor. It is NOT a GPU buffer. It lives on the host
    # (CPU). Each backend copies it to the device when needed and copies
    # results back. This keeps the interface dead simple.
    #
    # Example:
    #     v = Vector.new(data: [1.0, 2.0, 3.0], size: 3)
    #     v.data[0]  # => 1.0
    #     v.size     # => 3
    # ================================================================
    class Vector
      attr_reader :data, :size

      # Create a new Vector.
      #
      # @param data [Array<Float>] The vector elements.
      # @param size [Integer] The declared size (must match data.length).
      # @raise [ArgumentError] If data length doesn't match size.
      def initialize(data:, size:)
        if data.length != size
          raise ArgumentError,
            "Vector data has #{data.length} elements but size=#{size}"
        end

        @data = data
        @size = size
      end
    end

    # =====================================================================
    # Matrix -- a 2-D array of single-precision floats (flat storage)
    # =====================================================================

    # A 2-D array of single-precision floats stored as a flat Array.
    #
    # ================================================================
    # A 2-D ARRAY OF SINGLE-PRECISION FLOATS
    # ================================================================
    #
    # Stored as a flat Array in row-major order by default:
    #
    #     Matrix.new(data: [1,2,3,4,5,6], rows: 2, cols: 3)
    #
    #     represents:  [ 1  2  3 ]
    #                  [ 4  5  6 ]
    #
    #     data[i * cols + j] = element at row i, column j
    #
    # The Matrix type is deliberately simple -- it's a container for
    # moving data between the caller and the BLAS backend. The backend
    # handles device memory management internally.
    # ================================================================
    class Matrix
      attr_reader :data, :rows, :cols, :order

      # Create a new Matrix.
      #
      # @param data [Array<Float>] The flat array of elements.
      # @param rows [Integer] Number of rows.
      # @param cols [Integer] Number of columns.
      # @param order [String] Storage order (default: ROW_MAJOR).
      # @raise [ArgumentError] If data length doesn't match rows * cols.
      def initialize(data:, rows:, cols:, order: StorageOrder::ROW_MAJOR)
        expected = rows * cols
        if data.length != expected
          raise ArgumentError,
            "Matrix data has #{data.length} elements " \
            "but shape is #{rows}x#{cols} = #{expected}"
        end

        @data = data
        @rows = rows
        @cols = cols
        @order = order
      end
    end
  end
end
