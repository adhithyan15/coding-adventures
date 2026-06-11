//! # Dimensional types — strict units for adjudication arithmetic (ADJ constraints, track A1).
//!
//! Adjudication mixes quantities that must **not** be added blindly: dollars,
//! euros, days, percentages, bare counts. Adding `usd + days` or `usd + eur`
//! is a category error that should be *rejected by the engine*, not silently
//! produce a meaningless number. This module gives every value a **dimension**
//! and defines the small, total algebra of which operations are legal.
//!
//! It generalises [`numeric_magnitude`](crate::numeric_magnitude) (step 2,
//! which read only the leading number): [`dimensioned_value`] reads the leading
//! number **and** infers the dimension from the typed-value wrapper. The
//! [`compute`](crate::compute) evaluator stays numeric for now; the
//! faithfulness gate (track A4) wires this in so a `let` formula that mixes
//! incompatible dimensions is a clean error.
//!
//! ## The dimensions
//!
//! | surface value          | term shape                          | dimension          |
//! |------------------------|-------------------------------------|--------------------|
//! | `18000`                | `Num`                               | `Scalar`           |
//! | `money(18000, usd)`    | `Compound{money,[Num, usd]}`        | `Money("usd")`     |
//! | `quantity(40, mg_dl)`  | `Compound{quantity,[Num, mg_dl]}`   | `Unit("mg_dl")`    |
//! | `percentage(40)`       | `Compound{percentage,[Num]}`        | `Percent`          |
//! | `duration(365, days)`  | `Compound{duration,[Num, days]}`    | `Duration("days")` |
//! | `count(3)`             | `Compound{count,[Num]}`             | `Scalar`           |
//!
//! Dates and times get their own dimensions in track A3 (they need day-ordinal
//! semantics from `datetime-core`, not a leading-scalar magnitude).
//!
//! ## The algebra (what [`combine`](Dimension::combine) enforces)
//!
//! - **add / sub** — operands must share a dimension. `usd + usd` ✓,
//!   `usd + eur` ✗ (a conversion fact is required, track A2), `usd + days` ✗.
//!   `Scalar` is the additive identity dimension (it combines with anything of
//!   the same dimension only — `3 + money(…)` is still rejected; scalars add to
//!   scalars). The result keeps the shared dimension.
//! - **mul** — `Money × Scalar → Money` (scale a price), `Unit(a) × Scalar →
//!   Unit(a)`, `Percent × X → X` (apply a percentage), `Scalar × Scalar →
//!   Scalar`. Two non-scalar dimensions multiply to a composite `Unit("a·b")`
//!   tag the faithfulness gate can inspect.
//! - **div** — `Money / Money → Scalar` (a ratio), `Unit(a) / Unit(a) → Scalar`
//!   (units cancel — the CSF:serum ratio is dimensionless), `X / Scalar → X`,
//!   `X / Percent → X`. Otherwise a composite `Unit("a/b")`.
//!
//! Percent is modelled as a dimension, not pre-divided by 100; applying it
//! (`× percentage(10)` meaning "10% of") is the caller's lowering concern — the
//! algebra here only tracks that the *dimension* of `X × Percent` is `X`.

use logic_core::{Number, Term};

/// The dimension of a value. Strings carry the unit/currency tag verbatim
/// (`"usd"`, `"mg_dl"`, `"days"`) so the audit trail shows exactly what the
/// source said; the engine compares tags by equality, it does not interpret
/// them (it does not "know" usd is dollars — only that `usd ≠ eur`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Dimension {
    /// A plain number: a count, a ratio, an index. The identity dimension.
    Scalar,
    /// An amount of money in a currency (`Money("usd")`).
    Money(String),
    /// A quantity in a named unit (`Unit("mg_dl")`).
    Unit(String),
    /// A percentage (tracked as its own dimension; not pre-divided by 100).
    Percent,
    /// A length of time in a named unit (`Duration("days")`).
    Duration(String),
    /// A calendar date (a point in time, not a magnitude). Date arithmetic
    /// goes through the dedicated [`datetime`](crate::datetime) functions
    /// (`days_between`, `date_add`, `before`/`after`), never through
    /// [`combine`](Dimension::combine) — adding two dates is meaningless, and
    /// `Date − Date → Duration` / `Date + Duration → Date` have their own
    /// (ordinal) semantics. `combine` therefore rejects any `Date` operand.
    Date,
}

