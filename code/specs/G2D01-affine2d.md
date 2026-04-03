# G2D01 — Affine2D: The 2D Affine Transformation Matrix

## Overview

The `affine2d` package provides the canonical 2D affine transformation matrix
used by every major 2D graphics system on the planet. A single compact
representation — six floating-point numbers — encodes any combination of
translation, rotation, uniform and non-uniform scaling, and shearing in a
composable, invertible form.

Dependencies:

- `point2d` (G2D00) — Point and Rect types for input/output
- `trig` (PHY00) — `sin`, `cos`, `tan` for rotation and skew factories

```
affine2d (G2D01)
├── point2d (G2D00)
└── trig (PHY00)
```

---

## Why Affine Transforms?

A 2D graphics system needs to answer one question over and over:

> Given a shape defined in _local_ coordinates (e.g., the vertices of a glyph
> in font design units), where does it appear in _screen_ coordinates after
> scaling to the requested font size, rotating, and positioning on the page?

Naive answer: write a separate function for each transform: one for translation,
one for rotation, one for scale, then call them in sequence. This works but
is fragile — you must remember the exact call order, and combining two such
sequences requires tracking an ever-growing list of individual transforms.

Better answer: represent _any_ transform as a matrix. The composition of two
transforms is just matrix multiplication — one operation, regardless of how
complex each transform is. A rendering pipeline accumulates transforms by
multiplying matrices; applying a transform to a point is a single matrix–vector
multiply.

---

## Homogeneous Coordinates

To unify translation (which is an _addition_) with rotation and scaling (which
are _multiplications_), we embed 2D points in 3D using **homogeneous
coordinates**. A 2D point $(x, y)$ is represented as the 3D column vector:

$$\begin{pmatrix} x \\ y \\ 1 \end{pmatrix}$$

The extra $1$ in the third component makes translation expressible as
multiplication. A general affine transform is then a 3×3 matrix:

$$\mathbf{M} = \begin{pmatrix} a & c & e \\ b & d & f \\ 0 & 0 & 1 \end{pmatrix}$$

Applying the transform to a point:

$$\mathbf{M} \begin{pmatrix} x \\ y \\ 1 \end{pmatrix} = \begin{pmatrix} ax + cy + e \\ bx + dy + f \\ 1 \end{pmatrix}$$

The third row of the result is always $1$, confirming the output is a valid 2D
point. The output x-coordinate is $ax + cy + e$; the output y-coordinate is
$bx + dy + f$.

### Why the third row is always [0, 0, 1]

An **affine transform** is one that preserves:
- straight lines (lines remain lines)
- parallelism (parallel lines remain parallel)
- ratios of distances along a line

The condition "third row = [0, 0, 1]" is exactly what distinguishes affine from
_projective_ transforms (which have non-zero values in the third row and can
map parallel lines to converging lines — the so-called perspective projection
used in 3D graphics).

Since the third row never changes for 2D affine transforms, we never need to
store it. This gives us the **6-float representation**:

$$[a, \; b, \; c, \; d, \; e, \; f]$$

This is the representation used by:

| API | Field names | Our names |
|-----|-------------|-----------|
| SVG `matrix()` | `matrix(a,b,c,d,e,f)` | a, b, c, d, e, f |
| HTML Canvas `setTransform` | `setTransform(a,b,c,d,e,f)` | a, b, c, d, e, f |
| PDF `cm` operator | `a b c d e f cm` | a, b, c, d, e, f |
| Cairo `cairo_matrix_t` | `xx, yx, xy, yy, x0, y0` | a=xx, b=yx, c=xy, d=yy, e=x0, f=y0 |
| Core Graphics `CGAffineTransform` | `a, b, c, d, tx, ty` | a, b, c, d, e=tx, f=ty |
| Direct2D `Matrix3x2F` | `_11, _12, _21, _22, _31, _32` | a=_11, b=_12, c=_21, d=_22, e=_31, f=_32 |

All six are the same six numbers — just different field names. Learning this
representation once means reading any 2D graphics API immediately.

