-- =============================================================================
-- coding_adventures.arc2d — Elliptical Arc (Center Form and SVG Endpoint Form)
-- =============================================================================
--
-- CenterArc: { kind="center_arc", center, rx, ry, start_angle, sweep_angle, x_rotation }
-- SvgArc:    { kind="svg_arc", from_pt, to_pt, rx, ry, x_rotation, large_arc, sweep }
-- =============================================================================

local trig = require("coding_adventures.trig")
local p2d  = require("coding_adventures.point2d")
local bz   = require("coding_adventures.bezier2d")

local arc2d = {}

--- Create a CenterArc.
function arc2d.new_center_arc(center, rx, ry, start_angle, sweep_angle, x_rotation)
    return {
        kind="center_arc", center=center,
        rx=rx, ry=ry,
        start_angle=start_angle, sweep_angle=sweep_angle,
        x_rotation=x_rotation
    }
end

--- Create an SvgArc.
function arc2d.new_svg_arc(from_pt, to_pt, rx, ry, x_rotation, large_arc, sweep)
    return {
        kind="svg_arc",
        from_pt=from_pt, to_pt=to_pt,
        rx=rx, ry=ry, x_rotation=x_rotation,
        large_arc=large_arc, sweep=sweep
    }
end

-- ---------------------------------------------------------------------------
-- CenterArc
-- ---------------------------------------------------------------------------

--- Evaluate at t ∈ [0,1].
function arc2d.eval_arc(a, t)
    local theta = a.start_angle + t * a.sweep_angle
    local cos_t = trig.cos(theta)
    local sin_t = trig.sin(theta)
    local lx = a.rx * cos_t
    local ly = a.ry * sin_t
    local cos_r = trig.cos(a.x_rotation)
    local sin_r = trig.sin(a.x_rotation)
    return p2d.new_point(
        a.center.x + cos_r*lx - sin_r*ly,
        a.center.y + sin_r*lx + cos_r*ly
    )
end

--- Tangent direction at t (unnormalized).
function arc2d.tangent_arc(a, t)
    local theta = a.start_angle + t * a.sweep_angle
    local cos_t = trig.cos(theta)
    local sin_t = trig.sin(theta)
    local cos_r = trig.cos(a.x_rotation)
    local sin_r = trig.sin(a.x_rotation)
    local dlx = -a.rx * sin_t
    local dly =  a.ry * cos_t
    return p2d.new_point(
        a.sweep_angle * (cos_r*dlx - sin_r*dly),
        a.sweep_angle * (sin_r*dlx + cos_r*dly)
    )
end

--- Bounding box via 100-point sampling.
function arc2d.bbox_arc(a)
    local p0 = arc2d.eval_arc(a, 0)
    local min_x, max_x = p0.x, p0.x
    local min_y, max_y = p0.y, p0.y
    for i = 1, 100 do
        local p = arc2d.eval_arc(a, i / 100.0)
        min_x = math.min(min_x, p.x); max_x = math.max(max_x, p.x)
        min_y = math.min(min_y, p.y); max_y = math.max(max_y, p.y)
    end
    return p2d.new_rect(min_x, min_y, max_x-min_x, max_y-min_y)
end

