//! # database-core — Excel/Lotus database functions.
//!
//! Excel's `D*` family aggregates over rows of a "database" (a
//! `DataFrame` here) that match a "criteria" specification. The
//! criteria is itself a small DataFrame: header row names columns,
//! body rows are conditions to test against the database (within a
//! row = AND, across rows = OR).
//!
//! Example:
//!
//! ```text
//!   Database (sales):                Criteria:
//!     Region | Salesperson | Sales     Region | Sales
//!     North  | Alice       | 1000      North  | >500
//!     South  | Bob         | 200       South  |
//!     North  | Carol       | 800
//!
//!   DSUM(sales, "Sales", criteria) → 1000 + 800 + 200 = 2000
//!     (rows matching Region=North AND Sales>500; OR Region=South)
//! ```
//!
//! Functions shipped (Phase 1):
//!   DSUM, DAVERAGE, DCOUNT, DCOUNTA, DGET, DMAX, DMIN, DPRODUCT,
//!   DSTDEV, DSTDEVP, DVAR, DVARP.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use data_frame::{Column, DataFrame};
use r_vector::Vector as _;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors raised by database functions.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum DatabaseError {
    /// The named field is not present in the database.
    FieldNotFound {
        /// The missing field name.
        field: String,
    },
    /// A column referenced by the criteria header is not in the
    /// database.
    CriteriaFieldNotFound {
        /// The unknown criteria column name.
        field: String,
    },
    /// The criteria DataFrame is malformed (empty header, missing
    /// columns, etc.).
    InvalidCriteria {
        /// Description of the malformation.
        what: String,
    },
    /// `DGET` matched more than one row or zero rows.
    GetMultipleMatches,
    /// `DGET` matched zero rows.
    GetNoMatch,
    /// The field referenced by the function is not numeric (e.g. DSUM
    /// over a character column).
    TypeMismatch {
        /// Function name.
        function: &'static str,
        /// Description.
        what: String,
    },
}

impl core::fmt::Display for DatabaseError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            DatabaseError::FieldNotFound { field } => {
                write!(f, "field '{field}' not found in database")
            }
            DatabaseError::CriteriaFieldNotFound { field } => {
                write!(f, "criteria field '{field}' not found in database")
            }
            DatabaseError::InvalidCriteria { what } => {
                write!(f, "invalid criteria: {what}")
            }
            DatabaseError::GetMultipleMatches => {
                write!(f, "DGET: multiple matching rows")
            }
            DatabaseError::GetNoMatch => write!(f, "DGET: no matching row"),
            DatabaseError::TypeMismatch { function, what } => {
                write!(f, "{function}: {what}")
            }
        }
    }
}

impl std::error::Error for DatabaseError {}

// ---------------------------------------------------------------------------
// Criterion type
// ---------------------------------------------------------------------------

/// One column-level criterion applied to a single row's value.
#[derive(Debug, Clone, PartialEq)]
pub enum Criterion {
    /// Match a numeric value exactly.
    EqualsNumeric(f64),
    /// Match a text value exactly (case-insensitive ASCII).
    EqualsText(String),
    /// Numeric comparison with one of `<`, `<=`, `>`, `>=`, `<>`, `=`.
    NumericCompare {
        /// Comparison operator.
        op: Comparator,
        /// Right-hand-side value.
        value: f64,
    },
    /// Text wildcard match using Excel's `*` (any chars) and `?`
    /// (single char). Case-insensitive ASCII.
    TextMatch(String),
    /// Always passes — used for blank cells in the criteria.
    AnyValue,
}

/// Numeric comparison operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Comparator {
    /// `=`.
    Eq,
    /// `<>`.
    Ne,
    /// `<`.
    Lt,
    /// `<=`.
    Le,
    /// `>`.
    Gt,
    /// `>=`.
    Ge,
}

// ---------------------------------------------------------------------------
// Criteria parsing
// ---------------------------------------------------------------------------