---

## The Full 3×3 Multiplication

For two transforms $\mathbf{M}$ and $\mathbf{N}$:

$$\mathbf{M} = \begin{pmatrix} a_1 & c_1 & e_1 \\ b_1 & d_1 & f_1 \\ 0 & 0 & 1 \end{pmatrix} \qquad \mathbf{N} = \begin{pmatrix} a_2 & c_2 & e_2 \\ b_2 & d_2 & f_2 \\ 0 & 0 & 1 \end{pmatrix}$$

The product $\mathbf{M} \cdot \mathbf{N}$ is:

$$\begin{pmatrix}
a_1 a_2 + c_1 b_2 & a_1 c_2 + c_1 d_2 & a_1 e_2 + c_1 f_2 + e_1 \\
b_1 a_2 + d_1 b_2 & b_1 c_2 + d_1 d_2 & b_1 e_2 + d_1 f_2 + f_1 \\
0 & 0 & 1
\end{pmatrix}$$

Reading off the six resulting components:

$$a' = a_1 a_2 + c_1 b_2$$
$$b' = b_1 a_2 + d_1 b_2$$
$$c' = a_1 c_2 + c_1 d_2$$
$$d' = b_1 c_2 + d_1 d_2$$
$$e' = a_1 e_2 + c_1 f_2 + e_1$$
$$f' = b_1 e_2 + d_1 f_2 + f_1$$

These six formulas are the complete implementation of `multiply`. Notice:
- The top-left 2×2 block ($a, b, c, d$) multiplies as a regular 2×2 matrix.
- The translation components ($e, f$) transform the translation of $\mathbf{N}$
  through the linear part of $\mathbf{M}$, then add $\mathbf{M}$'s own
  translation.

---

## Transform Composition Order

**Critical concept**: matrix multiplication is **not commutative**.
$\mathbf{M} \cdot \mathbf{N} \ne \mathbf{N} \cdot \mathbf{M}$ in general.

When we write `M.multiply(N)`, the transform $\mathbf{N}$ is applied _first_
to the point, then $\mathbf{M}$ is applied. This is the standard mathematical
convention (right-to-left application order):

$$\text{result} = \mathbf{M} \cdot \mathbf{N} \cdot \mathbf{p} = \mathbf{M} \cdot (\mathbf{N} \cdot \mathbf{p})$$

**Worked example — non-commutativity**:

```
T = translate(10, 0)    // shift right by 10
R = rotate(π/2)         // rotate 90° CCW

Point p = (1, 0)

T.multiply(R).apply_to_point(p):
  1. Apply R first: (1,0) → (0, 1)   [rotated 90°]
  2. Apply T next:  (0,1) → (10, 1)  [shifted right]
  Result: (10, 1)

R.multiply(T).apply_to_point(p):
  1. Apply T first: (1,0) → (11, 0)  [shifted right]
  2. Apply R next:  (11,0) → (0, 11) [rotated 90°]
  Result: (0, 11)
```

Same transforms, different order, different results. Always think: _what should
happen first to the point?_ The last transform in the chain (`multiply` chain)
acts first on the point.

---

## API Reference

### Factory Functions

All factory functions return an `Affine2D` value type. They do not mutate;
to combine transforms, use `multiply`.

```
identity() → Affine2D
```
The do-nothing transform. Every point maps to itself.

$$\begin{pmatrix} 1 & 0 & 0 \\ 0 & 1 & 0 \\ 0 & 0 & 1 \end{pmatrix} \quad \Longrightarrow \quad [a=1, b=0, c=0, d=1, e=0, f=0]$$

```
translate(tx: f64, ty: f64) → Affine2D
```
Shift every point right by `tx` and down by `ty`:

$$[a=1, b=0, c=0, d=1, e=tx, f=ty]$$

Applied to $(x, y)$: $x' = x + tx$, $y' = y + ty$.

```
rotate(angle: f64) → Affine2D
```
Rotate counterclockwise about the origin by `angle` radians.

