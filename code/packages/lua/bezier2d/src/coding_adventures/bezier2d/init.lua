-- =============================================================================
-- coding_adventures.bezier2d — Quadratic and Cubic Bezier Curves
-- =============================================================================
--
-- Curves are plain tables:
--   QuadraticBezier: { kind="quad", p0, p1, p2 }
--   CubicBezier:     { kind="cubic", p0, p1, p2, p3 }
-- Points are {x, y} tables from point2d.
-- =============================================================================

local trig = require("coding_adventures.trig")
local p2d  = require("coding_adventures.point2d")

local bezier2d = {}
local MAX_FLATTENING_DEPTH = 32
local MAX_FLATTENING_SUBDIVISIONS = 65535

local function finite(value)
    if value ~= value or math.abs(value) == math.huge then
        error("Bezier subdivision produced nonfinite geometry", 0)
    end
    return value
end

local function scaled_distance(left, right)
    local dx = finite(left.x - right.x)
    local dy = finite(left.y - right.y)
    local scale = math.max(math.abs(dx), math.abs(dy))
    if scale == 0 then return 0 end
    local nx, ny = dx / scale, dy / scale
    return finite(scale * math.sqrt(finite(nx * nx + ny * ny)))
end

local function distance_to_segment(point, start_point, end_point)
    local dx = finite(end_point.x - start_point.x)
    local dy = finite(end_point.y - start_point.y)
    local scale = math.max(math.abs(dx), math.abs(dy))
    if scale == 0 then return scaled_distance(point, start_point) end
    local nx, ny = dx / scale, dy / scale
    local denominator = finite(nx * nx + ny * ny)
    local nox = finite(finite(point.x - start_point.x) / scale)
    local noy = finite(finite(point.y - start_point.y) / scale)
    local projection = finite(finite(nox * nx + noy * ny) / denominator)
    projection = math.max(0, math.min(1, projection))
    local nearest = p2d.new_point(
        finite(start_point.x + projection * dx),
        finite(start_point.y + projection * dy))
    return scaled_distance(point, nearest)
end

