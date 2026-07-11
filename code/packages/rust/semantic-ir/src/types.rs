//! SIR type carriers.
//!
//! Per SIR10, the SIR carries optional type information but does
//! **not** infer or verify it.  A frontend either supplies a
//! `SirType` or leaves the slot `None`; the IR round-trips that
//! decision faithfully.  Backends may use the type for narrowing or
//! reject contradictions, but they may not synthesize missing types.
//!
//! # SIR21 — the type is the semantics
//!
//! The v0 type set was a flat enum where `Int` meant a vague
//! "64-bit-ish integer".  That is enough to transpile Ruby → {Python,
//! JS, Go, Rust} where every value is boxed and every integer is
//! arbitrary precision.  It is **not** enough for two things we now
//! want (see [SIR21](../../../../specs/SIR21-type-system-and-integer-semantics.md)):
//!
//!   1. Targets where the machine integer matters (C's `int32_t` wraps,
//!      `size_t` is unsigned, signed overflow is UB).
//!   2. Sources that already carry types (C, typed Python, Java, C#).
//!
//! Both hinge on the *same* missing information, so carrying it makes
//! both sound at once.  The keystone is that an integer is no longer a
//! bare `Int` — it is an [`IntSpec`] of `(width, signed, overflow)`,
//! and the min/max/modulus bounds are a *pure function* of that spec
//! (they are never stored).  A `u32{wrap}` add on Ruby emits
//! `(a + b) & 0xFFFFFFFF`; the same spec on C emits a native
//! `uint32_t`; **the type prescribes the observable result**.
//!
//! This module is SIR21 milestone **T1a** (the Phase-0 mechanical
//! remap): it renames the top type `Any → Dynamic` and widens
//! `Int → Int { spec }`, defaulting the old flat `Int` to an
//! arbitrary-precision spec so every existing module lowers
//! *identically*.  Nothing here selects operations or masks values yet
//! — that is T3 and the per-backend milestones.  This is a carrier,
//! still not a verifier.

use std::fmt;

/// The bit-width of an integer.
///
/// `Arbitrary` is the dynamic-language integer (Ruby/Python): it never
/// overflows, it grows.  It is the *only* width a purely-dynamic
/// frontend emits, and the bridge that a `Bignum`-declaring backend
/// must support.  The fixed widths exist for typed sources/targets.
///
/// | variant     | bits | example native type            |
/// |-------------|------|--------------------------------|
/// | `W8`        | 8    | `i8` / `u8`, C `char`          |
/// | `W16`       | 16   | `i16` / `u16`, C `short`       |
/// | `W32`       | 32   | `i32` / `u32`, C `int`         |
/// | `W64`       | 64   | `i64` / `u64`, C `long long`   |
/// | `W128`      | 128  | `i128` / `u128`               |
/// | `Arbitrary` | ∞    | Ruby `Integer`, Python `int`, JS `BigInt` |
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IntWidth {
    W8,
    W16,
    W32,
    W64,
    W128,
    Arbitrary,
}

impl IntWidth {
    /// The number of bits, or `None` for `Arbitrary` (unbounded).
    pub fn bits(self) -> Option<u32> {
        match self {
            IntWidth::W8 => Some(8),
            IntWidth::W16 => Some(16),
            IntWidth::W32 => Some(32),
            IntWidth::W64 => Some(64),
            IntWidth::W128 => Some(128),
            IntWidth::Arbitrary => None,
        }
    }
}

impl fmt::Display for IntWidth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IntWidth::W8 => write!(f, "8"),
            IntWidth::W16 => write!(f, "16"),
            IntWidth::W32 => write!(f, "32"),
            IntWidth::W64 => write!(f, "64"),
            IntWidth::W128 => write!(f, "128"),
            IntWidth::Arbitrary => write!(f, "arb"),
        }
    }
}

