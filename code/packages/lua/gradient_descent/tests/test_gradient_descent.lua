local gd = require("src.gradient_descent")

describe("gradient_descent", function()
  it("sgd", function()
    local res = gd.sgd({1.0, 2.0}, {0.1, 0.2}, 0.5)
    assert.is_true(math.abs(res[1] - 0.95) < 0.0001)
    assert.is_true(math.abs(res[2] - 1.9) < 0.0001)
  end)
end)