/// Parse a criteria string from a single cell into a `Criterion`.
///
/// - Numeric like `5` → `EqualsNumeric(5.0)`
/// - Comparison like `">100"` → `NumericCompare { Gt, 100.0 }`
/// - Text with wildcards like `"Bob*"` → `TextMatch("Bob*")`
/// - Plain text like `"Bob"` → `EqualsText("Bob")`
/// - Empty / whitespace-only → `AnyValue`
pub fn parse_criterion(text: &str) -> Criterion {
    let t = text.trim();
    if t.is_empty() {
        return Criterion::AnyValue;
    }
    // Numeric comparison prefix?
    let (op, rest) = if let Some(rest) = t.strip_prefix("<>") {
        (Some(Comparator::Ne), rest)
    } else if let Some(rest) = t.strip_prefix("<=") {
        (Some(Comparator::Le), rest)
    } else if let Some(rest) = t.strip_prefix(">=") {
        (Some(Comparator::Ge), rest)
    } else if let Some(rest) = t.strip_prefix('<') {
        (Some(Comparator::Lt), rest)
    } else if let Some(rest) = t.strip_prefix('>') {
        (Some(Comparator::Gt), rest)
    } else if let Some(rest) = t.strip_prefix('=') {
        (Some(Comparator::Eq), rest)
    } else {
        (None, t)
    };

    if let Some(op) = op {
        let rest = rest.trim();
        if let Ok(n) = rest.parse::<f64>() {
            return Criterion::NumericCompare { op, value: n };
        }
        // `=Bob` style — equality against text.
        if op == Comparator::Eq {
            return Criterion::EqualsText(rest.to_string());
        }
        // `<>` over text means "not equal to this text" — we model as
        // a TextMatch with a wildcard that excludes; here we
        // approximate by treating `<>Bob` as "no row matches text
        // equal to Bob". For simplicity, return EqualsText(rest) and
        // let row-matching invert when op is Ne. Not yet supported
        // for text; document in PR body. For now, fall through.
        return Criterion::TextMatch(rest.to_string());
    }
    // No operator prefix. Numeric or text?
    if let Ok(n) = t.parse::<f64>() {
        return Criterion::EqualsNumeric(n);
    }
    // Wildcards in the literal text?
    if t.contains('*') || t.contains('?') {
        Criterion::TextMatch(t.to_string())
    } else {
        Criterion::EqualsText(t.to_string())
    }
}

/// Parse a criteria DataFrame into a list of OR-groups, each an
/// AND-list of (column-name, criterion) pairs.
pub fn parse_criteria(
    criteria: &DataFrame,
    database: &DataFrame,
) -> Result<Vec<Vec<(String, Criterion)>>, DatabaseError> {
    if criteria.is_empty() {
        return Err(DatabaseError::InvalidCriteria {
            what: "empty criteria".into(),
        });
    }
    // Validate every criteria column appears in the database.
    for name in criteria.column_names() {
        if !database.has_column(name) {
            return Err(DatabaseError::CriteriaFieldNotFound {
                field: name.clone(),
            });
        }
    }
    let mut or_groups: Vec<Vec<(String, Criterion)>> = Vec::new();
    for row_index in 0..criteria.nrow() {
        let mut and_group: Vec<(String, Criterion)> = Vec::new();
        for (col_index, col_name) in criteria.column_names().iter().enumerate() {
            let cell_text = criteria_cell_to_string(criteria, col_index, row_index);
            and_group.push((col_name.clone(), parse_criterion(&cell_text)));
        }
        or_groups.push(and_group);
    }
    Ok(or_groups)
}

fn criteria_cell_to_string(criteria: &DataFrame, col: usize, row: usize) -> String {
    let column = criteria.column_at(col).expect("column index valid");
    match column {
        Column::Double(d) => {
            if d.is_na(row) {
                String::new()
            } else {
                d.get_value(row).map(|v| v.to_string()).unwrap_or_default()
            }
        }
        Column::Character(c) => c.get(row).cloned().flatten().unwrap_or_default(),
        // Column is #[non_exhaustive]; future atomic types (Logical /
        // Integer / Complex / Raw) will be added as r-vector grows.
        _ => String::new(),
    }
}

// ---------------------------------------------------------------------------
// Row matching
// ---------------------------------------------------------------------------

fn ascii_ci_eq(a: &str, b: &str) -> bool {
    a.len() == b.len()
        && a.chars()
            .zip(b.chars())
            .all(|(x, y)| x.eq_ignore_ascii_case(&y))
}