/// What happens when an integer operation exceeds its width.
///
/// This is the crux of "the type is the semantics": two languages that
/// write `a + b` mean *different things* when the result overflows, and
/// SIR records which.
///
/// | variant     | on overflow                    | source example            |
/// |-------------|--------------------------------|---------------------------|
/// | `Wrap`      | modular `2^n`                  | C unsigned; masking target |
/// | `Trap`      | panic / raise                  | Swift; Rust debug          |
/// | `Saturate`  | clamp to `min`/`max`           | DSP; Rust `saturating_*`   |
/// | `Checked`   | produce `Optional`/`None`      | Rust `checked_*`           |
/// | `Undefined` | UB — backend MAY choose, MUST record | C signed overflow    |
/// | `Arbitrary` | never overflows, grows         | Ruby/Python (width=`Arbitrary` only) |
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Overflow {
    Wrap,
    Trap,
    Saturate,
    Checked,
    Undefined,
    Arbitrary,
}

impl fmt::Display for Overflow {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Overflow::Wrap => "wrap",
            Overflow::Trap => "trap",
            Overflow::Saturate => "sat",
            Overflow::Checked => "checked",
            Overflow::Undefined => "ub",
            Overflow::Arbitrary => "arb",
        };
        write!(f, "{}", s)
    }
}

/// The full descriptor of an integer type: width × signedness ×
/// overflow behaviour.
///
/// **Min/max/modulus are derived, never stored.**  For a concrete
/// `(width, signed)` the bounds are a pure function (see [`Self::min`],
/// [`Self::max`], [`Self::modulus`]); storing them would be redundant
/// state that could drift.  A program that *reflects* on limits (C's
/// `INT_MAX`, Rust's `i32::MAX`) reads them through those functions,
/// which const-fold at emit time.
///
/// # Examples
///
/// ```
/// use semantic_ir::types::{IntSpec, IntWidth, Overflow};
///
/// // C's `unsigned int`
/// let u32_wrap = IntSpec::sized(IntWidth::W32, false, Overflow::Wrap);
/// assert_eq!(u32_wrap.min(), Some(0));
/// assert_eq!(u32_wrap.max(), Some(4_294_967_295));
/// assert_eq!(u32_wrap.modulus(), Some(1u128 << 32));
///
/// // Ruby / Python integer: never overflows, grows
/// let big = IntSpec::arbitrary();
/// assert_eq!(big.max(), None);        // unbounded
/// assert_eq!(big.modulus(), None);    // no wrap
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IntSpec {
    pub width: IntWidth,
    pub signed: bool,
    pub overflow: Overflow,
}

impl IntSpec {
    /// The arbitrary-precision integer — Ruby/Python semantics.
    ///
    /// This is the default the v0 flat `Int` maps to: the historical
    /// dynamic pipeline never masked integers (Ruby → Python both have
    /// growing integers), so the *faithful* default is `Arbitrary`, not
    /// a fixed 64-bit width.  A frontend that means a machine `i64`
    /// must now say so explicitly via [`Self::sized`].
    pub const fn arbitrary() -> Self {
        IntSpec {
            width: IntWidth::Arbitrary,
            signed: true,
            overflow: Overflow::Arbitrary,
        }
    }

    /// A fixed-width integer with an explicit overflow mode.
    pub const fn sized(width: IntWidth, signed: bool, overflow: Overflow) -> Self {
        IntSpec {
            width,
            signed,
            overflow,
        }
    }

    /// `true` iff this is the arbitrary-precision (dynamic) integer.
    pub fn is_arbitrary(&self) -> bool {
        matches!(self.width, IntWidth::Arbitrary)
    }

