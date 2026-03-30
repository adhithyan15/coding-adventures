-- ============================================================================
-- matrix — 2D matrix type with arithmetic and linear-algebra operations
-- ============================================================================
--
-- This module implements a two-dimensional matrix type in pure Lua.  It is
-- designed to be readable and educational rather than optimised for raw
-- throughput.  Every algorithm is explained at the level of the mathematics
-- behind it so that a reader new to linear algebra can follow along.
--
-- ## What Is a Matrix?
--
-- A matrix is a rectangular grid of numbers arranged in rows and columns.
-- We write an m×n matrix (m rows, n columns) as:
--
--       ⎡ a₁₁  a₁₂  …  a₁ₙ ⎤
--   A = ⎢ a₂₁  a₂₂  …  a₂ₙ ⎥
--       ⎣ aₘ₁  aₘ₂  …  aₘₙ ⎦
--
-- We index elements as A[i][j] where i is the row (1-based) and j is the
-- column (1-based).  Lua tables start at index 1, so this mapping is
-- natural.
--
-- ## Representation
--
-- Each Matrix value is a plain Lua table with three fields:
--
--   matrix.rows  — number of rows   (positive integer)
--   matrix.cols  — number of columns (positive integer)
--   matrix.data  — an array of rows; each row is an array of numbers
--                  data[i][j] = element at row i, column j
--
-- This "array of arrays" layout is straightforward to construct and index.
-- It is not cache-optimal (a flat 1D array would be faster), but clarity
-- wins for an educational implementation.
--
-- ## Usage
--
--   local M = require("coding_adventures.matrix")
--
--   local A = M.zeros(2, 3)            -- 2x3 matrix of zeros
--   local B = M.new_2d({{1,2},{3,4}})  -- 2x2 from nested table
--   local C, err = M.dot(B, B)         -- matrix multiplication
--
-- ============================================================================

local M = {}

M.VERSION = "0.1.0"

-- ============================================================================
-- Internal helpers
-- ============================================================================

--- make_matrix wraps a populated data table together with dimension metadata
-- into the canonical matrix representation used by this module.
--
-- @param rows  number of rows
-- @param cols  number of columns
-- @param data  array of row-arrays (data[i][j] = element)
-- @return      a matrix table {rows=, cols=, data=}
local function make_matrix(rows, cols, data)
    return { rows = rows, cols = cols, data = data }
end

--- check_same_dims returns an error string if A and B have different
-- dimensions, or nil if they are compatible for element-wise operations.
--
-- Element-wise operations (add, subtract) require the two matrices to be
-- the same shape: same number of rows AND same number of columns.
--
-- @param A  first matrix
-- @param B  second matrix
-- @return   nil (ok) or error string
local function check_same_dims(A, B)
    if A.rows ~= B.rows or A.cols ~= B.cols then
        return string.format(
            "dimension mismatch: (%d×%d) vs (%d×%d)",
            A.rows, A.cols, B.rows, B.cols
        )
    end
    return nil
end

-- ============================================================================
-- Constructors
-- ============================================================================

--- zeros creates an (rows × cols) matrix filled with 0.0.
--
-- The zero matrix is the additive identity for matrix addition:
--
--     A + 0 = A   for any matrix A with the same shape
--
-- It is also useful as the starting point before accumulating values.
--
-- @param rows  number of rows (positive integer)
-- @param cols  number of columns (positive integer)
-- @return      matrix with every element = 0.0
function M.zeros(rows, cols)
    local data = {}
    for i = 1, rows do
        data[i] = {}
        for j = 1, cols do
            -- Explicitly store 0.0 (float) rather than 0 (integer) to keep
            -- all elements consistently floating-point.
            data[i][j] = 0.0
        end
    end
    return make_matrix(rows, cols, data)
end

