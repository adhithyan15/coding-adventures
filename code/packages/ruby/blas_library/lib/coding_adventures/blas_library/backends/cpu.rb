# frozen_string_literal: true

# ---------------------------------------------------------------------------
# CpuBlas -- pure Ruby reference implementation of BLAS.
# ---------------------------------------------------------------------------
#
# === Why a CPU Backend? ===
#
# The CPU backend serves two critical purposes:
#
# 1. **Universal fallback** -- it works everywhere, on any machine, with no
#    GPU drivers or hardware requirements. If everything else fails, CPU works.
#
# 2. **Reference implementation** -- every other backend (CUDA, Metal, etc.) is
#    tested against the CPU backend's results. If CudaBlas and CpuBlas disagree,
#    the bug is in CudaBlas.
#
# === How It Works ===
#
# Every BLAS operation is implemented with explicit Ruby loops. No C extensions,
# no tricks -- just loops and arithmetic. This makes every operation completely
# transparent:
#
#     SAXPY:  result[i] = alpha * x[i] + y[i]
#     GEMM:   C[i][j] += A[i][k] * B[k][j]  for each i, j, k
#     DOT:    sum of x[i] * y[i] for each i
#
# === Performance ===
#
# The CPU backend is SLOW. O(n^3) for GEMM with Ruby loop overhead on every
# element. A 1000x1000 matrix multiply takes seconds. But that's fine -- the
# CPU backend optimizes for **clarity**, not speed. The GPU backends optimize
# for speed.
#
# === ML Extensions ===
#
# The CPU backend implements ALL ML extensions (activation functions, softmax,
# layer normalization, batch normalization, conv2d, attention). These use the
# Math module for exp, sqrt, tanh, etc.
#
# === Duck Typing (BLAS Backend Protocol) ===
#
# In Ruby, we use duck typing instead of explicit protocols. A valid BLAS
# backend must respond to these methods:
#
#   Properties:
#     - name          -> String   (e.g., "cpu", "cuda")
#     - device_name   -> String   (e.g., "CPU (pure Ruby)")
#
#   Level 1 (Vector-Vector, O(n)):
#     - saxpy(alpha, x, y)        -> Vector
#     - sdot(x, y)                -> Float
#     - snrm2(x)                  -> Float
#     - sscal(alpha, x)           -> Vector
#     - sasum(x)                  -> Float
#     - isamax(x)                 -> Integer
#     - scopy(x)                  -> Vector
#     - sswap(x, y)               -> [Vector, Vector]
#
#   Level 2 (Matrix-Vector, O(n^2)):
#     - sgemv(trans, alpha, a, x, beta, y) -> Vector
#     - sger(alpha, x, y, a)               -> Matrix
#
#   Level 3 (Matrix-Matrix, O(n^3)):
#     - sgemm(trans_a, trans_b, alpha, a, b, beta, c) -> Matrix
#     - ssymm(side, alpha, a, b, beta, c)             -> Matrix
#     - sgemm_batched(trans_a, trans_b, alpha, a_list, b_list, beta, c_list) -> Array<Matrix>
#
#   ML Extensions (optional):
#     - relu(x)                           -> Matrix
#     - gelu(x)                           -> Matrix
#     - sigmoid(x)                        -> Matrix
#     - tanh_activation(x)                -> Matrix
#     - softmax(x, axis: -1)              -> Matrix
#     - layer_norm(x, gamma, beta, eps:)  -> Matrix
#     - batch_norm(x, gamma, beta, running_mean, running_var, eps:, training:) -> Matrix
#     - conv2d(input_mat, weight, bias: nil, stride: 1, padding: 0) -> Matrix
#     - attention(q, k, v, mask: nil, scale: nil) -> Matrix

