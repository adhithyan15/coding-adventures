//! # rigidbody-6dof
//!
//! GAME40: deterministic, zero-allocation 6-DOF rigid-body dynamics integrator
//! and ground contact solver for Era 4 (`rigidbody-6dof`, `aero-flight-model`,
//! `game-flight-sim`).
//!
//! ## Physical Formulation
//!
//! Each rigid body evolves in 3D world space using six degrees of freedom:
//! - Linear position `position: Vec3` and linear momentum `linear_momentum: Vec3`
//!   (`p = m * v`), so translational motion obeys `dx/dt = p / m` and `dp/dt = F`.
//! - Unit orientation quaternion `orientation: Quat` (body-to-world rotation `R`)
//!   and world-frame angular momentum `angular_momentum: Vec3` (`L = I_world * omega`),
//!   so rotational dynamics obey Euler's equation:
//!   `dL/dt = tau_world` with `omega_world = R * I_body^-1 * R^T * L`
//!   (equivalently `I_body * alpha_body + omega_body x (I_body * omega_body) = tau_body`),
//!   and quaternion kinematics `dq/dt = 0.5 * (omega_world, 0) * q`.
//!
//! Two explicit time integrators are provided via [`IntegratorKind`]:
//! 1. [`IntegratorKind::VelocityVerlet`]: 2nd-order symplectic Verlet with
//!    midpoint rotational tensor evaluation and exponential quaternion map update.
//! 2. [`IntegratorKind::RK4`]: 4th-order Runge-Kutta over the full 13-scalar state
//!    `(x, p, q, L)` with post-step quaternion renormalization.
//!
//! ## Ground Contact Models
//!
//! Multi-point contact probes (such as aircraft landing gear struts or hull vertices)
//! interact with an arbitrary [`Plane`] via [`GroundContactParams`]:
//! - [`ContactModel::Penalty`]: unilateral normal spring-damper with smooth
//!   regularized Coulomb friction.
//! - [`ContactModel::Impulse`]: restitution + Coulomb friction sequential impulse
//!   resolution with positional depenetration.
//!
//! ## Safety, Determinism, and Resource Guarantees
//!
//! * `#![forbid(unsafe_code)]`; `#![deny(missing_docs)]`.
//! * Zero ambient capabilities (`"capabilities": []`, `CAP001`).
//! * Zero heap allocation in every hot-loop method (`step_in_place`,
//!   `step_with_contacts_in_place`), verified via `CountingAllocator` (`VER07`).
//! * Atomic error semantics: on any validation failure or numerical divergence,
//!   [`RigidBodyState`] is left completely unmodified.
//! * Strict CWE-20 / CWE-190 / CWE-369 / CWE-400 / CWE-835 guards on mass,
//!   inertia tensor symmetry/positive-definiteness, time step, substeps,
//!   contact count, restitution/friction domains, and state finiteness.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

/// Generated capability verification witnesses.
pub mod capabilities;

pub use capabilities::PureCapabilities;

use math3d::{Mat3, Math3dError, Plane, Quat, Vec3};

/// Numerical epsilon for near-zero vector/quaternion/divisor guards (CWE-369).
pub const EPSILON: f64 = math3d::EPSILON;

/// Maximum safe finite coordinate/momentum/force magnitude (`1e12`, CWE-190 / CWE-400).
pub const MAX_COORD: f64 = math3d::MAX_COORD;

/// Minimum strictly positive rigid-body mass in kilograms (`1e-6`, CWE-369).
pub const MIN_MASS: f64 = 1e-6;

/// Maximum finite rigid-body mass in kilograms (`1e9`, CWE-20 / CWE-400).
///
/// Joint domain envelope with [`MAX_COORD`] (`1e12`): keeping `mass <= 1e9 kg`
/// ensures typical gravitational/aerodynamic forces (`~10 * mass`) and
/// rigid-body inertia tensor entries remain safely below [`MAX_COORD`].
pub const MAX_MASS: f64 = 1.0e9;

/// Minimum scale-normalized determinant & leading principal minor threshold (`1e-12`, CWE-369).
///
/// Applied to the normalized inertia tensor `I / s` where `s = max(|I_ii|)`.
pub const MIN_DET: f64 = 1e-12;

/// Minimum positive diagonal inertia scale `s = max(|I_ii|)` to prevent `1/s` overflow.
pub const MIN_INERTIA_SCALE: f64 = 1e-18;

/// Relative tolerance for checking symmetry of the 3x3 body inertia tensor
/// (`|I_ij - I_ji| <= INERTIA_SYMMETRY_TOL * max(1, max(|I|))`).
pub const INERTIA_SYMMETRY_TOL: f64 = 1e-9;

/// Maximum single-step time interval in seconds (`0.25` s, CWE-20 / CWE-400).
pub const MAX_DT: f64 = 0.25;

/// Maximum substeps allowed per contact step (`64`, CWE-400 / CWE-835).
pub const MAX_SUBSTEPS: usize = 64;

/// Maximum contact probe points allowed per step (`64`, CWE-400 / CWE-835).
pub const MAX_CONTACT_POINTS: usize = 64;

const RESTITUTION_VEL_THRESHOLD: f64 = 0.05;
const PENETRATION_SLOP: f64 = 1e-4;
const FRICTION_SLIP_THRESHOLD: f64 = 0.05;

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

/// Failure modes for every fallible operation in `rigidbody-6dof`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RigidBodyError {
    /// A scalar, vector, matrix, or quaternion input contains `NaN` or `+/-Infinity` (CWE-20).
    NonFiniteInput,
    /// Rigid-body mass is non-finite, below [`MIN_MASS`], or above [`MAX_MASS`] (CWE-20 / CWE-369).
    InvalidMass,
    /// Inertia tensor is asymmetric, not positive-definite, or has `det <= MIN_DET` (CWE-369).
    InvalidInertiaTensor,
    /// A contact normal or orientation quaternion has zero or near-zero length (CWE-369).
    ZeroLengthNormal,
    /// Time step `dt` is non-finite, `<= 0`, or exceeds [`MAX_DT`] (CWE-20).
    InvalidTimeStep,
    /// Requested substep count is `0` or exceeds [`MAX_SUBSTEPS`] (CWE-400 / CWE-835).
    SubstepLimitExceeded,
    /// Provided contact probe slice exceeds [`MAX_CONTACT_POINTS`] (CWE-400).
    TooManyContactPoints,
    /// Contact parameter violates domain (`restitution in [0, 1]`, `friction >= 0`, etc., CWE-20).
    InvalidContactParameter,
    /// Integrated state became non-finite or exceeded [`MAX_COORD`] (atomic rollback preserved).
    NumericalDivergence,
    /// Underlying [`Math3dError`] propagated from `math3d`.
    MathError(Math3dError),
}

impl From<Math3dError> for RigidBodyError {
    #[inline]
    fn from(err: Math3dError) -> Self {
        match err {
            Math3dError::NonFiniteInput => Self::NonFiniteInput,
            Math3dError::ZeroLength => Self::ZeroLengthNormal,
            Math3dError::ValueOutOfBounds => Self::NumericalDivergence,
            other => Self::MathError(other),
        }
    }
}

