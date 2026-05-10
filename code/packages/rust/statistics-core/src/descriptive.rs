use crate::StatsError;
use numeric_tower::Number;
use r_vector::{is_na_real, na_real, Double};

type StatsResult<T> = Result<T, StatsError>;

pub fn sum(x: &Double, na_rm: bool) -> StatsResult<Number> {
    match reduction_values("sum", x, na_rm)? {
        ReductionValues::Na => Ok(na_number()),
        ReductionValues::Values(values) => Ok(Number::Float(kahan_sum(values.iter().copied()))),
    }
}

pub fn prod(x: &Double, na_rm: bool) -> StatsResult<Number> {
    match reduction_values("prod", x, na_rm)? {
        ReductionValues::Na => Ok(na_number()),
        ReductionValues::Values(values) => Ok(Number::Float(values.iter().product())),
    }
}

pub fn mean(x: &Double, na_rm: bool) -> StatsResult<Number> {
    match reduction_values("mean", x, na_rm)? {
        ReductionValues::Na => Ok(na_number()),
        ReductionValues::Values(values) if values.is_empty() => Ok(Number::Float(f64::NAN)),
        ReductionValues::Values(values) => Ok(Number::Float(
            kahan_sum(values.iter().copied()) / values.len() as f64,
        )),
    }
}

pub fn median(x: &Double, na_rm: bool) -> StatsResult<Number> {
    match reduction_values("median", x, na_rm)? {
        ReductionValues::Na => Ok(na_number()),
        ReductionValues::Values(values) if values.is_empty() => Ok(Number::Float(f64::NAN)),
        ReductionValues::Values(mut values) => {
            values.sort_by(|left, right| left.total_cmp(right));
            let middle = values.len() / 2;
            if values.len() % 2 == 1 {
                Ok(Number::Float(values[middle]))
            } else {
                Ok(Number::Float(
                    values[middle - 1] / 2.0 + values[middle] / 2.0,
                ))
            }
        }
    }
}

pub fn mode(x: &Double, na_rm: bool) -> StatsResult<Double> {
    match reduction_values("mode", x, na_rm)? {
        ReductionValues::Na => Ok(Double::singleton(na_real())),
        ReductionValues::Values(values) if values.is_empty() => Ok(Double::from_values(vec![])),
        ReductionValues::Values(mut values) => {
            values.sort_by(|left, right| left.total_cmp(right));

            let mut best_count = 1usize;
            let mut current_count = 1usize;
            let mut groups = Vec::new();

            for index in 1..=values.len() {
                if index < values.len() && same_f64_value(values[index], values[index - 1]) {
                    current_count += 1;
                    continue;
                }

                let value = values[index - 1];
                groups.push((value, current_count));
                best_count = best_count.max(current_count);
                current_count = 1;
            }

            if best_count == 1 {
                return Ok(Double::from_values(vec![]));
            }

            Ok(Double::from_values(
                groups
                    .into_iter()
                    .filter_map(|(value, count)| (count == best_count).then_some(value))
                    .collect(),
            ))
        }
    }
}

pub fn var(x: &Double, na_rm: bool) -> StatsResult<Number> {
    variance_impl("var", x, na_rm, VarianceKind::Sample)
}

pub fn sd(x: &Double, na_rm: bool) -> StatsResult<Number> {
    match var(x, na_rm)? {
        Number::Float(value) if is_na_real(value) => Ok(na_number()),
        Number::Float(value) => Ok(Number::Float(value.sqrt())),
        other => Ok(other),
    }
}

pub fn var_pop(x: &Double, na_rm: bool) -> StatsResult<Number> {
    variance_impl("var_pop", x, na_rm, VarianceKind::Population)
}

pub fn sd_pop(x: &Double, na_rm: bool) -> StatsResult<Number> {
    match var_pop(x, na_rm)? {
        Number::Float(value) if is_na_real(value) => Ok(na_number()),
        Number::Float(value) => Ok(Number::Float(value.sqrt())),
        other => Ok(other),
    }
}