    /// The smallest representable value, or `None` if unbounded
    /// (`Arbitrary`).  Signed: `-2^(n-1)`; unsigned: `0`.
    pub fn min(&self) -> Option<i128> {
        let bits = self.width.bits()?;
        if !self.signed {
            return Some(0);
        }
        // -(2^(bits-1)).  For 128-bit the *positive* intermediate 2^127
        // is unrepresentable in i128 (max is 2^127−1), but the answer
        // −2^127 == `i128::MIN` is — so special-case rather than
        // computing `-(1i128 << 127)`, which negates `i128::MIN` and
        // panics on overflow in debug builds.
        Some(if bits >= 128 {
            i128::MIN
        } else {
            -(1i128 << (bits - 1))
        })
    }

    /// The largest representable value, or `None` if unbounded.
    /// Signed: `2^(n-1) - 1`; unsigned: `2^n - 1`.
    pub fn max(&self) -> Option<i128> {
        let bits = self.width.bits()?;
        if self.signed {
            // 2^(bits-1) − 1.  Same 128-bit corner as `min`: the answer
            // 2^127−1 == `i128::MAX` is representable but the naive
            // `(1i128 << 127) − 1` overflows the intermediate.
            Some(if bits >= 128 {
                i128::MAX
            } else {
                (1i128 << (bits - 1)) - 1
            })
        } else {
            // 2^n - 1.  Fits i128 for n ≤ 127; W128 unsigned max is the
            // one case that would overflow i128, so compute in u128 and
            // saturate the report to i128::MAX for that corner (a
            // reflection query on u128 is vanishingly rare and refined
            // later; the wrap math itself uses `modulus`, not `max`).
            let m = 1u128.checked_shl(bits).map(|v| v - 1).unwrap_or(u128::MAX);
            Some(m.min(i128::MAX as u128) as i128)
        }
    }

    /// The wrap modulus `2^n`, or `None` if unbounded.
    pub fn modulus(&self) -> Option<u128> {
        let bits = self.width.bits()?;
        Some(1u128.checked_shl(bits).unwrap_or(0)) // W128 → 2^128 overflows u128 → 0 sentinel (unused; masking uses bit-width ops)
    }
}

impl fmt::Display for IntSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // The default arbitrary-precision spec prints as bare `int` so
        // that pre-SIR21 modules round-trip their text unchanged.  Any
        // other spec prints its full `(int <sign><width> <overflow>)`
        // shape, e.g. `(int u32 wrap)`, `(int i64 trap)`.
        if *self == IntSpec::arbitrary() {
            return write!(f, "int");
        }
        let sign = if self.signed { "i" } else { "u" };
        write!(f, "(int {}{} {})", sign, self.width, self.overflow)
    }
}

