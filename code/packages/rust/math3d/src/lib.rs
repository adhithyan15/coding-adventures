//! # math3d
//!
//! GAME30: deterministic, zero-allocation 3D vector, matrix, quaternion, plane,
//! axis-aligned bounding box, ray, and frustum math library for Era 3
//! (`raycast-dda`, `bsp-portal25d`, `raster3d-soft`) and Era 4
//! (`rigidbody-6dof`, `aero-flight-model`, `render3d-gpu-bridge`).
//!
//! ## Determinism and Precision
//!
//! All trigonometric and square-root operations delegate to [`trig`] (`PHY00`),
//! which computes bit-exact IEEE-754 `f64` values from pure arithmetic with no
//! platform `libm` divergence and no fused multiply-add contraction.
//!
//! Matrices use **column-major** storage (`cols[col][row]`), matching OpenGL,
//! Vulkan, Metal, and WGPU uniform layouts. Both OpenGL (`[-1, 1]`) and
//! zero-to-one (`[0, 1]`) depth clip conventions are provided for perspective,
//! orthographic, and frustum plane extraction.
//!
//! ## Safety and Resource Model
//!
//! * `#![forbid(unsafe_code)]`; zero pointer arithmetic anywhere.
//! * Zero heap allocation: every type is a fixed-size stack `Copy` value.
//! * Strict validation (CWE-20 / CWE-190 / CWE-369 / CWE-400): non-finite
//!   (`NaN` / `+/-Inf`) inputs and overflows are rejected, near-zero divisors
//!   are guarded with [`EPSILON`], singular matrices are rejected, and
//!   projection/frustum invariants (`near > 0`, `far > near`, `fov_y in (0, pi)`,
//!   `aspect > 0`) are enforced on construction.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

/// Generated capability verification witnesses.
pub mod capabilities;

pub use capabilities::PureCapabilities;

/// Numerical epsilon for near-zero denominator and singularity guards (CWE-369).
pub const EPSILON: f64 = 1e-12;

/// Maximum safe coordinate magnitude (`+/-1.0e12`, CWE-190 / CWE-400).
pub const MAX_COORD: f64 = 1.0e12;

/// Maximum angle magnitude accepted by `trig::sin` / `trig::cos` (`2^29` rad).
pub const MAX_ANGLE_RAD: f64 = 536_870_912.0;

/// Fixed-size 3-element vector array alias (for capability analyzer compatibility).
pub type Vec3Array = [f64; 3];

/// Fixed-size 4-element vector array alias.
pub type Vec4Array = [f64; 4];

/// Fixed-size quaternion `[x, y, z, w]` array alias.
pub type QuatArray = [f64; 4];

/// Column-major 3x3 matrix columns alias (`cols[col][row]`).
pub type Mat3Cols = [[f64; 3]; 3];

/// Column-major 3x3 flat array alias (`9` scalars).
pub type Mat3Flat = [f64; 9];

/// Column-major 4x4 `f64` matrix columns alias (`cols[col][row]`).
pub type Mat4Cols = [[f64; 4]; 4];

/// Column-major 4x4 `f32` matrix columns alias for GPU uniform upload.
pub type Mat4ColsF32 = [[f32; 4]; 4];

/// Column-major 4x4 `f64` flat array alias (`16` scalars).
pub type Mat4Flat = [f64; 16];

/// Column-major 4x4 `f32` flat array alias (`16` scalars).
pub type Mat4FlatF32 = [f32; 16];

/// Six frustum planes ordered `[left, right, bottom, top, near, far]`.
pub type FrustumPlanes = [Plane; 6];

/// Eight corner vertices of an [`Aabb3`].
pub type Aabb3Corners = [Vec3; 8];

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Failure modes for every fallible geometric or algebraic operation in `math3d`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Math3dError {
    /// An input scalar, component, or matrix entry is `NaN` or `+/-Infinity` (CWE-20).
    NonFiniteInput,
    /// A coordinate, angle, or computed result exceeds safe magnitude bounds (CWE-190 / CWE-400).
    ValueOutOfBounds,
    /// A vector, normal, quaternion, or ray direction has zero or near-zero length (CWE-369).
    ZeroLength,
    /// A matrix determinant is near zero and cannot be inverted (CWE-369).
    SingularMatrix,
    /// Projection bounds violate `near > 0`, `far > near`, `fov_y in (0, pi)`, `aspect > 0`, or non-degenerate extents.
    InvalidProjection,
    /// `look_at` eye and target coincide or the up vector is collinear with the forward axis.
    DegenerateBasis,
    /// Bounding volume violates `min <= max` or `radius >= 0`.
    InvalidBounds,
    /// Scalar parameter (such as interpolation `t` outside `[0, 1]`) is invalid.
    InvalidParameter,
}

impl std::fmt::Display for Math3dError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NonFiniteInput => write!(f, "input contains NaN or Infinity"),
            Self::ValueOutOfBounds => write!(f, "value exceeds safe finite magnitude bounds"),
            Self::ZeroLength => write!(f, "vector, quaternion, or divisor has zero length"),
            Self::SingularMatrix => write!(f, "matrix is singular and cannot be inverted"),
            Self::InvalidProjection => write!(f, "projection parameters are invalid"),
            Self::DegenerateBasis => write!(f, "camera eye, target, and up vectors are degenerate"),
            Self::InvalidBounds => write!(f, "bounding volume constraints are violated"),
            Self::InvalidParameter => write!(f, "scalar parameter is out of valid domain"),
        }
    }
}

impl std::error::Error for Math3dError {}

// ---------------------------------------------------------------------------
// Internal validation helpers (never panic)
// ---------------------------------------------------------------------------

#[inline]
fn check_finite(v: f64) -> Result<f64, Math3dError> {
    if !v.is_finite() {
        return Err(Math3dError::NonFiniteInput);
    }
    Ok(v)
}

#[inline]
fn check_coord(v: f64) -> Result<f64, Math3dError> {
    if !v.is_finite() {
        return Err(Math3dError::NonFiniteInput);
    }
    if v.abs() > MAX_COORD {
        return Err(Math3dError::ValueOutOfBounds);
    }
    Ok(v)
}

#[inline]
fn check_angle(rad: f64) -> Result<f64, Math3dError> {
    if !rad.is_finite() {
        return Err(Math3dError::NonFiniteInput);
    }
    if rad.abs() > MAX_ANGLE_RAD {
        return Err(Math3dError::ValueOutOfBounds);
    }
    Ok(rad)
}

#[inline]
fn check_result_finite(v: f64) -> Result<f64, Math3dError> {
    if !v.is_finite() {
        return Err(Math3dError::ValueOutOfBounds);
    }
    Ok(v)
}

#[inline]
fn safe_sqrt(x: f64) -> Result<f64, Math3dError> {
    if !x.is_finite() {
        return Err(Math3dError::ValueOutOfBounds);
    }
    if x < 0.0 {
        return Err(Math3dError::InvalidParameter);
    }
    Ok(trig::sqrt(x))
}

// ---------------------------------------------------------------------------
// Vec3
// ---------------------------------------------------------------------------

/// A 3D Euclidean vector or point `(x, y, z)` in `f64` precision.
///
/// Stack-allocated `Copy` value (24 bytes). Arithmetic methods validate
/// finiteness so `NaN` and `+/-Inf` cannot silently propagate across a simulation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vec3 {
    /// X coordinate.
    pub x: f64,
    /// Y coordinate.
    pub y: f64,
    /// Z coordinate.
    pub z: f64,
}