--- new_2d creates a matrix from a nested Lua table (array of row-arrays).
--
-- The number of rows is inferred from the outer array length; the number
-- of columns is inferred from the length of the first row.  All rows must
-- have the same length (we do not validate this here for simplicity).
--
-- ## Example
--
--   M.new_2d({{1, 2, 3},
--             {4, 5, 6}})  -- 2×3 matrix
--
-- @param data  nested table {{row1}, {row2}, …}
-- @return      matrix
function M.new_2d(data)
    local rows = #data
    local cols = (rows > 0) and #data[1] or 0

    -- Make a deep copy so the caller's table cannot accidentally mutate
    -- the matrix's internal state.
    local copy = {}
    for i = 1, rows do
        copy[i] = {}
        for j = 1, cols do
            copy[i][j] = data[i][j]
        end
    end

    return make_matrix(rows, cols, copy)
end

--- new_1d creates a 1×n row vector from a flat Lua table.
--
-- A row vector is a matrix with a single row.  Many operations in machine
-- learning (bias addition, elementwise scaling of a batch) are naturally
-- expressed in terms of row vectors.
--
-- @param data  flat table {v₁, v₂, …, vₙ}
-- @return      1×n matrix
function M.new_1d(data)
    local n    = #data
    local copy = {}
    for j = 1, n do
        copy[j] = data[j]
    end
    return make_matrix(1, n, { copy })
end

--- new_scalar creates a 1×1 matrix containing a single value.
--
-- Scalar matrices are convenient placeholders when an algorithm expects a
-- matrix but the operand is logically a single number.
--
-- @param val  a number
-- @return     1×1 matrix
function M.new_scalar(val)
    return make_matrix(1, 1, { { val } })
end

-- ============================================================================
-- Element access
-- ============================================================================

--- get returns the element at row i, column j (1-based).
--
-- @param mat  a matrix
-- @param i    row index (1-based)
-- @param j    column index (1-based)
-- @return     the scalar value at position (i, j)
function M.get(mat, i, j)
    return mat.data[i][j]
end

--- set replaces the element at row i, column j with val.
--
-- @param mat  a matrix (mutated in place)
-- @param i    row index (1-based)
-- @param j    column index (1-based)
-- @param val  new value
function M.set(mat, i, j, val)
    mat.data[i][j] = val
end

-- ============================================================================
-- Arithmetic: element-wise
-- ============================================================================

--- add computes A + B element-wise.
--
-- ## Definition
--
-- Matrix addition is defined only when A and B have the same dimensions.
-- Each element of the result is the sum of the corresponding elements:
--
--     (A + B)[i][j] = A[i][j] + B[i][j]
--
-- ## Properties
--
-- - Commutative: A + B = B + A
-- - Associative: (A + B) + C = A + (B + C)
-- - The zero matrix is the additive identity: A + 0 = A
--
-- @param A  matrix (m×n)
-- @param B  matrix (must also be m×n)
-- @return   (result matrix, nil) on success, or (nil, error_string) on failure
function M.add(A, B)
    local err = check_same_dims(A, B)
    if err then return nil, err end

    local result = M.zeros(A.rows, A.cols)
    for i = 1, A.rows do
        for j = 1, A.cols do
            result.data[i][j] = A.data[i][j] + B.data[i][j]
        end
    end
    return result, nil
end

--- add_scalar adds a scalar value to every element of A.
--
-- This is equivalent to adding a matrix of the same shape filled with
-- the scalar value, but much more efficient (no temporary matrix needed).
--
--     (A + s)[i][j] = A[i][j] + s
--
-- @param A       matrix
-- @param scalar  a number to add to every element
-- @return        new matrix with scalar added to every element
function M.add_scalar(A, scalar)
    local result = M.zeros(A.rows, A.cols)
    for i = 1, A.rows do
        for j = 1, A.cols do
            result.data[i][j] = A.data[i][j] + scalar
        end
    end
    return result
end

--- subtract computes A - B element-wise.
--
-- Same dimension rules as `add`.
--
--     (A - B)[i][j] = A[i][j] - B[i][j]
--
-- @param A  matrix (m×n)
-- @param B  matrix (must also be m×n)
-- @return   (result matrix, nil) on success, or (nil, error_string) on failure
function M.subtract(A, B)
    local err = check_same_dims(A, B)
    if err then return nil, err end

    local result = M.zeros(A.rows, A.cols)
    for i = 1, A.rows do
        for j = 1, A.cols do
            result.data[i][j] = A.data[i][j] - B.data[i][j]
        end
    end
    return result, nil
