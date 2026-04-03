-- =============================================================================
-- coding_adventures.affine2d — 2D Affine Transformation Matrix
-- =============================================================================
--
-- Stored as a table {a, b, c, d, e, f} matching SVG matrix(a,b,c,d,e,f):
--
--   [ a  c  e ]
--   [ b  d  f ]
--   [ 0  0  1 ]
--
-- Mapping: x' = a*x + c*y + e
--          y' = b*x + d*y + f
-- =============================================================================

local trig = require("coding_adventures.trig")

local affine2d = {}

--- Create an affine matrix from 6 values.
local function new(a, b, c, d, e, f)
    return { a=a, b=b, c=c, d=d, e=e, f=f }
end

--- Identity transform.
function affine2d.identity()
    return new(1, 0, 0, 1, 0, 0)
end

--- Pure translation.
function affine2d.translate(tx, ty)
    return new(1, 0, 0, 1, tx, ty)
end

--- CCW rotation by angle_rad.
function affine2d.rotate(angle_rad)
    local c = trig.cos(angle_rad)
    local s = trig.sin(angle_rad)
    return new(c, s, -s, c, 0, 0)
end

--- CCW rotation about pivot (px, py).
function affine2d.rotate_around(angle_rad, px, py)
    return affine2d.compose(
        affine2d.compose(affine2d.translate(px, py), affine2d.rotate(angle_rad)),
        affine2d.translate(-px, -py))
end

--- Non-uniform scale.
function affine2d.scale(sx, sy)
    return new(sx, 0, 0, sy, 0, 0)
end

--- Uniform scale.
function affine2d.scale_uniform(s)
    return affine2d.scale(s, s)
end

--- Shear along x-axis.
function affine2d.skew_x(angle_rad)
    return new(1, 0, trig.tan(angle_rad), 1, 0, 0)
end

--- Shear along y-axis.
function affine2d.skew_y(angle_rad)
    return new(1, trig.tan(angle_rad), 0, 1, 0, 0)
end

--- Compose: apply `a` first, then `b`.
function affine2d.compose(a, b)
    return new(
        a.a*b.a + a.c*b.b,
        a.b*b.a + a.d*b.b,
        a.a*b.c + a.c*b.d,
        a.b*b.c + a.d*b.d,
        a.a*b.e + a.c*b.f + a.e,
        a.b*b.e + a.d*b.f + a.f
    )
end

--- Apply to a position point (includes translation).
function affine2d.apply_to_point(m, pt)
    return { x = m.a*pt.x + m.c*pt.y + m.e, y = m.b*pt.x + m.d*pt.y + m.f }
end

--- Apply to a direction vector (excludes translation).
function affine2d.apply_to_vector(m, v)
    return { x = m.a*v.x + m.c*v.y, y = m.b*v.x + m.d*v.y }
end

--- Determinant: ad - bc.
function affine2d.determinant(m)
    return m.a * m.d - m.b * m.c
end

--- Inverse, or nil if singular.
function affine2d.invert(m)
    local det = affine2d.determinant(m)
    if math.abs(det) < 1e-12 then return nil end
    local inv = 1.0 / det
    return new(
        m.d * inv, -m.b * inv,
        -m.c * inv, m.a * inv,
        (m.c*m.f - m.d*m.e) * inv,
        (m.b*m.e - m.a*m.f) * inv
    )
end

--- True if identity.
function affine2d.is_identity(m)
    return m.a==1 and m.b==0 and m.c==0 and m.d==1 and m.e==0 and m.f==0
end

--- True if translation-only.
function affine2d.is_translation_only(m)
    return m.a==1 and m.b==0 and m.c==0 and m.d==1
end

--- Return {a, b, c, d, e, f} as an array.
function affine2d.to_array(m)
    return {m.a, m.b, m.c, m.d, m.e, m.f}
end

return affine2d