impl std::fmt::Display for RigidBodyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NonFiniteInput => write!(f, "input contains NaN or Infinity"),
            Self::InvalidMass => write!(f, "mass must be finite and in [MIN_MASS, MAX_MASS]"),
            Self::InvalidInertiaTensor => {
                write!(
                    f,
                    "inertia tensor must be symmetric positive-definite with det > MIN_DET"
                )
            }
            Self::ZeroLengthNormal => {
                write!(
                    f,
                    "orientation quaternion or contact normal has zero length"
                )
            }
            Self::InvalidTimeStep => write!(f, "time step dt must be finite and in (0, MAX_DT]"),
            Self::SubstepLimitExceeded => {
                write!(f, "substeps must be in 1..=MAX_SUBSTEPS")
            }
            Self::TooManyContactPoints => {
                write!(f, "contact probe count exceeds MAX_CONTACT_POINTS")
            }
            Self::InvalidContactParameter => write!(f, "contact parameter is out of valid domain"),
            Self::NumericalDivergence => {
                write!(
                    f,
                    "simulation state diverged beyond finite magnitude bounds"
                )
            }
            Self::MathError(e) => write!(f, "math3d error: {e}"),
        }
    }
}

impl std::error::Error for RigidBodyError {}

// ---------------------------------------------------------------------------
// Internal validation & linear algebra helpers
// ---------------------------------------------------------------------------

#[inline]
fn check_finite(v: f64) -> Result<f64, RigidBodyError> {
    if !v.is_finite() {
        return Err(RigidBodyError::NonFiniteInput);
    }
    Ok(v)
}

#[inline]
fn check_bounded_vec3(v: Vec3) -> Result<Vec3, RigidBodyError> {
    if !v.vec3_is_finite() {
        return Err(RigidBodyError::NonFiniteInput);
    }
    if v.x.abs() > MAX_COORD || v.y.abs() > MAX_COORD || v.z.abs() > MAX_COORD {
        return Err(RigidBodyError::NumericalDivergence);
    }
    Ok(v)
}

#[inline]
fn check_quat_valid(q: Quat) -> Result<Quat, RigidBodyError> {
    if !q.quat_is_finite() {
        return Err(RigidBodyError::NonFiniteInput);
    }
    let nsq = q.quat_norm_sq()?;
    if nsq <= EPSILON * EPSILON {
        return Err(RigidBodyError::ZeroLengthNormal);
    }
    Ok(q)
}

#[inline]
fn check_dt(dt: f64) -> Result<f64, RigidBodyError> {
    if !dt.is_finite() {
        return Err(RigidBodyError::NonFiniteInput);
    }
    if dt <= 0.0 || dt > MAX_DT {
        return Err(RigidBodyError::InvalidTimeStep);
    }
    Ok(dt)
}

#[inline]
fn vec3_add_checked(a: Vec3, b: Vec3) -> Result<Vec3, RigidBodyError> {
    let out = a.vec3_add(b)?;
    check_bounded_vec3(out)
}

#[inline]
fn vec3_scale_checked(v: Vec3, s: f64) -> Result<Vec3, RigidBodyError> {
    check_finite(s)?;
    let out = v.vec3_scale(s)?;
    check_bounded_vec3(out)
}

#[inline]
fn quat_add_scaled(q: Quat, dq: Quat, s: f64) -> Result<Quat, RigidBodyError> {
    check_finite(s)?;
    let out = Quat::quat_from_xyzw(
        q.x + dq.x * s,
        q.y + dq.y * s,
        q.z + dq.z * s,
        q.w + dq.w * s,
    )?;
    if out.quat_norm_sq()? <= EPSILON * EPSILON {
        return Err(RigidBodyError::NumericalDivergence);
    }
    Ok(out)
}

#[inline]
fn quat_derivative_world(omega_world: Vec3, q: Quat) -> Result<Quat, RigidBodyError> {
    let omega_q = Quat::quat_from_xyzw(omega_world.x, omega_world.y, omega_world.z, 0.0)?;
    let prod = omega_q.quat_mul(q)?;
    Ok(Quat::quat_from_xyzw(
        prod.x * 0.5,
        prod.y * 0.5,
        prod.z * 0.5,
        prod.w * 0.5,
    )?)
}

#[inline]
fn advance_quat_exponential(q: Quat, omega: Vec3, dt: f64) -> Result<Quat, RigidBodyError> {
    let omega_len = omega.length()?;
    let angle = omega_len * dt;
    if !angle.is_finite() {
        return Err(RigidBodyError::NumericalDivergence);
    }
    let next = if angle > 1e-8 {
        let axis = omega.vec3_scale(1.0 / omega_len)?;
        let dq = Quat::from_axis_angle(axis, angle)?;
        dq.quat_mul(q)?
    } else {
        let half_dt = 0.5 * dt;
        let dq =
            Quat::quat_from_xyzw(omega.x * half_dt, omega.y * half_dt, omega.z * half_dt, 1.0)?;
        dq.quat_mul(q)?
    };
    Ok(next.quat_normalize()?)
}

// ---------------------------------------------------------------------------
// MassProperties
// ---------------------------------------------------------------------------

/// Mass and body-frame 3x3 inertia tensor properties of a rigid body.
///
/// Enforces positive finite mass (`>= MIN_MASS`), strict tensor symmetry
/// (`|I_ij - I_ji| <= INERTIA_SYMMETRY_TOL`), positive-definiteness via
/// Sylvester's leading principal minors (`m1 > 0`, `m2 > 0`, `det > MIN_DET`),
/// and precomputes both `inv_mass` and `inv_inertia_body`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MassProperties {
    mass: f64,
    inv_mass: f64,
    inertia_body: Mat3,
    inv_inertia_body: Mat3,
}