impl Vec3 {
    /// Construct a `Vec3` directly from `(x, y, z)`.
    ///
    /// Use [`Vec3::try_vec3`] when validating untrusted inputs against
    /// [`MAX_COORD`] and finiteness upfront.
    #[inline]
    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    /// The zero vector `(0, 0, 0)`.
    #[inline]
    pub const fn zero() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }

    /// Unit vector along `+X`: `(1, 0, 0)`.
    #[inline]
    pub const fn unit_x() -> Self {
        Self {
            x: 1.0,
            y: 0.0,
            z: 0.0,
        }
    }

    /// Unit vector along `+Y`: `(0, 1, 0)`.
    #[inline]
    pub const fn unit_y() -> Self {
        Self {
            x: 0.0,
            y: 1.0,
            z: 0.0,
        }
    }

    /// Unit vector along `+Z`: `(0, 0, 1)`.
    #[inline]
    pub const fn unit_z() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 1.0,
        }
    }

    /// Construct a `Vec3`, rejecting non-finite components and values exceeding [`MAX_COORD`].
    #[inline]
    pub fn try_vec3(x: f64, y: f64, z: f64) -> Result<Self, Math3dError> {
        Ok(Self {
            x: check_coord(x)?,
            y: check_coord(y)?,
            z: check_coord(z)?,
        })
    }

    /// Construct a `Vec3` from `[x, y, z]`, rejecting non-finite components.
    #[inline]
    pub fn vec3_from_array(a: Vec3Array) -> Result<Self, Math3dError> {
        Ok(Self {
            x: check_finite(a[0])?,
            y: check_finite(a[1])?,
            z: check_finite(a[2])?,
        })
    }

    /// Convert to `[x, y, z]`.
    #[inline]
    pub const fn vec3_to_array(self) -> Vec3Array {
        [self.x, self.y, self.z]
    }

    /// Return `true` iff all three components are finite.
    #[inline]
    pub fn vec3_is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite() && self.z.is_finite()
    }

    /// Component-wise addition `self + rhs`.
    #[inline]
    pub fn vec3_add(self, rhs: Vec3) -> Result<Vec3, Math3dError> {
        if !self.vec3_is_finite() || !rhs.vec3_is_finite() {
            return Err(Math3dError::NonFiniteInput);
        }
        Ok(Self {
            x: check_result_finite(self.x + rhs.x)?,
            y: check_result_finite(self.y + rhs.y)?,
            z: check_result_finite(self.z + rhs.z)?,
        })
    }

    /// Component-wise subtraction `self - rhs`.
    #[inline]
    pub fn vec3_sub(self, rhs: Vec3) -> Result<Vec3, Math3dError> {
        if !self.vec3_is_finite() || !rhs.vec3_is_finite() {
            return Err(Math3dError::NonFiniteInput);
        }
        Ok(Self {
            x: check_result_finite(self.x - rhs.x)?,
            y: check_result_finite(self.y - rhs.y)?,
            z: check_result_finite(self.z - rhs.z)?,
        })
    }

    /// Scalar multiplication `self * s`.
    #[inline]
    pub fn vec3_scale(self, s: f64) -> Result<Vec3, Math3dError> {
        if !self.vec3_is_finite() || !s.is_finite() {
            return Err(Math3dError::NonFiniteInput);
        }
        Ok(Self {
            x: check_result_finite(self.x * s)?,
            y: check_result_finite(self.y * s)?,
            z: check_result_finite(self.z * s)?,
        })
    }

    /// Additive inverse `-self`.
    #[inline]
    pub fn vec3_neg(self) -> Result<Vec3, Math3dError> {
        if !self.vec3_is_finite() {
            return Err(Math3dError::NonFiniteInput);
        }
        Ok(Self {
            x: -self.x,
            y: -self.y,
            z: -self.z,
        })
    }

    /// Euclidean dot product: `self.x * rhs.x + self.y * rhs.y + self.z * rhs.z`.
    #[inline]
    pub fn dot(self, rhs: Vec3) -> Result<f64, Math3dError> {
        if !self.vec3_is_finite() || !rhs.vec3_is_finite() {
            return Err(Math3dError::NonFiniteInput);
        }
        check_result_finite(self.x * rhs.x + self.y * rhs.y + self.z * rhs.z)
    }

    /// Right-handed 3D cross product `self x rhs`.
    ///
    /// Satisfies `unit_x().cross(unit_y()) == unit_z()`.
    #[inline]
    pub fn cross(self, rhs: Vec3) -> Result<Vec3, Math3dError> {
        if !self.vec3_is_finite() || !rhs.vec3_is_finite() {
            return Err(Math3dError::NonFiniteInput);
        }
        Ok(Self {
            x: check_result_finite(self.y * rhs.z - self.z * rhs.y)?,
            y: check_result_finite(self.z * rhs.x - self.x * rhs.z)?,
            z: check_result_finite(self.x * rhs.y - self.y * rhs.x)?,
        })
    }

    /// Squared Euclidean length: `x^2 + y^2 + z^2`.
    #[inline]
    pub fn length_sq(self) -> Result<f64, Math3dError> {
        self.dot(self)
    }

    /// Euclidean length `sqrt(x^2 + y^2 + z^2)` via deterministic [`trig::sqrt`].
    #[inline]
    pub fn length(self) -> Result<f64, Math3dError> {
        let lsq = self.length_sq()?;
        safe_sqrt(lsq)
    }

    /// Normalize to a unit vector.
    ///
    /// Rejects non-finite inputs with [`Math3dError::NonFiniteInput`] and zero or
    /// near-zero vectors (`length <= EPSILON`) with [`Math3dError::ZeroLength`].
    #[inline]
    pub fn normalize(self) -> Result<Vec3, Math3dError> {
        let len = self.length()?;
        if len <= EPSILON {
            return Err(Math3dError::ZeroLength);
        }
        let inv = 1.0 / len;
        Ok(Self {
            x: check_result_finite(self.x * inv)?,
            y: check_result_finite(self.y * inv)?,
            z: check_result_finite(self.z * inv)?,
        })
    }

    /// Euclidean distance between `self` and `other`.
    #[inline]
    pub fn distance(self, other: Vec3) -> Result<f64, Math3dError> {
        self.vec3_sub(other)?.length()
    }

    /// Linear interpolation `self + t * (other - self)` for finite `t`.
    #[inline]
    pub fn vec3_lerp(self, other: Vec3, t: f64) -> Result<Vec3, Math3dError> {
        check_finite(t)?;
        let diff = other.vec3_sub(self)?;
        self.vec3_add(diff.vec3_scale(t)?)
    }

    /// Component-wise minimum of `self` and `other`.
    #[inline]
    pub fn vec3_min(self, other: Vec3) -> Result<Vec3, Math3dError> {
        if !self.vec3_is_finite() || !other.vec3_is_finite() {
            return Err(Math3dError::NonFiniteInput);
        }
        Ok(Self {
            x: self.x.min(other.x),
            y: self.y.min(other.y),
            z: self.z.min(other.z),
        })
    }

    /// Component-wise maximum of `self` and `other`.
    #[inline]
    pub fn vec3_max(self, other: Vec3) -> Result<Vec3, Math3dError> {
        if !self.vec3_is_finite() || !other.vec3_is_finite() {
            return Err(Math3dError::NonFiniteInput);
        }
        Ok(Self {
            x: self.x.max(other.x),
            y: self.y.max(other.y),
            z: self.z.max(other.z),
        })
    }

    /// Reflect incident vector `self` about `normal` (normal is normalized internally).
    ///
    /// Computes `self - 2 * dot(self, n) * n`.
    #[inline]
    pub fn reflect(self, normal: Vec3) -> Result<Vec3, Math3dError> {
        let n = normal.normalize()?;
        let d = self.dot(n)?;
        let scale = check_result_finite(2.0 * d)?;
        self.vec3_sub(n.vec3_scale(scale)?)
    }
}

// ---------------------------------------------------------------------------
// Vec4
// ---------------------------------------------------------------------------

/// A 4D homogeneous vector `(x, y, z, w)` in `f64` precision.
///
/// Used for homogeneous clip-space coordinates (`w = 1` for positions, `w = 0`
/// for directions) and perspective division in software rasterizers (`GAME33`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vec4 {
    /// X coordinate.
    pub x: f64,
    /// Y coordinate.
    pub y: f64,
    /// Z coordinate.
    pub z: f64,
    /// Homogeneous W coordinate.
    pub w: f64,
}

impl Vec4 {
    /// Construct a `Vec4` directly from `(x, y, z, w)`.
    #[inline]
    pub const fn from_xyzw(x: f64, y: f64, z: f64, w: f64) -> Self {
        Self { x, y, z, w }
    }

    /// Construct a `Vec4`, rejecting non-finite inputs and components beyond [`MAX_COORD`].
    #[inline]
    pub fn try_vec4(x: f64, y: f64, z: f64, w: f64) -> Result<Self, Math3dError> {
        Ok(Self {
            x: check_coord(x)?,
            y: check_coord(y)?,
            z: check_coord(z)?,
            w: check_coord(w)?,
        })
    }

    /// Lift a [`Vec3`] into homogeneous coordinates with the given `w`.
    #[inline]
    pub const fn vec4_from_vec3(v: Vec3, w: f64) -> Self {
        Self {
            x: v.x,
            y: v.y,
            z: v.z,
            w,
        }
    }

    /// Extract the `(x, y, z)` components as a [`Vec3`].
    #[inline]
    pub const fn vec4_xyz(self) -> Vec3 {
        Vec3::new(self.x, self.y, self.z)
    }

    /// Construct a `Vec4` from `[x, y, z, w]`, rejecting non-finite components.
    #[inline]
    pub fn vec4_from_array(a: Vec4Array) -> Result<Self, Math3dError> {
        Ok(Self {
            x: check_finite(a[0])?,
            y: check_finite(a[1])?,
            z: check_finite(a[2])?,
            w: check_finite(a[3])?,
        })
    }

    /// Convert to `[x, y, z, w]`.
    #[inline]
    pub const fn vec4_to_array(self) -> Vec4Array {
        [self.x, self.y, self.z, self.w]
    }

    /// Return `true` iff all four components are finite.
    #[inline]
    pub fn vec4_is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite() && self.z.is_finite() && self.w.is_finite()
    }

    /// Component-wise addition `self + rhs`.
    #[inline]
    pub fn vec4_add(self, rhs: Vec4) -> Result<Vec4, Math3dError> {
        if !self.vec4_is_finite() || !rhs.vec4_is_finite() {
            return Err(Math3dError::NonFiniteInput);
        }
        Ok(Self {
            x: check_result_finite(self.x + rhs.x)?,
            y: check_result_finite(self.y + rhs.y)?,
            z: check_result_finite(self.z + rhs.z)?,
            w: check_result_finite(self.w + rhs.w)?,
        })
    }

    /// Component-wise subtraction `self - rhs`.
    #[inline]
    pub fn vec4_sub(self, rhs: Vec4) -> Result<Vec4, Math3dError> {
        if !self.vec4_is_finite() || !rhs.vec4_is_finite() {
            return Err(Math3dError::NonFiniteInput);
        }
        Ok(Self {
            x: check_result_finite(self.x - rhs.x)?,
            y: check_result_finite(self.y - rhs.y)?,
            z: check_result_finite(self.z - rhs.z)?,
            w: check_result_finite(self.w - rhs.w)?,
        })
    }

    /// Scalar multiplication `self * s`.
    #[inline]
    pub fn vec4_scale(self, s: f64) -> Result<Vec4, Math3dError> {
        if !self.vec4_is_finite() || !s.is_finite() {
            return Err(Math3dError::NonFiniteInput);
        }
        Ok(Self {
            x: check_result_finite(self.x * s)?,
            y: check_result_finite(self.y * s)?,
            z: check_result_finite(self.z * s)?,
            w: check_result_finite(self.w * s)?,
        })
    }

    /// 4D Euclidean dot product.
    #[inline]
    pub fn vec4_dot(self, rhs: Vec4) -> Result<f64, Math3dError> {
        if !self.vec4_is_finite() || !rhs.vec4_is_finite() {
            return Err(Math3dError::NonFiniteInput);
        }
        check_result_finite(self.x * rhs.x + self.y * rhs.y + self.z * rhs.z + self.w * rhs.w)
    }

    /// 4D Euclidean length.
    #[inline]
    pub fn vec4_length(self) -> Result<f64, Math3dError> {
        let lsq = self.vec4_dot(self)?;
        safe_sqrt(lsq)
    }

    /// Normalize to a unit 4D vector (rejects length <= [`EPSILON`]).
    #[inline]
    pub fn vec4_normalize(self) -> Result<Vec4, Math3dError> {
        let len = self.vec4_length()?;
        if len <= EPSILON {
            return Err(Math3dError::ZeroLength);
        }
        let inv = 1.0 / len;
        Ok(Self {
            x: check_result_finite(self.x * inv)?,
            y: check_result_finite(self.y * inv)?,
            z: check_result_finite(self.z * inv)?,
            w: check_result_finite(self.w * inv)?,
        })
    }

    /// Perform homogeneous perspective division `(x / w, y / w, z / w)`.
    ///
    /// Rejects `|w| <= EPSILON` with [`Math3dError::ZeroLength`] (CWE-369).
    #[inline]
    pub fn perspective_divide(self) -> Result<Vec3, Math3dError> {
        if !self.vec4_is_finite() {
            return Err(Math3dError::NonFiniteInput);
        }
        if self.w.abs() <= EPSILON {
            return Err(Math3dError::ZeroLength);
        }
        let inv_w = 1.0 / self.w;
        Ok(Vec3::new(
            check_result_finite(self.x * inv_w)?,
            check_result_finite(self.y * inv_w)?,
            check_result_finite(self.z * inv_w)?,
        ))
    }
}