**Derivation**: a unit vector at angle $\theta$ from the X-axis is
$(\cos\theta, \sin\theta)$. After rotating by $\phi$, it becomes
$(\cos(\theta+\phi), \sin(\theta+\phi))$. Expanding with addition formulas:

$$\cos(\theta + \phi) = \cos\theta\cos\phi - \sin\theta\sin\phi$$
$$\sin(\theta + \phi) = \sin\theta\cos\phi + \cos\theta\sin\phi$$

Reading off the matrix (each column is where a basis vector lands):
- X basis $(1,0)$ goes to $(\cos\phi, \sin\phi)$ → first column
- Y basis $(0,1)$ goes to $(-\sin\phi, \cos\phi)$ → second column

$$\begin{pmatrix} \cos\phi & -\sin\phi & 0 \\ \sin\phi & \cos\phi & 0 \\ 0 & 0 & 1 \end{pmatrix}$$

In our 6-float representation:

$$[a=\cos\phi, \; b=\sin\phi, \; c=-\sin\phi, \; d=\cos\phi, \; e=0, \; f=0]$$

Must use `trig.cos(angle)` and `trig.sin(angle)` from PHY00.

**Worked example** — rotate 90° CCW ($\phi = \pi/2$, $\cos = 0$, $\sin = 1$):

```
[a=0, b=1, c=-1, d=0, e=0, f=0]
Point (1, 0) → x' = 0*1 + (-1)*0 + 0 = 0
               y' = 1*1 +    0*0 + 0 = 1
→ (0, 1)   ✓ (right → up, as expected for 90° CCW)
```

```
rotate_around(center: Point, angle: f64) → Affine2D
```
Rotate about an arbitrary center point rather than the origin.

The trick: translate so `center` moves to the origin, rotate, then translate
back.

$$R_\text{around} = \text{translate}(c_x, c_y) \cdot \text{rotate}(\phi) \cdot \text{translate}(-c_x, -c_y)$$

Apply right-to-left to a point $p$:
1. `translate(-cx, -cy)`: moves center to origin; $p$ shifts by $(-c_x, -c_y)$.
2. `rotate(φ)`: rotates the shifted point around the origin.
3. `translate(cx, cy)`: moves the origin back to center.

Implementation: compose the three matrices using `multiply`:

```
translate(cx, cy).multiply(rotate(angle)).multiply(translate(-cx, -cy))
```

```
scale(sx: f64, sy: f64) → Affine2D
```
Non-uniform scaling. X coordinates multiply by `sx`, Y by `sy`.

$$[a=sx, \; b=0, \; c=0, \; d=sy, \; e=0, \; f=0]$$

If `sx != sy`, circles become ellipses. If either is negative, the coordinate
is reflected across the corresponding axis. If `sx == sy`, use `scale_uniform`
instead for clarity.

```
scale_uniform(s: f64) → Affine2D
```
Uniform scaling: `scale(s, s)`. Circles remain circles; angles are preserved.

```
skew_x(angle: f64) → Affine2D
```
Horizontal shear. Moves points horizontally in proportion to their Y distance
from the X axis. The formula uses `trig.tan(angle)`:

$$[a=1, \; b=0, \; c=\tan(\text{angle}), \; d=1, \; e=0, \; f=0]$$

Applied to $(x, y)$: $x' = x + y \cdot \tan(\text{angle})$, $y' = y$.

Geometrically: a square becomes a parallelogram. The Y axis is tilted while
the X axis stays fixed.

```
skew_y(angle: f64) → Affine2D
```
Vertical shear. Moves points vertically in proportion to their X coordinate:

$$[a=1, \; b=\tan(\text{angle}), \; c=0, \; d=1, \; e=0, \; f=0]$$

### Core Operations

```
multiply(self, other: Affine2D) → Affine2D
```
Compose two transforms. `self.multiply(other)` applies `other` first, then
`self`. The six scalar formulas (derived above):

