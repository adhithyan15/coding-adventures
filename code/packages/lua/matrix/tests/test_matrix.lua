-- ============================================================================
-- Tests for matrix — 2D matrix type
-- ============================================================================
--
-- These tests cover all public functions of the matrix module:
--   - Constructors: zeros, new_2d, new_1d, new_scalar
--   - Element access: get, set
--   - Arithmetic: add, add_scalar, subtract, scale
--   - Transpose
--   - Dot product (matrix multiplication)
--
-- ## Floating-point comparison
--
-- We compare floating-point values with a tolerance of 1e-10.  Most matrix
-- arithmetic is exact for small integer inputs, but we use a tolerance to
-- protect against any platform-specific rounding differences.

package.path = "../src/?.lua;" .. "../src/?/init.lua;" .. package.path

local M = require("coding_adventures.matrix")

--- near returns true if |a - b| < tol.
local function near(a, b, tol)
    tol = tol or 1e-10
    return math.abs(a - b) < tol
end

--- mat_equal returns true if two matrices have the same shape and all
-- elements are within tolerance of each other.
local function mat_equal(A, B, tol)
    if A.rows ~= B.rows or A.cols ~= B.cols then return false end
    for i = 1, A.rows do
        for j = 1, A.cols do
            if not near(A.data[i][j], B.data[i][j], tol) then
                return false
            end
        end
    end
    return true
end

-- ============================================================================
-- Version
-- ============================================================================

describe("module metadata", function()
    it("has the correct version string", function()
        assert.are.equal("0.1.0", M.VERSION)
    end)
end)

-- ============================================================================
-- zeros
-- ============================================================================

describe("zeros", function()
    it("creates a matrix with the correct dimensions", function()
        local A = M.zeros(3, 4)
        assert.are.equal(3, A.rows)
        assert.are.equal(4, A.cols)
    end)

    it("fills every element with 0.0", function()
        local A = M.zeros(2, 3)
        for i = 1, 2 do
            for j = 1, 3 do
                assert.is_true(near(A.data[i][j], 0.0))
            end
        end
    end)

    it("creates a 1×1 zero matrix", function()
        local A = M.zeros(1, 1)
        assert.are.equal(1, A.rows)
        assert.are.equal(1, A.cols)
        assert.is_true(near(A.data[1][1], 0.0))
    end)

    it("creates a tall matrix (more rows than cols)", function()
        local A = M.zeros(5, 2)
        assert.are.equal(5, A.rows)
        assert.are.equal(2, A.cols)
    end)

    it("creates a wide matrix (more cols than rows)", function()
        local A = M.zeros(2, 5)
        assert.are.equal(2, A.rows)
        assert.are.equal(5, A.cols)
    end)
end)

-- ============================================================================
-- new_2d
-- ============================================================================

describe("new_2d", function()
    it("creates a 2×2 matrix from nested tables", function()
        local A = M.new_2d({{1, 2}, {3, 4}})
        assert.are.equal(2, A.rows)
        assert.are.equal(2, A.cols)
        assert.is_true(near(A.data[1][1], 1.0))
        assert.is_true(near(A.data[1][2], 2.0))
        assert.is_true(near(A.data[2][1], 3.0))
        assert.is_true(near(A.data[2][2], 4.0))
    end)

    it("creates a 2×3 matrix correctly", function()
        local A = M.new_2d({{1, 2, 3}, {4, 5, 6}})
        assert.are.equal(2, A.rows)
        assert.are.equal(3, A.cols)
        assert.is_true(near(A.data[1][3], 3.0))
        assert.is_true(near(A.data[2][1], 4.0))
    end)

    it("deep-copies the input (mutation of source does not affect matrix)", function()
        local src = {{1, 2}, {3, 4}}
        local A = M.new_2d(src)
        src[1][1] = 99   -- mutate source
        -- The matrix should still have the original value.
        assert.is_true(near(A.data[1][1], 1.0))
    end)
end)

-- ============================================================================
-- new_1d
-- ============================================================================

