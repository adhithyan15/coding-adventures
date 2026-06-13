//! # Currency / unit conversions (ADJ constraints, track A2).
//!
//! Track A1 made `usd + eur` a [`DimError::Mismatch`](crate::DimError) — the
//! engine refuses to add two currencies blindly. This module supplies the
//! *only* thing that licenses such an addition: an **explicit, provenanced
//! conversion fact**.
//!
//! ```text
//! convert money(1, usd) = money(0.92, eur)     % a fact, with a citation
//! ```
//!
//! reads as "1 usd = 0.92 eur". With that fact present, `usd + eur` is allowed:
//! the engine converts one operand into the other's currency *using the rate*
//! and adds. The conversion is not a coercion the engine invents — it is data
//! the rulebook provided, so it carries [`Provenance`](crate::Provenance) and
//! (once `compute` is dimension-aware in track A4) appears as its own node in
//! the derivation tree. No fact ⇒ no conversion ⇒ the strict A1 mismatch stands.
//!
//! Scope (v1): **direct and inverse** rates only (`usd→eur` also gives
//! `eur→usd` at `1/rate`). No transitive chaining (`usd→eur→gbp`) — that is a
//! later refinement; a missing path is a clean `None`, never a guess. The
//! mechanism is currency-first but unit-agnostic: the tags are opaque strings,
//! so the same table converts any [`Dimension`](crate::Dimension) whose tag
//! matches (e.g. `mg_dl ↔ mmol_l` if such a fact is supplied).

use crate::dimension::{Dimension, Dimensioned};
use crate::Provenance;

/// A single conversion fact: "1 unit of `from` equals `rate` units of `to`".
/// `from`/`to` are the dimension tags (`"usd"`, `"eur"`). Carries a citation
/// so the audit trail shows where the rate came from.
#[derive(Debug, Clone, PartialEq)]
pub struct Conversion {
    pub from: String,
    pub to: String,
    pub rate: f64,
    pub provenance: Provenance,
}

impl Conversion {
    /// "1 `from` = `rate` `to`", validating the rate. An exchange rate is
    /// strictly positive and finite; an attacker-supplied `0`, negative,
    /// `NaN`, or `inf` rate is rejected as a clean [`ConvError::BadRate`]
    /// rather than panicking — this is the entry point a surface-`convert`
    /// lowerer should call (mirroring the LR/probability guards, which return
    /// errors for untrusted numerics instead of aborting the process).
    pub fn try_new(
        from: impl Into<String>,
        to: impl Into<String>,
        rate: f64,
    ) -> Result<Self, ConvError> {
        if !(rate.is_finite() && rate > 0.0) {
            return Err(ConvError::BadRate { rate });
        }
        Ok(Self {
            from: from.into(),
            to: to.into(),
            rate,
            provenance: Provenance::unattributed(),
        })
    }

    /// Trusted/test convenience: "1 `from` = `rate` `to`", panicking on a
    /// non-positive or non-finite rate. Prefer [`try_new`](Self::try_new) for
    /// any rate that originates from parsed input.
    pub fn new(from: impl Into<String>, to: impl Into<String>, rate: f64) -> Self {
        Self::try_new(from, to, rate)
            .unwrap_or_else(|_| panic!("Conversion::new requires a finite, positive rate; got {rate}"))
    }

    /// Builder-style: attach a citation.
    pub fn with_provenance(mut self, provenance: Provenance) -> Self {
        self.provenance = provenance;
        self
    }
}

/// Why a conversion could not be performed.
#[derive(Debug, Clone, PartialEq)]
pub enum ConvError {
    /// No conversion fact (direct or inverse) links `from` to `to`.
    NoRate { from: String, to: String },
    /// The value being converted has no convertible (tagged) dimension —
    /// e.g. a `Scalar`, which has no currency/unit to convert.
    NotConvertible { dim: String },
    /// A conversion rate that is not finite and strictly positive.
    BadRate { rate: f64 },
    /// A converted/combined magnitude that is non-finite (`NaN`/`±∞`) — e.g.
    /// overflow or `∞ − ∞`. Rejected rather than propagated, matching
    /// [`ComputeError::NonFinite`](crate::compute::ComputeError) so a
    /// non-finite cannot silently flow into a verdict.
    NonFinite,
}

/// A set of conversion facts. Resolves a rate between two tags via a direct
/// fact, its inverse, or the identity (`from == to`).
#[derive(Debug, Clone, Default)]
pub struct ConversionTable {
    conversions: Vec<Conversion>,
}

impl ConversionTable {
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a conversion fact (and implicitly its inverse).
    pub fn add(&mut self, conversion: Conversion) {
        self.conversions.push(conversion);
    }

