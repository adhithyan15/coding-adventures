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
-- Raises an error if the indices are out of bounds.
--
-- @param mat  a matrix
-- @param i    row index (1-based)
-- @param j    column index (1-based)
-- @return     the scalar value at position (i, j)
function M.get(mat, i, j)
    if i < 1 or i > mat.rows or j < 1 or j > mat.cols then
        error(string.format(
            "index out of bounds: (%d, %d) for (%d×%d) matrix",
            i, j, mat.rows, mat.cols
        ))
    end
    return mat.data[i][j]
end

--- set returns a **new** matrix with the element at (i, j) replaced by val.
--
-- The original matrix is not mutated — this follows the immutability
-- principle described in ML03.  All operations return new values.
--
-- @param mat  a matrix
-- @param i    row index (1-based)
-- @param j    column index (1-based)
-- @param val  new value
-- @return     a new matrix identical to mat except data[i][j] = val
function M.set(mat, i, j, val)
    if i < 1 or i > mat.rows or j < 1 or j > mat.cols then
        error(string.format(
            "index out of bounds: (%d, %d) for (%d×%d) matrix",
            i, j, mat.rows, mat.cols
        ))
    end
    -- Deep copy the matrix, then overwrite the target element.
    local copy = {}
    for r = 1, mat.rows do
        copy[r] = {}
        for c = 1, mat.cols do
            copy[r][c] = mat.data[r][c]
        end
    end
    copy[i][j] = val
    return make_matrix(mat.rows, mat.cols, copy)
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

-- ============================================================================
-- Reductions
-- ============================================================================
--
-- A "reduction" collapses a matrix (or part of it) into a smaller result.
-- Think of it as folding all elements through an accumulator function.

--- sum returns the sum of all elements in the matrix.
--
-- For a 2×2 matrix [[1,2],[3,4]], sum = 1 + 2 + 3 + 4 = 10.
--
-- @param mat  a matrix
-- @return     a number
function M.sum(mat)
    local total = 0.0
    for i = 1, mat.rows do
        for j = 1, mat.cols do
            total = total + mat.data[i][j]
        end
    end
    return total
end

--- sum_rows collapses each row into a single value, producing an n×1
-- column vector where each element is the sum of that row's elements.
--
-- Example: [[1,2],[3,4]] -> [[3],[7]]
--
-- @param mat  a matrix (m×n)
-- @return     a column vector (m×1)
function M.sum_rows(mat)
    local data = {}
    for i = 1, mat.rows do
        local s = 0.0
        for j = 1, mat.cols do
            s = s + mat.data[i][j]
        end
        data[i] = { s }
    end
    return make_matrix(mat.rows, 1, data)
end

--- sum_cols collapses each column into a single value, producing a 1×m
-- row vector where each element is the sum of that column's elements.
--
-- Example: [[1,2],[3,4]] -> [[4,6]]
--
-- @param mat  a matrix (m×n)
-- @return     a row vector (1×n)
function M.sum_cols(mat)
    local row = {}
    for j = 1, mat.cols do
        local s = 0.0
        for i = 1, mat.rows do
            s = s + mat.data[i][j]
        end
        row[j] = s
    end
    return make_matrix(1, mat.cols, { row })
end

--- mean returns the arithmetic mean of all elements.
--
-- mean = sum / count, where count = rows * cols.
--
-- @param mat  a matrix
-- @return     a number
function M.mean(mat)
    return M.sum(mat) / (mat.rows * mat.cols)
end

--- min returns the minimum element value in the matrix.
--
-- Scans every element and keeps the smallest.
--
-- @param mat  a matrix
-- @return     a number
function M.min(mat)
    local best = mat.data[1][1]
    for i = 1, mat.rows do
        for j = 1, mat.cols do
            if mat.data[i][j] < best then
                best = mat.data[i][j]
            end
        end
    end
    return best
end

--- max returns the maximum element value in the matrix.
--
-- @param mat  a matrix
-- @return     a number
function M.max(mat)
    local best = mat.data[1][1]
    for i = 1, mat.rows do
        for j = 1, mat.cols do
            if mat.data[i][j] > best then
                best = mat.data[i][j]
            end
        end
    end
    return best
end

--- argmin returns the (row, col) position of the minimum element (1-based).
--
-- When there are ties, the first occurrence in row-major order wins.
--
-- @param mat  a matrix
-- @return     row (1-based), col (1-based)
function M.argmin(mat)
    local best = mat.data[1][1]
    local bi, bj = 1, 1
    for i = 1, mat.rows do
        for j = 1, mat.cols do
            if mat.data[i][j] < best then
                best = mat.data[i][j]
                bi, bj = i, j
            end
        end
    end
    return bi, bj