end

--- scale multiplies every element of A by a scalar.
--
--     (s · A)[i][j] = s · A[i][j]
--
-- Scaling is a specific case of scalar multiplication.  It stretches
-- (or compresses, if |s| < 1) the matrix without changing its shape.
--
-- @param A       matrix
-- @param scalar  multiplication factor
-- @return        new matrix with every element multiplied by scalar
function M.scale(A, scalar)
    local result = M.zeros(A.rows, A.cols)
    for i = 1, A.rows do
        for j = 1, A.cols do
            result.data[i][j] = A.data[i][j] * scalar
        end
    end
    return result
end

-- ============================================================================
-- Transpose
-- ============================================================================

--- transpose flips A across its main diagonal: (Aᵀ)[i][j] = A[j][i].
--
-- ## Definition
--
-- The transpose of an m×n matrix is an n×m matrix where rows and columns
-- are swapped:
--
--       ⎡ 1 2 3 ⎤ᵀ    ⎡ 1 4 ⎤
--       ⎣ 4 5 6 ⎦  =  ⎢ 2 5 ⎥
--                      ⎣ 3 6 ⎦
--
-- ## Properties
--
-- - (Aᵀ)ᵀ = A   (double transpose recovers the original)
-- - (A + B)ᵀ = Aᵀ + Bᵀ
-- - (AB)ᵀ = BᵀAᵀ  (note the reversed order!)
-- - The transpose of a row vector is a column vector and vice versa.
--
-- @param A  matrix (m×n)
-- @return   new matrix (n×m) that is the transpose of A
function M.transpose(A)
    -- Result dimensions are swapped: m×n becomes n×m.
    local result = M.zeros(A.cols, A.rows)
    for i = 1, A.rows do
        for j = 1, A.cols do
            -- Swap row and column indices.
            result.data[j][i] = A.data[i][j]
        end
    end
    return result
end

-- ============================================================================
-- Dot product (matrix multiplication)
-- ============================================================================

--- dot computes the matrix product A · B.
--
-- ## Definition
--
-- The matrix product C = A · B is defined when A is m×k and B is k×n
-- (A's column count must equal B's row count).  The result is m×n:
--
--     C[i][j] = Σ_{l=1}^{k}  A[i][l] · B[l][j]
--
-- Think of it as: each element of C is the dot product of a row of A
-- with a column of B.
--
-- ## Why the Dimension Constraint?
--
-- When we compute the dot product of row i of A (length k) with column j
-- of B (length k), both must have the same length k.  If A.cols ≠ B.rows
-- the operation is mathematically undefined.
--
-- ## Properties
--
-- - NOT commutative in general: A·B ≠ B·A
-- - Associative: (A·B)·C = A·(B·C)
-- - Distributive over addition: A·(B+C) = A·B + A·C
-- - Identity: I·A = A·I = A (where I is the identity matrix)
--
-- ## Example (2×2)
--
--   A = [[1, 2], [3, 4]]
--   B = [[5, 6], [7, 8]]
--   C[1][1] = 1*5 + 2*7 = 19
--   C[1][2] = 1*6 + 2*8 = 22
--   C[2][1] = 3*5 + 4*7 = 43
--   C[2][2] = 3*6 + 4*8 = 50
--
-- @param A  matrix (m×k)
-- @param B  matrix (k×n)
-- @return   (m×n result matrix, nil) on success, or (nil, error_string)
function M.dot(A, B)
    -- The inner dimensions must agree.
    if A.cols ~= B.rows then
        return nil, string.format(
            "dot: inner dimensions must match; got (%d×%d) · (%d×%d)",
            A.rows, A.cols, B.rows, B.cols
        )
    end

    local m      = A.rows
    local k      = A.cols   -- = B.rows (the shared inner dimension)
    local n      = B.cols
    local result = M.zeros(m, n)

    for i = 1, m do
        for j = 1, n do
            -- Compute the inner product of row i of A with column j of B.
            local sum = 0.0
            for l = 1, k do
                sum = sum + A.data[i][l] * B.data[l][j]
            end
            result.data[i][j] = sum
        end
    end

    return result, nil
end

return M