    /// The multiplicative rate to take a magnitude in `from` to `to`
    /// (`to_magnitude = from_magnitude * rate`), or `None` if no fact links
    /// them. `from == to` is the identity `1.0`; an inverse fact gives
    /// `1.0 / rate`. The most-recently-added matching fact wins, so a later
    /// `convert` supersedes an earlier one.
    pub fn rate(&self, from: &str, to: &str) -> Option<f64> {
        if from == to {
            return Some(1.0);
        }
        self.conversions.iter().rev().find_map(|c| {
            if c.from == from && c.to == to {
                Some(c.rate)
            } else if c.from == to && c.to == from {
                Some(1.0 / c.rate)
            } else {
                None
            }
        })
    }

    /// The conversion fact (for provenance) that links `from`→`to` directly or
    /// inversely, if any. Returns the clause so the audit trail can cite the
    /// rate that was applied.
    pub fn fact(&self, from: &str, to: &str) -> Option<&Conversion> {
        self.conversions
            .iter()
            .rev()
            .find(|c| (c.from == from && c.to == to) || (c.from == to && c.to == from))
    }
}

/// The convertible tag of a dimension (`Money("usd") → "usd"`,
/// `Unit("mg_dl") → "mg_dl"`), or `None` for dimensions with nothing to
/// convert (`Scalar`, `Percent`).
fn convertible_tag(dim: &Dimension) -> Option<&str> {
    match dim {
        Dimension::Money(c) => Some(c),
        Dimension::Unit(u) => Some(u),
        Dimension::Duration(u) => Some(u),
        // Scalar/Percent have nothing to convert; Date is a point in time, not
        // a magnitude — its arithmetic is in the `datetime` module.
        Dimension::Scalar | Dimension::Percent | Dimension::Date => None,
    }
}

/// Re-wrap a converted magnitude in the same *kind* of dimension as `like`,
/// but with the target tag (so converting a `Money("usd")` yields a
/// `Money("eur")`, not a `Unit("eur")`).
fn retag(like: &Dimension, target_tag: &str) -> Dimension {
    match like {
        Dimension::Money(_) => Dimension::Money(target_tag.to_string()),
        Dimension::Unit(_) => Dimension::Unit(target_tag.to_string()),
        Dimension::Duration(_) => Dimension::Duration(target_tag.to_string()),
        other => other.clone(),
    }
}

/// Convert a dimensioned value into the `target` dimension using the table.
/// Both must be the same *kind* of dimension (money→money, unit→unit) with
/// convertible tags, and a rate must link them. `Scalar`/`Percent` are not
/// convertible.
pub fn convert_value(
    value: &Dimensioned,
    target: &Dimension,
    table: &ConversionTable,
) -> Result<Dimensioned, ConvError> {
    let from_tag = convertible_tag(&value.dim).ok_or_else(|| ConvError::NotConvertible {
        dim: value.dim.tag(),
    })?;
    let to_tag = convertible_tag(target).ok_or_else(|| ConvError::NotConvertible {
        dim: target.tag(),
    })?;
    let rate = table.rate(from_tag, to_tag).ok_or_else(|| ConvError::NoRate {
        from: from_tag.to_string(),
        to: to_tag.to_string(),
    })?;
    let magnitude = value.magnitude * rate;
    if !magnitude.is_finite() {
        return Err(ConvError::NonFinite);
    }
    Ok(Dimensioned::new(magnitude, retag(&value.dim, to_tag)))
}