local function flatten(curve, tolerance, controls)
    if type(tolerance) ~= "number" or tolerance ~= tolerance or math.abs(tolerance) == math.huge or tolerance <= 0 then
        error("tolerance must be finite and positive", 0)
    end
    for _, point in ipairs(controls) do
        if type(point.x) ~= "number" or type(point.y) ~= "number"
            or point.x ~= point.x or point.y ~= point.y
            or math.abs(point.x) == math.huge or math.abs(point.y) == math.huge then
            error("control points must be finite", 0)
        end
    end

    local points = {controls[1]}
    local pending = {{curve=curve, depth=0}}
    local subdivisions = 0
    while #pending > 0 do
        local item = table.remove(pending)
        local current = item.curve
        local endpoint = current.kind == "quad" and current.p2 or current.p3
        local off_curve = current.kind == "quad" and {current.p1} or {current.p1, current.p2}
        local flatness = 0
        for _, point in ipairs(off_curve) do
            flatness = math.max(flatness, distance_to_segment(point, current.p0, endpoint))
        end
        if flatness <= tolerance then
            points[#points + 1] = endpoint
        else
            if item.depth >= MAX_FLATTENING_DEPTH or subdivisions >= MAX_FLATTENING_SUBDIVISIONS then
                error("Bezier flattening exceeded the depth or subdivision budget", 0)
            end
            subdivisions = subdivisions + 1
            local left, right
            if current.kind == "quad" then
                left, right = bezier2d.split_quad(current, 0.5)
            else
                left, right = bezier2d.split_cubic(current, 0.5)
            end
            pending[#pending + 1] = {curve=right, depth=item.depth + 1}
            pending[#pending + 1] = {curve=left, depth=item.depth + 1}
        end
    end
    return points
end

function bezier2d.new_quad(p0, p1, p2)
    return { kind="quad", p0=p0, p1=p1, p2=p2 }
end

function bezier2d.new_cubic(p0, p1, p2, p3)
    return { kind="cubic", p0=p0, p1=p1, p2=p2, p3=p3 }
end

-- ---------------------------------------------------------------------------
-- Quadratic Bezier
-- ---------------------------------------------------------------------------

--- Evaluate at t via de Casteljau.
function bezier2d.eval_quad(q, t)
    local q0 = p2d.lerp(q.p0, q.p1, t)
    local q1 = p2d.lerp(q.p1, q.p2, t)
    return p2d.lerp(q0, q1, t)
end

--- Tangent vector at t.
function bezier2d.deriv_quad(q, t)
    local d0 = p2d.subtract(q.p1, q.p0)
    local d1 = p2d.subtract(q.p2, q.p1)
    return p2d.scale(p2d.lerp(d0, d1, t), 2)
end

--- Split into (left, right) at t.
function bezier2d.split_quad(q, t)
    local q0 = p2d.lerp(q.p0, q.p1, t)
    local q1 = p2d.lerp(q.p1, q.p2, t)
    local m  = p2d.lerp(q0, q1, t)
    return bezier2d.new_quad(q.p0, q0, m), bezier2d.new_quad(m, q1, q.p2)
end

--- Adaptive polyline within tolerance. Returns a list of {x,y} points.
function bezier2d.polyline_quad(q, tolerance)
    return flatten(q, tolerance, {q.p0, q.p1, q.p2})
end

--- Tight bounding box of a quadratic Bezier.
function bezier2d.bbox_quad(q)
    local min_x = math.min(q.p0.x, q.p2.x)
    local max_x = math.max(q.p0.x, q.p2.x)
    local min_y = math.min(q.p0.y, q.p2.y)
    local max_y = math.max(q.p0.y, q.p2.y)

    local dx = q.p0.x - 2*q.p1.x + q.p2.x
    if math.abs(dx) > 1e-12 then
        local tx = (q.p0.x - q.p1.x) / dx
        if tx > 0 and tx < 1 then
            local px = bezier2d.eval_quad(q, tx).x
            min_x = math.min(min_x, px)
            max_x = math.max(max_x, px)
        end
    end
    local dy = q.p0.y - 2*q.p1.y + q.p2.y
    if math.abs(dy) > 1e-12 then
        local ty = (q.p0.y - q.p1.y) / dy
        if ty > 0 and ty < 1 then
            local py = bezier2d.eval_quad(q, ty).y
            min_y = math.min(min_y, py)
            max_y = math.max(max_y, py)
        end
    end
    return p2d.new_rect(min_x, min_y, max_x-min_x, max_y-min_y)
end

--- Degree elevation: quadratic to equivalent cubic.
function bezier2d.elevate_quad(q)
    local q1 = p2d.add(p2d.scale(q.p0, 1/3), p2d.scale(q.p1, 2/3))
    local q2 = p2d.add(p2d.scale(q.p1, 2/3), p2d.scale(q.p2, 1/3))
    return bezier2d.new_cubic(q.p0, q1, q2, q.p2)
end

-- ---------------------------------------------------------------------------
-- Cubic Bezier
-- ---------------------------------------------------------------------------

--- Evaluate at t via de Casteljau.
function bezier2d.eval_cubic(c, t)
    local p01  = p2d.lerp(c.p0, c.p1, t)
    local p12  = p2d.lerp(c.p1, c.p2, t)
    local p23  = p2d.lerp(c.p2, c.p3, t)
    local p012 = p2d.lerp(p01, p12, t)
    local p123 = p2d.lerp(p12, p23, t)
    return p2d.lerp(p012, p123, t)
end

--- Tangent at t.
function bezier2d.deriv_cubic(c, t)
    local d0 = p2d.subtract(c.p1, c.p0)
    local d1 = p2d.subtract(c.p2, c.p1)
    local d2 = p2d.subtract(c.p3, c.p2)
    local one_t = 1 - t
    local r = p2d.add(p2d.add(
        p2d.scale(d0, one_t*one_t),
        p2d.scale(d1, 2*one_t*t)),
        p2d.scale(d2, t*t))
    return p2d.scale(r, 3)
end

--- Split cubic at t.
function bezier2d.split_cubic(c, t)
    local p01   = p2d.lerp(c.p0, c.p1, t)
    local p12   = p2d.lerp(c.p1, c.p2, t)
    local p23   = p2d.lerp(c.p2, c.p3, t)
    local p012  = p2d.lerp(p01, p12, t)
    local p123  = p2d.lerp(p12, p23, t)
    local p0123 = p2d.lerp(p012, p123, t)
    return bezier2d.new_cubic(c.p0, p01, p012, p0123),
           bezier2d.new_cubic(p0123, p123, p23, c.p3)
end

--- Adaptive polyline.
function bezier2d.polyline_cubic(c, tolerance)
    return flatten(c, tolerance, {c.p0, c.p1, c.p2, c.p3})
end

--- Tight bounding box.
function bezier2d.bbox_cubic(c)
    local min_x = math.min(c.p0.x, c.p3.x)
    local max_x = math.max(c.p0.x, c.p3.x)
    local min_y = math.min(c.p0.y, c.p3.y)
    local max_y = math.max(c.p0.y, c.p3.y)

    local function process_extrema(v0, v1, v2, v3, get_val)
        local a = -3*v0 + 9*v1 - 9*v2 + 3*v3
        local b =  6*v0 - 12*v1 + 6*v2
        local cv = -3*v0 + 3*v1
        if math.abs(a) < 1e-12 then
            if math.abs(b) > 1e-12 then
                local tx = -cv / b
                if tx > 0 and tx < 1 then get_val(tx) end
            end
        else
            local disc = b*b - 4*a*cv
            if disc >= 0 then
                local sq = trig.sqrt(disc)
                for _, tx in ipairs({(-b+sq)/(2*a), (-b-sq)/(2*a)}) do
                    if tx > 0 and tx < 1 then get_val(tx) end
                end
            end
        end
    end

    process_extrema(c.p0.x, c.p1.x, c.p2.x, c.p3.x, function(tx)
        local px = bezier2d.eval_cubic(c, tx).x
        min_x = math.min(min_x, px); max_x = math.max(max_x, px)
    end)
    process_extrema(c.p0.y, c.p1.y, c.p2.y, c.p3.y, function(ty)
        local py = bezier2d.eval_cubic(c, ty).y
        min_y = math.min(min_y, py); max_y = math.max(max_y, py)
    end)

    return p2d.new_rect(min_x, min_y, max_x-min_x, max_y-min_y)
end

return bezier2d