impl MassProperties {
    /// Construct [`MassProperties`] from `mass` and a symmetric positive-definite
    /// body-frame 3x3 inertia tensor `inertia_body`.
    pub fn mass_props_from_tensor(mass: f64, inertia_body: Mat3) -> Result<Self, RigidBodyError> {
        if !mass.is_finite() {
            return Err(RigidBodyError::NonFiniteInput);
        }
        if !(MIN_MASS..=MAX_MASS).contains(&mass) {
            return Err(RigidBodyError::InvalidMass);
        }

        let c = inertia_body.cols;
        for col in c {
            for v in col {
                if !v.is_finite() {
                    return Err(RigidBodyError::NonFiniteInput);
                }
                if v.abs() > MAX_COORD {
                    return Err(RigidBodyError::InvalidInertiaTensor);
                }
            }
        }

        // Verify scale-relative symmetry: columns in Mat3 are c[col][row].
        let s_diag = c[0][0].abs().max(c[1][1].abs()).max(c[2][2].abs());
        let s_off = c[0][1]
            .abs()
            .max(c[1][0].abs())
            .max(c[0][2].abs())
            .max(c[2][0].abs())
            .max(c[1][2].abs())
            .max(c[2][1].abs());
        let sym_tol = INERTIA_SYMMETRY_TOL * 1.0_f64.max(s_diag).max(s_off);
        let diff_01 = (c[0][1] - c[1][0]).abs();
        let diff_02 = (c[0][2] - c[2][0]).abs();
        let diff_12 = (c[1][2] - c[2][1]).abs();
        if diff_01 > sym_tol || diff_02 > sym_tol || diff_12 > sym_tol {
            return Err(RigidBodyError::InvalidInertiaTensor);
        }

        // Exact symmetrization within tolerance for deterministic inversion.
        let i00 = c[0][0];
        let i11 = c[1][1];
        let i22 = c[2][2];
        let i01 = 0.5 * (c[0][1] + c[1][0]);
        let i02 = 0.5 * (c[0][2] + c[2][0]);
        let i12 = 0.5 * (c[1][2] + c[2][1]);
        let sym = Mat3::mat3_from_cols([[i00, i01, i02], [i01, i11, i12], [i02, i12, i22]])?;

        // Scale-relative Sylvester's criterion on normalized tensor `I / s`,
        // where `s = max(I_ii)`. Equivalent to:
        //   m1 > MIN_DET * s, m2 > MIN_DET * s^2, det > MIN_DET * s^3
        // without `s^3` overflow/underflow for small or large rigid bodies.
        if i00 <= 0.0 || i11 <= 0.0 || i22 <= 0.0 {
            return Err(RigidBodyError::InvalidInertiaTensor);
        }
        let s = i00.max(i11).max(i22);
        if !s.is_finite() || s <= MIN_INERTIA_SCALE {
            return Err(RigidBodyError::InvalidInertiaTensor);
        }
        let inv_s = 1.0 / s;
        if !inv_s.is_finite() {
            return Err(RigidBodyError::InvalidInertiaTensor);
        }
        let norm = Mat3::mat3_from_cols([
            [i00 * inv_s, i01 * inv_s, i02 * inv_s],
            [i01 * inv_s, i11 * inv_s, i12 * inv_s],
            [i02 * inv_s, i12 * inv_s, i22 * inv_s],
        ])?;
        let nc = norm.cols;
        let m1 = nc[0][0];
        let m2 = nc[0][0] * nc[1][1] - nc[0][1] * nc[0][1];
        let det = norm.mat3_determinant()?;
        if !m2.is_finite() || !det.is_finite() || m1 <= MIN_DET || m2 <= MIN_DET || det <= MIN_DET {
            return Err(RigidBodyError::InvalidInertiaTensor);
        }

        let inv_norm = norm
            .mat3_invert()
            .map_err(|_| RigidBodyError::InvalidInertiaTensor)?;
        let inv_inertia_body = Mat3::mat3_from_cols([
            [
                inv_norm.cols[0][0] * inv_s,
                inv_norm.cols[0][1] * inv_s,
                inv_norm.cols[0][2] * inv_s,
            ],
            [
                inv_norm.cols[1][0] * inv_s,
                inv_norm.cols[1][1] * inv_s,
                inv_norm.cols[1][2] * inv_s,
            ],
            [
                inv_norm.cols[2][0] * inv_s,
                inv_norm.cols[2][1] * inv_s,
                inv_norm.cols[2][2] * inv_s,
            ],
        ])
        .map_err(|_| RigidBodyError::InvalidInertiaTensor)?;
        let inv_mass = 1.0 / mass;
        if !inv_mass.is_finite() {
            return Err(RigidBodyError::InvalidMass);
        }

        Ok(Self {
            mass,
            inv_mass,
            inertia_body: sym,
            inv_inertia_body,
        })
    }

    /// Construct [`MassProperties`] from principal moments of inertia `diag(ixx, iyy, izz)`.
    pub fn mass_props_from_diagonal(
        mass: f64,
        ixx: f64,
        iyy: f64,
        izz: f64,
    ) -> Result<Self, RigidBodyError> {
        let tensor = Mat3::mat3_from_scale(ixx, iyy, izz)?;
        Self::mass_props_from_tensor(mass, tensor)
    }

    /// Construct [`MassProperties`] for a uniform solid cuboid with `half_extents = (hx, hy, hz)`.
    ///
    /// Uses `I_xx = (1/3) * m * (hy^2 + hz^2)` (since full side `a = 2 * hx`, `(1/12)*m*(b^2+c^2) = (1/3)*m*(hy^2+hz^2)`).
    pub fn mass_props_from_solid_box(
        mass: f64,
        half_extents: Vec3,
    ) -> Result<Self, RigidBodyError> {
        if !mass.is_finite() || !half_extents.vec3_is_finite() {
            return Err(RigidBodyError::NonFiniteInput);
        }
        if !(MIN_MASS..=MAX_MASS).contains(&mass) {
            return Err(RigidBodyError::InvalidMass);
        }
        if half_extents.x <= 0.0 || half_extents.y <= 0.0 || half_extents.z <= 0.0 {
            return Err(RigidBodyError::InvalidInertiaTensor);
        }
        let hx2 = half_extents.x * half_extents.x;
        let hy2 = half_extents.y * half_extents.y;
        let hz2 = half_extents.z * half_extents.z;
        let factor = mass / 3.0;
        let ixx = factor * (hy2 + hz2);
        let iyy = factor * (hx2 + hz2);
        let izz = factor * (hx2 + hy2);
        Self::mass_props_from_diagonal(mass, ixx, iyy, izz)
    }

    /// Construct [`MassProperties`] for a uniform solid sphere of radius `radius` (`I = 2/5 * m * r^2`).
    pub fn mass_props_from_solid_sphere(mass: f64, radius: f64) -> Result<Self, RigidBodyError> {
        if !mass.is_finite() || !radius.is_finite() {
            return Err(RigidBodyError::NonFiniteInput);
        }
        if !(MIN_MASS..=MAX_MASS).contains(&mass) {
            return Err(RigidBodyError::InvalidMass);
        }
        if radius <= 0.0 {
            return Err(RigidBodyError::InvalidInertiaTensor);
        }
        let inertia = 0.4 * mass * radius * radius;
        Self::mass_props_from_diagonal(mass, inertia, inertia, inertia)
    }

    /// Return the scalar mass in kilograms.
    #[inline]
    pub const fn mass_props_mass(&self) -> f64 {
        self.mass
    }

    /// Return the precomputed inverse mass `1 / mass`.
    #[inline]
    pub const fn mass_props_inv_mass(&self) -> f64 {
        self.inv_mass
    }

    /// Return the body-frame 3x3 inertia tensor.
    #[inline]
    pub const fn mass_props_inertia_body(&self) -> Mat3 {
        self.inertia_body
    }

    /// Return the precomputed body-frame inverse 3x3 inertia tensor.
    #[inline]
    pub const fn mass_props_inv_inertia_body(&self) -> Mat3 {
        self.inv_inertia_body
    }

    /// Re-validate all mass and inertia invariants.
    pub fn mass_props_validate(&self) -> Result<(), RigidBodyError> {
        let _ = Self::mass_props_from_tensor(self.mass, self.inertia_body)?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// IntegratorKind
// ---------------------------------------------------------------------------

/// Numerical integration scheme for 6-DOF rigid-body dynamics.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntegratorKind {
    /// 2nd-order Symplectic Velocity-Verlet with midpoint inertia tensor
    /// evaluation and exponential quaternion update.
    VelocityVerlet,
    /// 4th-order Runge-Kutta (RK4) over `(position, linear_momentum, orientation, angular_momentum)`
    /// with post-step quaternion renormalization.
    RK4,
}

// ---------------------------------------------------------------------------
// Ground Contact Model
// ---------------------------------------------------------------------------

/// Contact response law used by [`GroundContactParams`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ContactModel {
    /// Unilateral penalty spring-damper normal response with smooth Coulomb friction.
    Penalty {
        /// Normal spring stiffness `k_n >= 0` (N/m).
        stiffness: f64,
        /// Normal damping coefficient `c_n >= 0` (N*s/m).
        damping: f64,
        /// Coulomb friction coefficient `mu >= 0`.
        friction: f64,
    },
    /// Rigid-body sequential impulse response with restitution and Coulomb friction.
    Impulse {
        /// Coefficient of restitution `e in [0, 1]`.
        restitution: f64,
        /// Coulomb friction coefficient `mu >= 0`.
        friction: f64,
        /// Fractional positional depenetration gain in `[0, 1]`.
        position_correction: f64,
    },
}