end

--- argmax returns the (row, col) position of the maximum element (1-based).
--
-- When there are ties, the first occurrence in row-major order wins.
--
-- @param mat  a matrix
-- @return     row (1-based), col (1-based)
function M.argmax(mat)
    local best = mat.data[1][1]
    local bi, bj = 1, 1
    for i = 1, mat.rows do
        for j = 1, mat.cols do
            if mat.data[i][j] > best then
                best = mat.data[i][j]
                bi, bj = i, j
            end
        end
    end
    return bi, bj
end

-- ============================================================================
-- Element-wise math
-- ============================================================================
--
-- These operations apply a mathematical function independently to every
-- element of the matrix, producing a new matrix of the same shape.

--- map applies a function to every element, returning a new matrix.
--
-- The function receives a single number and must return a single number.
--
-- Example: M.map(mat, function(x) return x * 2 end)
--
-- @param mat  a matrix
-- @param fn   a function (number -> number)
-- @return     a new matrix of the same shape
function M.map(mat, fn)
    local data = {}
    for i = 1, mat.rows do
        data[i] = {}
        for j = 1, mat.cols do
            data[i][j] = fn(mat.data[i][j])
        end
    end
    return make_matrix(mat.rows, mat.cols, data)
end

--- sqrt returns a new matrix where every element is the square root
-- of the corresponding element in the original.
--
-- @param mat  a matrix (all elements should be >= 0)
-- @return     a new matrix
function M.sqrt(mat)
    return M.map(mat, math.sqrt)
end

--- abs returns a new matrix where every element is the absolute value
-- of the corresponding element in the original.
--
-- @param mat  a matrix
-- @return     a new matrix
function M.abs(mat)
    return M.map(mat, math.abs)
end

--- pow raises every element to the given exponent, returning a new matrix.
--
-- Example: M.pow(mat, 2.0) squares every element.
--
-- @param mat  a matrix
-- @param exp  the exponent
-- @return     a new matrix
function M.pow(mat, exp)
    return M.map(mat, function(x) return x ^ exp end)
end

-- ============================================================================
-- Shape operations
-- ============================================================================
--
-- Shape operations change the arrangement of elements without altering
-- their values.  They always return new matrices.

--- flatten converts any matrix to a 1×n row vector, reading elements
-- in row-major order.
--
-- Example: [[1,2],[3,4]] -> [[1,2,3,4]]
--
-- @param mat  a matrix
-- @return     a 1×(rows*cols) row vector
function M.flatten(mat)
    local flat = {}
    local idx = 1
    for i = 1, mat.rows do
        for j = 1, mat.cols do
            flat[idx] = mat.data[i][j]
            idx = idx + 1
        end
    end
    return make_matrix(1, mat.rows * mat.cols, { flat })
end

--- reshape rearranges a matrix into the given dimensions.
--
-- The total number of elements (new_rows * new_cols) must equal the
-- original (mat.rows * mat.cols).  Elements are read in row-major order
-- from the source and written in row-major order to the target.
--
-- @param mat       a matrix
-- @param new_rows  desired number of rows
-- @param new_cols  desired number of columns
-- @return          new matrix, or raises error on size mismatch
function M.reshape(mat, new_rows, new_cols)
    local total = mat.rows * mat.cols
    if new_rows * new_cols ~= total then
        error(string.format(
            "reshape: cannot reshape (%d×%d) = %d elements into (%d×%d) = %d elements",
            mat.rows, mat.cols, total, new_rows, new_cols, new_rows * new_cols
        ))
    end

    -- Read all elements in row-major order.
    local flat = {}
    local idx = 1
    for i = 1, mat.rows do
        for j = 1, mat.cols do
            flat[idx] = mat.data[i][j]
            idx = idx + 1
        end
    end

    -- Write them back in the new shape.
    local data = {}
    idx = 1
    for i = 1, new_rows do
        data[i] = {}
        for j = 1, new_cols do
            data[i][j] = flat[idx]
            idx = idx + 1
        end
    end

    return make_matrix(new_rows, new_cols, data)
end