--- Approximate with cubic Bezier segments (≤ π/2 each).
function arc2d.to_cubic_beziers(a)
    local half_pi = trig.PI / 2
    local n_seg = math.max(1, math.ceil(math.abs(a.sweep_angle) / half_pi))
    local seg_sweep = a.sweep_angle / n_seg
    local cos_r = trig.cos(a.x_rotation)
    local sin_r = trig.sin(a.x_rotation)
    local cx, cy, rx, ry = a.center.x, a.center.y, a.rx, a.ry

    local function l2w(lx, ly)
        return p2d.new_point(cx + cos_r*lx - sin_r*ly, cy + sin_r*lx + cos_r*ly)
    end

    local curves = {}
    for i = 0, n_seg-1 do
        local t0 = a.start_angle + i * seg_sweep
        local t1 = t0 + seg_sweep
        local k  = (4.0/3.0) * trig.tan(seg_sweep / 4)
        local cos0, sin0 = trig.cos(t0), trig.sin(t0)
        local cos1, sin1 = trig.cos(t1), trig.sin(t1)
        local p0 = l2w(rx*cos0, ry*sin0)
        local p3 = l2w(rx*cos1, ry*sin1)
        local p1 = l2w(rx*cos0 - k*rx*sin0, ry*sin0 + k*ry*cos0)
        local p2 = l2w(rx*cos1 + k*rx*sin1, ry*sin1 - k*ry*cos1)
        curves[#curves+1] = bz.new_cubic(p0, p1, p2, p3)
    end
    return curves
end

-- ---------------------------------------------------------------------------
-- SvgArc
-- ---------------------------------------------------------------------------

local function angle_between(ux, uy, vx, vy)
    local dot   = ux*vx + uy*vy
    local mag_u = trig.sqrt(ux*ux + uy*uy)
    local mag_v = trig.sqrt(vx*vx + vy*vy)
    if mag_u < 1e-12 or mag_v < 1e-12 then return 0 end
    local cos_a = math.max(-1, math.min(1, dot / (mag_u * mag_v)))
    local sin_a = trig.sqrt(1 - cos_a*cos_a)
    local angle = trig.atan2(sin_a, cos_a)
    if ux*vy - uy*vx < 0 then angle = -angle end
    return angle
end

--- Convert SvgArc to CenterArc. Returns nil if degenerate.
function arc2d.to_center_arc(s)
    if s.from_pt.x == s.to_pt.x and s.from_pt.y == s.to_pt.y then return nil end
    local rx = math.abs(s.rx)
    local ry = math.abs(s.ry)
    if rx < 1e-12 or ry < 1e-12 then return nil end

    local cos_r = trig.cos(s.x_rotation)
    local sin_r = trig.sin(s.x_rotation)
    local dx2 = (s.from_pt.x - s.to_pt.x) / 2
    local dy2 = (s.from_pt.y - s.to_pt.y) / 2
    local x1p =  cos_r*dx2 + sin_r*dy2
    local y1p = -sin_r*dx2 + cos_r*dy2

    local lam = (x1p/rx)^2 + (y1p/ry)^2
    if lam > 1 then
        lam = trig.sqrt(lam)
        rx = rx * lam
        ry = ry * lam
    end

    local rx2, ry2 = rx*rx, ry*ry
    local x1p2, y1p2 = x1p*x1p, y1p*y1p
    local num = rx2*ry2 - rx2*y1p2 - ry2*x1p2
    local den = rx2*y1p2 + ry2*x1p2
    if den < 1e-24 then return nil end

    local sq_val = num/den > 0 and trig.sqrt(num/den) or 0
    -- XOR: large_arc ~= sweep → positive
    local sq = (s.large_arc ~= s.sweep) and sq_val or -sq_val

    local cxp =  sq * rx * y1p / ry
    local cyp = -sq * ry * x1p / rx
    local mx = (s.from_pt.x + s.to_pt.x) / 2
    local my = (s.from_pt.y + s.to_pt.y) / 2
    local center_x = cos_r*cxp - sin_r*cyp + mx
    local center_y = sin_r*cxp + cos_r*cyp + my

    local ux = (x1p - cxp) / rx
    local uy = (y1p - cyp) / ry
    local vx = (-x1p - cxp) / rx
    local vy = (-y1p - cyp) / ry
    local start_angle = trig.atan2(uy, ux)
    local sweep_angle = angle_between(ux, uy, vx, vy)

    if not s.sweep and sweep_angle > 0 then
        sweep_angle = sweep_angle - trig.TWO_PI
    elseif s.sweep and sweep_angle < 0 then
        sweep_angle = sweep_angle + trig.TWO_PI
    end

    return arc2d.new_center_arc(
        p2d.new_point(center_x, center_y),
        rx, ry, start_angle, sweep_angle, s.x_rotation
    )
end

return arc2d