// ---------------------------------------------------------------------------
// Quat
// ---------------------------------------------------------------------------

/// A quaternion `(x, y, z, w)` representing 3D orientation (`w` is the scalar part).
///
/// The identity rotation is `(0, 0, 0, 1)`. Unit quaternions avoid gimbal lock
/// and interpolate smoothly along geodesics on `S^3` via [`Quat::slerp`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Quat {
    /// Vector X component.
    pub x: f64,
    /// Vector Y component.
    pub y: f64,
    /// Vector Z component.
    pub z: f64,
    /// Scalar W component.
    pub w: f64,
}

impl Quat {
    /// The identity quaternion `(0, 0, 0, 1)`.
    #[inline]
    pub const fn quat_identity() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            w: 1.0,
        }
    }

    /// Construct a quaternion from `(x, y, z, w)`, rejecting non-finite components.
    #[inline]
    pub fn quat_from_xyzw(x: f64, y: f64, z: f64, w: f64) -> Result<Self, Math3dError> {
        Ok(Self {
            x: check_finite(x)?,
            y: check_finite(y)?,
            z: check_finite(z)?,
            w: check_finite(w)?,
        })
    }

    /// Construct a quaternion from `[x, y, z, w]`, rejecting non-finite components.
    #[inline]
    pub fn quat_from_array(a: QuatArray) -> Result<Self, Math3dError> {
        Self::quat_from_xyzw(a[0], a[1], a[2], a[3])
    }

    /// Convert to `[x, y, z, w]`.
    #[inline]
    pub const fn quat_to_array(self) -> QuatArray {
        [self.x, self.y, self.z, self.w]
    }

    /// Return `true` iff all four components are finite.
    #[inline]
    pub fn quat_is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite() && self.z.is_finite() && self.w.is_finite()
    }

    /// Squared norm `x^2 + y^2 + z^2 + w^2`.
    #[inline]
    pub fn quat_norm_sq(self) -> Result<f64, Math3dError> {
        if !self.quat_is_finite() {
            return Err(Math3dError::NonFiniteInput);
        }
        check_result_finite(self.x * self.x + self.y * self.y + self.z * self.z + self.w * self.w)
    }

    /// Normalize to a unit quaternion, rejecting zero-length quaternions.
    #[inline]
    pub fn quat_normalize(self) -> Result<Self, Math3dError> {
        let nsq = self.quat_norm_sq()?;
        let len = safe_sqrt(nsq)?;
        if len <= EPSILON {
            return Err(Math3dError::ZeroLength);
        }
        let inv = 1.0 / len;
        Ok(Self {
            x: check_result_finite(self.x * inv)?,
            y: check_result_finite(self.y * inv)?,
            z: check_result_finite(self.z * inv)?,
            w: check_result_finite(self.w * inv)?,
        })
    }

    /// Quaternion conjugate `(-x, -y, -z, w)` (equals inverse for unit quaternions).
    #[inline]
    pub fn conjugate(self) -> Result<Self, Math3dError> {
        if !self.quat_is_finite() {
            return Err(Math3dError::NonFiniteInput);
        }
        Ok(Self {
            x: -self.x,
            y: -self.y,
            z: -self.z,
            w: self.w,
        })
    }

    /// Multiplicative inverse `conjugate(q) / ||q||^2`.
    #[inline]
    pub fn quat_invert(self) -> Result<Self, Math3dError> {
        let nsq = self.quat_norm_sq()?;
        if nsq <= EPSILON * EPSILON {
            return Err(Math3dError::ZeroLength);
        }
        let inv = 1.0 / nsq;
        Ok(Self {
            x: check_result_finite(-self.x * inv)?,
            y: check_result_finite(-self.y * inv)?,
            z: check_result_finite(-self.z * inv)?,
            w: check_result_finite(self.w * inv)?,
        })
    }

    /// Hamilton product `self * rhs` (applies `rhs` rotation, then `self` rotation).
    #[inline]
    pub fn quat_mul(self, rhs: Quat) -> Result<Quat, Math3dError> {
        if !self.quat_is_finite() || !rhs.quat_is_finite() {
            return Err(Math3dError::NonFiniteInput);
        }
        Ok(Self {
            x: check_result_finite(
                self.w * rhs.x + self.x * rhs.w + self.y * rhs.z - self.z * rhs.y,
            )?,
            y: check_result_finite(
                self.w * rhs.y - self.x * rhs.z + self.y * rhs.w + self.z * rhs.x,
            )?,
            z: check_result_finite(
                self.w * rhs.z + self.x * rhs.y - self.y * rhs.x + self.z * rhs.w,
            )?,
            w: check_result_finite(
                self.w * rhs.w - self.x * rhs.x - self.y * rhs.y - self.z * rhs.z,
            )?,
        })
    }

    /// Construct a unit quaternion rotating by `angle_rad` radians around `axis`.
    ///
    /// Rejects zero-length axes and angles outside `[-MAX_ANGLE_RAD, MAX_ANGLE_RAD]`.
    #[inline]
    pub fn from_axis_angle(axis: Vec3, angle_rad: f64) -> Result<Self, Math3dError> {
        let unit = axis.normalize()?;
        let rad = check_angle(angle_rad)?;
        let half = rad * 0.5;
        let s = trig::sin(half);
        let c = trig::cos(half);
        Ok(Self {
            x: unit.x * s,
            y: unit.y * s,
            z: unit.z * s,
            w: c,
        })
    }

    /// Construct a unit quaternion from extrinsic X-Y-Z Euler angles `(rx, ry, rz)` in radians:
    /// `Rz(rz) * Ry(ry) * Rx(rx)`.
    pub fn from_euler_xyz(rx: f64, ry: f64, rz: f64) -> Result<Self, Math3dError> {
        let qx = Self::from_axis_angle(Vec3::unit_x(), rx)?;
        let qy = Self::from_axis_angle(Vec3::unit_y(), ry)?;
        let qz = Self::from_axis_angle(Vec3::unit_z(), rz)?;
        qz.quat_mul(qy)?.quat_mul(qx)
    }

    /// Construct a unit quaternion from Y-X-Z camera/flight Euler angles `(yaw, pitch, roll)`
    /// in radians: `Ry(yaw) * Rx(pitch) * Rz(roll)`.
    pub fn from_euler_yxz(yaw: f64, pitch: f64, roll: f64) -> Result<Self, Math3dError> {
        let qy = Self::from_axis_angle(Vec3::unit_y(), yaw)?;
        let qx = Self::from_axis_angle(Vec3::unit_x(), pitch)?;
        let qz = Self::from_axis_angle(Vec3::unit_z(), roll)?;
        qy.quat_mul(qx)?.quat_mul(qz)
    }

    /// Normalized linear interpolation (NLERP) along the shortest hemisphere arc for `t in [0, 1]`.
    pub fn nlerp(self, other: Quat, t: f64) -> Result<Quat, Math3dError> {
        check_finite(t)?;
        if !(0.0..=1.0).contains(&t) {
            return Err(Math3dError::InvalidParameter);
        }
        let a = self.quat_normalize()?;
        let b_norm = other.quat_normalize()?;
        let dot = a.x * b_norm.x + a.y * b_norm.y + a.z * b_norm.z + a.w * b_norm.w;
        let b = if dot < 0.0 {
            Self {
                x: -b_norm.x,
                y: -b_norm.y,
                z: -b_norm.z,
                w: -b_norm.w,
            }
        } else {
            b_norm
        };
        let one_minus_t = 1.0 - t;
        Self {
            x: a.x * one_minus_t + b.x * t,
            y: a.y * one_minus_t + b.y * t,
            z: a.z * one_minus_t + b.z * t,
            w: a.w * one_minus_t + b.w * t,
        }
        .quat_normalize()
    }

    /// Spherical linear interpolation (SLERP) along the shortest hemisphere arc for `t in [0, 1]`.
    ///
    /// Falls back smoothly to [`Quat::nlerp`] when `dot > 1 - 1e-6` to avoid dividing by
    /// near-zero `sin(theta)` (CWE-369).
    pub fn slerp(self, other: Quat, t: f64) -> Result<Quat, Math3dError> {
        check_finite(t)?;
        if !(0.0..=1.0).contains(&t) {
            return Err(Math3dError::InvalidParameter);
        }
        let a = self.quat_normalize()?;
        let b_norm = other.quat_normalize()?;
        let mut dot = a.x * b_norm.x + a.y * b_norm.y + a.z * b_norm.z + a.w * b_norm.w;
        let b = if dot < 0.0 {
            dot = -dot;
            Self {
                x: -b_norm.x,
                y: -b_norm.y,
                z: -b_norm.z,
                w: -b_norm.w,
            }
        } else {
            b_norm
        };
        if dot > 1.0 - 1e-6 {
            let one_minus_t = 1.0 - t;
            return Self {
                x: a.x * one_minus_t + b.x * t,
                y: a.y * one_minus_t + b.y * t,
                z: a.z * one_minus_t + b.z * t,
                w: a.w * one_minus_t + b.w * t,
            }
            .quat_normalize();
        }
        let clamped_dot = dot.min(1.0);
        let sin_theta = trig::sqrt((1.0 - clamped_dot * clamped_dot).max(0.0));
        if sin_theta <= EPSILON {
            return a.nlerp(b, t);
        }
        let theta = trig::atan2(sin_theta, clamped_dot);
        let w0 = trig::sin((1.0 - t) * theta) / sin_theta;
        let w1 = trig::sin(t * theta) / sin_theta;
        Self {
            x: a.x * w0 + b.x * w1,
            y: a.y * w0 + b.y * w1,
            z: a.z * w0 + b.z * w1,
            w: a.w * w0 + b.w * w1,
        }
        .quat_normalize()
    }

    /// Rotate a 3D vector `v` by this quaternion (normalized internally).
    ///
    /// Uses the optimized Rodrigues-quaternion identity:
    /// `v' = v + w * (2 * q_xyz x v) + q_xyz x (2 * q_xyz x v)`.
    #[inline]
    pub fn rotate_vec3(self, v: Vec3) -> Result<Vec3, Math3dError> {
        if !v.vec3_is_finite() {
            return Err(Math3dError::NonFiniteInput);
        }
        let q = self.quat_normalize()?;
        let q_xyz = Vec3::new(q.x, q.y, q.z);
        let t = q_xyz.cross(v)?.vec3_scale(2.0)?;
        let term1 = t.vec3_scale(q.w)?;
        let term2 = q_xyz.cross(t)?;
        v.vec3_add(term1)?.vec3_add(term2)
    }
}