describe("new_1d", function()
    it("creates a 1×n row vector", function()
        local A = M.new_1d({5, 6, 7, 8})
        assert.are.equal(1, A.rows)
        assert.are.equal(4, A.cols)
        assert.is_true(near(A.data[1][1], 5.0))
        assert.is_true(near(A.data[1][4], 8.0))
    end)

    it("creates a 1×1 vector from a single-element table", function()
        local A = M.new_1d({42})
        assert.are.equal(1, A.rows)
        assert.are.equal(1, A.cols)
        assert.is_true(near(A.data[1][1], 42.0))
    end)
end)

-- ============================================================================
-- new_scalar
-- ============================================================================

describe("new_scalar", function()
    it("creates a 1×1 matrix containing the value", function()
        local A = M.new_scalar(3.14)
        assert.are.equal(1, A.rows)
        assert.are.equal(1, A.cols)
        assert.is_true(near(A.data[1][1], 3.14))
    end)

    it("works with zero", function()
        local A = M.new_scalar(0.0)
        assert.is_true(near(A.data[1][1], 0.0))
    end)

    it("works with negative values", function()
        local A = M.new_scalar(-7.5)
        assert.is_true(near(A.data[1][1], -7.5))
    end)
end)

-- ============================================================================
-- get and set
-- ============================================================================

describe("get and set", function()
    it("get returns the element at (i,j)", function()
        local A = M.new_2d({{1, 2}, {3, 4}})
        assert.is_true(near(M.get(A, 1, 1), 1.0))
        assert.is_true(near(M.get(A, 2, 2), 4.0))
    end)

    it("set updates the element at (i,j)", function()
        local A = M.zeros(2, 2)
        M.set(A, 1, 2, 99.0)
        assert.is_true(near(M.get(A, 1, 2), 99.0))
        -- Other elements unchanged.
        assert.is_true(near(M.get(A, 1, 1), 0.0))
    end)

    it("set and get round-trip", function()
        local A = M.zeros(3, 3)
        for i = 1, 3 do
            for j = 1, 3 do
                M.set(A, i, j, i * 10 + j)
            end
        end
        for i = 1, 3 do
            for j = 1, 3 do
                assert.is_true(near(M.get(A, i, j), i * 10 + j))
            end
        end
    end)
end)

-- ============================================================================
-- add
-- ============================================================================

describe("add", function()
    it("adds two 2×2 matrices element-wise", function()
        local A = M.new_2d({{1, 2}, {3, 4}})
        local B = M.new_2d({{5, 6}, {7, 8}})
        -- Expected: {{6,8},{10,12}}
        local C, err = M.add(A, B)
        assert.is_nil(err)
        assert.is_true(near(C.data[1][1], 6.0))
        assert.is_true(near(C.data[1][2], 8.0))
        assert.is_true(near(C.data[2][1], 10.0))
        assert.is_true(near(C.data[2][2], 12.0))
    end)

    it("A + zeros == A", function()
        local A    = M.new_2d({{1, 2}, {3, 4}})
        local zero = M.zeros(2, 2)
        local C, _ = M.add(A, zero)
        assert.is_true(mat_equal(C, A))
    end)

    it("is commutative: A + B == B + A", function()
        local A    = M.new_2d({{1, 2}, {3, 4}})
        local B    = M.new_2d({{9, 8}, {7, 6}})
        local AB, _ = M.add(A, B)
        local BA, _ = M.add(B, A)
        assert.is_true(mat_equal(AB, BA))
    end)

    it("returns an error for incompatible dimensions", function()
        local A = M.zeros(2, 3)
        local B = M.zeros(3, 2)
        local _, err = M.add(A, B)
        assert.is_not_nil(err)
        assert.is_true(type(err) == "string")
    end)

    it("works for 1×1 matrices", function()
        local A = M.new_scalar(3.0)
        local B = M.new_scalar(4.0)
        local C, _ = M.add(A, B)
        assert.is_true(near(C.data[1][1], 7.0))
    end)
end)

-- ============================================================================
-- add_scalar
-- ============================================================================