module CodingAdventures
  module BlasLibrary
    module Backends
      # Helper: access a matrix element respecting the transpose flag.
      #
      # ================================================================
      # VIRTUAL TRANSPOSE -- NO COPY NEEDED
      # ================================================================
      #
      # Instead of physically transposing a matrix (allocating new memory
      # and rearranging elements), we just swap the row/col indices:
      #
      #     NO_TRANS: A[row][col] = data[row * cols + col]
      #     TRANS:    A[row][col] = data[col * cols + row]
      #               (swap row and col, keep the original cols stride)
      #
      # This is how real BLAS libraries handle transpose -- the data stays
      # in place, only the access pattern changes.
      # ================================================================
      module BlasHelpers
        module_function

        def get_element(m, row, col, trans)
          if trans == Transpose::TRANS
            # Transposed: logical (row, col) maps to physical (col, row)
            m.data[col * m.cols + row]
          else
            # Not transposed: direct access
            m.data[row * m.cols + col]
          end
        end

        # Get the effective (rows, cols) after applying the transpose flag.
        #
        # A 2x3 matrix transposed becomes 3x2:
        #     NO_TRANS: (2, 3) -> (2, 3)
        #     TRANS:    (2, 3) -> (3, 2)
        def effective_shape(m, trans)
          if trans == Transpose::TRANS
            [m.cols, m.rows]
          else
            [m.rows, m.cols]
          end
        end
      end

      class CpuBlas
        # ================================================================
        # CPU BLAS -- THE REFERENCE IMPLEMENTATION
        # ================================================================
        #
        # This class implements both the core BLAS protocol and the ML
        # extensions protocol using nothing but Ruby loops and the Math
        # standard library.
        #
        # Every other backend's correctness is measured against this one.
        # If CudaBlas.sgemm() and CpuBlas.sgemm() disagree on the result,
        # the bug is in CudaBlas, not CpuBlas.
        #
        # Usage:
        #     blas = CpuBlas.new
        #     result = blas.saxpy(2.0, x, y)
        #     result = blas.sgemm(Transpose::NO_TRANS, Transpose::NO_TRANS, 1.0, a, b, 0.0, c)
        # ================================================================

        # =================================================================
        # Identity properties
        # =================================================================

        # Backend identifier.
        def name
          "cpu"
        end

        # Human-readable device name.
        def device_name
          "CPU (pure Ruby)"
        end

        # =================================================================
        # LEVEL 1: VECTOR-VECTOR OPERATIONS -- O(n)
        # =================================================================

        # SAXPY: result = alpha * x + y
        #
        # ================================================================
        # SAXPY -- THE HELLO WORLD OF BLAS
        # ================================================================
        #
        # S = Single precision, A = Alpha, X = vector X, P = Plus, Y = vector Y
        #
        # This is the simplest BLAS operation and our running example
        # since Layer 11 (logic gates). Each element:
        #
        #     result[i] = alpha * x[i] + y[i]
        #
        # Time complexity: O(n) -- one pass through both vectors.
        # ================================================================
        #
        # @param alpha [Float] Scalar multiplier.
        # @param x [Vector] Input vector.
        # @param y [Vector] Input vector.
        # @return [Vector] Result vector.
        # @raise [ArgumentError] If x.size != y.size.
        def saxpy(alpha, x, y)
          if x.size != y.size
            raise ArgumentError,
              "SAXPY dimension mismatch: x.size=#{x.size} != y.size=#{y.size}"
          end
          result = Array.new(x.size) { |i| alpha * x.data[i] + y.data[i] }
          Vector.new(data: result, size: x.size)
        end

        # DOT product: result = sum(x[i] * y[i])
        #
        # ================================================================
        # DOT PRODUCT -- FOUNDATION OF SIMILARITY
        # ================================================================
        #
        # The dot product measures how "aligned" two vectors are:
        # - Parallel vectors: large positive dot product
        # - Perpendicular vectors: dot product = 0
        # - Anti-parallel: large negative dot product
        #
        # It's also the building block of matrix multiply (GEMM is
        # just a grid of dot products).
        #
        # Time complexity: O(n)
        # ================================================================
        #
        # @param x [Vector] Input vector.
        # @param y [Vector] Input vector.
        # @return [Float] The dot product.
        # @raise [ArgumentError] If x.size != y.size.
        def sdot(x, y)
          if x.size != y.size
            raise ArgumentError,
              "DOT dimension mismatch: x.size=#{x.size} != y.size=#{y.size}"
          end
          sum = 0.0
          x.size.times { |i| sum += x.data[i] * y.data[i] }
          sum
        end

        # Euclidean norm: result = sqrt(sum(x[i]^2))
        #
        # ================================================================
        # EUCLIDEAN NORM (L2 NORM)
        # ================================================================
        #
        # The "length" of a vector in Euclidean space. Used for:
        # - Normalizing vectors (dividing by the norm to get unit vectors)
        # - Convergence checks (is the gradient small enough?)
        # - Regularization (keeping weights small)
        #
        # Numerically: sqrt(x[0]^2 + x[1]^2 + ... + x[n-1]^2)
        #
        # Time complexity: O(n)
        # ================================================================
        #
        # @param x [Vector] Input vector.
        # @return [Float] The Euclidean norm (>= 0).
        def snrm2(x)
          sum_sq = 0.0
          x.data.each { |xi| sum_sq += xi * xi }
          Math.sqrt(sum_sq)
        end

        # Scale: result = alpha * x
        #
        # Multiply every element by the scalar alpha.
        # Time complexity: O(n)
        #
        # @param alpha [Float] Scalar multiplier.
        # @param x [Vector] Input vector.
        # @return [Vector] Scaled vector.
        def sscal(alpha, x)
          Vector.new(data: x.data.map { |xi| alpha * xi }, size: x.size)
        end

        # Absolute sum (L1 norm): result = sum(|x[i]|)
        #
        # Also called the Manhattan distance or taxicab norm. Used in
        # L1 regularization (LASSO) which encourages sparsity.
        #
        # Time complexity: O(n)
        #
        # @param x [Vector] Input vector.
        # @return [Float] The L1 norm (>= 0).
        def sasum(x)
          x.data.sum { |xi| xi.abs }
        end

        # Index of maximum absolute value: argmax(|x[i]|)
        #
        # Returns the 0-based index of the element with the largest
        # absolute value. Used in partial pivoting for LU decomposition
        # to improve numerical stability.
        #
        # Time complexity: O(n)
        #
        # @param x [Vector] Input vector.
        # @return [Integer] 0-based index of max absolute value.
        def isamax(x)
          return 0 if x.size == 0

          max_idx = 0
          max_val = x.data[0].abs
          (1...x.size).each do |i|
            val = x.data[i].abs
            if val > max_val
              max_val = val
              max_idx = i
            end
          end
          max_idx
        end

        # Copy: result = x (deep copy)
        #
        # Creates a completely independent copy. Modifying the result
        # does not affect the original.
        #
        # Time complexity: O(n)
        #
        # @param x [Vector] Input vector.
        # @return [Vector] A deep copy of x.
        def scopy(x)
          Vector.new(data: x.data.dup, size: x.size)
        end

        # Swap: exchange the contents of x and y.
        #
        # Returns [new_x, new_y] where new_x has y's data and new_y
        # has x's data. The originals are not modified.
        #
        # Time complexity: O(n)
        #
        # @param x [Vector] First vector.
        # @param y [Vector] Second vector.
        # @return [Array<Vector>] Two-element array [new_x, new_y].
        # @raise [ArgumentError] If x.size != y.size.
        def sswap(x, y)
          if x.size != y.size
            raise ArgumentError,
              "SWAP dimension mismatch: x.size=#{x.size} != y.size=#{y.size}"
          end
          [
            Vector.new(data: y.data.dup, size: y.size),
            Vector.new(data: x.data.dup, size: x.size)
          ]
        end

        # =================================================================
        # LEVEL 2: MATRIX-VECTOR OPERATIONS -- O(n^2)
        # =================================================================

        # General Matrix-Vector multiply: y = alpha * op(A) * x + beta * y
        #
        # ================================================================
        # GEMV -- MATRIX TIMES VECTOR
        # ================================================================
        #
        # op(A) is the matrix A, optionally transposed:
        #     NO_TRANS: op(A) = A        (M x N)
        #     TRANS:    op(A) = A^T      (N x M)
        #
        # After applying the transpose:
        #     op(A) has shape (m x n)
        #     x must have size n
        #     y must have size m
        #     result has size m
        #
        # Each element of the result:
        #     result[i] = alpha * sum(op(A)[i][k] * x[k], k=0..n-1) + beta * y[i]
        #
        # Time complexity: O(M * N)
        # ================================================================
        #
        # @param trans [String] Transpose flag (Transpose::NO_TRANS or Transpose::TRANS).
        # @param alpha [Float] Scalar for the product.
        # @param a [Matrix] Input matrix.
        # @param x [Vector] Input vector.
        # @param beta [Float] Scalar for y.
        # @param y [Vector] Input vector.
        # @return [Vector] Result vector.
        def sgemv(trans, alpha, a, x, beta, y)
          m, n = BlasHelpers.effective_shape(a, trans)

          if x.size != n
            raise ArgumentError,
              "GEMV dimension mismatch: op(A) is #{m}x#{n} but x.size=#{x.size}"
          end
          if y.size != m
            raise ArgumentError,
              "GEMV dimension mismatch: op(A) is #{m}x#{n} but y.size=#{y.size}"
          end

          result = Array.new(m, 0.0)
          m.times do |i|
            s = 0.0
            n.times do |k|
              s += BlasHelpers.get_element(a, i, k, trans) * x.data[k]
            end
            result[i] = alpha * s + beta * y.data[i]
          end

          Vector.new(data: result, size: m)
        end

        # Outer product (rank-1 update): A = alpha * x * y^T + A
        #
        # ================================================================
        # GER -- OUTER PRODUCT
        # ================================================================
        #
        # The outer product of two vectors creates a matrix:
        #
        #     x = [a, b]     y = [c, d, e]
        #
        #     x * y^T = [ a*c  a*d  a*e ]
        #               [ b*c  b*d  b*e ]
        #
        # Then we scale by alpha and add to the existing matrix A.
        # Each element: result[i][j] = alpha * x[i] * y[j] + A[i][j]
        #
        # Time complexity: O(M * N)
        # ================================================================
        #
        # @param alpha [Float] Scalar multiplier.
        # @param x [Vector] Column vector (M elements).
        # @param y [Vector] Row vector (N elements).
        # @param a [Matrix] Existing matrix to update.
        # @return [Matrix] Updated matrix.
        def sger(alpha, x, y, a)
          if a.rows != x.size
            raise ArgumentError,
              "GER dimension mismatch: A.rows=#{a.rows} != x.size=#{x.size}"
          end
          if a.cols != y.size
            raise ArgumentError,
              "GER dimension mismatch: A.cols=#{a.cols} != y.size=#{y.size}"
          end

          result = a.data.dup
          a.rows.times do |i|
            a.cols.times do |j|
              result[i * a.cols + j] += alpha * x.data[i] * y.data[j]
            end
          end

          Matrix.new(data: result, rows: a.rows, cols: a.cols, order: a.order)
        end

        # =================================================================
        # LEVEL 3: MATRIX-MATRIX OPERATIONS -- O(n^3)
        # =================================================================

        # General Matrix Multiply: C = alpha * op(A) * op(B) + beta * C
        #
        # ================================================================
        # GEMM -- THE MOST IMPORTANT FUNCTION IN ALL OF COMPUTING
        # ================================================================
        #
        # This is the function that NVIDIA employs entire teams to
        # optimize. 70-90% of ML training time is spent here.
        #
        # C = alpha * op(A) * op(B) + beta * C
        #
        # where:
        #     op(A) has shape (M x K)
        #     op(B) has shape (K x N)
        #     C     has shape (M x N)
        #
        # The triple nested loop:
        #     for i in 0...M          # row of C
        #       for j in 0...N        # col of C
        #         sum = 0.0
        #         for k in 0...K      # shared dimension
        #           sum += op(A)[i][k] * op(B)[k][j]
        #         C[i][j] = alpha * sum + beta * C[i][j]
        #
        # Common special cases:
        #     C = A * B        -> alpha=1, beta=0
        #     C = A^T * B      -> trans_a=TRANS, alpha=1, beta=0
        #     C += A * B       -> alpha=1, beta=1
        #     C = 2*A*B + 3*C  -> alpha=2, beta=3
        #
        # Time complexity: O(M * N * K)
        # ================================================================
        #
        # @param trans_a [String] Transpose flag for A.
        # @param trans_b [String] Transpose flag for B.
        # @param alpha [Float] Scalar for the product.
        # @param a [Matrix] First input matrix.
        # @param b [Matrix] Second input matrix.
        # @param beta [Float] Scalar for C.
        # @param c [Matrix] Accumulator matrix.
        # @return [Matrix] Result matrix.
        def sgemm(trans_a, trans_b, alpha, a, b, beta, c)
          # Determine effective shapes after transpose
          m, k_a = BlasHelpers.effective_shape(a, trans_a)
          k_b, n = BlasHelpers.effective_shape(b, trans_b)

          # The inner dimensions must match
          if k_a != k_b
            raise ArgumentError,
              "GEMM dimension mismatch: op(A) is #{m}x#{k_a}, " \
              "op(B) is #{k_b}x#{n}. Inner dimensions #{k_a} != #{k_b}"
          end
          k = k_a

          # C must have shape (M x N)
          if c.rows != m || c.cols != n
            raise ArgumentError,
              "GEMM dimension mismatch: result should be #{m}x#{n} " \
              "but C is #{c.rows}x#{c.cols}"
          end

          # The triple nested loop -- the heart of linear algebra
          result = Array.new(m * n, 0.0)
          m.times do |i|
            n.times do |j|
              s = 0.0
              k.times do |kk|
                s += BlasHelpers.get_element(a, i, kk, trans_a) *
                  BlasHelpers.get_element(b, kk, j, trans_b)
              end
              result[i * n + j] = alpha * s + beta * c.data[i * c.cols + j]
            end
          end

          Matrix.new(data: result, rows: m, cols: n, order: c.order)
        end

        # Symmetric Matrix Multiply.
        #
        # ================================================================
        # SYMM -- SYMMETRIC MATRIX MULTIPLY
        # ================================================================
        #
        # Like GEMM, but exploits the fact that A is symmetric (A = A^T).
        # The backend only needs to read half of A.
        #
        # LEFT:  C = alpha * A * B + beta * C
        # RIGHT: C = alpha * B * A + beta * C
        #
        # A must be square (rows == cols).
        # ================================================================
        #
        # @param side [String] Side::LEFT or Side::RIGHT.
        # @param alpha [Float] Scalar multiplier.
        # @param a [Matrix] Symmetric matrix (must be square).
        # @param b [Matrix] Input matrix.
        # @param beta [Float] Scalar for C.
        # @param c [Matrix] Accumulator matrix.
        # @return [Matrix] Result matrix.
        def ssymm(side, alpha, a, b, beta, c)
          if a.rows != a.cols
            raise ArgumentError, "SSYMM: A must be square but is #{a.rows}x#{a.cols}"
          end

          if side == Side::LEFT
            m = a.rows
            n = b.cols
            if b.rows != m
              raise ArgumentError, "SSYMM LEFT: A is #{m}x#{m} but B.rows=#{b.rows}"
            end
          else
            m = b.rows
            n = a.rows
            if b.cols != n
              raise ArgumentError, "SSYMM RIGHT: A is #{n}x#{n} but B.cols=#{b.cols}"
            end
          end

          if c.rows != m || c.cols != n
            raise ArgumentError, "SSYMM: C should be #{m}x#{n} but is #{c.rows}x#{c.cols}"
          end

          # Use sgemm with NO_TRANS for both -- A is symmetric so A = A^T
          if side == Side::LEFT
            sgemm(Transpose::NO_TRANS, Transpose::NO_TRANS, alpha, a, b, beta, c)
          else
            sgemm(Transpose::NO_TRANS, Transpose::NO_TRANS, alpha, b, a, beta, c)
          end
        end

        # Batched GEMM: multiple independent GEMMs.
        #
        # ================================================================
        # BATCHED GEMM -- MANY MATRIX MULTIPLIES AT ONCE
        # ================================================================
        #
        # Used for multi-head attention (each head is a separate GEMM),
        # batched inference (each sample is a separate GEMM), and more.
        #
        # On a GPU, all GEMMs can run in parallel. On CPU, we just loop.
        # ================================================================
        #
        # @param trans_a [String] Transpose flag for A matrices.
        # @param trans_b [String] Transpose flag for B matrices.
        # @param alpha [Float] Scalar for the products.
        # @param a_list [Array<Matrix>] List of A matrices.
        # @param b_list [Array<Matrix>] List of B matrices.
        # @param beta [Float] Scalar for C matrices.
        # @param c_list [Array<Matrix>] List of C matrices.
        # @return [Array<Matrix>] List of result matrices.
        def sgemm_batched(trans_a, trans_b, alpha, a_list, b_list, beta, c_list)
          if a_list.length != b_list.length || b_list.length != c_list.length
            raise ArgumentError,
              "Batched GEMM: batch sizes don't match: " \
              "A=#{a_list.length}, B=#{b_list.length}, C=#{c_list.length}"
          end

          a_list.zip(b_list, c_list).map do |a, b, c_mat|
            sgemm(trans_a, trans_b, alpha, a, b, beta, c_mat)
          end
        end

        # =================================================================
        # ML EXTENSIONS: Activation Functions
        # =================================================================

        # ReLU activation: max(0, x)
        #
        # ================================================================
        # RELU -- RECTIFIED LINEAR UNIT
        # ================================================================
        #
        # The most common activation function in deep learning:
        #     relu(x) = max(0, x)
        #
        # Truth table for a single element:
        #     x < 0  -> 0.0    (negative inputs are zeroed)
        #     x >= 0 -> x      (positive inputs pass through)
        #
        # ReLU is popular because:
        # 1. It's extremely fast to compute (just a comparison)
        # 2. It doesn't saturate for positive values (no vanishing gradient)
        # 3. It produces sparse activations (many zeros)
        # ================================================================
        #
        # @param x [Matrix] Input matrix.
        # @return [Matrix] Matrix with ReLU applied element-wise.
        def relu(x)
          Matrix.new(
            data: x.data.map { |v| [0.0, v].max },
            rows: x.rows,
            cols: x.cols,
            order: x.order
          )
        end

        # GELU activation: x * Phi(x) where Phi is the CDF of N(0,1).
        #
        # ================================================================
        # GELU -- GAUSSIAN ERROR LINEAR UNIT
        # ================================================================
        #
        # Used in GPT, BERT, and modern Transformers. Unlike ReLU which
        # has a hard cutoff at 0, GELU smoothly transitions:
        #
        #     gelu(x) = x * 0.5 * (1 + tanh(sqrt(2/pi) * (x + 0.044715 * x^3)))
        #
        # This approximation (from Hendrycks & Gimpel, 2016) is what
        # PyTorch and TensorFlow use.
        # ================================================================
        #
        # @param x [Matrix] Input matrix.
        # @return [Matrix] Matrix with GELU applied element-wise.
        def gelu(x)
          sqrt_2_over_pi = Math.sqrt(2.0 / Math::PI)
          result = x.data.map do |v|
            inner = sqrt_2_over_pi * (v + 0.044715 * v * v * v)
            0.5 * v * (1.0 + Math.tanh(inner))
          end
          Matrix.new(data: result, rows: x.rows, cols: x.cols, order: x.order)
        end

        # Sigmoid activation: 1 / (1 + exp(-x))
        #
        # ================================================================
        # SIGMOID -- THE LOGISTIC FUNCTION
        # ================================================================
        #
        # Maps any real number to the range (0, 1):
        #     sigmoid(-inf) -> 0
        #     sigmoid(0)    -> 0.5
        #     sigmoid(+inf) -> 1
        #
        # Used as the output activation for binary classification and
        # as a gate in LSTMs.
        #
        # Numerically stable implementation: for large negative x,
        # exp(-x) overflows. We use: if x >= 0, compute as 1/(1+exp(-x));
        # if x < 0, compute as exp(x)/(1+exp(x)).
        # ================================================================
        #
        # @param x [Matrix] Input matrix.
        # @return [Matrix] Matrix with sigmoid applied element-wise.
        def sigmoid(x)
          result = x.data.map do |v|
            if v >= 0
              1.0 / (1.0 + Math.exp(-v))
            else
              ev = Math.exp(v)
              ev / (1.0 + ev)
            end
          end
          Matrix.new(data: result, rows: x.rows, cols: x.cols, order: x.order)
        end

        # Tanh activation: tanh(x)
        #
        # Maps any real number to (-1, 1). Used in RNNs and as an
        # activation function. Related to sigmoid: tanh(x) = 2*sigmoid(2x) - 1.
        #
        # @param x [Matrix] Input matrix.
        # @return [Matrix] Matrix with tanh applied element-wise.
        def tanh_activation(x)
          Matrix.new(
            data: x.data.map { |v| Math.tanh(v) },
            rows: x.rows,
            cols: x.cols,
            order: x.order
          )
        end

        # =================================================================
        # ML EXTENSIONS: Softmax
        # =================================================================

        # Numerically stable softmax along an axis.
        #
        # ================================================================
        # SOFTMAX -- PROBABILITY DISTRIBUTION OVER A VECTOR
        # ================================================================
        #
        # Converts a vector of real numbers into a probability distribution:
        #     softmax(x)[i] = exp(x[i]) / sum(exp(x[j]))
        #
        # The NAIVE implementation overflows for large x because exp(710)
        # is infinity in float64. The STABLE version subtracts the max first:
        #     softmax(x)[i] = exp(x[i] - max(x)) / sum(exp(x[j] - max(x)))
        #
        # This works because softmax is invariant to constant shifts:
        #     softmax(x + c) = softmax(x)  for any constant c
        #
        # axis=-1 means "along the last dimension" (columns for 2D).
        # For a 2D matrix, this means each ROW becomes a probability
        # distribution that sums to 1.0.
        # ================================================================
        #
        # @param x [Matrix] Input matrix.
        # @param axis [Integer] Which axis to normalize along (-1 or 1 = rows, 0 = columns).
        # @return [Matrix] Softmax-normalized matrix.
        def softmax(x, axis: -1)
          # Normalize axis
          axis = 1 if axis == -1

          if axis == 1
            # Softmax along each row
            result = []
            x.rows.times do |i|
              row = x.data[i * x.cols, x.cols]
              max_val = row.max
              exps = row.map { |v| Math.exp(v - max_val) }
              total = exps.sum
              exps.each { |e| result << e / total }
            end
          else
            # axis == 0: softmax along each column
            result = x.data.dup
            x.cols.times do |j|
              col = Array.new(x.rows) { |i| x.data[i * x.cols + j] }
              max_val = col.max
              exps = col.map { |v| Math.exp(v - max_val) }
              total = exps.sum
              x.rows.times do |i|
                result[i * x.cols + j] = exps[i] / total
              end
            end
          end

          Matrix.new(data: result, rows: x.rows, cols: x.cols, order: x.order)
        end

        # =================================================================
        # ML EXTENSIONS: Normalization
        # =================================================================

        # Layer Normalization (Ba et al., 2016).
        #
        # ================================================================
        # LAYER NORM -- NORMALIZE EACH SAMPLE INDEPENDENTLY
        # ================================================================
        #
        # For each row (sample) in the matrix:
        #     1. Compute mean: mu = sum(x) / n
        #     2. Compute variance: var = sum((x - mu)^2) / n
        #     3. Normalize: x_hat = (x - mu) / sqrt(var + eps)
        #     4. Scale and shift: result = gamma * x_hat + beta
        #
        # gamma and beta are learnable parameters (one per feature).
        #
        # Used in: Transformers, GPT, BERT (before every attention/FFN block)
        # ================================================================
        #
        # @param x [Matrix] Input matrix.
        # @param gamma [Vector] Scale parameter (one per feature).
        # @param beta [Vector] Shift parameter (one per feature).
        # @param eps [Float] Small constant for numerical stability.
        # @return [Matrix] Normalized matrix.
        def layer_norm(x, gamma, beta, eps: 1e-5)
          if gamma.size != x.cols
            raise ArgumentError, "LayerNorm: gamma.size=#{gamma.size} != x.cols=#{x.cols}"
          end
          if beta.size != x.cols
            raise ArgumentError, "LayerNorm: beta.size=#{beta.size} != x.cols=#{x.cols}"
          end

          result = Array.new(x.rows * x.cols, 0.0)
          n = x.cols

          x.rows.times do |i|
            row = x.data[i * n, n]

            # Step 1: mean
            mean = row.sum / n

            # Step 2: variance
            var = row.sum { |v| (v - mean)**2 } / n

            # Step 3 & 4: normalize, scale, shift
            inv_std = 1.0 / Math.sqrt(var + eps)
            n.times do |j|
              x_hat = (row[j] - mean) * inv_std
              result[i * n + j] = gamma.data[j] * x_hat + beta.data[j]
            end
          end

          Matrix.new(data: result, rows: x.rows, cols: x.cols, order: x.order)
        end

        # Batch Normalization (Ioffe & Szegedy, 2015).
        #
        # ================================================================
        # BATCH NORM -- NORMALIZE EACH FEATURE ACROSS THE BATCH
        # ================================================================
        #
        # Unlike layer norm (which normalizes each sample), batch norm
        # normalizes each FEATURE across all samples in the batch:
        #
        # Training mode:
        #     mean_j = sum(x[i][j] for i in batch) / batch_size
        #     var_j  = sum((x[i][j] - mean_j)^2 for i in batch) / batch_size
        #     x_hat[i][j] = (x[i][j] - mean_j) / sqrt(var_j + eps)
        #     result[i][j] = gamma[j] * x_hat[i][j] + beta[j]
        #
        # Inference mode:
        #     Uses running_mean and running_var instead of batch statistics.
        #
        # Used in: CNNs, ResNets, most non-Transformer architectures
        # ================================================================
        #
        # @param x [Matrix] Input matrix.
        # @param gamma [Vector] Scale parameter.
        # @param beta [Vector] Shift parameter.
        # @param running_mean [Vector] Running mean for inference.
        # @param running_var [Vector] Running variance for inference.
        # @param eps [Float] Small constant for numerical stability.
        # @param training [Boolean] Whether to use batch stats or running stats.
        # @return [Matrix] Normalized matrix.
        def batch_norm(x, gamma, beta, running_mean, running_var, eps: 1e-5, training: false)
          if gamma.size != x.cols
            raise ArgumentError, "BatchNorm: gamma.size=#{gamma.size} != x.cols=#{x.cols}"
          end
          if beta.size != x.cols
            raise ArgumentError, "BatchNorm: beta.size=#{beta.size} != x.cols=#{x.cols}"
          end

          result = Array.new(x.rows * x.cols, 0.0)
          batch_size = x.rows
          n_features = x.cols

          if training
            # Compute batch statistics
            n_features.times do |j|
              col = Array.new(batch_size) { |i| x.data[i * n_features + j] }
              mean = col.sum / batch_size
              var = col.sum { |v| (v - mean)**2 } / batch_size
              inv_std = 1.0 / Math.sqrt(var + eps)
              batch_size.times do |i|
                x_hat = (col[i] - mean) * inv_std
                result[i * n_features + j] = gamma.data[j] * x_hat + beta.data[j]
              end
            end
          else
            # Use running statistics
            n_features.times do |j|
              mean = running_mean.data[j]
              var = running_var.data[j]
              inv_std = 1.0 / Math.sqrt(var + eps)
              batch_size.times do |i|
                x_hat = (x.data[i * n_features + j] - mean) * inv_std
                result[i * n_features + j] = gamma.data[j] * x_hat + beta.data[j]
              end
            end
          end

          Matrix.new(data: result, rows: x.rows, cols: x.cols, order: x.order)
        end

        # =================================================================
        # ML EXTENSIONS: Convolution
        # =================================================================

        # 2D Convolution via im2col + GEMM.
        #
        # ================================================================
        # CONV2D -- SIMPLIFIED 2D CONVOLUTION
        # ================================================================
        #
        # We treat input_mat as a 2D spatial feature map (height x width)
        # and weight as a 2D filter (kH x kW). This is a simplified
        # single-channel convolution for demonstration.
        #
        # Steps:
        # 1. Apply padding if needed
        # 2. Extract all patches (im2col style) into columns
        # 3. Flatten weight into a row vector
        # 4. Compute dot product of weight with each patch
        #
        # The output has shape:
        #     out_h = (height + 2*padding - kH) / stride + 1
        #     out_w = (width + 2*padding - kW) / stride + 1
        # ================================================================
        #
        # @param input_mat [Matrix] Input feature map (height x width).
        # @param weight [Matrix] Convolution filter (kH x kW).
        # @param bias [Vector, nil] Optional bias (size 1 for single filter).
        # @param stride [Integer] Stride of the convolution.
        # @param padding [Integer] Zero-padding on each side.
        # @return [Matrix] Output feature map.
        def conv2d(input_mat, weight, bias: nil, stride: 1, padding: 0)
          h_in = input_mat.rows
          w_in = input_mat.cols
          k_h = weight.rows
          k_w = weight.cols

          # Output dimensions
          out_h = (h_in + 2 * padding - k_h) / stride + 1
          out_w = (w_in + 2 * padding - k_w) / stride + 1

          if out_h <= 0 || out_w <= 0
            raise ArgumentError,
              "Conv2d: output dimensions are non-positive: #{out_h}x#{out_w}"
          end

          # Create padded input if needed
          if padding > 0
            padded_h = h_in + 2 * padding
            padded_w = w_in + 2 * padding
            padded = Array.new(padded_h * padded_w, 0.0)
            h_in.times do |i|
              w_in.times do |j|
                padded[(i + padding) * padded_w + (j + padding)] = input_mat.data[i * w_in + j]
              end
            end
          else
            padded_w = w_in
            padded = input_mat.data.dup
          end

          # Compute convolution
          result = Array.new(out_h * out_w, 0.0)
          weight_flat = weight.data

          out_h.times do |oh|
            out_w.times do |ow|
              s = 0.0
              k_h.times do |kh|
                k_w.times do |kw|
                  ih = oh * stride + kh
                  iw = ow * stride + kw
                  s += padded[ih * padded_w + iw] * weight_flat[kh * k_w + kw]
                end
              end
              if bias
                s += bias.data[0] if bias.size > 0
              end
              result[oh * out_w + ow] = s
            end
          end

          Matrix.new(data: result, rows: out_h, cols: out_w)
        end

        # =================================================================
        # ML EXTENSIONS: Attention
        # =================================================================

        # Scaled Dot-Product Attention (Vaswani et al., 2017).
        #
        # ================================================================
        # ATTENTION -- THE CORE OF TRANSFORMERS
        # ================================================================
        #
        # Attention(Q, K, V) = softmax(Q * K^T / sqrt(d_k)) * V
        #
        # Steps:
        # 1. scores = Q * K^T                     (SGEMM, Level 3)
        # 2. scores = scores / scale               (element-wise)
        # 3. if mask: scores = scores + mask        (element-wise)
        # 4. weights = softmax(scores, axis=-1)    (ML extension)
        # 5. output = weights * V                  (SGEMM, Level 3)
        #
        # Q shape: (seq_len x d_k)
        # K shape: (seq_len x d_k)
        # V shape: (seq_len x d_v)
        # Returns: (seq_len x d_v)
        #
        # This is the function that enables GPT, BERT, and every
        # Transformer model to attend to different parts of the input.
        # ================================================================
        #
        # @param q [Matrix] Query matrix (seq_len x d_k).
        # @param k [Matrix] Key matrix (seq_len x d_k).
        # @param v [Matrix] Value matrix (seq_len x d_v).
        # @param mask [Matrix, nil] Optional additive mask.
        # @param scale [Float, nil] Scaling factor (default: sqrt(d_k)).
        # @return [Matrix] Attention output (seq_len x d_v).
        def attention(q, k, v, mask: nil, scale: nil)
          d_k = q.cols
          scale ||= Math.sqrt(d_k.to_f)

          # Step 1: scores = Q * K^T using SGEMM
          seq_len = q.rows
          scores_c = Matrix.new(data: Array.new(seq_len * k.rows, 0.0), rows: seq_len, cols: k.rows)
          scores = sgemm(Transpose::NO_TRANS, Transpose::TRANS, 1.0, q, k, 0.0, scores_c)

          # Step 2: scale
          scaled_data = scores.data.map { |val| val / scale }

          # Step 3: apply mask (additive, typically -inf for masked positions)
          if mask
            scaled_data.length.times do |i|
              scaled_data[i] += mask.data[i]
            end
          end

          scores_matrix = Matrix.new(data: scaled_data, rows: scores.rows, cols: scores.cols)

          # Step 4: softmax along the last dimension (each row)
          weights = softmax(scores_matrix, axis: -1)

          # Step 5: output = weights * V using SGEMM
          output_c = Matrix.new(
            data: Array.new(weights.rows * v.cols, 0.0),
            rows: weights.rows,
            cols: v.cols
          )
          sgemm(Transpose::NO_TRANS, Transpose::NO_TRANS, 1.0, weights, v, 0.0, output_c)
        end
      end
    end
  end
end