// ---------------------------------------------------------------------------
// Mat3
// ---------------------------------------------------------------------------

/// A column-major 3x3 matrix (`cols[col][row]`) in `f64` precision.
///
/// Represents linear transformations in 3D (rotation, scale, inertia tensors
/// in `rigidbody-6dof`, and normal matrices).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mat3 {
    /// Columns `[col][row]`.
    pub cols: Mat3Cols,
}

impl Mat3 {
    /// The 3x3 identity matrix.
    #[inline]
    pub const fn mat3_identity() -> Self {
        Self {
            cols: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        }
    }

    /// Construct a `Mat3` from column-major columns, rejecting non-finite entries.
    pub fn mat3_from_cols(cols: Mat3Cols) -> Result<Self, Math3dError> {
        for col in cols {
            for v in col {
                check_finite(v)?;
            }
        }
        Ok(Self { cols })
    }

    /// Return the column-major columns `[[f64; 3]; 3]`.
    #[inline]
    pub const fn mat3_to_cols(self) -> Mat3Cols {
        self.cols
    }

    /// Construct a `Mat3` from a column-major flat array of 9 scalars.
    pub fn mat3_from_flat(flat: Mat3Flat) -> Result<Self, Math3dError> {
        for v in flat {
            check_finite(v)?;
        }
        Ok(Self {
            cols: [
                [flat[0], flat[1], flat[2]],
                [flat[3], flat[4], flat[5]],
                [flat[6], flat[7], flat[8]],
            ],
        })
    }

    /// Flatten to a column-major 9-element array.
    #[inline]
    pub const fn mat3_to_flat(self) -> Mat3Flat {
        [
            self.cols[0][0],
            self.cols[0][1],
            self.cols[0][2],
            self.cols[1][0],
            self.cols[1][1],
            self.cols[1][2],
            self.cols[2][0],
            self.cols[2][1],
            self.cols[2][2],
        ]
    }

    /// Convert a [`Quat`] (normalized internally) into a 3x3 rotation matrix.
    pub fn mat3_from_quat(q: Quat) -> Result<Self, Math3dError> {
        let u = q.quat_normalize()?;
        let x2 = u.x + u.x;
        let y2 = u.y + u.y;
        let z2 = u.z + u.z;
        let xx = u.x * x2;
        let xy = u.x * y2;
        let xz = u.x * z2;
        let yy = u.y * y2;
        let yz = u.y * z2;
        let zz = u.z * z2;
        let wx = u.w * x2;
        let wy = u.w * y2;
        let wz = u.w * z2;
        Ok(Self {
            cols: [
                [1.0 - (yy + zz), xy + wz, xz - wy],
                [xy - wz, 1.0 - (xx + zz), yz + wx],
                [xz + wy, yz - wx, 1.0 - (xx + yy)],
            ],
        })
    }

    /// Construct a diagonal scaling matrix `diag(sx, sy, sz)`.
    pub fn mat3_from_scale(sx: f64, sy: f64, sz: f64) -> Result<Self, Math3dError> {
        Ok(Self {
            cols: [
                [check_finite(sx)?, 0.0, 0.0],
                [0.0, check_finite(sy)?, 0.0],
                [0.0, 0.0, check_finite(sz)?],
            ],
        })
    }

    /// Return the transpose of this 3x3 matrix.
    pub fn mat3_transpose(self) -> Result<Self, Math3dError> {
        Self::mat3_from_cols([
            [self.cols[0][0], self.cols[1][0], self.cols[2][0]],
            [self.cols[0][1], self.cols[1][1], self.cols[2][1]],
            [self.cols[0][2], self.cols[1][2], self.cols[2][2]],
        ])
    }

    /// Compute the 3x3 determinant.
    pub fn mat3_determinant(self) -> Result<f64, Math3dError> {
        let c = &self.cols;
        for col in c {
            for &v in col {
                check_finite(v)?;
            }
        }
        let d = c[0][0] * (c[1][1] * c[2][2] - c[2][1] * c[1][2])
            - c[1][0] * (c[0][1] * c[2][2] - c[2][1] * c[0][2])
            + c[2][0] * (c[0][1] * c[1][2] - c[1][1] * c[0][2]);
        check_result_finite(d)
    }

    /// Invert this 3x3 matrix, rejecting singular matrices (`|det| <= EPSILON`).
    pub fn mat3_invert(self) -> Result<Self, Math3dError> {
        let det = self.mat3_determinant()?;
        if det.abs() <= EPSILON {
            return Err(Math3dError::SingularMatrix);
        }
        let inv_det = 1.0 / det;
        let c = &self.cols;
        let out = [
            [
                check_result_finite((c[1][1] * c[2][2] - c[2][1] * c[1][2]) * inv_det)?,
                check_result_finite((c[2][1] * c[0][2] - c[0][1] * c[2][2]) * inv_det)?,
                check_result_finite((c[0][1] * c[1][2] - c[1][1] * c[0][2]) * inv_det)?,
            ],
            [
                check_result_finite((c[2][0] * c[1][2] - c[1][0] * c[2][2]) * inv_det)?,
                check_result_finite((c[0][0] * c[2][2] - c[2][0] * c[0][2]) * inv_det)?,
                check_result_finite((c[1][0] * c[0][2] - c[0][0] * c[1][2]) * inv_det)?,
            ],
            [
                check_result_finite((c[1][0] * c[2][1] - c[2][0] * c[1][1]) * inv_det)?,
                check_result_finite((c[2][0] * c[0][1] - c[0][0] * c[2][1]) * inv_det)?,
                check_result_finite((c[0][0] * c[1][1] - c[1][0] * c[0][1]) * inv_det)?,
            ],
        ];
        Ok(Self { cols: out })
    }

    /// Matrix multiplication `self * rhs`.
    pub fn mat3_mul(self, rhs: &Mat3) -> Result<Mat3, Math3dError> {
        for col in self.cols {
            for v in col {
                check_finite(v)?;
            }
        }
        for col in rhs.cols {
            for v in col {
                check_finite(v)?;
            }
        }
        let mut cols = [[0.0; 3]; 3];
        for (c, out_col) in cols.iter_mut().enumerate() {
            for (r, slot) in out_col.iter_mut().enumerate() {
                let sum = self.cols[0][r] * rhs.cols[c][0]
                    + self.cols[1][r] * rhs.cols[c][1]
                    + self.cols[2][r] * rhs.cols[c][2];
                *slot = check_result_finite(sum)?;
            }
        }
        Ok(Mat3 { cols })
    }

    /// Multiply `self * v`.
    #[inline]
    pub fn mat3_mul_vec3(self, v: Vec3) -> Result<Vec3, Math3dError> {
        if !v.vec3_is_finite() {
            return Err(Math3dError::NonFiniteInput);
        }
        for col in self.cols {
            for entry in col {
                check_finite(entry)?;
            }
        }
        Ok(Vec3::new(
            check_result_finite(
                self.cols[0][0] * v.x + self.cols[1][0] * v.y + self.cols[2][0] * v.z,
            )?,
            check_result_finite(
                self.cols[0][1] * v.x + self.cols[1][1] * v.y + self.cols[2][1] * v.z,
            )?,
            check_result_finite(
                self.cols[0][2] * v.x + self.cols[1][2] * v.y + self.cols[2][2] * v.z,
            )?,
        ))
    }
}

// ---------------------------------------------------------------------------
// Mat4
// ---------------------------------------------------------------------------