/// Configuration for multi-point ground contact resolution against a [`Plane`].
///
/// The positive half-space of `plane` (`plane.signed_distance(p) > 0`) represents
/// free space above the ground; points with `signed_distance(p) <= 0` are in
/// contact or penetrating.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GroundContactParams {
    plane: Plane,
    model: ContactModel,
    substeps: usize,
}

impl GroundContactParams {
    /// Construct a penalty spring-damper ground contact configuration.
    pub fn contact_params_penalty(
        plane: Plane,
        stiffness: f64,
        damping: f64,
        friction: f64,
        substeps: usize,
    ) -> Result<Self, RigidBodyError> {
        let params = Self {
            plane,
            model: ContactModel::Penalty {
                stiffness,
                damping,
                friction,
            },
            substeps,
        };
        params.contact_params_validate()?;
        Ok(Self {
            plane: Plane::plane_from_normal_d(plane.normal, plane.d)?,
            model: params.model,
            substeps,
        })
    }

    /// Construct a restitution + Coulomb friction impulse ground contact configuration.
    pub fn contact_params_impulse(
        plane: Plane,
        restitution: f64,
        friction: f64,
        position_correction: f64,
        substeps: usize,
    ) -> Result<Self, RigidBodyError> {
        let params = Self {
            plane,
            model: ContactModel::Impulse {
                restitution,
                friction,
                position_correction,
            },
            substeps,
        };
        params.contact_params_validate()?;
        Ok(Self {
            plane: Plane::plane_from_normal_d(plane.normal, plane.d)?,
            model: params.model,
            substeps,
        })
    }

    /// Validate all ground plane, contact parameter, and substep bounds (CWE-20 / CWE-369 / CWE-400).
    pub fn contact_params_validate(&self) -> Result<Plane, RigidBodyError> {
        let norm_plane = Plane::plane_from_normal_d(self.plane.normal, self.plane.d)?;
        if self.substeps == 0 || self.substeps > MAX_SUBSTEPS {
            return Err(RigidBodyError::SubstepLimitExceeded);
        }
        match self.model {
            ContactModel::Penalty {
                stiffness,
                damping,
                friction,
            } => {
                if !stiffness.is_finite() || !damping.is_finite() || !friction.is_finite() {
                    return Err(RigidBodyError::NonFiniteInput);
                }
                if !(0.0..=MAX_COORD).contains(&stiffness)
                    || !(0.0..=MAX_COORD).contains(&damping)
                    || !(0.0..=MAX_COORD).contains(&friction)
                {
                    return Err(RigidBodyError::InvalidContactParameter);
                }
            }
            ContactModel::Impulse {
                restitution,
                friction,
                position_correction,
            } => {
                if !restitution.is_finite()
                    || !friction.is_finite()
                    || !position_correction.is_finite()
                {
                    return Err(RigidBodyError::NonFiniteInput);
                }
                if !(0.0..=1.0).contains(&restitution)
                    || !(0.0..=MAX_COORD).contains(&friction)
                    || !(0.0..=1.0).contains(&position_correction)
                {
                    return Err(RigidBodyError::InvalidContactParameter);
                }
            }
        }
        Ok(norm_plane)
    }

    /// Return the ground [`Plane`].
    #[inline]
    pub const fn contact_params_plane(&self) -> Plane {
        self.plane
    }

    /// Return the [`ContactModel`].
    #[inline]
    pub const fn contact_params_model(&self) -> ContactModel {
        self.model
    }

    /// Return the bounded substep count (`1..=MAX_SUBSTEPS`).
    #[inline]
    pub const fn contact_params_substeps(&self) -> usize {
        self.substeps
    }
}

/// Telemetry returned by [`RigidBodyState::step_with_contacts_report_in_place`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ContactStepReport {
    active_contacts: usize,
    max_penetration: f64,
    max_normal_force: f64,
}

impl ContactStepReport {
    /// Number of probe-substep contact events detected during the step.
    #[inline]
    pub const fn report_active_contacts(&self) -> usize {
        self.active_contacts
    }

    /// Maximum penetration depth observed across all probes (`>= 0.0`).
    #[inline]
    pub const fn report_max_penetration(&self) -> f64 {
        self.max_penetration
    }

    /// Peak normal contact force (for penalty mode) or normal impulse magnitude (for impulse mode).
    #[inline]
    pub const fn report_max_normal_force(&self) -> f64 {
        self.max_normal_force
    }
}

// ---------------------------------------------------------------------------
// RigidBodyState
// ---------------------------------------------------------------------------

/// Full 6-DOF state of a rigid body in world space.
///
/// Stack-allocated `Copy` value (152 bytes). Every mutating step operation
/// operates atomically on a local candidate copy and commits to `self` only
/// after all finiteness and bound invariants succeed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RigidBodyState {
    /// World-space center-of-mass position (m).
    pub position: Vec3,
    /// World-space linear momentum `p = m * v` (kg*m/s).
    pub linear_momentum: Vec3,
    /// Body-to-world unit orientation quaternion `(x, y, z, w)`.
    pub orientation: Quat,
    /// World-space angular momentum `L = I_world * omega` (kg*m^2/s).
    pub angular_momentum: Vec3,
    /// Accumulated world-space force `F` (N).
    pub force: Vec3,
    /// Accumulated world-space torque `tau` (N*m).
    pub torque: Vec3,
}

#[derive(Clone, Copy)]
struct StateDeriv {
    dx: Vec3,
    dp: Vec3,
    dq: Quat,
    dl: Vec3,
}

impl RigidBodyState {
    /// Construct a stationary [`RigidBodyState`] at `position` and normalized `orientation`.
    pub fn body_new(position: Vec3, orientation: Quat) -> Result<Self, RigidBodyError> {
        let pos = check_bounded_vec3(position)?;
        let q = check_quat_valid(orientation)?.quat_normalize()?;
        Ok(Self {
            position: pos,
            linear_momentum: Vec3::zero(),
            orientation: q,
            angular_momentum: Vec3::zero(),
            force: Vec3::zero(),
            torque: Vec3::zero(),
        })
    }

    /// Construct a resting [`RigidBodyState`] at the origin with identity orientation.
    #[inline]
    pub const fn body_identity() -> Self {
        Self {
            position: Vec3::zero(),
            linear_momentum: Vec3::zero(),
            orientation: Quat::quat_identity(),
            angular_momentum: Vec3::zero(),
            force: Vec3::zero(),
            torque: Vec3::zero(),
        }
    }

    /// Validate that all fields of `self` are finite, within [`MAX_COORD`],
    /// and that `orientation` has non-zero norm.
    pub fn body_validate(&self) -> Result<(), RigidBodyError> {
        check_bounded_vec3(self.position)?;
        check_bounded_vec3(self.linear_momentum)?;
        check_quat_valid(self.orientation)?;
        check_bounded_vec3(self.angular_momentum)?;
        check_bounded_vec3(self.force)?;
        check_bounded_vec3(self.torque)?;
        Ok(())
    }

    /// Zero out the accumulated world `force` and `torque` vectors.
    #[inline]
    pub fn body_clear_accumulators(&mut self) {
        self.force = Vec3::zero();
        self.torque = Vec3::zero();
    }

    /// Accumulate a world-space force `force` onto `self.force` atomically.
    pub fn apply_world_force(&mut self, force: Vec3) -> Result<(), RigidBodyError> {
        self.body_validate()?;
        check_bounded_vec3(force)?;
        let next_force = vec3_add_checked(self.force, force)?;
        self.force = next_force;
        Ok(())
    }