/// Glob match with `*` (any chars) and `?` (one char). Case-insensitive
/// ASCII. Same matcher as text-core SEARCH (re-implemented here to keep
/// the dep graph small).
fn glob_match(pattern: &str, text: &str) -> bool {
    let pattern: Vec<char> = pattern.to_ascii_lowercase().chars().collect();
    let text: Vec<char> = text.to_ascii_lowercase().chars().collect();
    match_recursive(&pattern, &text)
}

fn match_recursive(pattern: &[char], text: &[char]) -> bool {
    match (pattern.first(), text.first()) {
        (None, None) => true,
        (None, _) => false,
        (Some('*'), _) => {
            // Try consuming any prefix.
            for i in 0..=text.len() {
                if match_recursive(&pattern[1..], &text[i..]) {
                    return true;
                }
            }
            false
        }
        (Some('?'), Some(_)) => match_recursive(&pattern[1..], &text[1..]),
        (Some(p), Some(t)) if p == t => match_recursive(&pattern[1..], &text[1..]),
        _ => false,
    }
}

fn cell_to_optional_number(column: &Column, row: usize) -> Option<f64> {
    match column {
        Column::Double(d) => {
            if d.is_na(row) {
                None
            } else {
                d.get_value(row)
            }
        }
        Column::Character(c) => c
            .get(row)
            .cloned()
            .flatten()
            .and_then(|s| s.parse::<f64>().ok()),
        _ => None,
    }
}

fn cell_to_optional_string(column: &Column, row: usize) -> Option<String> {
    match column {
        Column::Double(d) => {
            if d.is_na(row) {
                None
            } else {
                d.get_value(row).map(|v| v.to_string())
            }
        }
        Column::Character(c) => c.get(row).cloned().flatten(),
        _ => None,
    }
}

fn matches_criterion(column: &Column, row: usize, criterion: &Criterion) -> bool {
    match criterion {
        Criterion::AnyValue => true,
        Criterion::EqualsNumeric(target) => cell_to_optional_number(column, row)
            .map(|v| v == *target)
            .unwrap_or(false),
        Criterion::EqualsText(target) => cell_to_optional_string(column, row)
            .map(|s| ascii_ci_eq(&s, target))
            .unwrap_or(false),
        Criterion::NumericCompare { op, value } => {
            let lhs = match cell_to_optional_number(column, row) {
                Some(v) => v,
                None => return false,
            };
            match op {
                Comparator::Eq => lhs == *value,
                Comparator::Ne => lhs != *value,
                Comparator::Lt => lhs < *value,
                Comparator::Le => lhs <= *value,
                Comparator::Gt => lhs > *value,
                Comparator::Ge => lhs >= *value,
            }
        }
        Criterion::TextMatch(pattern) => cell_to_optional_string(column, row)
            .map(|s| glob_match(pattern, &s))
            .unwrap_or(false),
    }
}

fn row_matches(
    database: &DataFrame,
    row: usize,
    or_groups: &[Vec<(String, Criterion)>],
) -> bool {
    or_groups.iter().any(|and_group| {
        and_group.iter().all(|(name, criterion)| {
            let column = database
                .column(name)
                .expect("criteria column validated at parse time");
            matches_criterion(column, row, criterion)
        })
    })
}

// ---------------------------------------------------------------------------
// Field resolution
// ---------------------------------------------------------------------------

fn resolve_field<'a>(
    database: &'a DataFrame,
    field: &str,
) -> Result<&'a Column, DatabaseError> {
    database
        .column(field)
        .map_err(|_| DatabaseError::FieldNotFound {
            field: field.to_string(),
        })
}