/// Add or subtract two dimensioned values, resolving a currency/unit mismatch
/// through the conversion table. Same dimension → a plain magnitude add/sub
/// (the A1 rule). Different tags of the same *kind* → convert `rhs` into
/// `lhs`'s dimension via a rate, then combine, returning the result in `lhs`'s
/// dimension. Genuinely incompatible kinds (`usd + days`) stay an error.
pub fn add_or_sub(
    subtract: bool,
    lhs: &Dimensioned,
    rhs: &Dimensioned,
    table: &ConversionTable,
) -> Result<Dimensioned, ConvError> {
    let rhs_in_lhs = if lhs.dim == rhs.dim {
        rhs.clone()
    } else {
        convert_value(rhs, &lhs.dim, table)?
    };
    let magnitude = if subtract {
        lhs.magnitude - rhs_in_lhs.magnitude
    } else {
        lhs.magnitude + rhs_in_lhs.magnitude
    };
    if !magnitude.is_finite() {
        return Err(ConvError::NonFinite);
    }
    Ok(Dimensioned::new(magnitude, lhs.dim.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dimension::Dimension;

    fn usd(m: f64) -> Dimensioned {
        Dimensioned::new(m, Dimension::Money("usd".into()))
    }
    fn eur(m: f64) -> Dimensioned {
        Dimensioned::new(m, Dimension::Money("eur".into()))
    }

    #[test]
    fn direct_inverse_and_identity_rates() {
        let mut t = ConversionTable::new();
        t.add(Conversion::new("usd", "eur", 0.92));
        assert_eq!(t.rate("usd", "eur"), Some(0.92));
        assert_eq!(t.rate("eur", "usd"), Some(1.0 / 0.92));
        assert_eq!(t.rate("usd", "usd"), Some(1.0));
        assert_eq!(t.rate("usd", "gbp"), None); // no transitive guess
    }

    #[test]
    fn converts_money_across_currencies() {
        let mut t = ConversionTable::new();
        t.add(Conversion::new("usd", "eur", 0.92));
        let got = convert_value(&usd(100.0), &Dimension::Money("eur".into()), &t).unwrap();
        assert_eq!(got.dim, Dimension::Money("eur".into()));
        assert!((got.magnitude - 92.0).abs() < 1e-9);
    }

    #[test]
    fn cross_currency_add_uses_the_rate() {
        // 100 usd + 92 eur, with 1 usd = 0.92 eur. Convert eur→usd: 92/0.92 = 100.
        let mut t = ConversionTable::new();
        t.add(Conversion::new("usd", "eur", 0.92));
        let sum = add_or_sub(false, &usd(100.0), &eur(92.0), &t).unwrap();
        assert_eq!(sum.dim, Dimension::Money("usd".into()));
        assert!((sum.magnitude - 200.0).abs() < 1e-9, "got {}", sum.magnitude);
    }

    #[test]
    fn same_currency_add_needs_no_table() {
        let t = ConversionTable::new();
        let sum = add_or_sub(false, &usd(100.0), &usd(50.0), &t).unwrap();
        assert!((sum.magnitude - 150.0).abs() < 1e-9);
    }

    #[test]
    fn cross_currency_without_a_fact_is_an_error() {
        let t = ConversionTable::new();
        assert!(matches!(
            add_or_sub(false, &usd(100.0), &eur(92.0), &t),
            Err(ConvError::NoRate { .. })
        ));
    }

    #[test]
    fn incompatible_kinds_are_not_convertible() {
        let mut t = ConversionTable::new();
        t.add(Conversion::new("usd", "eur", 0.92));
        let days = Dimensioned::new(5.0, Dimension::Duration("days".into()));
        // money → duration has no rate; converting is a clean error.
        assert!(matches!(
            convert_value(&usd(100.0), &Dimension::Duration("days".into()), &t),
            Err(ConvError::NoRate { .. })
        ));
        // adding usd + days errors too (no rate between usd and days).
        assert!(add_or_sub(false, &usd(100.0), &days, &t).is_err());
    }

    #[test]
    fn scalar_is_not_convertible() {
        let t = ConversionTable::new();
        assert!(matches!(
            convert_value(&Dimensioned::scalar(5.0), &Dimension::Money("usd".into()), &t),
            Err(ConvError::NotConvertible { .. })
        ));
    }

    #[test]
    #[should_panic(expected = "finite, positive rate")]
    fn non_positive_rate_panics_at_construction() {
        let _ = Conversion::new("usd", "eur", 0.0);
    }

    #[test]
    fn try_new_rejects_bad_rates_without_panicking() {
        for bad in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            assert!(matches!(
                Conversion::try_new("usd", "eur", bad),
                Err(ConvError::BadRate { .. })
            ));
        }
        assert!(Conversion::try_new("usd", "eur", 0.92).is_ok());
    }

    #[test]
    fn overflow_to_non_finite_is_rejected() {
        let mut t = ConversionTable::new();
        // a huge rate times a huge magnitude overflows to +inf → rejected.
        t.add(Conversion::new("usd", "eur", 1e308));
        assert_eq!(
            convert_value(&usd(1e308), &Dimension::Money("eur".into()), &t).unwrap_err(),
            ConvError::NonFinite
        );
    }

    #[test]
    fn fact_is_recoverable_for_provenance() {
        let mut t = ConversionTable::new();
        t.add(
            Conversion::new("usd", "eur", 0.92)
                .with_provenance(Provenance::cited("ECB 2026-06-11 reference rate")),
        );
        let f = t.fact("eur", "usd").expect("inverse fact found");
        assert_eq!(f.provenance.source, "ECB 2026-06-11 reference rate");
    }
}