pub fn min(x: &Double, na_rm: bool) -> StatsResult<Number> {
    match reduction_values("min", x, na_rm)? {
        ReductionValues::Na => Ok(na_number()),
        ReductionValues::Values(values) if values.is_empty() => Ok(Number::Float(f64::INFINITY)),
        ReductionValues::Values(values) => Ok(Number::Float(
            values
                .into_iter()
                .fold(f64::INFINITY, |acc, value| acc.min(value)),
        )),
    }
}

pub fn max(x: &Double, na_rm: bool) -> StatsResult<Number> {
    match reduction_values("max", x, na_rm)? {
        ReductionValues::Na => Ok(na_number()),
        ReductionValues::Values(values) if values.is_empty() => {
            Ok(Number::Float(f64::NEG_INFINITY))
        }
        ReductionValues::Values(values) => Ok(Number::Float(
            values
                .into_iter()
                .fold(f64::NEG_INFINITY, |acc, value| acc.max(value)),
        )),
    }
}

pub fn range(x: &Double, na_rm: bool) -> StatsResult<(Number, Number)> {
    match reduction_values("range", x, na_rm)? {
        ReductionValues::Na => Ok((na_number(), na_number())),
        ReductionValues::Values(values) if values.is_empty() => Ok((
            Number::Float(f64::INFINITY),
            Number::Float(f64::NEG_INFINITY),
        )),
        ReductionValues::Values(values) => {
            let mut min_value = f64::INFINITY;
            let mut max_value = f64::NEG_INFINITY;
            for value in values {
                min_value = min_value.min(value);
                max_value = max_value.max(value);
            }
            Ok((Number::Float(min_value), Number::Float(max_value)))
        }
    }
}

pub fn sumsq(x: &Double, na_rm: bool) -> StatsResult<Number> {
    match reduction_values("sumsq", x, na_rm)? {
        ReductionValues::Na => Ok(na_number()),
        ReductionValues::Values(values) => Ok(Number::Float(kahan_sum(
            values.into_iter().map(|value| value * value),
        ))),
    }
}

pub fn devsq(x: &Double, na_rm: bool) -> StatsResult<Number> {
    match reduction_values("devsq", x, na_rm)? {
        ReductionValues::Na => Ok(na_number()),
        ReductionValues::Values(values) if values.is_empty() => Ok(Number::Float(f64::NAN)),
        ReductionValues::Values(values) => {
            let mean = kahan_sum(values.iter().copied()) / values.len() as f64;
            Ok(Number::Float(kahan_sum(values.into_iter().map(|value| {
                let deviation = value - mean;
                deviation * deviation
            }))))
        }
    }
}

pub fn avedev(x: &Double, na_rm: bool) -> StatsResult<Number> {
    match reduction_values("avedev", x, na_rm)? {
        ReductionValues::Na => Ok(na_number()),
        ReductionValues::Values(values) if values.is_empty() => Ok(Number::Float(f64::NAN)),
        ReductionValues::Values(values) => {
            let mean = kahan_sum(values.iter().copied()) / values.len() as f64;
            let total = kahan_sum(values.iter().map(|value| (value - mean).abs()));
            Ok(Number::Float(total / values.len() as f64))
        }
    }
}

pub fn cumsum(x: &Double) -> Double {
    cumulative(x, 0.0, |acc, value| acc + value)
}

pub fn cumprod(x: &Double) -> Double {
    cumulative(x, 1.0, |acc, value| acc * value)
}

pub fn cummin(x: &Double) -> Double {
    cumulative(x, f64::INFINITY, |acc, value| acc.min(value))
}

pub fn cummax(x: &Double) -> Double {
    cumulative(x, f64::NEG_INFINITY, |acc, value| acc.max(value))
}

fn variance_impl(
    function: &'static str,
    x: &Double,
    na_rm: bool,
    kind: VarianceKind,
) -> StatsResult<Number> {
    match reduction_values(function, x, na_rm)? {
        ReductionValues::Na => Ok(na_number()),
        ReductionValues::Values(values) => {
            let n = values.len();
            if n == 0 || (kind == VarianceKind::Sample && n < 2) {
                return Ok(Number::Float(f64::NAN));
            }

            let mut count = 0.0;
            let mut mean = 0.0;
            let mut m2 = 0.0;

            for value in values {
                count += 1.0;
                let delta = value - mean;
                mean += delta / count;
                let delta2 = value - mean;
                m2 += delta * delta2;
            }

            let denominator = match kind {
                VarianceKind::Sample => count - 1.0,
                VarianceKind::Population => count,
            };
            Ok(Number::Float(m2 / denominator))
        }
    }
}