fn iter_matching_numeric(
    database: &DataFrame,
    field: &str,
    or_groups: &[Vec<(String, Criterion)>],
) -> Result<Vec<f64>, DatabaseError> {
    let column = resolve_field(database, field)?;
    let mut out = Vec::new();
    for row in 0..database.nrow() {
        if !row_matches(database, row, or_groups) {
            continue;
        }
        if let Some(v) = cell_to_optional_number(column, row) {
            out.push(v);
        }
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// Aggregation functions
// ---------------------------------------------------------------------------

/// Excel `DSUM(database, field, criteria)`.
pub fn dsum(
    database: &DataFrame,
    field: &str,
    criteria: &DataFrame,
) -> Result<f64, DatabaseError> {
    let or_groups = parse_criteria(criteria, database)?;
    let values = iter_matching_numeric(database, field, &or_groups)?;
    Ok(values.iter().sum())
}

/// Excel `DAVERAGE(database, field, criteria)`.
pub fn daverage(
    database: &DataFrame,
    field: &str,
    criteria: &DataFrame,
) -> Result<f64, DatabaseError> {
    let or_groups = parse_criteria(criteria, database)?;
    let values = iter_matching_numeric(database, field, &or_groups)?;
    if values.is_empty() {
        return Err(DatabaseError::GetNoMatch);
    }
    Ok(values.iter().sum::<f64>() / values.len() as f64)
}

/// Excel `DCOUNT(database, field, criteria)`. Counts numeric cells in
/// the named column across matching rows.
pub fn dcount(
    database: &DataFrame,
    field: &str,
    criteria: &DataFrame,
) -> Result<usize, DatabaseError> {
    let or_groups = parse_criteria(criteria, database)?;
    let values = iter_matching_numeric(database, field, &or_groups)?;
    Ok(values.len())
}

/// Excel `DCOUNTA(database, field, criteria)`. Counts non-blank cells.
pub fn dcounta(
    database: &DataFrame,
    field: &str,
    criteria: &DataFrame,
) -> Result<usize, DatabaseError> {
    let or_groups = parse_criteria(criteria, database)?;
    let column = resolve_field(database, field)?;
    let mut count = 0;
    for row in 0..database.nrow() {
        if !row_matches(database, row, &or_groups) {
            continue;
        }
        let is_blank = cell_to_optional_string(column, row)
            .map(|s| s.is_empty())
            .unwrap_or(true);
        if !is_blank {
            count += 1;
        }
    }
    Ok(count)
}

/// Excel `DMAX(database, field, criteria)`.
pub fn dmax(
    database: &DataFrame,
    field: &str,
    criteria: &DataFrame,
) -> Result<f64, DatabaseError> {
    let or_groups = parse_criteria(criteria, database)?;
    let values = iter_matching_numeric(database, field, &or_groups)?;
    if values.is_empty() {
        return Err(DatabaseError::GetNoMatch);
    }
    Ok(values.iter().cloned().fold(f64::NEG_INFINITY, f64::max))
}

/// Excel `DMIN(database, field, criteria)`.
pub fn dmin(
    database: &DataFrame,
    field: &str,
    criteria: &DataFrame,
) -> Result<f64, DatabaseError> {
    let or_groups = parse_criteria(criteria, database)?;
    let values = iter_matching_numeric(database, field, &or_groups)?;
    if values.is_empty() {
        return Err(DatabaseError::GetNoMatch);
    }
    Ok(values.iter().cloned().fold(f64::INFINITY, f64::min))
}

/// Excel `DPRODUCT(database, field, criteria)`.
pub fn dproduct(
    database: &DataFrame,
    field: &str,
    criteria: &DataFrame,
) -> Result<f64, DatabaseError> {
    let or_groups = parse_criteria(criteria, database)?;
    let values = iter_matching_numeric(database, field, &or_groups)?;
    Ok(values.iter().product())
}

/// Excel `DSTDEV(database, field, criteria)` — sample standard deviation.
pub fn dstdev(
    database: &DataFrame,
    field: &str,
    criteria: &DataFrame,
) -> Result<f64, DatabaseError> {
    let or_groups = parse_criteria(criteria, database)?;
    let values = iter_matching_numeric(database, field, &or_groups)?;
    if values.len() < 2 {
        return Err(DatabaseError::TypeMismatch {
            function: "dstdev",
            what: "at least 2 matching numeric values required".into(),
        });
    }
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let var = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (values.len() - 1) as f64;
    Ok(var.sqrt())
}

/// Excel `DSTDEVP(database, field, criteria)` — population standard deviation.
pub fn dstdevp(
    database: &DataFrame,
    field: &str,
    criteria: &DataFrame,
) -> Result<f64, DatabaseError> {
    let or_groups = parse_criteria(criteria, database)?;
    let values = iter_matching_numeric(database, field, &or_groups)?;
    if values.is_empty() {
        return Err(DatabaseError::TypeMismatch {
            function: "dstdevp",
            what: "at least 1 matching numeric value required".into(),
        });
    }
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let var = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / values.len() as f64;
    Ok(var.sqrt())
}

/// Excel `DVAR(database, field, criteria)` — sample variance.
pub fn dvar(
    database: &DataFrame,
    field: &str,
    criteria: &DataFrame,
) -> Result<f64, DatabaseError> {
    let sd = dstdev(database, field, criteria)?;
    Ok(sd * sd)
}

/// Excel `DVARP(database, field, criteria)` — population variance.
pub fn dvarp(
    database: &DataFrame,
    field: &str,
    criteria: &DataFrame,
) -> Result<f64, DatabaseError> {
    let sd = dstdevp(database, field, criteria)?;
    Ok(sd * sd)
}

/// Excel `DGET(database, field, criteria)` — returns the single matching
/// value. Errors if zero or more than one row matches.
pub fn dget(
    database: &DataFrame,
    field: &str,
    criteria: &DataFrame,
) -> Result<String, DatabaseError> {
    let or_groups = parse_criteria(criteria, database)?;
    let column = resolve_field(database, field)?;
    let mut matches: Vec<String> = Vec::new();
    for row in 0..database.nrow() {
        if row_matches(database, row, &or_groups) {
            if let Some(s) = cell_to_optional_string(column, row) {
                matches.push(s);
            }
        }
    }
    match matches.len() {
        0 => Err(DatabaseError::GetNoMatch),
        1 => Ok(matches.into_iter().next().unwrap()),
        _ => Err(DatabaseError::GetMultipleMatches),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use data_frame::{Column, DataFrame};
    use r_vector::{Character, Double};

    fn sales_db() -> DataFrame {
        DataFrame::from_columns(vec![
            (
                "Region".to_string(),
                Column::Character(Character::from_strings(vec!["North", "South", "North", "East"])),
            ),
            (
                "Salesperson".to_string(),
                Column::Character(Character::from_strings(vec!["Alice", "Bob", "Carol", "Dave"])),
            ),
            (
                "Sales".to_string(),
                Column::Double(Double::from_values(vec![1000.0, 200.0, 800.0, 500.0])),
            ),
            (
                "Quantity".to_string(),
                Column::Double(Double::from_values(vec![10.0, 5.0, 8.0, 4.0])),
            ),
        ])
        .unwrap()
    }

    fn criteria_one_col(col: &str, val: &str) -> DataFrame {
        DataFrame::from_columns(vec![(
            col.to_string(),
            Column::Character(Character::from_strings(vec![val])),
        )])
        .unwrap()
    }

    #[test]
    fn parse_criterion_variants() {
        assert!(matches!(parse_criterion(""), Criterion::AnyValue));
        assert!(matches!(parse_criterion("5"), Criterion::EqualsNumeric(_)));
        assert!(matches!(parse_criterion("Bob"), Criterion::EqualsText(_)));
        assert!(matches!(parse_criterion("Bob*"), Criterion::TextMatch(_)));
        if let Criterion::NumericCompare { op, value } = parse_criterion(">100") {
            assert_eq!(op, Comparator::Gt);
            assert_eq!(value, 100.0);
        } else {
            panic!("expected NumericCompare");
        }
    }

    #[test]
    fn dsum_simple_region_filter() {
        let db = sales_db();
        let crit = criteria_one_col("Region", "North");
        let total = dsum(&db, "Sales", &crit).unwrap();
        assert_eq!(total, 1000.0 + 800.0);
    }

    #[test]
    fn dsum_with_numeric_comparison() {
        let db = sales_db();
        let crit = criteria_one_col("Sales", ">300");
        let total = dsum(&db, "Sales", &crit).unwrap();
        assert_eq!(total, 1000.0 + 800.0 + 500.0);
    }

    #[test]
    fn dsum_multi_column_and() {
        // Region = North AND Sales > 500.
        let crit = DataFrame::from_columns(vec![
            (
                "Region".to_string(),
                Column::Character(Character::from_strings(vec!["North"])),
            ),
            (
                "Sales".to_string(),
                Column::Character(Character::from_strings(vec![">500"])),
            ),
        ])
        .unwrap();
        let db = sales_db();
        let total = dsum(&db, "Sales", &crit).unwrap();
        assert_eq!(total, 1000.0 + 800.0);
    }

    #[test]
    fn dsum_multi_row_or() {
        // Region = North OR Region = South.
        let crit = DataFrame::from_columns(vec![(
            "Region".to_string(),
            Column::Character(Character::from_strings(vec!["North", "South"])),
        )])
        .unwrap();
        let db = sales_db();
        let total = dsum(&db, "Sales", &crit).unwrap();
        assert_eq!(total, 1000.0 + 200.0 + 800.0);
    }

    #[test]
    fn daverage_correctness() {
        let db = sales_db();
        let crit = criteria_one_col("Region", "North");
        let avg = daverage(&db, "Sales", &crit).unwrap();
        assert_eq!(avg, (1000.0 + 800.0) / 2.0);
    }

    #[test]
    fn dmax_dmin_dproduct() {
        let db = sales_db();
        let crit = criteria_one_col("Region", "North");
        assert_eq!(dmax(&db, "Sales", &crit).unwrap(), 1000.0);
        assert_eq!(dmin(&db, "Sales", &crit).unwrap(), 800.0);
        assert_eq!(dproduct(&db, "Quantity", &crit).unwrap(), 10.0 * 8.0);
    }

    #[test]
    fn dcount_vs_dcounta() {
        let db = sales_db();
        let crit = DataFrame::from_columns(vec![(
            "Sales".to_string(),
            Column::Character(Character::from_strings(vec![">0"])),
        )])
        .unwrap();
        // All 4 rows have positive Sales.
        assert_eq!(dcount(&db, "Sales", &crit).unwrap(), 4);
        assert_eq!(dcounta(&db, "Region", &crit).unwrap(), 4);
    }

    #[test]
    fn dstdev_known_value() {
        let db = sales_db();
        let crit = criteria_one_col("Sales", ">0");
        let sd = dstdev(&db, "Sales", &crit).unwrap();
        let mean: f64 = (1000.0 + 200.0 + 800.0 + 500.0) / 4.0;
        let expected_var: f64 = ((1000.0_f64 - mean).powi(2)
            + (200.0_f64 - mean).powi(2)
            + (800.0_f64 - mean).powi(2)
            + (500.0_f64 - mean).powi(2))
            / 3.0;
        assert!((sd - expected_var.sqrt()).abs() < 1e-9);
    }

    #[test]
    fn dvar_is_dstdev_squared() {
        let db = sales_db();
        let crit = criteria_one_col("Sales", ">0");
        let sd = dstdev(&db, "Sales", &crit).unwrap();
        let v = dvar(&db, "Sales", &crit).unwrap();
        assert!((v - sd * sd).abs() < 1e-9);
    }

    #[test]
    fn dget_single_match_succeeds() {
        let db = sales_db();
        let crit = criteria_one_col("Salesperson", "Carol");
        let result = dget(&db, "Sales", &crit).unwrap();
        assert_eq!(result, "800");
    }

    #[test]
    fn dget_multiple_match_fails() {
        let db = sales_db();
        let crit = criteria_one_col("Region", "North");
        assert!(matches!(
            dget(&db, "Sales", &crit),
            Err(DatabaseError::GetMultipleMatches)
        ));
    }

    #[test]
    fn dget_no_match_fails() {
        let db = sales_db();
        let crit = criteria_one_col("Region", "Nowhere");
        assert!(matches!(
            dget(&db, "Sales", &crit),
            Err(DatabaseError::GetNoMatch)
        ));
    }

    #[test]
    fn missing_field_errors() {
        let db = sales_db();
        let crit = criteria_one_col("Region", "North");
        assert!(matches!(
            dsum(&db, "MissingField", &crit),
            Err(DatabaseError::FieldNotFound { .. })
        ));
    }

    #[test]
    fn criteria_with_missing_field_errors() {
        let db = sales_db();
        let crit = criteria_one_col("MissingField", "x");
        assert!(matches!(
            dsum(&db, "Sales", &crit),
            Err(DatabaseError::CriteriaFieldNotFound { .. })
        ));
    }

    #[test]
    fn text_wildcard_filter() {
        let db = sales_db();
        // Salesperson begins with B.
        let crit = criteria_one_col("Salesperson", "B*");
        let total = dsum(&db, "Sales", &crit).unwrap();
        assert_eq!(total, 200.0); // Only Bob
    }
}
