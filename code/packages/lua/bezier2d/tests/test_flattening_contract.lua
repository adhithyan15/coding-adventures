local bz = require("coding_adventures.bezier2d")
local p2d = require("coding_adventures.point2d")
local json = require("dkjson")

local fixture_file = assert(io.open("../../../../specs/fixtures/bezier2d-flattening-v1/cases.json", "rb"))
local fixture = assert(json.decode(fixture_file:read("*a")))
fixture_file:close()

local function curve_for(case)
    local points = {}
    for _, coordinates in ipairs(case.control_points) do
        points[#points + 1] = p2d.new_point(coordinates[1], coordinates[2])
    end
    if case.degree == 2 then return bz.new_quad(points[1], points[2], points[3]) end
    return bz.new_cubic(points[1], points[2], points[3], points[4])
end

local function flatten(curve, tolerance)
    if curve.kind == "quad" then return bz.polyline_quad(curve, tolerance) end
    return bz.polyline_cubic(curve, tolerance)
end

describe("G2D02 safe flattening", function()
    for _, case in ipairs(fixture.cases) do
        it(case.id, function()
            local curve = curve_for(case)
            if case.expected_disposition == "invalid-tolerance" then
                assert.has_error(function() flatten(curve, case.tolerance) end, "tolerance must be finite and positive")
                return
            end
            if case.expected_termination == "budget-error" then
                assert.has_error(function() flatten(curve, case.tolerance) end, "Bezier flattening exceeded the depth or subdivision budget")
                return
            end
            local points = flatten(curve, case.tolerance)
            assert.are.equal(curve.p0, points[1])
            assert.are.equal(case.degree == 2 and curve.p2 or curve.p3, points[#points])
            for _, point in ipairs(points) do
                assert.is_true(point.x == point.x and point.y == point.y and math.abs(point.x) < math.huge and math.abs(point.y) < math.huge)
            end
            if case.expected_disposition == "chord" then assert.are.equal(2, #points) else assert.is_true(#points > 2) end
        end)
    end

    it("rejects invalid native inputs and derived arithmetic", function()
        local straight = bz.new_quad(p2d.new_point(0, 0), p2d.new_point(0.5, 0), p2d.new_point(1, 0))
        for _, tolerance in ipairs({0, -1, math.huge, -math.huge}) do
            assert.has_error(function() bz.polyline_quad(straight, tolerance) end, "tolerance must be finite and positive")
        end
        local nonfinite = bz.new_cubic(p2d.new_point(0, 0), p2d.new_point(math.huge, 1), p2d.new_point(0, -1), p2d.new_point(0, 0))
        assert.has_error(function() bz.polyline_cubic(nonfinite, 0.1) end, "control points must be finite")
        local overflow = bz.new_quad(p2d.new_point(1e308, 0), p2d.new_point(0, 1), p2d.new_point(-1e308, 0))
        assert.has_error(function() bz.polyline_quad(overflow, 0.1) end, "Bezier subdivision produced nonfinite geometry")
    end)

    it("does not underflow subnormal geometry into a chord", function()
        local curve = bz.new_quad(p2d.new_point(0, 0), p2d.new_point(0, 1e-200), p2d.new_point(1e-200, 0))
        assert.has_error(function() bz.polyline_quad(curve, 5e-324) end, "Bezier flattening exceeded the depth or subdivision budget")
    end)
end)