/// A column-major 4x4 homogeneous matrix (`cols[col][row]`) in `f64` precision.
///
/// ```text
/// cols[0][0]  cols[1][0]  cols[2][0]  cols[3][0]   (x basis, y basis, z basis, translation.x)
/// cols[0][1]  cols[1][1]  cols[2][1]  cols[3][1]
/// cols[0][2]  cols[1][2]  cols[2][2]  cols[3][2]
/// cols[0][3]  cols[1][3]  cols[2][3]  cols[3][3]   (0,       0,       0,       1            )
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mat4 {
    /// Columns `[col][row]`.
    pub cols: Mat4Cols,
}

impl Mat4 {
    /// The 4x4 identity matrix.
    #[inline]
    pub const fn mat4_identity() -> Self {
        Self {
            cols: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    /// Construct a `Mat4` from column-major columns, rejecting non-finite entries.
    pub fn mat4_from_cols(cols: Mat4Cols) -> Result<Self, Math3dError> {
        for col in cols {
            for v in col {
                check_finite(v)?;
            }
        }
        Ok(Self { cols })
    }

    /// Return the column-major columns `[[f64; 4]; 4]`.
    #[inline]
    pub const fn mat4_to_cols(self) -> Mat4Cols {
        self.cols
    }

    /// Construct a `Mat4` from a column-major flat array of 16 scalars.
    pub fn mat4_from_flat(flat: Mat4Flat) -> Result<Self, Math3dError> {
        for v in flat {
            check_finite(v)?;
        }
        let mut cols = [[0.0; 4]; 4];
        let (chunks, _) = flat.as_chunks::<4>();
        for (c, chunk) in chunks.iter().enumerate() {
            cols[c] = *chunk;
        }
        Ok(Self { cols })
    }

    /// Flatten to a column-major 16-element `f64` array.
    #[inline]
    pub const fn mat4_to_flat(self) -> Mat4Flat {
        [
            self.cols[0][0],
            self.cols[0][1],
            self.cols[0][2],
            self.cols[0][3],
            self.cols[1][0],
            self.cols[1][1],
            self.cols[1][2],
            self.cols[1][3],
            self.cols[2][0],
            self.cols[2][1],
            self.cols[2][2],
            self.cols[2][3],
            self.cols[3][0],
            self.cols[3][1],
            self.cols[3][2],
            self.cols[3][3],
        ]
    }

    /// Convert to column-major `[[f32; 4]; 4]` for GPU uniform buffers, rejecting non-finite overflow.
    pub fn mat4_to_f32_cols(self) -> Result<Mat4ColsF32, Math3dError> {
        let mut out = [[0.0_f32; 4]; 4];
        for (c, col) in self.cols.iter().enumerate() {
            for (r, &v) in col.iter().enumerate() {
                check_finite(v)?;
                let f = v as f32;
                if !f.is_finite() {
                    return Err(Math3dError::ValueOutOfBounds);
                }
                out[c][r] = f;
            }
        }
        Ok(out)
    }

    /// Convert to a flat 16-element `f32` array, rejecting non-finite overflow.
    pub fn mat4_to_f32_flat(self) -> Result<Mat4FlatF32, Math3dError> {
        let cols = self.mat4_to_f32_cols()?;
        Ok([
            cols[0][0], cols[0][1], cols[0][2], cols[0][3], cols[1][0], cols[1][1], cols[1][2],
            cols[1][3], cols[2][0], cols[2][1], cols[2][2], cols[2][3], cols[3][0], cols[3][1],
            cols[3][2], cols[3][3],
        ])
    }

    /// Construct a pure translation matrix by `t`.
    pub fn mat4_from_translation(t: Vec3) -> Result<Self, Math3dError> {
        if !t.vec3_is_finite() {
            return Err(Math3dError::NonFiniteInput);
        }
        Ok(Self {
            cols: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [t.x, t.y, t.z, 1.0],
            ],
        })
    }

    /// Construct a pure scaling matrix by `(s.x, s.y, s.z)`.
    pub fn mat4_from_scale(s: Vec3) -> Result<Self, Math3dError> {
        if !s.vec3_is_finite() {
            return Err(Math3dError::NonFiniteInput);
        }
        Ok(Self {
            cols: [
                [s.x, 0.0, 0.0, 0.0],
                [0.0, s.y, 0.0, 0.0],
                [0.0, 0.0, s.z, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        })
    }

    /// Construct a 4x4 rotation matrix from a [`Quat`] (normalized internally).
    pub fn mat4_from_quat(q: Quat) -> Result<Self, Math3dError> {
        let r = Mat3::mat3_from_quat(q)?;
        Ok(Self {
            cols: [
                [r.cols[0][0], r.cols[0][1], r.cols[0][2], 0.0],
                [r.cols[1][0], r.cols[1][1], r.cols[1][2], 0.0],
                [r.cols[2][0], r.cols[2][1], r.cols[2][2], 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        })
    }

    /// Construct a Translation-Rotation-Scale (`T * R * S`) transform matrix.
    pub fn mat4_from_trs(
        translation: Vec3,
        rotation: Quat,
        scale: Vec3,
    ) -> Result<Self, Math3dError> {
        if !translation.vec3_is_finite() || !scale.vec3_is_finite() {
            return Err(Math3dError::NonFiniteInput);
        }
        let r = Mat3::mat3_from_quat(rotation)?;
        Ok(Self {
            cols: [
                [
                    check_result_finite(r.cols[0][0] * scale.x)?,
                    check_result_finite(r.cols[0][1] * scale.x)?,
                    check_result_finite(r.cols[0][2] * scale.x)?,
                    0.0,
                ],
                [
                    check_result_finite(r.cols[1][0] * scale.y)?,
                    check_result_finite(r.cols[1][1] * scale.y)?,
                    check_result_finite(r.cols[1][2] * scale.y)?,
                    0.0,
                ],
                [
                    check_result_finite(r.cols[2][0] * scale.z)?,
                    check_result_finite(r.cols[2][1] * scale.z)?,
                    check_result_finite(r.cols[2][2] * scale.z)?,
                    0.0,
                ],
                [translation.x, translation.y, translation.z, 1.0],
            ],
        })
    }

    fn validate_perspective_params(
        fov_y_rad: f64,
        aspect: f64,
        near: f64,
        far: f64,
    ) -> Result<(f64, f64, f64), Math3dError> {
        check_finite(fov_y_rad)?;
        check_finite(aspect)?;
        check_finite(near)?;
        check_finite(far)?;
        if fov_y_rad <= 0.0
            || fov_y_rad >= trig::PI
            || aspect <= EPSILON
            || near <= EPSILON
            || far <= near
        {
            return Err(Math3dError::InvalidProjection);
        }
        let half_tan = trig::tan(fov_y_rad * 0.5);
        if !half_tan.is_finite() || half_tan <= EPSILON {
            return Err(Math3dError::InvalidProjection);
        }
        let f = 1.0 / half_tan;
        if !f.is_finite() || f <= EPSILON {
            return Err(Math3dError::InvalidProjection);
        }
        let depth = check_result_finite(near - far)?;
        if depth.abs() <= EPSILON {
            return Err(Math3dError::InvalidProjection);
        }
        let m00 = check_result_finite(f / aspect)?;
        Ok((f, m00, depth))
    }

    /// Right-handed perspective projection matrix mapping NDC `z` to `[-1, 1]` (OpenGL style).
    ///
    /// Enforces:
    /// - all parameters finite,
    /// - `fov_y_rad` strictly in `(0, pi)` with `tan(fov_y_rad / 2) > EPSILON` and `f > EPSILON`,
    /// - `aspect > EPSILON`,
    /// - `near > EPSILON` and `far > near + EPSILON`.
    pub fn perspective(
        fov_y_rad: f64,
        aspect: f64,
        near: f64,
        far: f64,
    ) -> Result<Self, Math3dError> {
        let (f, m00, depth) = Self::validate_perspective_params(fov_y_rad, aspect, near, far)?;
        let m22 = check_result_finite(check_result_finite(far + near)? / depth)?;
        let m23 = check_result_finite(check_result_finite(2.0 * far * near)? / depth)?;
        Ok(Self {
            cols: [
                [m00, 0.0, 0.0, 0.0],
                [0.0, f, 0.0, 0.0],
                [0.0, 0.0, m22, -1.0],
                [0.0, 0.0, m23, 0.0],
            ],
        })
    }

    /// Right-handed perspective projection matrix mapping NDC `z` to `[0, 1]` (WGPU / Metal / Vulkan style).
    pub fn perspective_zo(
        fov_y_rad: f64,
        aspect: f64,
        near: f64,
        far: f64,
    ) -> Result<Self, Math3dError> {
        let (f, m00, depth) = Self::validate_perspective_params(fov_y_rad, aspect, near, far)?;
        let m22 = check_result_finite(far / depth)?;
        let m23 = check_result_finite(check_result_finite(far * near)? / depth)?;
        Ok(Self {
            cols: [
                [m00, 0.0, 0.0, 0.0],
                [0.0, f, 0.0, 0.0],
                [0.0, 0.0, m22, -1.0],
                [0.0, 0.0, m23, 0.0],
            ],
        })
    }

    /// Right-handed orthographic projection matrix mapping NDC `z` to `[-1, 1]`.
    ///
    /// Enforces `right > left`, `top > bottom`, and `far > near` beyond [`EPSILON`]
    /// without floating-point overflow.
    pub fn orthographic(
        left: f64,
        right: f64,
        bottom: f64,
        top: f64,
        near: f64,
        far: f64,
    ) -> Result<Self, Math3dError> {
        for v in [left, right, bottom, top, near, far] {
            check_finite(v)?;
        }
        let w = check_result_finite(right - left)?;
        let h = check_result_finite(top - bottom)?;
        let d = check_result_finite(far - near)?;
        if w <= EPSILON || h <= EPSILON || d <= EPSILON {
            return Err(Math3dError::InvalidProjection);
        }
        Ok(Self {
            cols: [
                [check_result_finite(2.0 / w)?, 0.0, 0.0, 0.0],
                [0.0, check_result_finite(2.0 / h)?, 0.0, 0.0],
                [0.0, 0.0, check_result_finite(-2.0 / d)?, 0.0],
                [
                    check_result_finite(-check_result_finite(right + left)? / w)?,
                    check_result_finite(-check_result_finite(top + bottom)? / h)?,
                    check_result_finite(-check_result_finite(far + near)? / d)?,
                    1.0,
                ],
            ],
        })
    }

    /// Right-handed view matrix transforming world coordinates into camera space.
    ///
    /// The camera is positioned at `eye`, looking toward `target`, with `up`
    /// indicating the world up hint. Rejects coincident `eye`/`target` or
    /// collinear `up` with [`Math3dError::DegenerateBasis`].
    pub fn look_at(eye: Vec3, target: Vec3, up: Vec3) -> Result<Self, Math3dError> {
        if !eye.vec3_is_finite() || !target.vec3_is_finite() || !up.vec3_is_finite() {
            return Err(Math3dError::NonFiniteInput);
        }
        let forward = target.vec3_sub(eye)?.normalize().map_err(|e| match e {
            Math3dError::ZeroLength => Math3dError::DegenerateBasis,
            other => other,
        })?;
        let right = forward.cross(up)?.normalize().map_err(|e| match e {
            Math3dError::ZeroLength => Math3dError::DegenerateBasis,
            other => other,
        })?;
        let cam_up = right.cross(forward)?;
        Ok(Self {
            cols: [
                [right.x, cam_up.x, -forward.x, 0.0],
                [right.y, cam_up.y, -forward.y, 0.0],
                [right.z, cam_up.z, -forward.z, 0.0],
                [
                    check_result_finite(-right.dot(eye)?)?,
                    check_result_finite(-cam_up.dot(eye)?)?,
                    check_result_finite(forward.dot(eye)?)?,
                    1.0,
                ],
            ],
        })
    }

    /// 4x4 matrix multiplication `self * rhs`.
    ///
    /// Zero heap allocation; runs in fixed O(1) time.
    #[allow(clippy::should_implement_trait)]
    pub fn mul(self, rhs: &Mat4) -> Result<Mat4, Math3dError> {
        for col in self.cols {
            for v in col {
                check_finite(v)?;
            }
        }
        for col in rhs.cols {
            for v in col {
                check_finite(v)?;
            }
        }
        let mut cols = [[0.0; 4]; 4];
        for (c, out_col) in cols.iter_mut().enumerate() {
            for (r, slot) in out_col.iter_mut().enumerate() {
                let sum = self.cols[0][r] * rhs.cols[c][0]
                    + self.cols[1][r] * rhs.cols[c][1]
                    + self.cols[2][r] * rhs.cols[c][2]
                    + self.cols[3][r] * rhs.cols[c][3];
                *slot = check_result_finite(sum)?;
            }
        }
        Ok(Mat4 { cols })
    }

    /// Multiply `self * v` for a 4D homogeneous vector.
    #[inline]
    pub fn mat4_mul_vec4(self, v: Vec4) -> Result<Vec4, Math3dError> {
        if !v.vec4_is_finite() {
            return Err(Math3dError::NonFiniteInput);
        }
        for col in self.cols {
            for entry in col {
                check_finite(entry)?;
            }
        }
        Ok(Vec4::from_xyzw(
            check_result_finite(
                self.cols[0][0] * v.x
                    + self.cols[1][0] * v.y
                    + self.cols[2][0] * v.z
                    + self.cols[3][0] * v.w,
            )?,
            check_result_finite(
                self.cols[0][1] * v.x
                    + self.cols[1][1] * v.y
                    + self.cols[2][1] * v.z
                    + self.cols[3][1] * v.w,
            )?,
            check_result_finite(
                self.cols[0][2] * v.x
                    + self.cols[1][2] * v.y
                    + self.cols[2][2] * v.z
                    + self.cols[3][2] * v.w,
            )?,
            check_result_finite(
                self.cols[0][3] * v.x
                    + self.cols[1][3] * v.y
                    + self.cols[2][3] * v.z
                    + self.cols[3][3] * v.w,
            )?,
        ))
    }

    /// Transform a 3D point (`w = 1`) with automatic perspective divide when `w != 1`.
    #[inline]
    pub fn transform_point3(self, p: Vec3) -> Result<Vec3, Math3dError> {
        let out = self.mat4_mul_vec4(Vec4::vec4_from_vec3(p, 1.0))?;
        out.perspective_divide()
    }

    /// Transform a 3D direction vector (`w = 0`, ignores translation).
    #[inline]
    pub fn transform_vector3(self, v: Vec3) -> Result<Vec3, Math3dError> {
        let out = self.mat4_mul_vec4(Vec4::vec4_from_vec3(v, 0.0))?;
        Ok(out.vec4_xyz())
    }

    /// Return the transpose of this 4x4 matrix.
    pub fn mat4_transpose(self) -> Result<Self, Math3dError> {
        Self::mat4_from_cols([
            [
                self.cols[0][0],
                self.cols[1][0],
                self.cols[2][0],
                self.cols[3][0],
            ],
            [
                self.cols[0][1],
                self.cols[1][1],
                self.cols[2][1],
                self.cols[3][1],
            ],
            [
                self.cols[0][2],
                self.cols[1][2],
                self.cols[2][2],
                self.cols[3][2],
            ],
            [
                self.cols[0][3],
                self.cols[1][3],
                self.cols[2][3],
                self.cols[3][3],
            ],
        ])
    }

    /// Compute the 4x4 determinant via 2x2 Laplace cofactor expansion.
    pub fn mat4_determinant(self) -> Result<f64, Math3dError> {
        for col in self.cols {
            for v in col {
                check_finite(v)?;
            }
        }
        let (a00, a01, a02, a03) = (
            self.cols[0][0],
            self.cols[1][0],
            self.cols[2][0],
            self.cols[3][0],
        );
        let (a10, a11, a12, a13) = (
            self.cols[0][1],
            self.cols[1][1],
            self.cols[2][1],
            self.cols[3][1],
        );
        let (a20, a21, a22, a23) = (
            self.cols[0][2],
            self.cols[1][2],
            self.cols[2][2],
            self.cols[3][2],
        );
        let (a30, a31, a32, a33) = (
            self.cols[0][3],
            self.cols[1][3],
            self.cols[2][3],
            self.cols[3][3],
        );

        let s0 = a00 * a11 - a10 * a01;
        let s1 = a00 * a12 - a10 * a02;
        let s2 = a00 * a13 - a10 * a03;
        let s3 = a01 * a12 - a11 * a02;
        let s4 = a01 * a13 - a11 * a03;
        let s5 = a02 * a13 - a12 * a03;

        let c5 = a22 * a33 - a32 * a23;
        let c4 = a21 * a33 - a31 * a23;
        let c3 = a21 * a32 - a31 * a22;
        let c2 = a20 * a33 - a30 * a23;
        let c1 = a20 * a32 - a30 * a22;
        let c0 = a20 * a31 - a30 * a21;

        check_result_finite(s0 * c5 - s1 * c4 + s2 * c3 + s3 * c2 - s4 * c1 + s5 * c0)
    }

    /// Invert this 4x4 matrix via exact cofactor expansion.
    ///
    /// Rejects singular matrices (`|det| <= EPSILON`) with [`Math3dError::SingularMatrix`].
    pub fn invert(self) -> Result<Self, Math3dError> {
        for col in self.cols {
            for v in col {
                check_finite(v)?;
            }
        }
        let (a00, a01, a02, a03) = (
            self.cols[0][0],
            self.cols[1][0],
            self.cols[2][0],
            self.cols[3][0],
        );
        let (a10, a11, a12, a13) = (
            self.cols[0][1],
            self.cols[1][1],
            self.cols[2][1],
            self.cols[3][1],
        );
        let (a20, a21, a22, a23) = (
            self.cols[0][2],
            self.cols[1][2],
            self.cols[2][2],
            self.cols[3][2],
        );
        let (a30, a31, a32, a33) = (
            self.cols[0][3],
            self.cols[1][3],
            self.cols[2][3],
            self.cols[3][3],
        );

        let s0 = a00 * a11 - a10 * a01;
        let s1 = a00 * a12 - a10 * a02;
        let s2 = a00 * a13 - a10 * a03;
        let s3 = a01 * a12 - a11 * a02;
        let s4 = a01 * a13 - a11 * a03;
        let s5 = a02 * a13 - a12 * a03;

        let c5 = a22 * a33 - a32 * a23;
        let c4 = a21 * a33 - a31 * a23;
        let c3 = a21 * a32 - a31 * a22;
        let c2 = a20 * a33 - a30 * a23;
        let c1 = a20 * a32 - a30 * a22;
        let c0 = a20 * a31 - a30 * a21;

        let det = s0 * c5 - s1 * c4 + s2 * c3 + s3 * c2 - s4 * c1 + s5 * c0;
        if !det.is_finite() || det.abs() <= EPSILON {
            return Err(Math3dError::SingularMatrix);
        }
        let inv_det = 1.0 / det;

        // Column-major inverse: cols[col][row] = inv(row, col)
        let cols = [
            [
                check_result_finite((a11 * c5 - a12 * c4 + a13 * c3) * inv_det)?,
                check_result_finite((-a10 * c5 + a12 * c2 - a13 * c1) * inv_det)?,
                check_result_finite((a10 * c4 - a11 * c2 + a13 * c0) * inv_det)?,
                check_result_finite((-a10 * c3 + a11 * c1 - a12 * c0) * inv_det)?,
            ],
            [
                check_result_finite((-a01 * c5 + a02 * c4 - a03 * c3) * inv_det)?,
                check_result_finite((a00 * c5 - a02 * c2 + a03 * c1) * inv_det)?,
                check_result_finite((-a00 * c4 + a01 * c2 - a03 * c0) * inv_det)?,
                check_result_finite((a00 * c3 - a01 * c1 + a02 * c0) * inv_det)?,
            ],
            [
                check_result_finite((a31 * s5 - a32 * s4 + a33 * s3) * inv_det)?,
                check_result_finite((-a30 * s5 + a32 * s2 - a33 * s1) * inv_det)?,
                check_result_finite((a30 * s4 - a31 * s2 + a33 * s0) * inv_det)?,
                check_result_finite((-a30 * s3 + a31 * s1 - a32 * s0) * inv_det)?,
            ],
            [
                check_result_finite((-a21 * s5 + a22 * s4 - a23 * s3) * inv_det)?,
                check_result_finite((a20 * s5 - a22 * s2 + a23 * s1) * inv_det)?,
                check_result_finite((-a20 * s4 + a21 * s2 - a23 * s0) * inv_det)?,
                check_result_finite((a20 * s3 - a21 * s1 + a22 * s0) * inv_det)?,
            ],
        ];
        Ok(Self { cols })
    }

    /// Extract the upper-left 3x3 linear submatrix.
    pub fn upper_mat3(self) -> Result<Mat3, Math3dError> {
        Mat3::mat3_from_cols([
            [self.cols[0][0], self.cols[0][1], self.cols[0][2]],
            [self.cols[1][0], self.cols[1][1], self.cols[1][2]],
            [self.cols[2][0], self.cols[2][1], self.cols[2][2]],
        ])
    }
}

// ---------------------------------------------------------------------------
// Plane
// ---------------------------------------------------------------------------

/// A 3D half-space plane defined by `normal.dot(p) + d = 0` with unit `normal`.
///
/// Points with `signed_distance(p) > 0` lie on the positive side (in front of
/// the plane / inside the frustum); points with `< 0` lie behind/outside.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Plane {
    /// Unit normal vector pointing into the positive half-space.
    pub normal: Vec3,
    /// Plane offset `d` in `normal.dot(p) + d = 0`.
    pub d: f64,
}

impl Plane {
    /// Construct a plane from `(normal, d)`, normalizing `normal` and scaling `d`
    /// consistently. Rejects zero-length normals or non-finite inputs.
    pub fn plane_from_normal_d(normal: Vec3, d: f64) -> Result<Self, Math3dError> {
        check_finite(d)?;
        let len = normal.length()?;
        if len <= EPSILON {
            return Err(Math3dError::ZeroLength);
        }
        let inv = 1.0 / len;
        Ok(Self {
            normal: Vec3::new(
                check_result_finite(normal.x * inv)?,
                check_result_finite(normal.y * inv)?,
                check_result_finite(normal.z * inv)?,
            ),
            d: check_result_finite(d * inv)?,
        })
    }

    /// Construct a plane passing through `point` with unitized `normal`.
    pub fn plane_from_point_normal(point: Vec3, normal: Vec3) -> Result<Self, Math3dError> {
        if !point.vec3_is_finite() {
            return Err(Math3dError::NonFiniteInput);
        }
        let unit = normal.normalize()?;
        let d = check_result_finite(-unit.dot(point)?)?;
        Ok(Self { normal: unit, d })
    }

    /// Construct a normalized plane from raw homogeneous coefficients `ax + by + cz + d = 0`.
    pub fn plane_from_raw(a: f64, b: f64, c: f64, d: f64) -> Result<Self, Math3dError> {
        Self::plane_from_normal_d(Vec3::new(a, b, c), d)
    }

    /// Signed distance from `point` to the plane: `normal.dot(point) + d`.
    #[inline]
    pub fn signed_distance(&self, point: Vec3) -> Result<f64, Math3dError> {
        let norm = Plane::plane_from_normal_d(self.normal, self.d)?;
        check_result_finite(norm.normal.dot(point)? + norm.d)
    }
}

// ---------------------------------------------------------------------------
// Aabb3 & Sphere
// ---------------------------------------------------------------------------

/// A 3D axis-aligned bounding box defined by `min` and `max` corners (`min <= max`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Aabb3 {
    /// Minimum corner `(min_x, min_y, min_z)`.
    pub min: Vec3,
    /// Maximum corner `(max_x, max_y, max_z)`.
    pub max: Vec3,
}

impl Aabb3 {
    /// Construct and validate an `Aabb3` from `min` and `max` corners.
    pub fn aabb3_from_min_max(min: Vec3, max: Vec3) -> Result<Self, Math3dError> {
        if !min.vec3_is_finite() || !max.vec3_is_finite() {
            return Err(Math3dError::NonFiniteInput);
        }
        if min.x > max.x || min.y > max.y || min.z > max.z {
            return Err(Math3dError::InvalidBounds);
        }
        Ok(Self { min, max })
    }

    /// Construct an `Aabb3` from `center` and non-negative `half_extents`.
    pub fn aabb3_from_center_half_extents(
        center: Vec3,
        half_extents: Vec3,
    ) -> Result<Self, Math3dError> {
        if !center.vec3_is_finite() || !half_extents.vec3_is_finite() {
            return Err(Math3dError::NonFiniteInput);
        }
        if half_extents.x < 0.0 || half_extents.y < 0.0 || half_extents.z < 0.0 {
            return Err(Math3dError::InvalidBounds);
        }
        Ok(Self {
            min: center.vec3_sub(half_extents)?,
            max: center.vec3_add(half_extents)?,
        })
    }

    fn validate_aabb(&self) -> Result<(), Math3dError> {
        if !self.min.vec3_is_finite() || !self.max.vec3_is_finite() {
            return Err(Math3dError::NonFiniteInput);
        }
        if self.min.x > self.max.x || self.min.y > self.max.y || self.min.z > self.max.z {
            return Err(Math3dError::InvalidBounds);
        }
        Ok(())
    }

    /// Center point `(min + max) * 0.5`.
    pub fn aabb3_center(&self) -> Result<Vec3, Math3dError> {
        self.validate_aabb()?;
        self.min.vec3_add(self.max)?.vec3_scale(0.5)
    }

    /// Full extent sizes `max - min`.
    pub fn aabb3_extents(&self) -> Result<Vec3, Math3dError> {
        self.validate_aabb()?;
        self.max.vec3_sub(self.min)
    }

    /// Return `true` iff `p` lies inside or on the boundary of `self`.
    pub fn aabb3_contains_point(&self, p: Vec3) -> Result<bool, Math3dError> {
        self.validate_aabb()?;
        if !p.vec3_is_finite() {
            return Err(Math3dError::NonFiniteInput);
        }
        Ok(p.x >= self.min.x
            && p.x <= self.max.x
            && p.y >= self.min.y
            && p.y <= self.max.y
            && p.z >= self.min.z
            && p.z <= self.max.z)
    }

    /// Return `true` iff `self` and `other` overlap (including touching faces).
    pub fn aabb3_intersects_aabb(&self, other: &Aabb3) -> Result<bool, Math3dError> {
        self.validate_aabb()?;
        other.validate_aabb()?;
        Ok(self.min.x <= other.max.x
            && self.max.x >= other.min.x
            && self.min.y <= other.max.y
            && self.max.y >= other.min.y
            && self.min.z <= other.max.z
            && self.max.z >= other.min.z)
    }

    /// Smallest `Aabb3` enclosing both `self` and `other`.
    pub fn aabb3_union(&self, other: &Aabb3) -> Result<Aabb3, Math3dError> {
        self.validate_aabb()?;
        other.validate_aabb()?;
        Ok(Aabb3 {
            min: self.min.vec3_min(other.min)?,
            max: self.max.vec3_max(other.max)?,
        })
    }

    /// Return all 8 corner vertices of `self`.
    pub fn aabb3_corners(&self) -> Result<Aabb3Corners, Math3dError> {
        self.validate_aabb()?;
        Ok([
            Vec3::new(self.min.x, self.min.y, self.min.z),
            Vec3::new(self.max.x, self.min.y, self.min.z),
            Vec3::new(self.min.x, self.max.y, self.min.z),
            Vec3::new(self.max.x, self.max.y, self.min.z),
            Vec3::new(self.min.x, self.min.y, self.max.z),
            Vec3::new(self.max.x, self.min.y, self.max.z),
            Vec3::new(self.min.x, self.max.y, self.max.z),
            Vec3::new(self.max.x, self.max.y, self.max.z),
        ])
    }
}

/// A 3D bounding sphere `(center, radius)` with `radius >= 0`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sphere {
    /// Center of the sphere.
    pub center: Vec3,
    /// Non-negative radius.
    pub radius: f64,
}

impl Sphere {
    /// Construct and validate a [`Sphere`] with `radius >= 0`.
    pub fn sphere_from_center_radius(center: Vec3, radius: f64) -> Result<Self, Math3dError> {
        if !center.vec3_is_finite() || !radius.is_finite() {
            return Err(Math3dError::NonFiniteInput);
        }
        if radius < 0.0 {
            return Err(Math3dError::InvalidBounds);
        }
        Ok(Self { center, radius })
    }
}

// ---------------------------------------------------------------------------
// Frustum
// ---------------------------------------------------------------------------

/// A 6-plane viewing frustum (`[left, right, bottom, top, near, far]`) with
/// inward-pointing unit normals.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Frustum {
    /// Frustum clipping planes ordered `[left, right, bottom, top, near, far]`.
    pub planes: FrustumPlanes,
}

fn mat4_row(m: &Mat4, r: usize) -> [f64; 4] {
    [m.cols[0][r], m.cols[1][r], m.cols[2][r], m.cols[3][r]]
}

fn plane_add_rows(a: [f64; 4], b: [f64; 4]) -> Result<Plane, Math3dError> {
    Plane::plane_from_raw(a[0] + b[0], a[1] + b[1], a[2] + b[2], a[3] + b[3])
}

fn plane_sub_rows(a: [f64; 4], b: [f64; 4]) -> Result<Plane, Math3dError> {
    Plane::plane_from_raw(a[0] - b[0], a[1] - b[1], a[2] - b[2], a[3] - b[3])
}

impl Frustum {
    /// Construct and validate a [`Frustum`] from 6 planes.
    pub fn frustum_from_planes(planes: FrustumPlanes) -> Result<Self, Math3dError> {
        let mut normalized = [planes[0]; 6];
        for (dst, src) in normalized.iter_mut().zip(planes.iter()) {
            *dst = Plane::plane_from_normal_d(src.normal, src.d)?;
        }
        Ok(Self { planes: normalized })
    }