describe("add_scalar", function()
    it("adds a scalar to every element", function()
        local A = M.new_2d({{1, 2}, {3, 4}})
        local B = M.add_scalar(A, 10)
        assert.is_true(near(B.data[1][1], 11.0))
        assert.is_true(near(B.data[1][2], 12.0))
        assert.is_true(near(B.data[2][1], 13.0))
        assert.is_true(near(B.data[2][2], 14.0))
    end)

    it("adding 0 is identity", function()
        local A = M.new_2d({{5, 6}, {7, 8}})
        local B = M.add_scalar(A, 0.0)
        assert.is_true(mat_equal(A, B))
    end)

    it("adding a negative scalar decrements every element", function()
        local A = M.new_2d({{5, 5}, {5, 5}})
        local B = M.add_scalar(A, -3)
        for i = 1, 2 do
            for j = 1, 2 do
                assert.is_true(near(B.data[i][j], 2.0))
            end
        end
    end)

    it("does not mutate the original matrix", function()
        local A = M.new_2d({{1, 2}})
        M.add_scalar(A, 100)
        assert.is_true(near(A.data[1][1], 1.0))
    end)
end)

-- ============================================================================
-- subtract
-- ============================================================================

describe("subtract", function()
    it("subtracts element-wise", function()
        local A = M.new_2d({{5, 6}, {7, 8}})
        local B = M.new_2d({{1, 2}, {3, 4}})
        local C, _ = M.subtract(A, B)
        assert.is_true(near(C.data[1][1], 4.0))
        assert.is_true(near(C.data[2][2], 4.0))
    end)

    it("A - A == zeros", function()
        local A    = M.new_2d({{9, 8}, {7, 6}})
        local C, _ = M.subtract(A, A)
        local zero = M.zeros(2, 2)
        assert.is_true(mat_equal(C, zero))
    end)

    it("A - zeros == A", function()
        local A    = M.new_2d({{3, 1}, {4, 2}})
        local z    = M.zeros(2, 2)
        local C, _ = M.subtract(A, z)
        assert.is_true(mat_equal(C, A))
    end)

    it("returns an error for incompatible dimensions", function()
        local A = M.zeros(2, 2)
        local B = M.zeros(2, 3)
        local _, err = M.subtract(A, B)
        assert.is_not_nil(err)
    end)
end)

-- ============================================================================
-- scale
-- ============================================================================

describe("scale", function()
    it("multiplies every element by the scalar", function()
        local A = M.new_2d({{1, 2}, {3, 4}})
        local B = M.scale(A, 3.0)
        assert.is_true(near(B.data[1][1],  3.0))
        assert.is_true(near(B.data[1][2],  6.0))
        assert.is_true(near(B.data[2][1],  9.0))
        assert.is_true(near(B.data[2][2], 12.0))
    end)

    it("scaling by 1 is identity", function()
        local A = M.new_2d({{7, 8}, {9, 10}})
        local B = M.scale(A, 1.0)
        assert.is_true(mat_equal(A, B))
    end)

    it("scaling by 0 yields zeros", function()
        local A    = M.new_2d({{1, 2}, {3, 4}})
        local B    = M.scale(A, 0.0)
        local zero = M.zeros(2, 2)
        assert.is_true(mat_equal(B, zero))
    end)

    it("scaling by -1 negates every element", function()
        local A = M.new_2d({{1, -2}, {3, 0}})
        local B = M.scale(A, -1.0)
        assert.is_true(near(B.data[1][1], -1.0))
        assert.is_true(near(B.data[1][2],  2.0))
        assert.is_true(near(B.data[2][1], -3.0))
        assert.is_true(near(B.data[2][2],  0.0))
    end)

    it("does not mutate the original matrix", function()
        local A = M.new_2d({{5, 5}})
        M.scale(A, 100)
        assert.is_true(near(A.data[1][1], 5.0))
    end)
end)

-- ============================================================================
-- transpose
-- ============================================================================

