// ============================================================================
// @coding-adventures/point2d — 2D Point/Vector and Axis-Aligned Bounding Box
// ============================================================================
//
// This module provides two foundational data types for 2D geometry:
//
//   - `Point` — a 2D position AND a 2D vector. A position ("where is
//     something?") and a direction+magnitude ("how far and which way?") are
//     both described by exactly two real numbers (x, y). Treating them as the
//     same type eliminates entire classes of representation-mismatch bugs.
//
//   - `Rect` — an axis-aligned bounding box (AABB), given by an origin corner
//     (x, y) and size (width, height). Used everywhere: hit-testing, dirty-
//     region tracking, clipping, conservative intersection tests.
//
// All operations produce NEW values — immutable/value-type semantics.
// This makes concurrent use safe and eliminates aliasing bugs.

import { atan2, sqrt } from "trig";

// ============================================================================
// Point
// ============================================================================

/** A 2D point (position) and 2D vector (direction + magnitude). */
export class Point {
  constructor(
    /** The horizontal coordinate. */
    public readonly x: number,
    /** The vertical coordinate. */
    public readonly y: number
  ) {}

  // -----------------------------------------------------------------------
  // Construction
  // -----------------------------------------------------------------------

  /** The point at the origin (0, 0). */
  static origin(): Point {
    return new Point(0, 0);
  }

  // -----------------------------------------------------------------------
  // Arithmetic
  // -----------------------------------------------------------------------

  /** Component-wise addition: (x1+x2, y1+y2). */
  add(other: Point): Point {
    return new Point(this.x + other.x, this.y + other.y);
  }

  /** Component-wise subtraction: (x1-x2, y1-y2). */
  subtract(other: Point): Point {
    return new Point(this.x - other.x, this.y - other.y);
  }

  /** Scalar multiplication: (s*x, s*y). */
  scale(s: number): Point {
    return new Point(this.x * s, this.y * s);
  }

  /** Additive inverse: (-x, -y). Equivalent to scale(-1). */
  negate(): Point {
    return new Point(-this.x, -this.y);
  }

  // -----------------------------------------------------------------------
  // Vector Operations
  // -----------------------------------------------------------------------

  /**
   * Dot product: x1*x2 + y1*y2.
   *
   * Encodes the angle θ between two vectors: u·v = |u||v|cos(θ).
   * Zero → perpendicular. Positive → same direction. Negative → opposite.
   */
  dot(other: Point): number {
    return this.x * other.x + this.y * other.y;
  }

  /**
   * 2D cross product (scalar): x1*y2 - y1*x2.
   *
   * Positive → other is to the LEFT of this (CCW turn).
   * Negative → other is to the RIGHT of this (CW turn).
   * Zero → collinear.
   */
  cross(other: Point): number {
    return this.x * other.y - this.y * other.x;
  }

  /**
   * Euclidean magnitude: sqrt(x²+y²).
   * Uses trig.sqrt from PHY00. Prefer magnitudeSquared() for comparisons.
   */
  magnitude(): number {
    return sqrt(this.x * this.x + this.y * this.y);
  }

  /** Squared magnitude: x²+y². No square root — cheaper for comparisons. */
  magnitudeSquared(): number {
    return this.x * this.x + this.y * this.y;
  }

  /**
   * Normalize to a unit vector (magnitude = 1).
   * Returns origin if the magnitude is zero.
   */
  normalize(): Point {
    const m = this.magnitude();
    if (m < 1e-12) return Point.origin();
    return new Point(this.x / m, this.y / m);
  }

  /** Euclidean distance to another point. */
  distance(other: Point): number {
    return this.subtract(other).magnitude();
  }

  /** Squared distance. No square root — cheaper for comparisons. */
  distanceSquared(other: Point): number {
    return this.subtract(other).magnitudeSquared();
  }

  // -----------------------------------------------------------------------
  // Interpolation and Direction
  // -----------------------------------------------------------------------

  /**
   * Linear interpolation: self + t*(other-self).
   *
   * t=0 → self; t=1 → other; t=0.5 → midpoint.
   * Values outside [0,1] extrapolate beyond the segment.
   */
  lerp(other: Point, t: number): Point {
    const dx = other.x - this.x;
    const dy = other.y - this.y;
    return new Point(this.x + t * dx, this.y + t * dy);
  }