    /// Gribb-Hartmann frustum plane extraction from an OpenGL `[-1, 1]` view-projection matrix.
    pub fn from_view_proj(vp: &Mat4) -> Result<Self, Math3dError> {
        for col in vp.cols {
            for v in col {
                check_finite(v)?;
            }
        }
        let r0 = mat4_row(vp, 0);
        let r1 = mat4_row(vp, 1);
        let r2 = mat4_row(vp, 2);
        let r3 = mat4_row(vp, 3);
        Ok(Self {
            planes: [
                plane_add_rows(r3, r0)?, // left
                plane_sub_rows(r3, r0)?, // right
                plane_add_rows(r3, r1)?, // bottom
                plane_sub_rows(r3, r1)?, // top
                plane_add_rows(r3, r2)?, // near (z >= -w)
                plane_sub_rows(r3, r2)?, // far  (z <= w)
            ],
        })
    }

    /// Gribb-Hartmann frustum plane extraction from a zero-to-one `[0, 1]` view-projection matrix.
    pub fn from_view_proj_zo(vp: &Mat4) -> Result<Self, Math3dError> {
        for col in vp.cols {
            for v in col {
                check_finite(v)?;
            }
        }
        let r0 = mat4_row(vp, 0);
        let r1 = mat4_row(vp, 1);
        let r2 = mat4_row(vp, 2);
        let r3 = mat4_row(vp, 3);
        Ok(Self {
            planes: [
                plane_add_rows(r3, r0)?,                            // left
                plane_sub_rows(r3, r0)?,                            // right
                plane_add_rows(r3, r1)?,                            // bottom
                plane_sub_rows(r3, r1)?,                            // top
                Plane::plane_from_raw(r2[0], r2[1], r2[2], r2[3])?, // near (z >= 0)
                plane_sub_rows(r3, r2)?,                            // far  (z <= w)
            ],
        })
    }