    /// Transform a body-local `body_force` into world space and add it to `self.force`.
    pub fn apply_body_force(&mut self, body_force: Vec3) -> Result<(), RigidBodyError> {
        self.body_validate()?;
        check_bounded_vec3(body_force)?;
        let q = self.orientation.quat_normalize()?;
        let world_force = check_bounded_vec3(q.rotate_vec3(body_force)?)?;
        let next_force = vec3_add_checked(self.force, world_force)?;
        self.force = next_force;
        Ok(())
    }

    /// Accumulate a world-space torque `torque` onto `self.torque` atomically.
    pub fn apply_world_torque(&mut self, torque: Vec3) -> Result<(), RigidBodyError> {
        self.body_validate()?;
        check_bounded_vec3(torque)?;
        let next_torque = vec3_add_checked(self.torque, torque)?;
        self.torque = next_torque;
        Ok(())
    }

    /// Transform a body-local `body_torque` into world space and add it to `self.torque`.
    pub fn apply_body_torque(&mut self, body_torque: Vec3) -> Result<(), RigidBodyError> {
        self.body_validate()?;
        check_bounded_vec3(body_torque)?;
        let q = self.orientation.quat_normalize()?;
        let world_torque = check_bounded_vec3(q.rotate_vec3(body_torque)?)?;
        let next_torque = vec3_add_checked(self.torque, world_torque)?;
        self.torque = next_torque;
        Ok(())
    }

    /// Apply a world-space `force` at `world_point`, accumulating both linear force
    /// and moment `(world_point - self.position) x force` atomically.
    pub fn apply_force_at_world_point(
        &mut self,
        force: Vec3,
        world_point: Vec3,
    ) -> Result<(), RigidBodyError> {
        self.body_validate()?;
        check_bounded_vec3(force)?;
        check_bounded_vec3(world_point)?;
        let r = check_bounded_vec3(world_point.vec3_sub(self.position)?)?;
        let tau = check_bounded_vec3(r.cross(force)?)?;
        let next_force = vec3_add_checked(self.force, force)?;
        let next_torque = vec3_add_checked(self.torque, tau)?;
        self.force = next_force;
        self.torque = next_torque;
        Ok(())
    }

    /// Apply a body-local `body_force` at a body-local `body_point` atomically.
    pub fn apply_force_at_body_point(
        &mut self,
        body_force: Vec3,
        body_point: Vec3,
    ) -> Result<(), RigidBodyError> {
        self.body_validate()?;
        check_bounded_vec3(body_force)?;
        check_bounded_vec3(body_point)?;
        let q = self.orientation.quat_normalize()?;
        let world_force = check_bounded_vec3(q.rotate_vec3(body_force)?)?;
        let world_r = check_bounded_vec3(q.rotate_vec3(body_point)?)?;
        let tau = check_bounded_vec3(world_r.cross(world_force)?)?;
        let next_force = vec3_add_checked(self.force, world_force)?;
        let next_torque = vec3_add_checked(self.torque, tau)?;
        self.force = next_force;
        self.torque = next_torque;
        Ok(())
    }

    /// Compute current world-space linear velocity `v = p / m`.
    #[inline]
    pub fn linear_velocity(&self, mass: &MassProperties) -> Result<Vec3, RigidBodyError> {
        self.body_validate()?;
        mass.mass_props_validate()?;
        vec3_scale_checked(self.linear_momentum, mass.inv_mass)
    }

    /// Compute the world-space inertia tensor `I_world = R * I_body * R^T`.
    pub fn world_inertia(&self, mass: &MassProperties) -> Result<Mat3, RigidBodyError> {
        self.body_validate()?;
        mass.mass_props_validate()?;
        let r = Mat3::mat3_from_quat(self.orientation)?;
        let rt = r.mat3_transpose()?;
        Ok(r.mat3_mul(&mass.inertia_body)?.mat3_mul(&rt)?)
    }

    /// Compute the world-space inverse inertia tensor `I_world^-1 = R * I_body^-1 * R^T`.
    #[inline]
    pub fn world_inv_inertia(&self, mass: &MassProperties) -> Result<Mat3, RigidBodyError> {
        self.body_validate()?;
        mass.mass_props_validate()?;
        Self::world_inv_inertia_for_quat(self.orientation, mass.inv_inertia_body)
    }

    #[inline]
    fn world_inv_inertia_for_quat(q: Quat, inv_i_body: Mat3) -> Result<Mat3, RigidBodyError> {
        let r = Mat3::mat3_from_quat(q)?;
        let rt = r.mat3_transpose()?;
        Ok(r.mat3_mul(&inv_i_body)?.mat3_mul(&rt)?)
    }

    /// Compute current world-space angular velocity `omega_world = I_world^-1 * L`.
    #[inline]
    pub fn angular_velocity_world(&self, mass: &MassProperties) -> Result<Vec3, RigidBodyError> {
        self.body_validate()?;
        mass.mass_props_validate()?;
        let inv_i_world =
            Self::world_inv_inertia_for_quat(self.orientation, mass.inv_inertia_body)?;
        check_bounded_vec3(inv_i_world.mat3_mul_vec3(self.angular_momentum)?)
    }

    /// Compute current body-space angular velocity `omega_body = I_body^-1 * (R^T * L)`.
    #[inline]
    pub fn angular_velocity_body(&self, mass: &MassProperties) -> Result<Vec3, RigidBodyError> {
        self.body_validate()?;
        mass.mass_props_validate()?;
        let q_inv = self.orientation.quat_normalize()?.conjugate()?;
        let l_body = q_inv.rotate_vec3(self.angular_momentum)?;
        check_bounded_vec3(mass.inv_inertia_body.mat3_mul_vec3(l_body)?)
    }

    /// Compute translational kinetic energy `T_trans = 0.5 * ||p||^2 / m`.
    #[inline]
    pub fn translational_kinetic_energy(
        &self,
        mass: &MassProperties,
    ) -> Result<f64, RigidBodyError> {
        self.body_validate()?;
        mass.mass_props_validate()?;
        let p_sq = self.linear_momentum.length_sq()?;
        let ke = 0.5 * p_sq * mass.inv_mass;
        check_finite(ke)
    }

    /// Compute rotational kinetic energy `T_rot = 0.5 * L . omega_world`.
    #[inline]
    pub fn rotational_kinetic_energy(&self, mass: &MassProperties) -> Result<f64, RigidBodyError> {
        let omega = self.angular_velocity_world(mass)?;
        let ke = 0.5 * self.angular_momentum.dot(omega)?;
        check_finite(ke)
    }

    /// Compute total kinetic energy `T = T_trans + T_rot`.
    #[inline]
    pub fn kinetic_energy(&self, mass: &MassProperties) -> Result<f64, RigidBodyError> {
        let t_trans = self.translational_kinetic_energy(mass)?;
        let t_rot = self.rotational_kinetic_energy(mass)?;
        check_finite(t_trans + t_rot)
    }

    /// Transform a body-local point `body_point` into world coordinates.
    #[inline]
    pub fn body_to_world_point(&self, body_point: Vec3) -> Result<Vec3, RigidBodyError> {
        self.body_validate()?;
        check_bounded_vec3(body_point)?;
        let rotated = self.orientation.quat_normalize()?.rotate_vec3(body_point)?;
        vec3_add_checked(self.position, rotated)
    }