impl Dimension {
    /// A human-readable tag for audit rendering (`"usd"`, `"scalar"`,
    /// `"mg_dl"`, `"%"`, `"days"`).
    pub fn tag(&self) -> String {
        match self {
            Dimension::Scalar => "scalar".to_string(),
            Dimension::Money(c) => c.clone(),
            Dimension::Unit(u) => u.clone(),
            Dimension::Percent => "%".to_string(),
            Dimension::Duration(u) => u.clone(),
            Dimension::Date => "date".to_string(),
        }
    }

    /// `true` iff this is the identity (`Scalar`) dimension.
    pub fn is_scalar(&self) -> bool {
        matches!(self, Dimension::Scalar)
    }

    /// The dimension of `lhs <op> rhs`, or a [`DimError`] if the operation is a
    /// category error. This is the whole point of the module: the engine, not
    /// the model, decides whether `usd + days` is allowed (it isn't).
    pub fn combine(op: DimOp, lhs: &Dimension, rhs: &Dimension) -> Result<Dimension, DimError> {
        // Dates are points in time, not magnitudes: their arithmetic
        // (Date−Date→Duration, Date+Duration→Date) lives in the `datetime`
        // module, so a Date reaching the generic algebra is a misuse.
        if *lhs == Dimension::Date || *rhs == Dimension::Date {
            return Err(DimError::Mismatch {
                op,
                lhs: lhs.tag(),
                rhs: rhs.tag(),
            });
        }
        match op {
            // Additive: dimensions must match exactly. No silent coercion.
            DimOp::Add | DimOp::Sub => {
                if lhs == rhs {
                    Ok(lhs.clone())
                } else {
                    Err(DimError::Mismatch {
                        op,
                        lhs: lhs.tag(),
                        rhs: rhs.tag(),
                    })
                }
            }
            DimOp::Mul => Ok(Self::combine_mul(lhs, rhs)),
            DimOp::Div => Self::combine_div(lhs, rhs),
        }
    }

    fn combine_mul(lhs: &Dimension, rhs: &Dimension) -> Dimension {
        match (lhs, rhs) {
            // Scalar / Percent are multiplicatively transparent: they scale the
            // other operand without changing its dimension.
            (Dimension::Scalar, d) | (d, Dimension::Scalar) => d.clone(),
            (Dimension::Percent, d) | (d, Dimension::Percent) => d.clone(),
            // Two genuine dimensions → a composite tag for the audit/gate.
            (a, b) => Dimension::Unit(format!("{}·{}", a.tag(), b.tag())),
        }
    }

    fn combine_div(lhs: &Dimension, rhs: &Dimension) -> Result<Dimension, DimError> {
        match (lhs, rhs) {
            // Division by zero-dimension scalar/percent leaves the dimension.
            (d, Dimension::Scalar) | (d, Dimension::Percent) => Ok(d.clone()),
            (Dimension::Scalar, Dimension::Scalar) => Ok(Dimension::Scalar),
            // Like over like cancels to a dimensionless ratio — the key case
            // (CSF:serum ratio, debt-to-income ratio, price ratios).
            (Dimension::Money(a), Dimension::Money(b)) if a == b => Ok(Dimension::Scalar),
            (Dimension::Unit(a), Dimension::Unit(b)) if a == b => Ok(Dimension::Scalar),
            (Dimension::Duration(a), Dimension::Duration(b)) if a == b => Ok(Dimension::Scalar),
            // Scalar / D is a reciprocal dimension; D1 / D2 (unlike) is composite.
            (a, b) => Ok(Dimension::Unit(format!("{}/{}", a.tag(), b.tag()))),
        }
    }
}