--- row extracts row i as a 1×cols matrix (1-based).
--
-- @param mat  a matrix
-- @param i    row index (1-based)
-- @return     a 1×cols matrix
function M.row(mat, i)
    if i < 1 or i > mat.rows then
        error(string.format("row: index %d out of bounds for %d rows", i, mat.rows))
    end
    local r = {}
    for j = 1, mat.cols do
        r[j] = mat.data[i][j]
    end
    return make_matrix(1, mat.cols, { r })
end

--- col extracts column j as a rows×1 matrix (1-based).
--
-- @param mat  a matrix
-- @param j    column index (1-based)
-- @return     a rows×1 matrix
function M.col(mat, j)
    if j < 1 or j > mat.cols then
        error(string.format("col: index %d out of bounds for %d cols", j, mat.cols))
    end
    local data = {}
    for i = 1, mat.rows do
        data[i] = { mat.data[i][j] }
    end
    return make_matrix(mat.rows, 1, data)
end

--- slice extracts a sub-matrix from rows [r0..r1) and columns [c0..c1).
--
-- Indices are 1-based and the range is half-open: r0 is inclusive, r1 is
-- exclusive (same convention as Python slicing, adjusted to 1-based).
-- So slice(mat, 1, 3, 1, 2) extracts rows 1..2, column 1.
--
-- @param mat  a matrix
-- @param r0   start row (inclusive, 1-based)
-- @param r1   end row (exclusive, 1-based)
-- @param c0   start col (inclusive, 1-based)
-- @param c1   end col (exclusive, 1-based)
-- @return     sub-matrix of shape (r1-r0) × (c1-c0)
function M.slice(mat, r0, r1, c0, c1)
    if r0 < 1 or r1 > mat.rows + 1 or c0 < 1 or c1 > mat.cols + 1 then
        error(string.format(
            "slice: bounds (%d:%d, %d:%d) out of range for (%d×%d) matrix",
            r0, r1, c0, c1, mat.rows, mat.cols
        ))
    end
    local nr = r1 - r0
    local nc = c1 - c0
    local data = {}
    for i = 1, nr do
        data[i] = {}
        for j = 1, nc do
            data[i][j] = mat.data[r0 + i - 1][c0 + j - 1]
        end
    end
    return make_matrix(nr, nc, data)
end

-- ============================================================================
-- Equality and comparison
-- ============================================================================

--- equals returns true if two matrices have exactly the same shape and
-- identical element values (floating-point exact equality).
--
-- @param A  a matrix
-- @param B  a matrix
-- @return   boolean
function M.equals(A, B)
    if A.rows ~= B.rows or A.cols ~= B.cols then return false end
    for i = 1, A.rows do
        for j = 1, A.cols do
            if A.data[i][j] ~= B.data[i][j] then return false end
        end
    end
    return true
end

--- close returns true if two matrices have the same shape and all
-- corresponding elements are within `tol` of each other.
--
-- This is the floating-point-safe version of `equals`.  The default
-- tolerance is 1e-9, which handles typical IEEE 754 rounding errors.
--
-- @param A    a matrix
-- @param B    a matrix
-- @param tol  tolerance (default 1e-9)
-- @return     boolean
function M.close(A, B, tol)
    tol = tol or 1e-9
    if A.rows ~= B.rows or A.cols ~= B.cols then return false end
    for i = 1, A.rows do
        for j = 1, A.cols do
            if math.abs(A.data[i][j] - B.data[i][j]) > tol then
                return false
            end
        end
    end
    return true
end

-- ============================================================================
-- Factory methods
-- ============================================================================

--- identity creates an n×n identity matrix.
--
-- The identity matrix has 1.0 on the main diagonal and 0.0 everywhere
-- else.  It is the multiplicative identity: I · A = A · I = A.
--
-- @param n  size (number of rows and columns)
-- @return   n×n identity matrix
function M.identity(n)
    local data = {}
    for i = 1, n do
        data[i] = {}
        for j = 1, n do
            data[i][j] = (i == j) and 1.0 or 0.0
        end
    end
    return make_matrix(n, n, data)
end

--- from_diagonal creates a square diagonal matrix from a list of values.
--
-- The values go on the main diagonal; all off-diagonal elements are 0.
--
-- Example: from_diagonal({2, 3}) -> [[2,0],[0,3]]
--
-- @param values  a flat table of numbers
-- @return        n×n matrix where n = #values
function M.from_diagonal(values)
    local n = #values
    local data = {}
    for i = 1, n do
        data[i] = {}
        for j = 1, n do
            data[i][j] = (i == j) and values[i] or 0.0
        end
    end
    return make_matrix(n, n, data)
end

return M