    /// Compute the world-space velocity of a point located at `world_point` rigidly
    /// attached to this body: `v + omega_world x (world_point - position)`.
    #[inline]
    pub fn world_point_velocity(
        &self,
        mass: &MassProperties,
        world_point: Vec3,
    ) -> Result<Vec3, RigidBodyError> {
        check_bounded_vec3(world_point)?;
        let v_cm = self.linear_velocity(mass)?;
        let omega = self.angular_velocity_world(mass)?;
        let r = world_point.vec3_sub(self.position)?;
        vec3_add_checked(v_cm, omega.cross(r)?)
    }

    /// Compute a canonical 64-bit FNV-1a digest of the entire rigid-body state.
    ///
    /// Canonicalizes `-0.0` to `+0.0` prior to hashing IEEE-754 little-endian bit
    /// patterns so mathematically identical states produce bit-exact digests.
    pub fn state_digest_fnv1a(&self) -> Result<u64, RigidBodyError> {
        self.body_validate()?;
        const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
        const FNV_PRIME: u64 = 0x0100_0000_01b3;

        let scalars = [
            self.position.x,
            self.position.y,
            self.position.z,
            self.linear_momentum.x,
            self.linear_momentum.y,
            self.linear_momentum.z,
            self.orientation.x,
            self.orientation.y,
            self.orientation.z,
            self.orientation.w,
            self.angular_momentum.x,
            self.angular_momentum.y,
            self.angular_momentum.z,
            self.force.x,
            self.force.y,
            self.force.z,
            self.torque.x,
            self.torque.y,
            self.torque.z,
        ];

        let mut hash = FNV_OFFSET;
        for v in scalars {
            // Canonicalize negative zero (-0.0 == +0.0) before hashing raw bits.
            let canon = if v == 0.0 { 0.0_f64 } else { v };
            for byte in canon.to_bits().to_le_bytes() {
                hash ^= u64::from(byte);
                hash = hash.wrapping_mul(FNV_PRIME);
            }
        }
        Ok(hash)
    }

    // -----------------------------------------------------------------------
    // Core Integrator Steps (zero-allocation, atomic error preservation)
    // -----------------------------------------------------------------------

    /// Advance the rigid-body state forward by `dt` seconds using `integrator`
    /// under the constant accumulated `self.force` and `self.torque`.
    ///
    /// Zero heap allocation; preserves `*self` unmodified on any error.
    pub fn step_in_place(
        &mut self,
        mass: &MassProperties,
        dt: f64,
        integrator: IntegratorKind,
    ) -> Result<(), RigidBodyError> {
        let mut candidate = *self;
        candidate.body_validate()?;
        mass.mass_props_validate()?;
        let dt = check_dt(dt)?;
        let f = candidate.force;
        let t = candidate.torque;
        Self::integrate_step_with_eval(&mut candidate, mass, dt, integrator, &mut |_s| Ok((f, t)))?;
        candidate.body_validate()?;
        *self = candidate;
        Ok(())
    }

    /// Advance the rigid-body state forward by `dt` seconds using `integrator`
    /// and a caller-provided state-dependent `(world_force, world_torque)` evaluator.
    ///
    /// Zero heap allocation; preserves `*self` unmodified on any error.
    pub fn step_with_force_fn_in_place<F>(
        &mut self,
        mass: &MassProperties,
        dt: f64,
        integrator: IntegratorKind,
        mut force_fn: F,
    ) -> Result<(), RigidBodyError>
    where
        F: FnMut(&RigidBodyState) -> Result<(Vec3, Vec3), RigidBodyError>,
    {
        let mut candidate = *self;
        candidate.body_validate()?;
        mass.mass_props_validate()?;
        let dt = check_dt(dt)?;
        Self::integrate_step_with_eval(&mut candidate, mass, dt, integrator, &mut force_fn)?;
        candidate.body_validate()?;
        *self = candidate;
        Ok(())
    }

    /// Advance the rigid-body state forward by `dt` seconds with ground contact
    /// resolution across `body_probes` (defined in body-local coordinates).
    ///
    /// Zero heap allocation; preserves `*self` unmodified on any error.
    pub fn step_with_contacts_in_place(
        &mut self,
        mass: &MassProperties,
        dt: f64,
        integrator: IntegratorKind,
        contact: &GroundContactParams,
        body_probes: &[Vec3],
    ) -> Result<(), RigidBodyError> {
        let _ =
            self.step_with_contacts_report_in_place(mass, dt, integrator, contact, body_probes)?;
        Ok(())
    }

    /// Advance the rigid-body state forward by `dt` seconds with ground contact
    /// resolution and return contact telemetry [`ContactStepReport`].
    ///
    /// Zero heap allocation; preserves `*self` unmodified on any error.
    pub fn step_with_contacts_report_in_place(
        &mut self,
        mass: &MassProperties,
        dt: f64,
        integrator: IntegratorKind,
        contact: &GroundContactParams,
        body_probes: &[Vec3],
    ) -> Result<ContactStepReport, RigidBodyError> {
        let mut candidate = *self;
        candidate.body_validate()?;
        mass.mass_props_validate()?;
        let dt = check_dt(dt)?;
        let plane = contact.contact_params_validate()?;
        if body_probes.len() > MAX_CONTACT_POINTS {
            return Err(RigidBodyError::TooManyContactPoints);
        }
        for &probe in body_probes {
            check_bounded_vec3(probe)?;
        }

        let substeps = contact.substeps;
        let sub_dt = dt / (substeps as f64);
        let base_force = candidate.force;
        let base_torque = candidate.torque;

        let mut report = ContactStepReport {
            active_contacts: 0,
            max_penetration: 0.0,
            max_normal_force: 0.0,
        };

        for _ in 0..substeps {
            match contact.model {
                ContactModel::Penalty {
                    stiffness,
                    damping,
                    friction,
                } => {
                    Self::integrate_step_with_eval(
                        &mut candidate,
                        mass,
                        sub_dt,
                        integrator,
                        &mut |s| {
                            let (cf, ct, _, _, _) = Self::eval_penalty_contacts(
                                s,
                                mass,
                                &plane,
                                body_probes,
                                stiffness,
                                damping,
                                friction,
                            )?;
                            Ok((
                                vec3_add_checked(base_force, cf)?,
                                vec3_add_checked(base_torque, ct)?,
                            ))
                        },
                    )?;
                    let (_, _, active, max_pen, max_fn) = Self::eval_penalty_contacts(
                        &candidate,
                        mass,
                        &plane,
                        body_probes,
                        stiffness,
                        damping,
                        friction,
                    )?;
                    report.active_contacts = report.active_contacts.saturating_add(active);
                    if max_pen > report.max_penetration {
                        report.max_penetration = max_pen;
                    }
                    if max_fn > report.max_normal_force {
                        report.max_normal_force = max_fn;
                    }
                }
                ContactModel::Impulse {
                    restitution,
                    friction,
                    position_correction,
                } => {
                    Self::integrate_step_with_eval(
                        &mut candidate,
                        mass,
                        sub_dt,
                        integrator,
                        &mut |_s| Ok((base_force, base_torque)),
                    )?;
                    let (active, max_pen, max_jn) = Self::resolve_impulse_contacts(
                        &mut candidate,
                        mass,
                        &plane,
                        body_probes,
                        restitution,
                        friction,
                        position_correction,
                    )?;
                    report.active_contacts = report.active_contacts.saturating_add(active);
                    if max_pen > report.max_penetration {
                        report.max_penetration = max_pen;
                    }
                    if max_jn > report.max_normal_force {
                        report.max_normal_force = max_jn;
                    }
                }
            }
        }

        candidate.body_validate()?;
        *self = candidate;
        Ok(report)
    }