fn cumulative<F>(x: &Double, identity: f64, mut step: F) -> Double
where
    F: FnMut(f64, f64) -> f64,
{
    let mut acc = identity;
    let mut sticky_na = false;
    let mut out = Vec::with_capacity(x.len());

    for value in x.iter() {
        if sticky_na || is_na_real(value) {
            sticky_na = true;
            out.push(na_real());
            continue;
        }
        acc = step(acc, value);
        out.push(acc);
    }

    Double::from_values(out)
}

#[derive(Debug, Clone, PartialEq)]
enum ReductionValues {
    Na,
    Values(Vec<f64>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VarianceKind {
    Sample,
    Population,
}

fn reduction_values(
    function: &'static str,
    x: &Double,
    na_rm: bool,
) -> StatsResult<ReductionValues> {
    if x.is_empty() && !na_rm {
        return Err(StatsError::EmptyInput { function, min_n: 1 });
    }

    let mut values = Vec::with_capacity(x.len());
    for value in x.iter() {
        if is_na_real(value) {
            if !na_rm {
                return Ok(ReductionValues::Na);
            }
            continue;
        }
        values.push(value);
    }

    Ok(ReductionValues::Values(values))
}

fn kahan_sum<I>(values: I) -> f64
where
    I: IntoIterator<Item = f64>,
{
    let mut sum = 0.0;
    let mut compensation = 0.0;

    for value in values {
        let y = value - compensation;
        let t = sum + y;
        compensation = (t - sum) - y;
        sum = t;
    }

    sum
}

fn na_number() -> Number {
    Number::Float(na_real())
}

fn same_f64_value(left: f64, right: f64) -> bool {
    left == right || left.to_bits() == right.to_bits()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_float(actual: Number, expected: f64) {
        match actual {
            Number::Float(value) => assert!(
                (value - expected).abs() <= 1e-12,
                "expected {expected}, got {value}"
            ),
            other => panic!("expected float, got {other:?}"),
        }
    }

    #[test]
    fn descriptive_examples_match_st01_vectors() {
        let x = Double::from_values(vec![2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0]);
        assert_float(mean(&x, false).unwrap(), 5.0);
        assert_float(median(&x, false).unwrap(), 4.5);
        assert_float(var(&x, false).unwrap(), 4.571428571428571);
        assert_float(var_pop(&x, false).unwrap(), 4.0);
    }

    #[test]
    fn na_propagates_unless_removed() {
        let x = Double::from_values(vec![1.0, na_real(), 3.0]);
        match mean(&x, false).unwrap() {
            Number::Float(value) => assert!(is_na_real(value)),
            other => panic!("expected float NA, got {other:?}"),
        }
        assert_float(mean(&x, true).unwrap(), 2.0);
    }

    #[test]
    fn empty_set_identities_follow_spec() {
        let x = Double::from_values(vec![na_real()]);
        assert_float(sum(&x, true).unwrap(), 0.0);
        assert_float(prod(&x, true).unwrap(), 1.0);

        match min(&x, true).unwrap() {
            Number::Float(value) => assert_eq!(value, f64::INFINITY),
            other => panic!("expected float, got {other:?}"),
        }

        match max(&x, true).unwrap() {
            Number::Float(value) => assert_eq!(value, f64::NEG_INFINITY),
            other => panic!("expected float, got {other:?}"),
        }
    }

    #[test]
    fn cumsum_turns_na_into_sticky_na() {
        let x = Double::from_values(vec![1.0, 2.0, na_real(), 4.0]);
        let out = cumsum(&x);
        assert_eq!(out.get_value(0), Some(1.0));
        assert_eq!(out.get_value(1), Some(3.0));
        assert!(is_na_real(out.get_value(2).unwrap()));
        assert!(is_na_real(out.get_value(3).unwrap()));
    }
}