/// A SIR type — purely a carrier, never inferred by the IR itself.
///
/// `Dynamic` is the top type and the default; every other variant is a
/// more-specific classification a frontend *may* supply.  A wholly-
/// `Dynamic` module is exactly the pre-SIR21 behaviour: boxed values,
/// runtime dispatch.
///
/// | Variant   | Meaning                                              |
/// |-----------|------------------------------------------------------|
/// | `Dynamic` | top type; unknown/any; the default (was `Any`)        |
/// | `Int(spec)` | integer with `(width, signed, overflow)` — see [`IntSpec`] |
/// | `Bool`    | boolean (true/false)                                  |
/// | `Nil`     | the singleton `nil` value                             |
/// | `Symbol`  | interned symbol (`'foo` in Twig)                      |
/// | `Str`     | string                                                |
/// | `Pair`    | cons cell (heap pair of two values)                   |
/// | `Closure` | any closure handle                                    |
/// | `Fn { params, ret }` | function type with typed params / return   |
/// | `Float`   | 64-bit IEEE-754 float (SIR16)                         |
/// | `Seq(elem)` | homogeneous sequence (`list`/`Array`/`Vec`) (SIR16) |
/// | `Map(val)` | string-keyed map (`dict`/`Object`/`HashMap`) (SIR16) |
/// | `Ptr { pointee, nullable }` | C/C++ pointer or reference (SIR21 T1b) |
/// | `Struct { name, fields }`   | nominal record / struct    (SIR21 T1b) |
/// | `Optional { inner }`        | nullable `T`-or-nil        (SIR21 T1b) |
/// | `NDArray { elem, rank }`    | dense N-D numeric array     (SIR22)    |
/// | `Rational`                  | exact rational scalar       (SIR22/SIR23) |
/// | `Complex`                   | complex scalar `{re, im}`   (SIR22/SIR23) |
/// | `SymExpr`                   | opaque symbolic-expression handle (SIR23) |
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SirType {
    /// Top type — unknown/any.  The default; renamed from `Any` in SIR21.
    Dynamic,
    /// Integer carrying its width/signedness/overflow semantics.
    Int(IntSpec),
    Bool,
    Nil,
    Symbol,
    Str,
    Pair,
    Closure,
    Fn {
        params: Vec<SirType>,
        ret: Box<SirType>,
    },
    // ── SIR16 (Python / JavaScript interop) ──────────────────────────
    /// 64-bit IEEE-754 float carrier.
    Float,
    /// Homogeneous sequence (`list`/`Array`/`Vec`-style).
    Seq(Box<SirType>),
    /// String-keyed map carrier (`dict`/`Object`/`HashMap`-style).
    Map(Box<SirType>),
    // ── SIR21 T1b (source-fidelity types for typed frontends) ────────
    /// A pointer or reference (C/C++ `T*`, `T&`).  Exists primarily for
    /// *source* fidelity — a C frontend needs it to record pointer shape
    /// (not aliasing/ownership).  `nullable` distinguishes a possibly-
    /// null pointer from a reference that is guaranteed non-null.
    /// Dynamic targets (Ruby/JS) lower `Ptr` to a plain reference;
    /// targets without pointers declare so in their manifest and reject.
    Ptr {
        pointee: Box<SirType>,
        nullable: bool,
    },
    /// A nominal record — a C `struct`, or a class's field bag.  `name`
    /// is the declared type name; `fields` are `(field-name, type)` in
    /// declaration order (order matters for C layout / positional init).
    /// Dynamic targets lower it to a record/object; targets without
    /// structs reject via the manifest.
    Struct {
        name: String,
        fields: Vec<(String, SirType)>,
    },
    /// A nullable value: `inner`-or-nil.  The type-level counterpart of
    /// an `Optional`/`Maybe`/`T?`.  Distinct from `Ptr { nullable }`
    /// (which is specifically a pointer) — `Optional` wraps *any* type.
    Optional {
        inner: Box<SirType>,
    },
    // ── SIR22 (array/matrix numeric-language IR extension) ───────────
    /// A dense N-dimensional numeric array (the MATLAB/Octave
    /// "everything is a matrix" model — see
    /// [SIR22](../../../../specs/SIR22-array-matrix-semantic-ir.md)).
    /// `rank` is `None` when a frontend cannot prove the dimensionality
    /// statically ("unknown/dynamic rank" per the spec); backends must
    /// handle the absent case explicitly rather than inferring one.
    /// Storage order (row- vs. column-major) is *not* part of the type —
    /// it is a manifest-level fact (`Feature::ArrayColumnMajor`), because
    /// it's a representation choice, not part of the array's shape.
    NDArray {
        elem: Box<SirType>,
        rank: Option<usize>,
    },
    /// An exact rational scalar (arbitrary-precision numerator/
    /// denominator).  This is a type-level carrier only — SIR22 adds no
    /// numerator/denominator *storage* at the type level (that's a
    /// runtime concern for the backend); the pair representation itself
    /// is shared with the symbolic-math extension
    /// [SIR23](../../../../specs/SIR23-symbolic-pattern-semantic-ir.md),
    /// landed once here rather than twice.
    Rational,
    /// A complex scalar (`{ re: f64, im: f64 }`).  Like `Rational`, this
    /// is a type-level carrier with no additional fields — the value
    /// representation is a backend/runtime concern — shared with the
    /// SIR23 symbolic extension.
    Complex,
    // ── SIR23 (symbolic expression + pattern/rewrite IR extension) ───
    /// An opaque symbolic-expression handle — mirrors
    /// `symbolic_ir::IRNode`'s own dynamically-shaped tree.  Carries no
    /// static shape: a value of this type may be a bare symbol, a
    /// number, or an arbitrarily nested `head[args…]` application, none
    /// of which the type itself distinguishes (the shape lives in the
    /// `Expr` tree — `SymSymbol`/`SymRational`/`SymApply`/etc. — not in
    /// the type carrier). See
    /// [SIR23](../../../../specs/SIR23-symbolic-pattern-semantic-ir.md).
    SymExpr,
}