    fn eval_derivative<F>(
        state: &RigidBodyState,
        mass: &MassProperties,
        force_fn: &mut F,
    ) -> Result<StateDeriv, RigidBodyError>
    where
        F: FnMut(&RigidBodyState) -> Result<(Vec3, Vec3), RigidBodyError>,
    {
        let (f_world, tau_world) = force_fn(state)?;
        check_bounded_vec3(f_world)?;
        check_bounded_vec3(tau_world)?;

        let q_raw = state.orientation;
        let dx = vec3_scale_checked(state.linear_momentum, mass.inv_mass)?;
        let inv_i_world = Self::world_inv_inertia_for_quat(q_raw, mass.inv_inertia_body)?;
        let omega = check_bounded_vec3(inv_i_world.mat3_mul_vec3(state.angular_momentum)?)?;
        let dq = quat_derivative_world(omega, q_raw)?;

        Ok(StateDeriv {
            dx,
            dp: f_world,
            dq,
            dl: tau_world,
        })
    }

    fn integrate_step_with_eval<F>(
        state: &mut RigidBodyState,
        mass: &MassProperties,
        dt: f64,
        integrator: IntegratorKind,
        force_fn: &mut F,
    ) -> Result<(), RigidBodyError>
    where
        F: FnMut(&RigidBodyState) -> Result<(Vec3, Vec3), RigidBodyError>,
    {
        match integrator {
            IntegratorKind::VelocityVerlet => {
                let (f0, tau0) = force_fn(state)?;
                check_bounded_vec3(f0)?;
                check_bounded_vec3(tau0)?;
                let half_dt = 0.5 * dt;

                // 1. Half-step momenta.
                let p_half =
                    vec3_add_checked(state.linear_momentum, vec3_scale_checked(f0, half_dt)?)?;
                let l_half =
                    vec3_add_checked(state.angular_momentum, vec3_scale_checked(tau0, half_dt)?)?;

                // 2. Full-step position and symplectic midpoint orientation.
                let v_half = vec3_scale_checked(p_half, mass.inv_mass)?;
                let x_next = vec3_add_checked(state.position, vec3_scale_checked(v_half, dt)?)?;

                let q0 = state.orientation.quat_normalize()?;
                let inv_i0 = Self::world_inv_inertia_for_quat(q0, mass.inv_inertia_body)?;
                let omega0 = check_bounded_vec3(inv_i0.mat3_mul_vec3(l_half)?)?;
                let q_mid = advance_quat_exponential(q0, omega0, half_dt)?;
                let inv_imid = Self::world_inv_inertia_for_quat(q_mid, mass.inv_inertia_body)?;
                let omega_mid = check_bounded_vec3(inv_imid.mat3_mul_vec3(l_half)?)?;
                let q_next = advance_quat_exponential(q0, omega_mid, dt)?;

                // 3. Evaluate forces at updated configuration & half-step momenta.
                let mid_state = RigidBodyState {
                    position: x_next,
                    linear_momentum: p_half,
                    orientation: q_next,
                    angular_momentum: l_half,
                    force: state.force,
                    torque: state.torque,
                };
                let (f1, tau1) = force_fn(&mid_state)?;
                check_bounded_vec3(f1)?;
                check_bounded_vec3(tau1)?;

                // 4. Second half-step momenta.
                let p_next = vec3_add_checked(p_half, vec3_scale_checked(f1, half_dt)?)?;
                let l_next = vec3_add_checked(l_half, vec3_scale_checked(tau1, half_dt)?)?;

                state.position = x_next;
                state.linear_momentum = p_next;
                state.orientation = q_next;
                state.angular_momentum = l_next;
                Ok(())
            }
            IntegratorKind::RK4 => {
                let s0 = RigidBodyState {
                    orientation: state.orientation.quat_normalize()?,
                    ..*state
                };
                let k1 = Self::eval_derivative(&s0, mass, force_fn)?;

                let half_dt = 0.5 * dt;
                let s2 = RigidBodyState {
                    position: vec3_add_checked(s0.position, vec3_scale_checked(k1.dx, half_dt)?)?,
                    linear_momentum: vec3_add_checked(
                        s0.linear_momentum,
                        vec3_scale_checked(k1.dp, half_dt)?,
                    )?,
                    orientation: quat_add_scaled(s0.orientation, k1.dq, half_dt)?,
                    angular_momentum: vec3_add_checked(
                        s0.angular_momentum,
                        vec3_scale_checked(k1.dl, half_dt)?,
                    )?,
                    force: s0.force,
                    torque: s0.torque,
                };
                let k2 = Self::eval_derivative(&s2, mass, force_fn)?;

                let s3 = RigidBodyState {
                    position: vec3_add_checked(s0.position, vec3_scale_checked(k2.dx, half_dt)?)?,
                    linear_momentum: vec3_add_checked(
                        s0.linear_momentum,
                        vec3_scale_checked(k2.dp, half_dt)?,
                    )?,
                    orientation: quat_add_scaled(s0.orientation, k2.dq, half_dt)?,
                    angular_momentum: vec3_add_checked(
                        s0.angular_momentum,
                        vec3_scale_checked(k2.dl, half_dt)?,
                    )?,
                    force: s0.force,
                    torque: s0.torque,
                };
                let k3 = Self::eval_derivative(&s3, mass, force_fn)?;

                let s4 = RigidBodyState {
                    position: vec3_add_checked(s0.position, vec3_scale_checked(k3.dx, dt)?)?,
                    linear_momentum: vec3_add_checked(
                        s0.linear_momentum,
                        vec3_scale_checked(k3.dp, dt)?,
                    )?,
                    orientation: quat_add_scaled(s0.orientation, k3.dq, dt)?,
                    angular_momentum: vec3_add_checked(
                        s0.angular_momentum,
                        vec3_scale_checked(k3.dl, dt)?,
                    )?,
                    force: s0.force,
                    torque: s0.torque,
                };
                let k4 = Self::eval_derivative(&s4, mass, force_fn)?;

                let w6 = dt / 6.0;
                let combine_vec =
                    |a: Vec3, b: Vec3, c: Vec3, d: Vec3| -> Result<Vec3, RigidBodyError> {
                        let sum = Vec3::new(
                            a.x + 2.0 * b.x + 2.0 * c.x + d.x,
                            a.y + 2.0 * b.y + 2.0 * c.y + d.y,
                            a.z + 2.0 * b.z + 2.0 * c.z + d.z,
                        );
                        vec3_scale_checked(sum, w6)
                    };

                let dx = combine_vec(k1.dx, k2.dx, k3.dx, k4.dx)?;
                let dp = combine_vec(k1.dp, k2.dp, k3.dp, k4.dp)?;
                let dl = combine_vec(k1.dl, k2.dl, k3.dl, k4.dl)?;
                let dq = Quat::quat_from_xyzw(
                    s0.orientation.x + w6 * (k1.dq.x + 2.0 * k2.dq.x + 2.0 * k3.dq.x + k4.dq.x),
                    s0.orientation.y + w6 * (k1.dq.y + 2.0 * k2.dq.y + 2.0 * k3.dq.y + k4.dq.y),
                    s0.orientation.z + w6 * (k1.dq.z + 2.0 * k2.dq.z + 2.0 * k3.dq.z + k4.dq.z),
                    s0.orientation.w + w6 * (k1.dq.w + 2.0 * k2.dq.w + 2.0 * k3.dq.w + k4.dq.w),
                )?;

                state.position = vec3_add_checked(s0.position, dx)?;
                state.linear_momentum = vec3_add_checked(s0.linear_momentum, dp)?;
                state.orientation = dq.quat_normalize()?;
                state.angular_momentum = vec3_add_checked(s0.angular_momentum, dl)?;
                Ok(())
            }
        }
    }