describe("transpose", function()
    it("transposes a 2×3 matrix to 3×2", function()
        -- A = [[1,2,3],[4,5,6]]
        -- Aᵀ = [[1,4],[2,5],[3,6]]
        local A  = M.new_2d({{1, 2, 3}, {4, 5, 6}})
        local AT = M.transpose(A)
        assert.are.equal(3, AT.rows)
        assert.are.equal(2, AT.cols)
        assert.is_true(near(AT.data[1][1], 1.0))
        assert.is_true(near(AT.data[1][2], 4.0))
        assert.is_true(near(AT.data[2][1], 2.0))
        assert.is_true(near(AT.data[2][2], 5.0))
        assert.is_true(near(AT.data[3][1], 3.0))
        assert.is_true(near(AT.data[3][2], 6.0))
    end)

    it("double transpose recovers the original matrix: (Aᵀ)ᵀ = A", function()
        local A   = M.new_2d({{1, 2, 3}, {4, 5, 6}})
        local ATA = M.transpose(M.transpose(A))
        assert.is_true(mat_equal(A, ATA))
    end)

    it("transpose of a square matrix flips the diagonal correctly", function()
        local A  = M.new_2d({{1, 2}, {3, 4}})
        local AT = M.transpose(A)
        -- Off-diagonal elements should swap.
        assert.is_true(near(AT.data[1][2], A.data[2][1]))
        assert.is_true(near(AT.data[2][1], A.data[1][2]))
        -- Diagonal elements should remain the same.
        assert.is_true(near(AT.data[1][1], A.data[1][1]))
        assert.is_true(near(AT.data[2][2], A.data[2][2]))
    end)

    it("transpose of a row vector gives a column vector", function()
        local v  = M.new_1d({1, 2, 3})   -- 1×3
        local vT = M.transpose(v)         -- should be 3×1
        assert.are.equal(3, vT.rows)
        assert.are.equal(1, vT.cols)
        assert.is_true(near(vT.data[1][1], 1.0))
        assert.is_true(near(vT.data[2][1], 2.0))
        assert.is_true(near(vT.data[3][1], 3.0))
    end)

    it("transpose of a 1×1 matrix is itself", function()
        local A  = M.new_scalar(5.0)
        local AT = M.transpose(A)
        assert.is_true(mat_equal(A, AT))
    end)
end)

-- ============================================================================
-- dot
-- ============================================================================

describe("dot", function()
    -- ## 2×2 × 2×2
    --
    -- A = [[1,2],[3,4]],  B = [[5,6],[7,8]]
    -- C[1][1] = 1*5 + 2*7 = 19
    -- C[1][2] = 1*6 + 2*8 = 22
    -- C[2][1] = 3*5 + 4*7 = 43
    -- C[2][2] = 3*6 + 4*8 = 50

    it("computes 2×2 matrix multiplication correctly", function()
        local A = M.new_2d({{1, 2}, {3, 4}})
        local B = M.new_2d({{5, 6}, {7, 8}})
        local C, err = M.dot(A, B)
        assert.is_nil(err)
        assert.are.equal(2, C.rows)
        assert.are.equal(2, C.cols)
        assert.is_true(near(C.data[1][1], 19.0))
        assert.is_true(near(C.data[1][2], 22.0))
        assert.is_true(near(C.data[2][1], 43.0))
        assert.is_true(near(C.data[2][2], 50.0))
    end)

    it("multiplies a 2×3 matrix by a 3×2 matrix to give a 2×2 result", function()
        -- A = [[1,2,3],[4,5,6]]  (2×3)
        -- B = [[7,8],[9,10],[11,12]]  (3×2)
        -- C = A·B  (2×2)
        -- C[1][1] = 1*7 + 2*9 + 3*11 = 7+18+33 = 58
        -- C[1][2] = 1*8 + 2*10 + 3*12 = 8+20+36 = 64
        -- C[2][1] = 4*7 + 5*9 + 6*11 = 28+45+66 = 139
        -- C[2][2] = 4*8 + 5*10 + 6*12 = 32+50+72 = 154
        local A = M.new_2d({{1, 2, 3}, {4, 5, 6}})
        local B = M.new_2d({{7, 8}, {9, 10}, {11, 12}})
        local C, err = M.dot(A, B)
        assert.is_nil(err)
        assert.are.equal(2, C.rows)
        assert.are.equal(2, C.cols)
        assert.is_true(near(C.data[1][1], 58.0))
        assert.is_true(near(C.data[1][2], 64.0))
        assert.is_true(near(C.data[2][1], 139.0))
        assert.is_true(near(C.data[2][2], 154.0))
    end)

    it("identity · A == A", function()
        -- The identity matrix I has 1 on the diagonal and 0 elsewhere.
        -- I · A = A for any compatible A.
        local I = M.new_2d({{1, 0}, {0, 1}})
        local A = M.new_2d({{3, 7}, {2, 5}})
        local IA, _ = M.dot(I, A)
        assert.is_true(mat_equal(IA, A))
    end)

    it("A · identity == A", function()
        local I  = M.new_2d({{1, 0}, {0, 1}})
        local A  = M.new_2d({{3, 7}, {2, 5}})
        local AI, _ = M.dot(A, I)
        assert.is_true(mat_equal(AI, A))
    end)

    it("dot is associative: (A·B)·C == A·(B·C)", function()
        local A = M.new_2d({{1, 2}, {3, 4}})
        local B = M.new_2d({{5, 0}, {0, 1}})
        local C = M.new_2d({{1, 1}, {1, 1}})
        local AB, _   = M.dot(A, B)
        local ABC1, _ = M.dot(AB, C)
        local BC, _   = M.dot(B, C)
        local ABC2, _ = M.dot(A, BC)
        assert.is_true(mat_equal(ABC1, ABC2))
    end)

    it("1×n · n×1 gives a 1×1 (row vector · col vector = scalar)", function()
        -- [1,2,3] · [4,5,6]ᵀ = 1*4 + 2*5 + 3*6 = 32
        local row = M.new_1d({1, 2, 3})             -- 1×3
        local col = M.transpose(M.new_1d({4, 5, 6})) -- 3×1
        local C, err = M.dot(row, col)
        assert.is_nil(err)
        assert.are.equal(1, C.rows)
        assert.are.equal(1, C.cols)
        assert.is_true(near(C.data[1][1], 32.0))
    end)

    it("returns an error when inner dimensions do not match", function()
        local A = M.zeros(2, 3)   -- 2×3
        local B = M.zeros(2, 2)   -- 2×2  — incompatible (need 3×?)
        local _, err = M.dot(A, B)
        assert.is_not_nil(err)
        assert.is_true(type(err) == "string")
    end)

    it("scalar (1×1) multiplication works", function()
        local s = M.new_scalar(3.0)
        local A = M.new_2d({{2, 4}, {6, 8}})
        -- 1×1 · 2×2 is not compatible — we scale instead.
        -- But 2×2 · 1×1... also not compatible.  Test 1×1 · 1×1:
        local a = M.new_scalar(5.0)
        local b = M.new_scalar(7.0)
        local C, err = M.dot(a, b)
        assert.is_nil(err)
        assert.is_true(near(C.data[1][1], 35.0))
    end)
end)

