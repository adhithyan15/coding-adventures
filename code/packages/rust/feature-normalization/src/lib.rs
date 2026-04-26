#[derive(Debug, Clone, PartialEq)]
pub struct StandardScaler {
    pub means: Vec<f64>,
    pub standard_deviations: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MinMaxScaler {
    pub minimums: Vec<f64>,
    pub maximums: Vec<f64>,
}

fn validate_matrix(rows: &[Vec<f64>]) -> Result<usize, &'static str> {
    if rows.is_empty() || rows[0].is_empty() {
        return Err("matrix must have at least one row and one column");
    }
    let width = rows[0].len();
    if rows.iter().any(|row| row.len() != width) {
        return Err("all rows must have the same number of columns");
    }
    Ok(width)
}

pub fn fit_standard_scaler(rows: &[Vec<f64>]) -> Result<StandardScaler, &'static str> {
    let width = validate_matrix(rows)?;
    let mut means = vec![0.0; width];
    for row in rows {
        for (col, value) in row.iter().enumerate() {
            means[col] += value;
        }
    }
    for mean in &mut means {
        *mean /= rows.len() as f64;
    }

    let mut standard_deviations = vec![0.0; width];
    for row in rows {
        for (col, value) in row.iter().enumerate() {
            let diff = value - means[col];
            standard_deviations[col] += diff * diff;
        }
    }
    for standard_deviation in &mut standard_deviations {
        *standard_deviation = (*standard_deviation / rows.len() as f64).sqrt();
    }
    Ok(StandardScaler {
        means,
        standard_deviations,
    })
}

pub fn transform_standard(
    rows: &[Vec<f64>],
    scaler: &StandardScaler,
) -> Result<Vec<Vec<f64>>, &'static str> {
    let width = validate_matrix(rows)?;
    if width != scaler.means.len() || width != scaler.standard_deviations.len() {
        return Err("matrix width must match scaler width");
    }
    Ok(rows
        .iter()
        .map(|row| {
            row.iter()
                .enumerate()
                .map(|(col, value)| {
                    if scaler.standard_deviations[col] == 0.0 {
                        0.0
                    } else {
                        (value - scaler.means[col]) / scaler.standard_deviations[col]
                    }
                })
                .collect()
        })
        .collect())
}

pub fn fit_min_max_scaler(rows: &[Vec<f64>]) -> Result<MinMaxScaler, &'static str> {
    let width = validate_matrix(rows)?;
    let mut minimums = rows[0].clone();
    let mut maximums = rows[0].clone();
    for row in rows.iter().skip(1) {
        for col in 0..width {
            minimums[col] = minimums[col].min(row[col]);
            maximums[col] = maximums[col].max(row[col]);
        }
    }
    Ok(MinMaxScaler { minimums, maximums })
}

pub fn transform_min_max(
    rows: &[Vec<f64>],
    scaler: &MinMaxScaler,
) -> Result<Vec<Vec<f64>>, &'static str> {
    let width = validate_matrix(rows)?;
    if width != scaler.minimums.len() || width != scaler.maximums.len() {
        return Err("matrix width must match scaler width");
    }
    Ok(rows
        .iter()
        .map(|row| {
            row.iter()
                .enumerate()
                .map(|(col, value)| {
                    let span = scaler.maximums[col] - scaler.minimums[col];
                    if span == 0.0 {
                        0.0
                    } else {
                        (value - scaler.minimums[col]) / span
                    }
                })
                .collect()
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(expected: f64, actual: f64) {
        assert!(
            (expected - actual).abs() <= 1e-9,
            "expected {expected}, got {actual}"
        );
    }

    fn rows() -> Vec<Vec<f64>> {
        vec![
            vec![1000.0, 3.0, 1.0],
            vec![1500.0, 4.0, 0.0],
            vec![2000.0, 5.0, 1.0],
        ]
    }

    #[test]
    fn standard_scaler_centers_and_scales_columns() {
        let data = rows();
        let scaler = fit_standard_scaler(&data).unwrap();
        assert_close(1500.0, scaler.means[0]);
        assert_close(4.0, scaler.means[1]);

        let transformed = transform_standard(&data, &scaler).unwrap();
        assert_close(-1.224744871391589, transformed[0][0]);
        assert_close(0.0, transformed[1][0]);
        assert_close(1.224744871391589, transformed[2][0]);
    }

    #[test]
    fn min_max_scaler_maps_to_unit_range() {
        let data = rows();
        let transformed = transform_min_max(&data, &fit_min_max_scaler(&data).unwrap()).unwrap();

        assert_eq!(transformed[0], vec![0.0, 0.0, 1.0]);
        assert_eq!(transformed[1], vec![0.5, 0.5, 0.0]);
        assert_eq!(transformed[2], vec![1.0, 1.0, 1.0]);
    }

    #[test]
    fn constant_columns_map_to_zero() {
        let data = vec![vec![1.0, 7.0], vec![2.0, 7.0]];
        let standard = transform_standard(&data, &fit_standard_scaler(&data).unwrap()).unwrap();
        let min_max = transform_min_max(&data, &fit_min_max_scaler(&data).unwrap()).unwrap();

        assert_close(0.0, standard[0][1]);
        assert_close(0.0, min_max[0][1]);
    }
}