impl SirType {
    /// Convenience constructor for `Fn`.
    pub fn function(params: Vec<SirType>, ret: SirType) -> Self {
        SirType::Fn {
            params,
            ret: Box::new(ret),
        }
    }

    /// The arbitrary-precision integer — the default `Int` (Ruby/Python
    /// semantics).  This is what the pre-SIR21 flat `Int` variant meant
    /// in the dynamic pipeline, so it is the behaviour-preserving remap.
    pub const fn int_default() -> Self {
        SirType::Int(IntSpec::arbitrary())
    }

    /// A fixed-width integer type.
    pub const fn int(width: IntWidth, signed: bool, overflow: Overflow) -> Self {
        SirType::Int(IntSpec::sized(width, signed, overflow))
    }

    /// `true` iff this is the top type `Dynamic`.
    pub fn is_dynamic(&self) -> bool {
        matches!(self, SirType::Dynamic)
    }

    /// A pointer/reference to `pointee`.  `nullable` marks a possibly-
    /// null pointer (vs a guaranteed non-null reference).
    pub fn ptr(pointee: SirType, nullable: bool) -> Self {
        SirType::Ptr {
            pointee: Box::new(pointee),
            nullable,
        }
    }

    /// A nominal record `name` with ordered `(field, type)` members.
    pub fn struct_type(name: impl Into<String>, fields: Vec<(String, SirType)>) -> Self {
        SirType::Struct {
            name: name.into(),
            fields,
        }
    }

    /// A nullable `inner`-or-nil value.
    pub fn optional(inner: SirType) -> Self {
        SirType::Optional {
            inner: Box::new(inner),
        }
    }

    /// A dense N-D numeric array of `elem`.  `rank = None` means
    /// unknown/dynamic rank (SIR22).
    pub fn ndarray(elem: SirType, rank: Option<usize>) -> Self {
        SirType::NDArray {
            elem: Box::new(elem),
            rank,
        }
    }
}