-- ============================================================================
-- Combined / property tests
-- ============================================================================

describe("combined properties", function()
    -- (A + B)ᵀ == Aᵀ + Bᵀ
    it("transpose distributes over addition", function()
        local A = M.new_2d({{1, 2}, {3, 4}})
        local B = M.new_2d({{5, 6}, {7, 8}})
        local AB, _ = M.add(A, B)
        local lhs   = M.transpose(AB)
        local rhs_A = M.transpose(A)
        local rhs_B = M.transpose(B)
        local rhs, _ = M.add(rhs_A, rhs_B)
        assert.is_true(mat_equal(lhs, rhs))
    end)

    -- scale(A, s) == add_scalar(zeros, 0) ... or just check distributivity:
    -- scale(A+B, s) == scale(A, s) + scale(B, s)
    it("scale distributes over addition", function()
        local A    = M.new_2d({{1, 2}, {3, 4}})
        local B    = M.new_2d({{5, 6}, {7, 8}})
        local s    = 3.0
        local AB, _ = M.add(A, B)
        local lhs   = M.scale(AB, s)
        local rhs_A = M.scale(A, s)
        local rhs_B = M.scale(B, s)
        local rhs, _ = M.add(rhs_A, rhs_B)
        assert.is_true(mat_equal(lhs, rhs))
    end)

    -- subtract is equivalent to adding the negated matrix
    it("A - B == A + scale(B, -1)", function()
        local A    = M.new_2d({{10, 20}, {30, 40}})
        local B    = M.new_2d({{1,  2},  {3,  4}})
        local sub, _ = M.subtract(A, B)
        local negB   = M.scale(B, -1.0)
        local add, _ = M.add(A, negB)
        assert.is_true(mat_equal(sub, add))
    end)
end)
