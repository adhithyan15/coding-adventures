use crate::StatsError;
use numeric_tower::Number;
use r_vector::{is_na_real, na_real, Double};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TieMethod {
    Average,
    Min,
    Max,
    First,
    Random,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PercentRankKind {
    Inclusive,
    Exclusive,
}

pub fn rank(x: &Double, ties: TieMethod) -> Double {
    let mut output = vec![na_real(); x.len()];
    let mut values: Vec<(usize, f64)> = x
        .iter()
        .enumerate()
        .filter(|(_, value)| !is_na_real(*value))
        .collect();

    if ties == TieMethod::First {
        values.sort_by(|left, right| {
            left.1
                .total_cmp(&right.1)
                .then_with(|| left.0.cmp(&right.0))
        });
        for (rank_index, (original_index, _)) in values.into_iter().enumerate() {
            output[original_index] = (rank_index + 1) as f64;
        }
        return Double::from_values(output);
    }

    values.sort_by(|left, right| left.1.total_cmp(&right.1));

    let mut i = 0usize;
    while i < values.len() {
        let start = i;
        let value = values[i].1;
        i += 1;
        while i < values.len() && same_f64_value(values[i].1, value) {
            i += 1;
        }

        let start_rank = start + 1;
        let end_rank = i;
        let assigned = match ties {
            TieMethod::Average | TieMethod::Random => (start_rank + end_rank) as f64 / 2.0,
            TieMethod::Min => start_rank as f64,
            TieMethod::Max => end_rank as f64,
            TieMethod::First => unreachable!("handled above"),
        };

        for (original_index, _) in &values[start..i] {
            output[*original_index] = assigned;
        }
    }

    Double::from_values(output)
}

pub fn large(x: &Double, k: usize) -> Result<Number, StatsError> {
    kth_order("large", x, k, true)
}

pub fn small(x: &Double, k: usize) -> Result<Number, StatsError> {
    kth_order("small", x, k, false)
}

pub fn percent_rank(x: &Double, value: f64, kind: PercentRankKind) -> Result<f64, StatsError> {
    let mut values: Vec<f64> = x.iter().filter(|value| !is_na_real(*value)).collect();
    values.sort_by(|left, right| left.total_cmp(right));

    if values.len() < 2 {
        return Err(StatsError::EmptyInput {
            function: "percent_rank",
            min_n: 2,
        });
    }

    let first = values[0];
    let last = values[values.len() - 1];
    if value < first || value > last {
        return Ok(f64::NAN);
    }

    if value == first {
        return Ok(match kind {
            PercentRankKind::Inclusive => 0.0,
            PercentRankKind::Exclusive => 1.0 / (values.len() as f64 + 1.0),
        });
    }

    if value == last {
        return Ok(match kind {
            PercentRankKind::Inclusive => 1.0,
            PercentRankKind::Exclusive => values.len() as f64 / (values.len() as f64 + 1.0),
        });
    }

    for pair in values.windows(2).enumerate() {
        let (index, window) = pair;
        let left = window[0];
        let right = window[1];
        if value >= left && value <= right {
            let fraction = if right == left {
                0.0
            } else {
                (value - left) / (right - left)
            };
            let raw_rank = index as f64 + fraction;
            return Ok(match kind {
                PercentRankKind::Inclusive => raw_rank / (values.len() as f64 - 1.0),
                PercentRankKind::Exclusive => (raw_rank + 1.0) / (values.len() as f64 + 1.0),
            });
        }
    }

    Ok(f64::NAN)
}

fn kth_order(
    function: &'static str,
    x: &Double,
    k: usize,
    descending: bool,
) -> Result<Number, StatsError> {
    if k == 0 {
        return Err(StatsError::BadParameter {
            name: "k",
            value: k.to_string(),
        });
    }

    let mut values: Vec<f64> = x.iter().filter(|value| !is_na_real(*value)).collect();
    if values.is_empty() {
        return Err(StatsError::EmptyInput { function, min_n: 1 });
    }

    values.sort_by(|left, right| {
        if descending {
            right.total_cmp(left)
        } else {
            left.total_cmp(right)
        }
    });

    values
        .get(k - 1)
        .copied()
        .map(Number::Float)
        .ok_or_else(|| StatsError::BadParameter {
            name: "k",
            value: k.to_string(),
        })
}

fn same_f64_value(left: f64, right: f64) -> bool {
    left == right || left.to_bits() == right.to_bits()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn average_rank_assigns_mean_rank_to_ties() {
        let x = Double::from_values(vec![10.0, 20.0, 20.0, 30.0, na_real()]);
        let out = rank(&x, TieMethod::Average);
        assert_eq!(out.get_value(0), Some(1.0));
        assert_eq!(out.get_value(1), Some(2.5));
        assert_eq!(out.get_value(2), Some(2.5));
        assert_eq!(out.get_value(3), Some(4.0));
        assert!(is_na_real(out.get_value(4).unwrap()));
    }

    #[test]
    fn large_and_small_are_one_based() {
        let x = Double::from_values(vec![3.0, 1.0, 4.0, 2.0]);
        assert_eq!(large(&x, 2).unwrap(), Number::Float(3.0));
        assert_eq!(small(&x, 2).unwrap(), Number::Float(2.0));
        assert!(small(&x, 0).is_err());
    }
}