    /// Return `true` iff `point` lies inside or on the boundary of all 6 frustum planes.
    pub fn contains_point(&self, point: Vec3) -> Result<bool, Math3dError> {
        if !point.vec3_is_finite() {
            return Err(Math3dError::NonFiniteInput);
        }
        for plane in &self.planes {
            if plane.signed_distance(point)? < 0.0 {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// Conservative sphere vs. frustum intersection test.
    pub fn intersects_sphere(&self, sphere: &Sphere) -> Result<bool, Math3dError> {
        if !sphere.center.vec3_is_finite() || !sphere.radius.is_finite() {
            return Err(Math3dError::NonFiniteInput);
        }
        if sphere.radius < 0.0 {
            return Err(Math3dError::InvalidBounds);
        }
        for plane in &self.planes {
            if plane.signed_distance(sphere.center)? < -sphere.radius {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// Conservative AABB vs. frustum intersection test using the positive-vertex (p-vertex) rule.
    ///
    /// Zero heap allocation; runs in fixed O(1) time (6 plane checks).
    #[inline]
    pub fn intersects_aabb(&self, aabb: &Aabb3) -> Result<bool, Math3dError> {
        aabb.validate_aabb()?;
        for plane in &self.planes {
            check_finite(plane.d)?;
            if !plane.normal.vec3_is_finite() {
                return Err(Math3dError::NonFiniteInput);
            }
            if plane.normal.length_sq()? <= EPSILON * EPSILON {
                return Err(Math3dError::ZeroLength);
            }
            let px = if plane.normal.x >= 0.0 {
                aabb.max.x
            } else {
                aabb.min.x
            };
            let py = if plane.normal.y >= 0.0 {
                aabb.max.y
            } else {
                aabb.min.y
            };
            let pz = if plane.normal.z >= 0.0 {
                aabb.max.z
            } else {
                aabb.min.z
            };
            let dist = check_result_finite(
                plane.normal.x * px + plane.normal.y * py + plane.normal.z * pz + plane.d,
            )?;
            if dist < 0.0 {
                return Ok(false);
            }
        }
        Ok(true)
    }
}

// ---------------------------------------------------------------------------
// Ray3 & Ray Intersections
// ---------------------------------------------------------------------------

/// A 3D ray `origin + t * dir` (`t >= 0`) with unit direction `dir`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ray3 {
    /// Ray origin point.
    pub origin: Vec3,
    /// Unit ray direction.
    pub dir: Vec3,
}

/// Result of a successful ray-primitive intersection.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RayHit {
    /// Non-negative distance along the unit ray (`point = origin + t * dir`).
    pub t: f64,
    /// World-space intersection point.
    pub point: Vec3,
    /// Surface normal at the intersection point.
    pub normal: Vec3,
}

impl Ray3 {
    /// Construct a [`Ray3`], normalizing `dir` and rejecting zero-length directions.
    pub fn ray3_from_origin_dir(origin: Vec3, dir: Vec3) -> Result<Self, Math3dError> {
        if !origin.vec3_is_finite() {
            return Err(Math3dError::NonFiniteInput);
        }
        let unit_dir = dir.normalize()?;
        Ok(Self {
            origin,
            dir: unit_dir,
        })
    }

    /// Evaluate point `origin + t * dir` along the ray.
    #[inline]
    pub fn ray3_at(&self, t: f64) -> Result<Vec3, Math3dError> {
        check_finite(t)?;
        let unit_dir = self.dir.normalize()?;
        self.origin.vec3_add(unit_dir.vec3_scale(t)?)
    }
}

/// Intersect a [`Ray3`] with a [`Plane`] (`t >= 0`).
///
/// Returns `Ok(None)` when the ray is parallel to the plane (`|normal.dot(dir)| <= EPSILON`)
/// or when the intersection lies strictly behind the ray origin (`t < 0`).
pub fn ray_plane_intersect(ray: &Ray3, plane: &Plane) -> Result<Option<RayHit>, Math3dError> {
    if !ray.origin.vec3_is_finite() {
        return Err(Math3dError::NonFiniteInput);
    }
    let dir = ray.dir.normalize()?;
    let norm = Plane::plane_from_normal_d(plane.normal, plane.d)?;
    let denom = norm.normal.dot(dir)?;
    if denom.abs() <= EPSILON {
        return Ok(None);
    }
    let t = check_result_finite(-(norm.normal.dot(ray.origin)? + norm.d) / denom)?;
    if t < 0.0 {
        return Ok(None);
    }
    let point = ray.origin.vec3_add(dir.vec3_scale(t)?)?;
    let hit_normal = if denom < 0.0 {
        norm.normal
    } else {
        norm.normal.vec3_neg()?
    };
    Ok(Some(RayHit {
        t,
        point,
        normal: hit_normal,
    }))
}

/// Intersect a [`Ray3`] with an [`Aabb3`] using the Kay-Kajiya slab method (`t >= 0`).
///
/// Parallel axes (`|dir_axis| <= EPSILON`) are handled without division by zero
/// (CWE-369). If the ray starts inside the AABB, returns `t = 0.0` at `ray.origin`.
pub fn ray_aabb_intersect(ray: &Ray3, aabb: &Aabb3) -> Result<Option<RayHit>, Math3dError> {
    if !ray.origin.vec3_is_finite() {
        return Err(Math3dError::NonFiniteInput);
    }
    let dir = ray.dir.normalize()?;
    aabb.validate_aabb()?;

    let mut t_min = f64::NEG_INFINITY;
    let mut t_max = f64::INFINITY;
    let mut hit_normal = Vec3::zero();

    let axes = [
        (
            ray.origin.x,
            dir.x,
            aabb.min.x,
            aabb.max.x,
            Vec3::new(-1.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
        ),
        (
            ray.origin.y,
            dir.y,
            aabb.min.y,
            aabb.max.y,
            Vec3::new(0.0, -1.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ),
        (
            ray.origin.z,
            dir.z,
            aabb.min.z,
            aabb.max.z,
            Vec3::new(0.0, 0.0, -1.0),
            Vec3::new(0.0, 0.0, 1.0),
        ),
    ];

    for (orig, d, bmin, bmax, n_neg, n_pos) in axes {
        if d.abs() <= EPSILON {
            if orig < bmin || orig > bmax {
                return Ok(None);
            }
            continue;
        }
        let inv_d = 1.0 / d;
        let t1 = check_result_finite((bmin - orig) * inv_d)?;
        let t2 = check_result_finite((bmax - orig) * inv_d)?;
        let (t_near, t_far, n_near) = if t1 <= t2 {
            (t1, t2, n_neg)
        } else {
            (t2, t1, n_pos)
        };
        if t_near > t_min {
            t_min = t_near;
            hit_normal = n_near;
        }
        if t_far < t_max {
            t_max = t_far;
        }
        if t_min > t_max {
            return Ok(None);
        }
    }

    if t_max < 0.0 {
        return Ok(None);
    }

    if t_min >= 0.0 {
        let point = ray.origin.vec3_add(dir.vec3_scale(t_min)?)?;
        Ok(Some(RayHit {
            t: t_min,
            point,
            normal: hit_normal,
        }))
    } else {
        Ok(Some(RayHit {
            t: 0.0,
            point: ray.origin,
            normal: dir.vec3_neg()?,
        }))
    }
}