    fn eval_penalty_contacts(
        state: &RigidBodyState,
        mass: &MassProperties,
        plane: &Plane,
        body_probes: &[Vec3],
        stiffness: f64,
        damping: f64,
        friction: f64,
    ) -> Result<(Vec3, Vec3, usize, f64, f64), RigidBodyError> {
        let q = state.orientation.quat_normalize()?;
        let v_cm = vec3_scale_checked(state.linear_momentum, mass.inv_mass)?;
        let inv_i_world = Self::world_inv_inertia_for_quat(q, mass.inv_inertia_body)?;
        let omega = check_bounded_vec3(inv_i_world.mat3_mul_vec3(state.angular_momentum)?)?;

        let mut total_force = Vec3::zero();
        let mut total_torque = Vec3::zero();
        let mut active_contacts = 0usize;
        let mut max_penetration = 0.0_f64;
        let mut max_normal_force = 0.0_f64;

        for &probe_body in body_probes {
            let r_world = check_bounded_vec3(q.rotate_vec3(probe_body)?)?;
            let p_world = vec3_add_checked(state.position, r_world)?;
            let signed_dist = check_finite(plane.signed_distance(p_world)?)?;
            if signed_dist >= 0.0 {
                continue;
            }

            active_contacts += 1;
            let penetration = -signed_dist;
            if penetration > max_penetration {
                max_penetration = penetration;
            }

            let v_probe = vec3_add_checked(v_cm, omega.cross(r_world)?)?;
            let v_n = check_finite(v_probe.dot(plane.normal)?)?;
            let f_n_raw = check_finite(stiffness * penetration - damping * v_n)?;
            let f_n = if f_n_raw > 0.0 { f_n_raw } else { 0.0 };
            if f_n > max_normal_force {
                max_normal_force = f_n;
            }
            if f_n <= 0.0 {
                continue;
            }

            let f_normal = vec3_scale_checked(plane.normal, f_n)?;
            let v_t = v_probe.vec3_sub(plane.normal.vec3_scale(v_n)?)?;
            let speed_t = check_finite(v_t.length()?)?;
            let f_tangent = if friction > 0.0 && speed_t > EPSILON {
                let ramp = (speed_t / FRICTION_SLIP_THRESHOLD).min(1.0);
                let f_t_mag = friction * f_n * ramp;
                vec3_scale_checked(v_t, -f_t_mag / speed_t)?
            } else {
                Vec3::zero()
            };

            let f_probe = vec3_add_checked(f_normal, f_tangent)?;
            let tau_probe = check_bounded_vec3(r_world.cross(f_probe)?)?;
            total_force = vec3_add_checked(total_force, f_probe)?;
            total_torque = vec3_add_checked(total_torque, tau_probe)?;
        }

        Ok((
            total_force,
            total_torque,
            active_contacts,
            max_penetration,
            max_normal_force,
        ))
    }

    fn resolve_impulse_contacts(
        state: &mut RigidBodyState,
        mass: &MassProperties,
        plane: &Plane,
        body_probes: &[Vec3],
        restitution: f64,
        friction: f64,
        position_correction: f64,
    ) -> Result<(usize, f64, f64), RigidBodyError> {
        let q = state.orientation.quat_normalize()?;
        let inv_i_world = Self::world_inv_inertia_for_quat(q, mass.inv_inertia_body)?;
        let mut active_contacts = 0usize;
        let mut max_penetration = 0.0_f64;
        let mut max_normal_impulse = 0.0_f64;

        for &probe_body in body_probes {
            let r_world = check_bounded_vec3(q.rotate_vec3(probe_body)?)?;
            let p_world = vec3_add_checked(state.position, r_world)?;
            let signed_dist = check_finite(plane.signed_distance(p_world)?)?;
            if signed_dist > 0.0 {
                continue;
            }

            active_contacts += 1;
            let penetration = -signed_dist;
            if penetration > max_penetration {
                max_penetration = penetration;
            }

            let v_cm = vec3_scale_checked(state.linear_momentum, mass.inv_mass)?;
            let omega = check_bounded_vec3(inv_i_world.mat3_mul_vec3(state.angular_momentum)?)?;
            let v_probe = vec3_add_checked(v_cm, omega.cross(r_world)?)?;
            let v_n = check_finite(v_probe.dot(plane.normal)?)?;

            if v_n >= 0.0 {
                continue;
            }

            let eff_restitution = if v_n < -RESTITUTION_VEL_THRESHOLD {
                restitution
            } else {
                0.0
            };

            let r_cross_n = r_world.cross(plane.normal)?;
            let ang_term_n = r_cross_n.dot(inv_i_world.mat3_mul_vec3(r_cross_n)?)?;
            let inv_m_eff_n = check_finite(mass.inv_mass + ang_term_n)?;
            if inv_m_eff_n <= EPSILON {
                return Err(RigidBodyError::NumericalDivergence);
            }

            let j_n = check_finite(-(1.0 + eff_restitution) * v_n / inv_m_eff_n)?;
            if j_n <= 0.0 {
                continue;
            }
            if j_n > max_normal_impulse {
                max_normal_impulse = j_n;
            }

            let j_n_vec = vec3_scale_checked(plane.normal, j_n)?;
            state.linear_momentum = vec3_add_checked(state.linear_momentum, j_n_vec)?;
            state.angular_momentum = vec3_add_checked(
                state.angular_momentum,
                check_bounded_vec3(r_world.cross(j_n_vec)?)?,
            )?;

            if friction > 0.0 {
                let v_cm_after = vec3_scale_checked(state.linear_momentum, mass.inv_mass)?;
                let omega_after =
                    check_bounded_vec3(inv_i_world.mat3_mul_vec3(state.angular_momentum)?)?;
                let v_after = vec3_add_checked(v_cm_after, omega_after.cross(r_world)?)?;
                let v_n_after = check_finite(v_after.dot(plane.normal)?)?;
                let v_t = v_after.vec3_sub(plane.normal.vec3_scale(v_n_after)?)?;
                let speed_t = check_finite(v_t.length()?)?;
                if speed_t > EPSILON {
                    let t_dir = v_t.vec3_scale(1.0 / speed_t)?;
                    let r_cross_t = r_world.cross(t_dir)?;
                    let ang_term_t = r_cross_t.dot(inv_i_world.mat3_mul_vec3(r_cross_t)?)?;
                    let inv_m_eff_t = check_finite(mass.inv_mass + ang_term_t)?;
                    if inv_m_eff_t > EPSILON {
                        let j_t_mag = (speed_t / inv_m_eff_t).min(friction * j_n);
                        let j_t_vec = vec3_scale_checked(t_dir, -j_t_mag)?;
                        state.linear_momentum = vec3_add_checked(state.linear_momentum, j_t_vec)?;
                        state.angular_momentum = vec3_add_checked(
                            state.angular_momentum,
                            check_bounded_vec3(r_world.cross(j_t_vec)?)?,
                        )?;
                    }
                }
            }
        }

        if position_correction > 0.0 && max_penetration > PENETRATION_SLOP {
            let corr = position_correction * (max_penetration - PENETRATION_SLOP);
            state.position =
                vec3_add_checked(state.position, vec3_scale_checked(plane.normal, corr)?)?;
        }

        Ok((active_contacts, max_penetration, max_normal_impulse))
    }
}
