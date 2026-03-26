//! Join logic — five SQL join types.
//!
//! SQL join types:
//!
//! | Type       | Description                                          |
//! |------------|------------------------------------------------------|
//! | INNER      | Only rows matching the ON condition on both sides    |
//! | LEFT OUTER | All left rows; unmatched left rows get NULL right    |
//! | RIGHT OUTER| All right rows; unmatched right rows get NULL left   |
//! | FULL OUTER | All rows from both sides; NULLs for unmatched sides  |
//! | CROSS      | Cartesian product (no ON condition)                  |
//!
//! # Implementation
//!
//! All joins use nested-loop algorithm: O(|left| × |right|).
//! Sufficient for an educational engine. Production engines use
//! hash joins or merge joins for large tables.

use std::collections::HashMap;
use std::collections::HashSet;

use parser::grammar_parser::ASTNodeOrToken;

use crate::data_source::SqlValue;
use crate::errors::ExecutionError;
use crate::expression::eval_expr;

/// Perform a SQL join between two sets of rows.
///
/// # Arguments
///
/// * `left_rows`   — qualified rows from the left table
/// * `right_rows`  — rows from the right table (will be qualified)
/// * `right_alias` — alias for the right table
/// * `join_type`   — `"INNER"`, `"LEFT"`, `"RIGHT"`, `"FULL"`, or `"CROSS"`
/// * `on_condition`— the ON expression node, or `None` for CROSS JOIN
pub fn perform_join(
    left_rows: &[HashMap<String, SqlValue>],
    right_rows: &[HashMap<String, SqlValue>],
    right_alias: &str,
    join_type: &str,
    on_condition: Option<&ASTNodeOrToken>,
) -> Result<Vec<HashMap<String, SqlValue>>, ExecutionError> {
    if join_type == "CROSS" || on_condition.is_none() {
        return Ok(cross_join(left_rows, right_alias, right_rows));
    }

    let condition = on_condition.unwrap();
    let null_right = null_row_for(right_rows, Some(right_alias));
    let null_left = null_row_for(left_rows, None);

    match join_type {
        "INNER" => inner_join(left_rows, right_rows, right_alias, condition),
        "LEFT" | "LEFT OUTER" => {
            left_join(left_rows, right_rows, right_alias, &null_right, condition)
        }
        "RIGHT" | "RIGHT OUTER" => {
            right_join(left_rows, right_rows, right_alias, &null_left, condition)
        }
        "FULL" | "FULL OUTER" => {
            full_join(left_rows, right_rows, right_alias, &null_left, &null_right, condition)
        }
        _ => inner_join(left_rows, right_rows, right_alias, condition),
    }
}

fn inner_join(
    left_rows: &[HashMap<String, SqlValue>],
    right_rows: &[HashMap<String, SqlValue>],
    right_alias: &str,
    condition: &ASTNodeOrToken,
) -> Result<Vec<HashMap<String, SqlValue>>, ExecutionError> {
    let mut result = Vec::new();
    for lrow in left_rows {
        for rrow in right_rows {
            let merged = merge_rows(lrow, right_alias, rrow);
            if test_condition(condition, &merged)? {
                result.push(merged);
            }
        }
    }
    Ok(result)
}

fn left_join(
    left_rows: &[HashMap<String, SqlValue>],
    right_rows: &[HashMap<String, SqlValue>],
    right_alias: &str,
    null_right: &HashMap<String, SqlValue>,
    condition: &ASTNodeOrToken,
) -> Result<Vec<HashMap<String, SqlValue>>, ExecutionError> {
    let mut result = Vec::new();
    for lrow in left_rows {
        let mut matched = false;
        for rrow in right_rows {
            let merged = merge_rows(lrow, right_alias, rrow);
            if test_condition(condition, &merged)? {
                result.push(merged);
                matched = true;
            }
        }
        if !matched {
            result.push(merge_rows(lrow, right_alias, null_right));
        }
    }
    Ok(result)
}

fn right_join(
    left_rows: &[HashMap<String, SqlValue>],
    right_rows: &[HashMap<String, SqlValue>],
    right_alias: &str,
    null_left: &HashMap<String, SqlValue>,
    condition: &ASTNodeOrToken,
) -> Result<Vec<HashMap<String, SqlValue>>, ExecutionError> {
    let mut result = Vec::new();
    for rrow in right_rows {
        let mut matched = false;
        for lrow in left_rows {
            let merged = merge_rows(lrow, right_alias, rrow);
            if test_condition(condition, &merged)? {
                result.push(merged);
                matched = true;
            }
        }
        if !matched {
            result.push(merge_rows(null_left, right_alias, rrow));
        }
    }
    Ok(result)
}

fn full_join(
    left_rows: &[HashMap<String, SqlValue>],
    right_rows: &[HashMap<String, SqlValue>],
    right_alias: &str,
    null_left: &HashMap<String, SqlValue>,
    null_right: &HashMap<String, SqlValue>,
    condition: &ASTNodeOrToken,
) -> Result<Vec<HashMap<String, SqlValue>>, ExecutionError> {
    let mut result = Vec::new();
    let mut matched_right: HashSet<usize> = HashSet::new();

    for lrow in left_rows {
        let mut left_matched = false;
        for (i, rrow) in right_rows.iter().enumerate() {
            let merged = merge_rows(lrow, right_alias, rrow);
            if test_condition(condition, &merged)? {
                result.push(merged);
                left_matched = true;
                matched_right.insert(i);
            }
        }
        if !left_matched {
            result.push(merge_rows(lrow, right_alias, null_right));
        }
    }

    for (i, rrow) in right_rows.iter().enumerate() {
        if !matched_right.contains(&i) {
            result.push(merge_rows(null_left, right_alias, rrow));
        }
    }

    Ok(result)
}

fn cross_join(
    left_rows: &[HashMap<String, SqlValue>],
    right_alias: &str,
    right_rows: &[HashMap<String, SqlValue>],
) -> Vec<HashMap<String, SqlValue>> {
    left_rows.iter().flat_map(|lrow| {
        right_rows.iter().map(move |rrow| merge_rows(lrow, right_alias, rrow))
    }).collect()
}

/// Merge a left row and a right row into a single flat map.
///
/// Right row keys are stored both with and without the `right_alias.` prefix.
pub fn merge_rows(
    left: &HashMap<String, SqlValue>,
    right_alias: &str,
    right: &HashMap<String, SqlValue>,
) -> HashMap<String, SqlValue> {
    let mut merged = left.clone();
    for (key, val) in right {
        if key.contains('.') {
            merged.insert(key.clone(), val.clone());
        } else {
            merged.insert(format!("{right_alias}.{key}"), val.clone());
            merged.insert(key.clone(), val.clone());
        }
    }
    merged
}

/// Build a NULL-value row from the schema of rows.
fn null_row_for(
    rows: &[HashMap<String, SqlValue>],
    _alias: Option<&str>,
) -> HashMap<String, SqlValue> {
    if rows.is_empty() { return HashMap::new(); }
    rows[0].keys().map(|k| (k.clone(), None)).collect()
}

/// Evaluate the ON condition; returns true if the condition is met.
fn test_condition(
    condition: &ASTNodeOrToken,
    merged: &HashMap<String, SqlValue>,
) -> Result<bool, ExecutionError> {
    let val = eval_expr(condition, merged)?;
    Ok(matches!(val, Some(crate::data_source::SqlPrimitive::Bool(true))))
}