  /**
   * Rotate 90° counterclockwise: (-y, x).
   *
   * Same magnitude as this. Calling twice gives negate().
   * Use for normals, stroke offsets, right-hand directions.
   */
  perpendicular(): Point {
    return new Point(-this.y, this.x);
  }

  /**
   * Direction angle in radians: atan2(y, x).
   *
   * Counterclockwise from positive X axis. Result in (-π, π].
   * Always calls trig.atan2 from PHY00.
   */
  angle(): number {
    return atan2(this.y, this.x);
  }
}

// ============================================================================
// Rect
// ============================================================================

/** An axis-aligned bounding box (AABB). */
export class Rect {
  constructor(
    /** X coordinate of the top-left corner. */
    public readonly x: number,
    /** Y coordinate of the top-left corner. */
    public readonly y: number,
    /** Width (extent in X). */
    public readonly width: number,
    /** Height (extent in Y). */
    public readonly height: number
  ) {}

  // -----------------------------------------------------------------------
  // Construction
  // -----------------------------------------------------------------------

  /**
   * Construct from two corner points.
   * min = top-left, max = bottom-right.
   */
  static fromPoints(min: Point, max: Point): Rect {
    return new Rect(min.x, min.y, max.x - min.x, max.y - min.y);
  }

  /** The empty rect at the origin: {0,0,0,0}. */
  static zero(): Rect {
    return new Rect(0, 0, 0, 0);
  }

  // -----------------------------------------------------------------------
  // Corner Accessors
  // -----------------------------------------------------------------------

  /** Top-left corner. */
  minPoint(): Point {
    return new Point(this.x, this.y);
  }

  /** Bottom-right corner: (x+width, y+height). */
  maxPoint(): Point {
    return new Point(this.x + this.width, this.y + this.height);
  }

  /** Center point: (x+width/2, y+height/2). */
  center(): Point {
    return new Point(this.x + this.width / 2, this.y + this.height / 2);
  }

  // -----------------------------------------------------------------------
  // Geometric Predicates
  // -----------------------------------------------------------------------

  /** True if width ≤ 0 or height ≤ 0 (zero-area rect). */
  isEmpty(): boolean {
    return this.width <= 0 || this.height <= 0;
  }

  /**
   * True if point p is inside this rect.
   *
   * Half-open interval: [x, x+width) × [y, y+height).
   * The top-left edge is inclusive; the bottom-right is exclusive.
   * This avoids double-counting when adjacent rects tile a surface.
   */
  containsPoint(p: Point): boolean {
    return (
      p.x >= this.x &&
      p.x < this.x + this.width &&
      p.y >= this.y &&
      p.y < this.y + this.height
    );
  }

  // -----------------------------------------------------------------------
  // Set Operations
  // -----------------------------------------------------------------------

  /**
   * Smallest rect containing both this and other.
   * If either is empty, returns the other.
   */
  union(other: Rect): Rect {
    if (this.isEmpty()) return other;
    if (other.isEmpty()) return this;
    const minX = Math.min(this.x, other.x);
    const minY = Math.min(this.y, other.y);
    const maxX = Math.max(this.x + this.width, other.x + other.width);
    const maxY = Math.max(this.y + this.height, other.y + other.height);
    return new Rect(minX, minY, maxX - minX, maxY - minY);
  }

  /**
   * Overlap region of this and other, or null if no overlap.
   *
   * Returns null if the overlap would have zero or negative area.
   */
  intersection(other: Rect): Rect | null {
    const ix = Math.max(this.x, other.x);
    const iy = Math.max(this.y, other.y);
    const iw =
      Math.min(this.x + this.width, other.x + other.width) - ix;
    const ih =
      Math.min(this.y + this.height, other.y + other.height) - iy;
    if (iw <= 0 || ih <= 0) return null;
    return new Rect(ix, iy, iw, ih);
  }

  /**
   * Grow all four edges outward by amount.
   *
   * Origin shifts by (-amount, -amount); dimensions grow by 2*amount each.
   * Negative amount shrinks the rect.
   */
  expandBy(amount: number): Rect {
    return new Rect(
      this.x - amount,
      this.y - amount,
      this.width + 2 * amount,
      this.height + 2 * amount
    );
  }
}
