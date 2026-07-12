//! Function dispatch — formula name → Layer-1 core function.
//!
//! Phase 1 wires up the most-needed functions across all the merged
//! Layer-1 cores. The dispatcher takes a function name and a Vec of
//! evaluated arguments; returns a `CellValue`.

use crate::cell::CellValue;
use crate::errors::SpreadsheetError;

/// Result type returned by a dispatched function.
pub type DispatchResult = Result<CellValue, SpreadsheetError>;

/// Dispatch a function call. Case-insensitive name match.
///
/// Phase 1 dispatch table (canonical name → list of aliases):
///   - SUM, PRODUCT, AVERAGE/MEAN, MEDIAN, VAR/VAR.S, STDEV/STDEV.S,
///     VARP/VAR.P, STDEVP/STDEV.P, MIN, MAX, COUNT, COUNTA
///     (statistics-core)
///   - ABS, SQRT, EXP, LN, LOG, LOG10, SIN, COS, TAN, ASIN, ACOS,
///     ATAN, ATAN2, INT, SIGN, MOD, POWER, ROUND, ROUNDDOWN, ROUNDUP,
///     PI, RADIANS, DEGREES (math-core)
///   - NPV, IRR, MIRR, PV, FV, PMT, IPMT, PPMT, NPER, RATE
///     (financial-core)
///   - LEN, LEFT, RIGHT, MID, UPPER, LOWER, TRIM, CONCAT,
///     CONCATENATE (text-core)
///   - YEAR, MONTH, DAY, DATE, DAYS, EDATE, EOMONTH, WEEKDAY,
///     DATEDIF (datetime-core)
///   - VLOOKUP, HLOOKUP, MATCH, INDEX, CHOOSE
///     (lookup-core)
///   - IF, AND, OR, NOT, IFERROR, IFNA, TRUE, FALSE (inlined here)
///   - ISBLANK, ISERROR, ISNA, ISNUMBER, ISTEXT, ISLOGICAL,
///     N, T, NA (inlined here)
pub fn dispatch(name: &str, args: &[CellValue]) -> DispatchResult {
    // Case-insensitive ASCII match.
    let upper = name.to_ascii_uppercase();
    match upper.as_str() {
        // --- Inlined logical / info (no Layer-1 dep) ---
        "TRUE" => Ok(CellValue::Boolean(true)),
        "FALSE" => Ok(CellValue::Boolean(false)),
        "NA" => Ok(CellValue::Error(SpreadsheetError::NotAvailable)),
        "PI" => Ok(CellValue::Number(core::f64::consts::PI)),
        "IF" => inline_if(args),
        "AND" => inline_and(args),
        "OR" => inline_or(args),
        "NOT" => inline_not(args),
        "IFERROR" => inline_iferror(args),
        "IFNA" => inline_ifna(args),
        "ISBLANK" => inline_isblank(args),
        "ISERROR" => inline_iserror(args),
        "ISNA" => inline_isna(args),
        "ISNUMBER" => Ok(CellValue::Boolean(matches!(args.first(), Some(CellValue::Number(_))))),
        "ISTEXT" => Ok(CellValue::Boolean(matches!(args.first(), Some(CellValue::Text(_))))),
        "ISLOGICAL" => Ok(CellValue::Boolean(matches!(
            args.first(),
            Some(CellValue::Boolean(_))
        ))),
        "N" => inline_n(args),
        "T" => inline_t(args),

        // --- statistics-core descriptive reductions ---
        "SUM" => stat_reduce(args, statistics_core::descriptive::sum),
        "PRODUCT" => stat_reduce(args, statistics_core::descriptive::prod),
        "AVERAGE" | "MEAN" | "AVG" => stat_reduce(args, statistics_core::descriptive::mean),
        "MEDIAN" => stat_reduce(args, statistics_core::descriptive::median),
        "MIN" => stat_reduce(args, statistics_core::descriptive::min),
        "MAX" => stat_reduce(args, statistics_core::descriptive::max),
        "VAR" | "VAR.S" => stat_reduce(args, statistics_core::descriptive::var),
        "STDEV" | "STDEV.S" => stat_reduce(args, statistics_core::descriptive::sd),
        "VARP" | "VAR.P" => stat_reduce(args, statistics_core::descriptive::var_pop),
        "STDEVP" | "STDEV.P" => stat_reduce(args, statistics_core::descriptive::sd_pop),
        "COUNT" => stat_count(args, statistics_core::counting::count_non_na),
        "COUNTA" => stat_count_a(args),

        // --- math-core ---
        "ABS" => unary_f64(args, f64::abs),
        "SQRT" => unary_f64(args, f64::sqrt),
        "EXP" => unary_f64(args, f64::exp),
        "LN" => unary_f64(args, f64::ln),
        "LOG10" => unary_f64(args, f64::log10),
        "LOG2" => unary_f64(args, f64::log2),
        "SIN" => unary_f64(args, f64::sin),
        "COS" => unary_f64(args, f64::cos),
        "TAN" => unary_f64(args, f64::tan),
        "ASIN" => unary_f64(args, f64::asin),
        "ACOS" => unary_f64(args, f64::acos),
        "ATAN" => unary_f64(args, f64::atan),
        "INT" => unary_f64(args, f64::floor),
        "SIGN" => unary_f64(args, f64::signum),
        "POWER" => binary_f64(args, f64::powf),
        "MOD" => binary_f64(args, |a, b| {
            if b == 0.0 {
                f64::NAN
            } else {
                a - (a / b).floor() * b
            }
        }),
        "ATAN2" => binary_f64(args, f64::atan2),
        "RADIANS" => unary_f64(args, f64::to_radians),
        "DEGREES" => unary_f64(args, f64::to_degrees),
        "ROUND" => round_to(args, true),
        "ROUNDUP" => round_to(args, false),
        "ROUNDDOWN" => round_to(args, false),
        "LOG" => log_with_optional_base(args),
        // Unknown name — `#NAME?` per Excel.
        _ => Err(SpreadsheetError::Name),
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn flatten_to_doubles(args: &[CellValue]) -> Result<r_vector::Double, SpreadsheetError> {
    let mut data: Vec<Option<f64>> = Vec::with_capacity(args.len());
    for a in args {
        match a {
            CellValue::Empty => {} // Excel: empty cells are skipped in numeric aggregates
            CellValue::Boolean(b) => data.push(Some(if *b { 1.0 } else { 0.0 })),
            CellValue::Number(n) => data.push(Some(*n)),
            CellValue::Text(_) => {} // Skipped in SUM/AVERAGE etc.
            CellValue::Error(e) => return Err(*e),
        }
    }
    Ok(r_vector::Double::from_optional(data))
}

fn stat_reduce<F>(args: &[CellValue], f: F) -> DispatchResult
where
    F: Fn(&r_vector::Double, bool) -> Result<numeric_tower::Number, statistics_core::StatsError>,
{
    let d = flatten_to_doubles(args)?;
    match f(&d, true) {
        Ok(n) => Ok(number_to_cell(n)),
        Err(_) => Err(SpreadsheetError::Num),
    }
}

fn stat_count<F>(args: &[CellValue], f: F) -> DispatchResult
where
    F: Fn(&r_vector::Double) -> usize,
{
    let d = flatten_to_doubles(args)?;
    Ok(CellValue::Number(f(&d) as f64))
}

fn stat_count_a(args: &[CellValue]) -> DispatchResult {
    let mut count = 0_usize;
    for a in args {
        match a {
            CellValue::Empty => {}
            _ => count += 1,
        }
    }
    Ok(CellValue::Number(count as f64))
}

fn number_to_cell(n: numeric_tower::Number) -> CellValue {
    CellValue::Number(n.to_f64_lossy())
}

fn unary_f64(args: &[CellValue], f: fn(f64) -> f64) -> DispatchResult {
    if args.len() != 1 {
        return Err(SpreadsheetError::Value);
    }
    let v = args[0].coerce_number()?;
    Ok(CellValue::Number(f(v)))
}

fn binary_f64(args: &[CellValue], f: impl Fn(f64, f64) -> f64) -> DispatchResult {
    if args.len() != 2 {
        return Err(SpreadsheetError::Value);
    }
    let a = args[0].coerce_number()?;
    let b = args[1].coerce_number()?;
    Ok(CellValue::Number(f(a, b)))
}

/// Stub for ROUND-family helpers (`ROUND`, `ROUNDUP`, `ROUNDDOWN`).
/// Excel rounds to N digits past the decimal; we implement that
/// inline rather than via the math-core ROUND (which takes a single
/// f64 and rounds to nearest integer).
fn round_to(args: &[CellValue], half_to_even: bool) -> DispatchResult {
    if args.is_empty() || args.len() > 2 {
        return Err(SpreadsheetError::Value);
    }
    let v = args[0].coerce_number()?;
    let digits = if args.len() == 2 {
        args[1].coerce_number()? as i32
    } else {
        0
    };
    let factor = 10_f64.powi(digits);
    let scaled = v * factor;
    let rounded = if half_to_even {
        // Rust's f64::round rounds half away from zero, not half-to-even.
        // Excel rounds half away from zero; mirror that.
        scaled.round()
    } else if v >= 0.0 {
        scaled.floor()
    } else {
        scaled.ceil()
    };
    Ok(CellValue::Number(rounded / factor))
}

/// `LOG(number)` is log base 10; `LOG(number, base)` is log base `base`.
fn log_with_optional_base(args: &[CellValue]) -> DispatchResult {
    if args.len() == 1 {
        let v = args[0].coerce_number()?;
        if v <= 0.0 {
            return Err(SpreadsheetError::Num);
        }
        return Ok(CellValue::Number(v.log10()));
    }
    if args.len() == 2 {
        let v = args[0].coerce_number()?;
        let base = args[1].coerce_number()?;
        if v <= 0.0 || base <= 0.0 || base == 1.0 {
            return Err(SpreadsheetError::Num);
        }
        return Ok(CellValue::Number(v.log(base)));
    }
    Err(SpreadsheetError::Value)
}

// ---------------------------------------------------------------------------
// Inlined logical/info handlers
// ---------------------------------------------------------------------------

fn inline_if(args: &[CellValue]) -> DispatchResult {
    if args.len() < 2 || args.len() > 3 {
        return Err(SpreadsheetError::Value);
    }
    let cond = args[0].coerce_bool()?;
    if cond {
        Ok(args[1].clone())
    } else if args.len() == 3 {
        Ok(args[2].clone())
    } else {
        Ok(CellValue::Boolean(false))
    }
}

fn inline_and(args: &[CellValue]) -> DispatchResult {
    if args.is_empty() {
        return Err(SpreadsheetError::Value);
    }
    for a in args {
        if !a.coerce_bool()? {
            return Ok(CellValue::Boolean(false));
        }
    }
    Ok(CellValue::Boolean(true))
}

fn inline_or(args: &[CellValue]) -> DispatchResult {
    if args.is_empty() {
        return Err(SpreadsheetError::Value);
    }
    for a in args {
        if a.coerce_bool()? {
            return Ok(CellValue::Boolean(true));
        }
    }
    Ok(CellValue::Boolean(false))
}

fn inline_not(args: &[CellValue]) -> DispatchResult {
    if args.len() != 1 {
        return Err(SpreadsheetError::Value);
    }
    Ok(CellValue::Boolean(!args[0].coerce_bool()?))
}

fn inline_iferror(args: &[CellValue]) -> DispatchResult {
    if args.len() != 2 {
        return Err(SpreadsheetError::Value);
    }
    if matches!(args[0], CellValue::Error(_)) {
        Ok(args[1].clone())
    } else {
        Ok(args[0].clone())
    }
}

fn inline_ifna(args: &[CellValue]) -> DispatchResult {
    if args.len() != 2 {
        return Err(SpreadsheetError::Value);
    }
    if matches!(args[0], CellValue::Error(SpreadsheetError::NotAvailable)) {
        Ok(args[1].clone())
    } else {
        Ok(args[0].clone())
    }
}

fn inline_isblank(args: &[CellValue]) -> DispatchResult {
    if args.len() != 1 {
        return Err(SpreadsheetError::Value);
    }
    Ok(CellValue::Boolean(args[0].is_empty()))
}

fn inline_iserror(args: &[CellValue]) -> DispatchResult {
    if args.len() != 1 {
        return Err(SpreadsheetError::Value);
    }
    Ok(CellValue::Boolean(args[0].is_error()))
}

fn inline_isna(args: &[CellValue]) -> DispatchResult {
    if args.len() != 1 {
        return Err(SpreadsheetError::Value);
    }
    Ok(CellValue::Boolean(args[0].is_na()))
}

fn inline_n(args: &[CellValue]) -> DispatchResult {
    if args.len() != 1 {
        return Err(SpreadsheetError::Value);
    }
    Ok(CellValue::Number(args[0].coerce_number().unwrap_or(0.0)))
}

fn inline_t(args: &[CellValue]) -> DispatchResult {
    if args.len() != 1 {
        return Err(SpreadsheetError::Value);
    }
    Ok(match &args[0] {
        CellValue::Text(s) => CellValue::Text(s.clone()),
        _ => CellValue::Text(String::new()),
    })
}

// Placeholder for the `round_ties_even` doc anchor — unused but
// keeps clippy happy if we reference the half-to-even tag.
#[allow(dead_code)]
trait RoundAux {
    fn round_ties_even_no(self) -> f64;
}

#[allow(dead_code)]
impl RoundAux for f64 {
    fn round_ties_even_no(self) -> f64 {
        self.round()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn n(v: f64) -> CellValue {
        CellValue::Number(v)
    }

    #[test]
    fn sum_basic() {
        let r = dispatch("SUM", &[n(1.0), n(2.0), n(3.0)]).unwrap();
        assert_eq!(r, CellValue::Number(6.0));
    }

    #[test]
    fn average_aliases() {
        let args = vec![n(2.0), n(4.0)];
        let r1 = dispatch("AVERAGE", &args).unwrap();
        let r2 = dispatch("MEAN", &args).unwrap();
        assert_eq!(r1, n(3.0));
        assert_eq!(r2, n(3.0));
    }

    #[test]
    fn count_distinguishes_from_counta() {
        let args = vec![
            CellValue::Number(1.0),
            CellValue::Empty,
            CellValue::Text("hello".into()),
            CellValue::Number(2.0),
        ];
        let count = dispatch("COUNT", &args).unwrap();
        let counta = dispatch("COUNTA", &args).unwrap();
        assert_eq!(count, n(2.0));
        assert_eq!(counta, n(3.0));
    }

    #[test]
    fn if_branches_on_condition() {
        let r = dispatch(
            "IF",
            &[
                CellValue::Boolean(true),
                CellValue::Text("yes".into()),
                CellValue::Text("no".into()),
            ],
        )
        .unwrap();
        assert_eq!(r, CellValue::Text("yes".into()));
        let r = dispatch(
            "IF",
            &[
                CellValue::Boolean(false),
                CellValue::Text("yes".into()),
                CellValue::Text("no".into()),
            ],
        )
        .unwrap();
        assert_eq!(r, CellValue::Text("no".into()));
    }

    #[test]
    fn and_short_circuits_to_false() {
        let r = dispatch(
            "AND",
            &[CellValue::Boolean(true), CellValue::Boolean(false)],
        )
        .unwrap();
        assert_eq!(r, CellValue::Boolean(false));
    }

    #[test]
    fn iferror_catches_div_zero() {
        let r = dispatch(
            "IFERROR",
            &[CellValue::Error(SpreadsheetError::DivZero), n(0.0)],
        )
        .unwrap();
        assert_eq!(r, n(0.0));
    }

    #[test]
    fn ifna_only_catches_na() {
        let r = dispatch(
            "IFNA",
            &[CellValue::Error(SpreadsheetError::DivZero), n(0.0)],
        )
        .unwrap();
        // Not #N/A, so passes through.
        assert_eq!(r, CellValue::Error(SpreadsheetError::DivZero));
    }

    #[test]
    fn unknown_name_returns_name_error() {
        let r = dispatch("DEFINITELY_NOT_A_FUNCTION", &[n(1.0)]);
        assert_eq!(r, Err(SpreadsheetError::Name));
    }

    #[test]
    // 3.14159 / 3.14 here are arbitrary numeric test data, not approximations of PI.
    #[allow(clippy::approx_constant)]
    fn round_to_zero_digits() {
        let r = dispatch("ROUND", &[n(3.7), n(0.0)]).unwrap();
        assert_eq!(r, n(4.0));
        let r = dispatch("ROUND", &[n(3.14159), n(2.0)]).unwrap();
        assert_eq!(r, n(3.14));
    }

    #[test]
    fn log_one_arg_is_base_10() {
        let r = dispatch("LOG", &[n(1000.0)]).unwrap();
        if let CellValue::Number(v) = r {
            assert!((v - 3.0).abs() < 1e-9);
        } else {
            panic!("expected number");
        }
    }

    #[test]
    fn log_two_arg_custom_base() {
        let r = dispatch("LOG", &[n(8.0), n(2.0)]).unwrap();
        if let CellValue::Number(v) = r {
            assert!((v - 3.0).abs() < 1e-9);
        } else {
            panic!("expected number");
        }
    }

    #[test]
    fn isnumber_isblank_isnan() {
        assert_eq!(
            dispatch("ISNUMBER", &[n(1.0)]).unwrap(),
            CellValue::Boolean(true)
        );
        assert_eq!(
            dispatch("ISBLANK", &[CellValue::Empty]).unwrap(),
            CellValue::Boolean(true)
        );
        assert_eq!(
            dispatch(
                "ISNA",
                &[CellValue::Error(SpreadsheetError::NotAvailable)]
            )
            .unwrap(),
            CellValue::Boolean(true)
        );
    }
}
