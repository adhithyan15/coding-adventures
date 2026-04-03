-- =============================================================================
-- coding_adventures.point2d — Immutable 2D Point/Vector and Rect
-- =============================================================================
--
-- A Point is {x, y} — used as both a position and a 2D direction vector.
-- A Rect  is {x, y, width, height} — an axis-aligned bounding box.
--
-- All operations return new tables (pure functional style).
-- =============================================================================

local trig = require("coding_adventures.trig")

local point2d = {}

-- ---------------------------------------------------------------------------
-- Point
-- ---------------------------------------------------------------------------

--- Create a new Point.
function point2d.new_point(x, y)
    return { x = x, y = y }
end

local P = point2d

--- Add two points: (x1+x2, y1+y2).
function point2d.add(a, b)
    return P.new_point(a.x + b.x, a.y + b.y)
end

--- Subtract: a - b.
function point2d.subtract(a, b)
    return P.new_point(a.x - b.x, a.y - b.y)
end

--- Scale by scalar s.
function point2d.scale(a, s)
    return P.new_point(a.x * s, a.y * s)
end

--- Negate: (-x, -y).
function point2d.negate(a)
    return P.new_point(-a.x, -a.y)
end

--- Dot product: x1*x2 + y1*y2.
function point2d.dot(a, b)
    return a.x * b.x + a.y * b.y
end

--- 2D cross product: x1*y2 - y1*x2.
function point2d.cross(a, b)
    return a.x * b.y - a.y * b.x
end

--- Squared magnitude: x^2 + y^2.
function point2d.magnitude_squared(a)
    return a.x * a.x + a.y * a.y
end

--- Euclidean magnitude: sqrt(x^2 + y^2).
function point2d.magnitude(a)
    return trig.sqrt(P.magnitude_squared(a))
end

--- Normalize to unit length. Returns a unchanged if magnitude is zero.
function point2d.normalize(a)
    local m = P.magnitude(a)
    if m < 1e-15 then return a end
    return P.scale(a, 1.0 / m)
end

--- Squared distance between two points.
function point2d.distance_squared(a, b)
    return P.magnitude_squared(P.subtract(a, b))
end

--- Euclidean distance between two points.
function point2d.distance(a, b)
    return trig.sqrt(P.distance_squared(a, b))
end

--- Linear interpolation from a to b at parameter t.
function point2d.lerp(a, b, t)
    return P.new_point(a.x + t * (b.x - a.x), a.y + t * (b.y - a.y))
end

--- Perpendicular vector (90° CCW): (-y, x).
function point2d.perpendicular(a)
    return P.new_point(-a.y, a.x)
end

--- Angle of vector from +X axis, in radians.
function point2d.angle(a)
    return trig.atan2(a.y, a.x)
end

-- ---------------------------------------------------------------------------
-- Rect
-- ---------------------------------------------------------------------------

--- Create a new Rect.
function point2d.new_rect(x, y, w, h)
    return { x = x, y = y, width = w, height = h }
end

--- True if pt is inside the half-open rect [x, x+w) × [y, y+h).
function point2d.contains_point(r, pt)
    return pt.x >= r.x and pt.x < r.x + r.width
       and pt.y >= r.y and pt.y < r.y + r.height
end

--- Smallest Rect containing both r1 and r2.
function point2d.rect_union(r1, r2)
    local x0 = math.min(r1.x, r2.x)
    local y0 = math.min(r1.y, r2.y)
    local x1 = math.max(r1.x + r1.width,  r2.x + r2.width)
    local y1 = math.max(r1.y + r1.height, r2.y + r2.height)
    return P.new_rect(x0, y0, x1 - x0, y1 - y0)
end

--- Intersection of two rects, or nil if disjoint.
function point2d.rect_intersection(r1, r2)
    local x0 = math.max(r1.x, r2.x)
    local y0 = math.max(r1.y, r2.y)
    local x1 = math.min(r1.x + r1.width,  r2.x + r2.width)
    local y1 = math.min(r1.y + r1.height, r2.y + r2.height)
    if x1 <= x0 or y1 <= y0 then return nil end
    return P.new_rect(x0, y0, x1 - x0, y1 - y0)
end

--- Expand rect by margin on all four sides.
function point2d.rect_expand(r, margin)
    return P.new_rect(r.x - margin, r.y - margin,
        r.width + 2 * margin, r.height + 2 * margin)
end

return point2d
