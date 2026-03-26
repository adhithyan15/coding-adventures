//! Aggregate functions — COUNT, SUM, AVG, MIN, MAX.
//!
//! Aggregate functions summarize a group of rows into a single value.
//!
//! # NULL-skipping semantics
//!
//! All aggregate functions except `COUNT(*)` ignore NULL values.
//! If all values are NULL, SUM/AVG/MIN/MAX return NULL (`None`).
//! COUNT returns 0 in that case.
//!
//! # Aggregate key convention
//!
//! Computed values are stored back into the representative row under
//! keys like `"_agg_COUNT(*)"`, `"_agg_SUM(salary)"`, etc. so that
//! the SELECT projection and HAVING clause can reference them.

use std::collections::HashMap;

use crate::data_source::{SqlPrimitive, SqlValue};

/// Compute aggregate functions over a group of rows.
///
/// # Arguments
///
/// * `rows`      — all rows in the current group
/// * `agg_specs` — list of `(function_name, argument)` pairs,
///                 e.g. `[("COUNT", "*"), ("SUM", "salary")]`
///
/// # Returns
///
/// A map from `"_agg_FUNC(arg)"` keys to their computed `SqlValue`.
pub fn compute_aggregates(
    rows: &[HashMap<String, SqlValue>],
    agg_specs: &[(String, String)],
) -> HashMap<String, SqlValue> {
    let mut result = HashMap::new();
    for (func_name, arg) in agg_specs {
        let key = format!("_agg_{}({})", func_name.to_uppercase(), arg);
        let val = compute_one(rows, func_name, arg);
        result.insert(key, val);
    }
    result
}

/// Compute a single aggregate function over rows.
fn compute_one(rows: &[HashMap<String, SqlValue>], func: &str, arg: &str) -> SqlValue {
    match func.to_uppercase().as_str() {
        "COUNT" => {
            if arg == "*" {
                Some(SqlPrimitive::Int(rows.len() as i64))
            } else {
                let count = rows.iter()
                    .filter(|row| get_val(row, arg).is_some())
                    .count();
                Some(SqlPrimitive::Int(count as i64))
            }
        }
        "SUM" => {
            let vals: Vec<f64> = rows.iter()
                .filter_map(|row| to_float(get_val(row, arg)))
                .collect();
            if vals.is_empty() {
                None
            } else {
                let s: f64 = vals.iter().sum();
                // Return Int if result is a whole number
                if s.fract() == 0.0 && s.abs() < i64::MAX as f64 {
                    Some(SqlPrimitive::Int(s as i64))
                } else {
                    Some(SqlPrimitive::Float(s))
                }
            }
        }
        "AVG" => {
            let vals: Vec<f64> = rows.iter()
                .filter_map(|row| to_float(get_val(row, arg)))
                .collect();
            if vals.is_empty() {
                None
            } else {
                Some(SqlPrimitive::Float(vals.iter().sum::<f64>() / vals.len() as f64))
            }
        }
        "MIN" => {
            let vals: Vec<OrderableVal> = rows.iter()
                .filter_map(|row| as_orderable(get_val(row, arg)))
                .collect();
            vals.into_iter().min().map(orderable_to_sql)
        }
        "MAX" => {
            let vals: Vec<OrderableVal> = rows.iter()
                .filter_map(|row| as_orderable(get_val(row, arg)))
                .collect();
            vals.into_iter().max().map(orderable_to_sql)
        }
        _ => None,
    }
}

/// Get a column value from a row by name (case-insensitive).
fn get_val<'a>(row: &'a HashMap<String, SqlValue>, col: &str) -> &'a SqlValue {
    if let Some(v) = row.get(col) {
        return v;
    }
    let col_lower = col.to_lowercase();
    for (k, v) in row {
        if k.to_lowercase() == col_lower {
            return v;
        }
    }
    &None
}

/// Convert a SqlValue to f64 for numeric aggregation.
fn to_float(val: &SqlValue) -> Option<f64> {
    match val {
        Some(SqlPrimitive::Int(i)) => Some(*i as f64),
        Some(SqlPrimitive::Float(f)) => Some(*f),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Orderable wrapper for MIN/MAX
// ---------------------------------------------------------------------------

/// An orderable representation for comparing SqlPrimitive values.
///
/// We can't directly derive Ord on SqlPrimitive because f64 doesn't implement Ord.
/// We handle this by comparing floats with total ordering (NaN sorts last).
#[derive(PartialEq)]
enum OrderableVal {
    Int(i64),
    Float(f64),
    Text(String),
    Bool(bool),
}

impl Eq for OrderableVal {}

impl PartialOrd for OrderableVal {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for OrderableVal {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self, other) {
            (OrderableVal::Int(a), OrderableVal::Int(b)) => a.cmp(b),
            (OrderableVal::Float(a), OrderableVal::Float(b)) => {
                a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
            }
            (OrderableVal::Int(a), OrderableVal::Float(b)) => {
                (*a as f64).partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
            }
            (OrderableVal::Float(a), OrderableVal::Int(b)) => {
                a.partial_cmp(&(*b as f64)).unwrap_or(std::cmp::Ordering::Equal)
            }
            (OrderableVal::Text(a), OrderableVal::Text(b)) => a.cmp(b),
            (OrderableVal::Bool(a), OrderableVal::Bool(b)) => a.cmp(b),
            _ => std::cmp::Ordering::Equal,
        }
    }
}

fn as_orderable(val: &SqlValue) -> Option<OrderableVal> {
    match val {
        Some(SqlPrimitive::Int(i)) => Some(OrderableVal::Int(*i)),
        Some(SqlPrimitive::Float(f)) => Some(OrderableVal::Float(*f)),
        Some(SqlPrimitive::Text(s)) => Some(OrderableVal::Text(s.clone())),
        Some(SqlPrimitive::Bool(b)) => Some(OrderableVal::Bool(*b)),
        None => None,
    }
}

fn orderable_to_sql(val: OrderableVal) -> SqlPrimitive {
    match val {
        OrderableVal::Int(i) => SqlPrimitive::Int(i),
        OrderableVal::Float(f) => SqlPrimitive::Float(f),
        OrderableVal::Text(s) => SqlPrimitive::Text(s),
        OrderableVal::Bool(b) => SqlPrimitive::Bool(b),
    }
}