/// A binary operation, for [`Dimension::combine`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DimOp {
    Add,
    Sub,
    Mul,
    Div,
}

impl DimOp {
    pub fn symbol(&self) -> &'static str {
        match self {
            DimOp::Add => "+",
            DimOp::Sub => "-",
            DimOp::Mul => "*",
            DimOp::Div => "/",
        }
    }
}

/// A dimensional category error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DimError {
    /// `lhs <op> rhs` mixes incompatible dimensions (`usd + days`,
    /// `usd + eur` without a conversion). The tags name what was mixed so the
    /// audit reader sees exactly which units clashed.
    Mismatch {
        op: DimOp,
        lhs: String,
        rhs: String,
    },
}

/// A magnitude paired with its dimension — the unit-aware value.
#[derive(Debug, Clone, PartialEq)]
pub struct Dimensioned {
    pub magnitude: f64,
    pub dim: Dimension,
}

impl Dimensioned {
    pub fn new(magnitude: f64, dim: Dimension) -> Self {
        Self { magnitude, dim }
    }

    /// A bare scalar.
    pub fn scalar(magnitude: f64) -> Self {
        Self::new(magnitude, Dimension::Scalar)
    }
}

/// Read the dimensioned value of a term, if it has one. Generalises
/// [`numeric_magnitude`](crate::numeric_magnitude): the magnitude is the
/// leading numeric argument (as before), and the **dimension** is inferred from
/// the wrapper functor and its unit/currency tag.
///
/// - `Num` → `Scalar`
/// - `money(N, ccy)` → `Money(ccy)`; `quantity(N, unit)` → `Unit(unit)`;
///   `duration(N, unit)` → `Duration(unit)`
/// - `percentage(N)` → `Percent`; `count(N)` → `Scalar`
/// - an unrecognised single-number wrapper `f(N, …)` → `Unit(f)` (the functor
///   itself is taken as the unit tag, so a novel typed value still carries a
///   dimension rather than silently becoming a scalar).
///
/// Returns `None` for a term with no leading numeric magnitude.
pub fn dimensioned_value(value: &Term) -> Option<Dimensioned> {
    match value {
        Term::Num(Number::Int(i)) => Some(Dimensioned::scalar(*i as f64)),
        Term::Num(Number::Float(x)) => Some(Dimensioned::scalar(*x)),
        Term::Compound { functor, args } => {
            // Dates/times are multi-field points in time, not scalar-magnitude
            // wrappers — `date(2025, 1, 15)`'s leading `2025` is a year, not a
            // magnitude. They are read by the `datetime` module instead.
            if matches!(functor.as_str(), "date" | "time" | "datetime") {
                return None;
            }
            let magnitude = match args.first()? {
                Term::Num(Number::Int(i)) => *i as f64,
                Term::Num(Number::Float(x)) => *x,
                _ => return None,
            };
            let unit_tag = || match args.get(1) {
                Some(Term::Atom(u)) => u.clone(),
                Some(Term::Str(u)) => u.clone(),
                // A unit-less wrapper (e.g. `quantity(40)`): tag with the functor.
                _ => functor.clone(),
            };
            let dim = match functor.as_str() {
                "money" => Dimension::Money(unit_tag()),
                "quantity" => Dimension::Unit(unit_tag()),
                "duration" => Dimension::Duration(unit_tag()),
                "percentage" | "percent" => Dimension::Percent,
                "count" => Dimension::Scalar,
                // Any other single-number wrapper: take the functor as the unit.
                other => Dimension::Unit(other.to_string()),
            };
            Some(Dimensioned::new(magnitude, dim))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use logic_core::{atom, compound, float, int};

    #[test]
    fn reads_dimension_from_each_wrapper() {
        assert_eq!(dimensioned_value(&int(42)).unwrap().dim, Dimension::Scalar);
        assert_eq!(
            dimensioned_value(&float(3.5)).unwrap(),
            Dimensioned::scalar(3.5)
        );
        let m = dimensioned_value(&compound("money", vec![int(18000), atom("usd")])).unwrap();
        assert_eq!(m.magnitude, 18000.0);
        assert_eq!(m.dim, Dimension::Money("usd".into()));
        assert_eq!(
            dimensioned_value(&compound("quantity", vec![int(40), atom("mg_dl")]))
                .unwrap()
                .dim,
            Dimension::Unit("mg_dl".into())
        );
        assert_eq!(
            dimensioned_value(&compound("duration", vec![int(365), atom("days")]))
                .unwrap()
                .dim,
            Dimension::Duration("days".into())
        );
        assert_eq!(
            dimensioned_value(&compound("percentage", vec![int(40)]))
                .unwrap()
                .dim,
            Dimension::Percent
        );
        assert_eq!(
            dimensioned_value(&compound("count", vec![int(3)]))
                .unwrap()
                .dim,
            Dimension::Scalar
        );
    }

    #[test]
    fn unknown_wrapper_takes_functor_as_unit() {
        assert_eq!(
            dimensioned_value(&compound("widgets", vec![int(7)]))
                .unwrap()
                .dim,
            Dimension::Unit("widgets".into())
        );
    }

    #[test]
    fn no_leading_number_has_no_dimension() {
        assert!(dimensioned_value(&atom("usd")).is_none());
        assert!(dimensioned_value(&compound("pair", vec![atom("a"), int(1)])).is_none());
    }

    #[test]
    fn add_requires_matching_dimension() {
        let usd = Dimension::Money("usd".into());
        let eur = Dimension::Money("eur".into());
        let days = Dimension::Duration("days".into());
        assert_eq!(
            Dimension::combine(DimOp::Add, &usd, &usd).unwrap(),
            usd.clone()
        );
        // usd + eur is rejected (a conversion fact would be required — track A2).
        assert!(matches!(
            Dimension::combine(DimOp::Add, &usd, &eur),
            Err(DimError::Mismatch { .. })
        ));
        // usd + days is a category error.
        assert!(matches!(
            Dimension::combine(DimOp::Sub, &usd, &days),
            Err(DimError::Mismatch { .. })
        ));
        // scalars add to scalars.
        assert_eq!(
            Dimension::combine(DimOp::Add, &Dimension::Scalar, &Dimension::Scalar).unwrap(),
            Dimension::Scalar
        );
        // a scalar does NOT silently add to money.
        assert!(Dimension::combine(DimOp::Add, &Dimension::Scalar, &usd).is_err());
    }

    #[test]
    fn money_scaled_by_scalar_stays_money() {
        let usd = Dimension::Money("usd".into());
        assert_eq!(
            Dimension::combine(DimOp::Mul, &usd, &Dimension::Scalar).unwrap(),
            usd.clone()
        );
        assert_eq!(
            Dimension::combine(DimOp::Mul, &Dimension::Scalar, &usd).unwrap(),
            usd.clone()
        );
        // applying a percentage keeps the dimension.
        assert_eq!(
            Dimension::combine(DimOp::Mul, &usd, &Dimension::Percent).unwrap(),
            usd.clone()
        );
    }

    #[test]
    fn like_over_like_cancels_to_a_ratio() {
        let usd = Dimension::Money("usd".into());
        let mgdl = Dimension::Unit("mg_dl".into());
        assert_eq!(
            Dimension::combine(DimOp::Div, &usd, &usd).unwrap(),
            Dimension::Scalar
        );
        assert_eq!(
            Dimension::combine(DimOp::Div, &mgdl, &mgdl).unwrap(),
            Dimension::Scalar
        );
        // money / scalar stays money (e.g. a per-unit price).
        assert_eq!(
            Dimension::combine(DimOp::Div, &usd, &Dimension::Scalar).unwrap(),
            usd
        );
    }

    #[test]
    fn unlike_division_is_composite_not_an_error() {
        let usd = Dimension::Money("usd".into());
        let days = Dimension::Duration("days".into());
        // money / days → a composite rate tag (legal; the gate can inspect it).
        assert_eq!(
            Dimension::combine(DimOp::Div, &usd, &days).unwrap(),
            Dimension::Unit("usd/days".into())
        );
    }
}
