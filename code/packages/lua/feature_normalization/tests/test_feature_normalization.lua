package.path = "../src/?.lua;" .. "../src/?/init.lua;" .. package.path

local norm = require("coding_adventures.feature_normalization")

local function near(a, b, tol)
    tol = tol or 1e-9
    return math.abs(a - b) <= tol
end

local rows = {
    {1000.0, 3.0, 1.0},
    {1500.0, 4.0, 0.0},
    {2000.0, 5.0, 1.0},
}

describe("feature_normalization", function()
    it("standardizes columns", function()
        local scaler = norm.fit_standard_scaler(rows)
        assert.is_true(near(scaler.means[1], 1500.0))
        assert.is_true(near(scaler.means[2], 4.0))

        local transformed = norm.transform_standard(rows, scaler)
        assert.is_true(near(transformed[1][1], -1.224744871391589))
        assert.is_true(near(transformed[2][1], 0.0))
        assert.is_true(near(transformed[3][1], 1.224744871391589))
    end)

    it("min-max scales columns", function()
        local transformed = norm.transform_min_max(rows, norm.fit_min_max_scaler(rows))
        assert.are.equal(0.0, transformed[1][1])
        assert.are.equal(0.5, transformed[2][1])
        assert.are.equal(1.0, transformed[3][1])
        assert.are.equal(0.0, transformed[2][3])
    end)

    it("maps constant columns to zero", function()
        local constant = {{1.0, 7.0}, {2.0, 7.0}}
        assert.are.equal(0.0, norm.transform_standard(constant, norm.fit_standard_scaler(constant))[1][2])
        assert.are.equal(0.0, norm.transform_min_max(constant, norm.fit_min_max_scaler(constant))[1][2])
    end)
end)