impl fmt::Display for SirType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // The *enum variant* is `Dynamic` (SIR21 rename), but the
            // SIR text-format surface keyword for the top type stays
            // `any` — it is a serialization surface with no reader, and
            // every existing printed module / golden test uses `any`.
            // A `None` type slot (printer `type_or_any`) and an explicit
            // `Dynamic` therefore print identically, as they should.
            SirType::Dynamic => write!(f, "any"),
            SirType::Int(spec) => write!(f, "{}", spec),
            SirType::Bool => write!(f, "bool"),
            SirType::Nil => write!(f, "nil"),
            SirType::Symbol => write!(f, "symbol"),
            SirType::Str => write!(f, "str"),
            SirType::Pair => write!(f, "pair"),
            SirType::Closure => write!(f, "closure"),
            SirType::Fn { params, ret } => {
                write!(f, "(fn (")?;
                for (i, p) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "{}", p)?;
                }
                write!(f, ") {})", ret)
            }
            SirType::Float => write!(f, "float"),
            SirType::Seq(elem) => write!(f, "(seq {})", elem),
            SirType::Map(val) => write!(f, "(map {})", val),
            // SIR21 T1b tokens.  A nullable pointer prints `(ptr? T)`; a
            // non-null reference prints `(ptr T)`.
            SirType::Ptr { pointee, nullable } => {
                write!(f, "(ptr{} {})", if *nullable { "?" } else { "" }, pointee)
            }
            SirType::Struct { name, fields } => {
                write!(f, "(struct {}", name)?;
                for (fname, fty) in fields {
                    write!(f, " ({} {})", fname, fty)?;
                }
                write!(f, ")")
            }
            SirType::Optional { inner } => write!(f, "(optional {})", inner),
            // SIR22 tokens.  An unknown/dynamic rank prints `(ndarray T)`;
            // a statically-known rank prints `(ndarray T n)`.
            SirType::NDArray { elem, rank } => match rank {
                Some(r) => write!(f, "(ndarray {} {})", elem, r),
                None => write!(f, "(ndarray {})", elem),
            },
            SirType::Rational => write!(f, "rational"),
            SirType::Complex => write!(f, "complex"),
            // SIR23 token — a bare keyword, no fields, same shape as
            // `Rational`/`Complex` above.
            SirType::SymExpr => write!(f, "sym-expr"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_simple_types() {
        // Top type: enum variant `Dynamic`, surface keyword `any`.
        assert_eq!(format!("{}", SirType::Dynamic), "any");
        assert_eq!(format!("{}", SirType::int_default()), "int");
        assert_eq!(format!("{}", SirType::Bool), "bool");
        assert_eq!(format!("{}", SirType::Nil), "nil");
        assert_eq!(format!("{}", SirType::Symbol), "symbol");
        assert_eq!(format!("{}", SirType::Str), "str");
        assert_eq!(format!("{}", SirType::Pair), "pair");
        assert_eq!(format!("{}", SirType::Closure), "closure");
    }

    #[test]
    fn display_fn_type() {
        let t = SirType::function(
            vec![SirType::int_default(), SirType::int_default()],
            SirType::Bool,
        );
        assert_eq!(format!("{}", t), "(fn (int int) bool)");
    }

    #[test]
    fn display_fn_zero_args() {
        let t = SirType::function(vec![], SirType::Nil);
        assert_eq!(format!("{}", t), "(fn () nil)");
    }

    #[test]
    fn display_nested_fn() {
        // (fn (int) (fn () bool))
        let inner = SirType::function(vec![], SirType::Bool);
        let outer = SirType::function(vec![SirType::int_default()], inner);
        assert_eq!(format!("{}", outer), "(fn (int) (fn () bool))");
    }

    #[test]
    fn is_dynamic_distinguishes() {
        assert!(SirType::Dynamic.is_dynamic());
        assert!(!SirType::int_default().is_dynamic());
    }

    #[test]
    fn equality_round_trip() {
        let a = SirType::function(vec![SirType::int_default()], SirType::int_default());
        let b = SirType::function(vec![SirType::int_default()], SirType::int_default());
        assert_eq!(a, b);
    }

    // ── SIR21 integer-spec tests ──────────────────────────────────────

    #[test]
    fn default_int_is_arbitrary_precision() {
        // The behaviour-preserving remap: v0 `Int` == arbitrary.
        assert_eq!(SirType::int_default(), SirType::Int(IntSpec::arbitrary()));
        let IntSpec {
            width,
            signed,
            overflow,
        } = IntSpec::arbitrary();
        assert_eq!(width, IntWidth::Arbitrary);
        assert!(signed);
        assert_eq!(overflow, Overflow::Arbitrary);
    }

    #[test]
    fn arbitrary_int_prints_as_bare_int_for_roundtrip() {
        // Pre-SIR21 modules printed `int`; that must not change.
        assert_eq!(format!("{}", SirType::int_default()), "int");
    }

    #[test]
    fn sized_int_prints_full_shape() {
        let u32w = SirType::int(IntWidth::W32, false, Overflow::Wrap);
        assert_eq!(format!("{}", u32w), "(int u32 wrap)");
        let i64t = SirType::int(IntWidth::W64, true, Overflow::Trap);
        assert_eq!(format!("{}", i64t), "(int i64 trap)");
    }

    #[test]
    fn bounds_are_derived_signed() {
        let i8 = IntSpec::sized(IntWidth::W8, true, Overflow::Wrap);
        assert_eq!(i8.min(), Some(-128));
        assert_eq!(i8.max(), Some(127));
        assert_eq!(i8.modulus(), Some(256));
    }

    #[test]
    fn bounds_are_derived_unsigned() {
        let u8 = IntSpec::sized(IntWidth::W8, false, Overflow::Wrap);
        assert_eq!(u8.min(), Some(0));
        assert_eq!(u8.max(), Some(255));
        assert_eq!(u8.modulus(), Some(256));

        let u32 = IntSpec::sized(IntWidth::W32, false, Overflow::Wrap);
        assert_eq!(u32.min(), Some(0));
        assert_eq!(u32.max(), Some(4_294_967_295));
        assert_eq!(u32.modulus(), Some(1u128 << 32));
    }

    #[test]
    fn bounds_signed_64() {
        let i64 = IntSpec::sized(IntWidth::W64, true, Overflow::Wrap);
        assert_eq!(i64.min(), Some(i64::MIN as i128));
        assert_eq!(i64.max(), Some(i64::MAX as i128));
    }

    #[test]
    fn bounds_128_do_not_overflow() {
        // The 2^127 intermediate is unrepresentable in i128; the bounds
        // functions must special-case it instead of panicking.
        let i128_spec = IntSpec::sized(IntWidth::W128, true, Overflow::Wrap);
        assert_eq!(i128_spec.min(), Some(i128::MIN));
        assert_eq!(i128_spec.max(), Some(i128::MAX));

        let u128_spec = IntSpec::sized(IntWidth::W128, false, Overflow::Wrap);
        assert_eq!(u128_spec.min(), Some(0));
        assert_eq!(u128_spec.max(), Some(i128::MAX)); // saturated report; documented
        assert_eq!(u128_spec.modulus(), Some(0)); // 2^128 sentinel; documented
    }

    #[test]
    fn arbitrary_has_no_bounds() {
        let a = IntSpec::arbitrary();
        assert!(a.is_arbitrary());
        assert_eq!(a.min(), None);
        assert_eq!(a.max(), None);
        assert_eq!(a.modulus(), None);
    }

    #[test]
    fn width_bits() {
        assert_eq!(IntWidth::W8.bits(), Some(8));
        assert_eq!(IntWidth::W64.bits(), Some(64));
        assert_eq!(IntWidth::W128.bits(), Some(128));
        assert_eq!(IntWidth::Arbitrary.bits(), None);
    }

    #[test]
    fn overflow_display() {
        assert_eq!(format!("{}", Overflow::Wrap), "wrap");
        assert_eq!(format!("{}", Overflow::Trap), "trap");
        assert_eq!(format!("{}", Overflow::Saturate), "sat");
        assert_eq!(format!("{}", Overflow::Checked), "checked");
        assert_eq!(format!("{}", Overflow::Undefined), "ub");
        assert_eq!(format!("{}", Overflow::Arbitrary), "arb");
    }

    // ── SIR21 T1b source-fidelity type tests ──────────────────────────

    #[test]
    fn display_ptr_non_null_and_nullable() {
        let p = SirType::ptr(SirType::int(IntWidth::W32, true, Overflow::Wrap), false);
        assert_eq!(format!("{}", p), "(ptr (int i32 wrap))");
        let np = SirType::ptr(SirType::Str, true);
        assert_eq!(format!("{}", np), "(ptr? str)");
    }

    #[test]
    fn display_struct_ordered_fields() {
        let s = SirType::struct_type(
            "Point",
            vec![
                (
                    "x".into(),
                    SirType::int(IntWidth::W32, true, Overflow::Wrap),
                ),
                (
                    "y".into(),
                    SirType::int(IntWidth::W32, true, Overflow::Wrap),
                ),
            ],
        );
        assert_eq!(
            format!("{}", s),
            "(struct Point (x (int i32 wrap)) (y (int i32 wrap)))"
        );
    }

    #[test]
    fn display_struct_empty_fields() {
        let s = SirType::struct_type("Unit", vec![]);
        assert_eq!(format!("{}", s), "(struct Unit)");
    }

    #[test]
    fn display_optional() {
        let o = SirType::optional(SirType::Bool);
        assert_eq!(format!("{}", o), "(optional bool)");
        // Optional wraps any type, including a pointer.
        let op = SirType::optional(SirType::ptr(SirType::Str, false));
        assert_eq!(format!("{}", op), "(optional (ptr str))");
    }

    #[test]
    fn new_variants_are_not_dynamic() {
        assert!(!SirType::ptr(SirType::Str, false).is_dynamic());
        assert!(!SirType::optional(SirType::Str).is_dynamic());
        assert!(!SirType::struct_type("S", vec![]).is_dynamic());
    }

    #[test]
    fn nullable_flag_affects_equality() {
        let a = SirType::ptr(SirType::Str, false);
        let b = SirType::ptr(SirType::Str, true);
        assert_ne!(a, b);
    }

    // ── SIR22 array/matrix type tests ─────────────────────────────────

    #[test]
    fn display_ndarray_unknown_rank() {
        // rank: None — a frontend that can't prove rank statically.
        let t = SirType::ndarray(SirType::Float, None);
        assert_eq!(format!("{}", t), "(ndarray float)");
    }

    #[test]
    fn display_ndarray_known_rank() {
        let t = SirType::ndarray(SirType::Float, Some(2));
        assert_eq!(format!("{}", t), "(ndarray float 2)");
    }

    #[test]
    fn display_rational_and_complex() {
        assert_eq!(format!("{}", SirType::Rational), "rational");
        assert_eq!(format!("{}", SirType::Complex), "complex");
    }

    #[test]
    fn ndarray_rank_affects_equality() {
        let a = SirType::ndarray(SirType::Float, Some(1));
        let b = SirType::ndarray(SirType::Float, Some(2));
        let c = SirType::ndarray(SirType::Float, None);
        assert_ne!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn ndarray_elem_type_affects_equality() {
        let a = SirType::ndarray(SirType::Float, Some(2));
        let b = SirType::ndarray(SirType::int_default(), Some(2));
        assert_ne!(a, b);
    }

    #[test]
    fn new_sir22_variants_are_not_dynamic() {
        assert!(!SirType::ndarray(SirType::Float, None).is_dynamic());
        assert!(!SirType::Rational.is_dynamic());
        assert!(!SirType::Complex.is_dynamic());
    }

    #[test]
    fn ndarray_can_nest_ndarray_elem() {
        // A frontend representing a "ragged" higher-rank array as
        // nested NDArrays (rather than a single flat NDArray with a
        // known rank) is representable — the elem type is unconstrained.
        let inner = SirType::ndarray(SirType::Float, Some(1));
        let outer = SirType::ndarray(inner, None);
        assert_eq!(format!("{}", outer), "(ndarray (ndarray float 1))");
    }

    // ── SIR23 symbolic-expression type tests ──────────────────────────

    #[test]
    fn display_sym_expr() {
        assert_eq!(format!("{}", SirType::SymExpr), "sym-expr");
    }

    #[test]
    fn sym_expr_is_not_dynamic() {
        assert!(!SirType::SymExpr.is_dynamic());
    }

    #[test]
    fn sym_expr_equality() {
        assert_eq!(SirType::SymExpr, SirType::SymExpr);
        assert_ne!(SirType::SymExpr, SirType::Rational);
        assert_ne!(SirType::SymExpr, SirType::Complex);
    }
}