```
a' = self.a * other.a + self.c * other.b
b' = self.b * other.a + self.d * other.b
c' = self.a * other.c + self.c * other.d
d' = self.b * other.c + self.d * other.d
e' = self.a * other.e + self.c * other.f + self.e
f' = self.b * other.e + self.d * other.f + self.f
```

This is the hot path in any rendering pipeline. Keep it branchless and
inline-friendly.

```
apply_to_point(self, p: Point) → Point
```
Transform a position. Applies the full affine transform including translation:

$$x' = a \cdot p.x + c \cdot p.y + e$$
$$y' = b \cdot p.x + d \cdot p.y + f$$

```
apply_to_vector(self, v: Point) → Point
```
Transform a direction vector. Applies only the _linear_ part — the 2×2 block
— ignoring the translation $(e, f)$.

$$x' = a \cdot v.x + c \cdot v.y$$
$$y' = b \cdot v.x + d \cdot v.y$$

**Why the difference?** A position moves when you translate the coordinate
system. A vector (e.g., a surface normal, a velocity, a curve tangent) does
not — it represents a direction, which is translation-invariant. Using
`apply_to_point` on a vector would incorrectly shift it, producing wrong normals
and wrong tangent directions.

```
determinant(self) → f64
```
$\det = a \cdot d - b \cdot c$

The determinant of the 2×2 linear part. It measures:
- **Signed area scaling factor**: a unit square is scaled to an area of
  $|\det|$. If $\det = 2$, areas double. If $\det = -1$, the transform is
  area-preserving and orientation-reversing (a reflection).
- **Invertibility**: if $\det \approx 0$, the transform collapses 2D space into
  a line (or a point) and has no inverse.

```
invert(self) → Option<Affine2D>
```
The inverse transform: the unique $\mathbf{M}^{-1}$ such that
$\mathbf{M} \cdot \mathbf{M}^{-1} = \mathbf{I}$.

Returns `None` if $|\det| < \epsilon$ (numerically singular). Use
$\epsilon = 10^{-12}$ as the threshold.

**Derivation** — the inverse of the 3×3 affine matrix:

$$\mathbf{M}^{-1} = \frac{1}{\det} \begin{pmatrix} d & -c & cf - de \\ -b & a & be - af \\ 0 & 0 & \det \end{pmatrix}$$

In 6-float form:

```
inv_det = 1.0 / det
a' =  d * inv_det
b' = -b * inv_det
c' = -c * inv_det
d' =  a * inv_det
e' = (c*f - d*e) * inv_det
f' = (b*e - a*f) * inv_det
```

**Worked example** — inverse of translate(3, 5):

```
M = [1, 0, 0, 1, 3, 5]
det = 1*1 - 0*0 = 1
e' = (0*5 - 1*3) / 1 = -3
f' = (0*3 - 1*5) / 1 = -5
M⁻¹ = [1, 0, 0, 1, -3, -5]   // translate(-3, -5)  ✓
```

```
is_identity(self) → bool
```
True if the matrix is within floating-point epsilon of the identity. Use
$\epsilon = 10^{-10}$ for each component:

```
|a - 1| < ε  and  |b| < ε  and  |c| < ε
and  |d - 1| < ε  and  |e| < ε  and  |f| < ε
```

```
is_translation_only(self) → bool
```
True if $|b| < \epsilon$ and $|c| < \epsilon$ and $|a - 1| < \epsilon$ and
$|d - 1| < \epsilon$. The matrix has no rotation, scale, or shear — only a
pure shift. Useful as a fast path in renderers that handle translation cheaply.

```
to_array(self) → [f64; 6]
```
Return `[a, b, c, d, e, f]` as a flat array for passing to graphics APIs.
The field order matches SVG `matrix(a,b,c,d,e,f)` and Canvas `setTransform`.

---

## Practical Composition Patterns

### Object-space to screen-space

A common pattern in a rendering system:

```
// Given: object is defined in a unit square [0,1]×[0,1]
// Desired: appear at screen position (px, py), size (w, h), rotated by θ

transform =
  translate(px, py)
    .multiply(rotate(θ))
    .multiply(scale(w, h))

// Apply right-to-left to a vertex (vx, vy):
// 1. scale(w, h):     (vx*w, vy*h)
// 2. rotate(θ):       rotate the scaled point
// 3. translate(px,py): shift to screen position
```

### Accumulated parent-child transform

In a scene graph, each node has a local transform. The world transform is the
product of all ancestor transforms:

```
world = root.transform
          .multiply(child.transform)
          .multiply(grandchild.transform)
```

Each `.multiply()` concatenates one more level. Applying `world` to a local
vertex gives its world position in a single matrix–vector multiply.

---

## Cross-Language Implementation Notes

### Floating-point epsilon

Use $\epsilon = 10^{-10}$ for `is_identity` and `is_translation_only`.
Use $\epsilon = 10^{-12}$ for singular matrix detection in `invert`.

### trig dependency

`rotate` requires `trig.cos` and `trig.sin` from PHY00.
`skew_x` and `skew_y` require `trig.tan` from PHY00.
PHY00 must expose `tan(x) = sin(x) / cos(x)` (or implement it via the Taylor
series for `sin`/`cos`). Check whether PHY00 currently exports `tan`; if not,
the `affine2d` package may implement it locally as `sin/cos`.

### Value type

`Affine2D` must be a value type (struct, not a heap object) in every language
where that is practical. In Rust it derives `Copy`. In Go it is a plain struct.
In TypeScript it can be a plain object (no mutation, so references are safe).
In Elixir it is a plain map or struct. In Lua it is a plain table.

---

## Required Test Coverage

1. `identity().apply_to_point(p) == p` for any p.
2. `translate(3,4).apply_to_point(Point::new(1,1)) == Point::new(4,5)`.
3. `rotate(π/2).apply_to_point(Point::new(1,0)) ≈ Point::new(0,1)`.
4. `rotate_around(Point::new(5,5), π/2).apply_to_point(Point::new(6,5)) ≈ Point::new(5,6)`.
5. `scale(2,3).apply_to_point(Point::new(1,1)) == Point::new(2,3)`.
6. `translate(1,0).multiply(rotate(π/2))` ≠ `rotate(π/2).multiply(translate(1,0))` (non-commutativity).
7. `M.multiply(M.invert()) ≈ identity()` for a non-singular M.
8. Scale-zero matrix: `scale(0,1).invert() == None`.
9. `translate(5,3).determinant() == 1.0`.
10. `scale(2,3).determinant() == 6.0`.
11. `rotate(θ).determinant() ≈ 1.0` for any θ (rotation preserves area).
12. `apply_to_vector` ignores translation: `translate(99,99).apply_to_vector(v) == v`.
13. `to_array()` returns `[a,b,c,d,e,f]` in the correct order.
14. `is_identity()` true for `identity()`, false for `translate(0.001, 0)`.
15. `is_translation_only()` true for `translate(5,3)`, false for `rotate(0.1)`.

Coverage threshold: ≥ 95% lines.

---

## Package Matrix

| Language   | Directory                                      | Module/Namespace                         |
|------------|------------------------------------------------|------------------------------------------|
| Rust       | `code/packages/rust/affine2d/`                 | `affine2d`                               |
| TypeScript | `code/packages/typescript/affine2d/`           | `@coding-adventures/affine2d`            |
| Python     | `code/packages/python/affine2d/`               | `affine2d`                               |
| Ruby       | `code/packages/ruby/affine2d/`                 | `CodingAdventures::Affine2D`             |
| Go         | `code/packages/go/affine2d/`                   | `affine2d`                               |
| Elixir     | `code/packages/elixir/affine2d/`               | `CodingAdventures.Affine2D`              |
| Lua        | `code/packages/lua/affine2d/`                  | `coding_adventures.affine2d`             |
| Perl       | `code/packages/perl/affine2d/`                 | `CodingAdventures::Affine2D`             |
| Swift      | `code/packages/swift/affine2d/`                | `Affine2D`                               |
