//! Data-frame access: the `$`, `[[ ]]`, and 2-D `[i, j]` operators.
//!
//! A data frame is a list of equal-length, named columns (`SValue::DataFrame`).
//! These helpers implement the three access forms S/R use to read from one:
//! `df$name` and `df[["name"]]` pull a whole column, while `df[i, j]` subsets by
//! rows and columns. They are deliberately tolerant — `$`/`[[` on a plain vector
//! still work where the analogous R behavior is well-defined — and they see
//! through an explicit S3 class wrapper.

use crate::error::{SError, SResult};
use crate::value::{index, SValue};
use r_vector::is_na_real;

/// `df$name` — a column by name. Errors on a non-data-frame or an unknown name.
pub fn column_by_name(value: &SValue, name: &str) -> SResult<SValue> {
    match value {
        SValue::DataFrame { names, columns } => names
            .iter()
            .position(|n| n == name)
            .map(|i| columns[i].clone())
            .ok_or_else(|| SError::Index(format!("undefined column '{name}'"))),
        SValue::Classed { inner, .. } => column_by_name(inner, name),
        other => Err(SError::TypeError(format!(
            "$ operator is invalid for {}",
            other.type_name()
        ))),
    }
}

/// `x[[k]]` — extract a single column (data frame, by name or 1-based position)
/// or a single element (any other vector).
pub fn extract(value: &SValue, key: &SValue) -> SResult<SValue> {
    match value {
        SValue::DataFrame { columns, .. } => match key {
            SValue::Character(v) => {
                let name = v
                    .first()
                    .and_then(|o| o.clone())
                    .ok_or_else(|| SError::Index("invalid column name".into()))?;
                column_by_name(value, &name)
            }
            _ => {
                let pos = scalar_index(key)?;
                columns
                    .get(pos)
                    .cloned()
                    .ok_or_else(|| SError::Index("subscript out of bounds".into()))
            }
        },
        SValue::Classed { inner, .. } => extract(inner, key),
        other => {
            // `x[[i]]` on a plain vector is single-element extraction.
            let pos = scalar_index(key)?;
            index(other, &SValue::scalar((pos + 1) as f64))
        }
    }
}

/// `df[rows, cols]` — a 2-D subset. Returns the lone column (as a vector) when a
/// single column is selected, otherwise a narrower data frame.
pub fn index2d(value: &SValue, rows: &SValue, cols: &SValue) -> SResult<SValue> {
    match value {
        SValue::DataFrame { names, columns } => {
            let picks = resolve_columns(names, columns.len(), cols)?;
            let selected: Vec<SValue> = picks
                .iter()
                .map(|&ci| index(&columns[ci], rows))
                .collect::<SResult<_>>()?;
            if selected.len() == 1 {
                Ok(selected.into_iter().next().unwrap())
            } else {
                Ok(SValue::DataFrame {
                    names: picks.iter().map(|&i| names[i].clone()).collect(),
                    columns: selected,
                })
            }
        }
        SValue::Classed { inner, .. } => index2d(inner, rows, cols),
        other => Err(SError::Index(format!(
            "incorrect number of dimensions for {}",
            other.type_name()
        ))),
    }
}

/// Resolve a column subscript (character by name, or numeric 1-based positions)
/// into 0-based column indices.
fn resolve_columns(names: &[String], ncol: usize, cols: &SValue) -> SResult<Vec<usize>> {
    match cols {
        SValue::Character(v) => v
            .iter()
            .map(|o| {
                let n = o
                    .as_ref()
                    .ok_or_else(|| SError::Index("NA column name".into()))?;
                names
                    .iter()
                    .position(|x| x == n)
                    .ok_or_else(|| SError::Index(format!("undefined column '{n}'")))
            })
            .collect(),
        _ => {
            let d = cols.as_double()?;
            d.iter()
                .filter(|x| !is_na_real(*x))
                .map(|x| {
                    let pos = x as usize;
                    if pos == 0 || pos > ncol {
                        Err(SError::Index("column subscript out of bounds".into()))
                    } else {
                        Ok(pos - 1)
                    }
                })
                .collect()
        }
    }
}

/// A single 0-based position from a (1-based) scalar subscript.
fn scalar_index(key: &SValue) -> SResult<usize> {
    let one_based = key
        .as_double()?
        .get_value(0)
        .filter(|x| !is_na_real(*x) && *x >= 1.0)
        .ok_or_else(|| SError::Index("invalid subscript".into()))?;
    Ok(one_based as usize - 1)
}
