pub fn sgd(weights: &[f64], gradients: &[f64], learning_rate: f64) -> Result<Vec<f64>, &'static str> {
    if weights.len() != gradients.len() || weights.is_empty() {
        return Err("Arrays must have the same non-zero length");
    }

    let mut res = Vec::with_capacity(weights.len());
    for i in 0..weights.len() {
        res.push(weights[i] - (learning_rate * gradients[i]));
    }
    
    Ok(res)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn almost_equal(a: f64, b: f64) -> bool {
        (a - b).abs() <= 1e-6
    }

    #[test]
    fn test_sgd() {
        let weights = &[1.0, -0.5, 2.0];
        let gradients = &[0.1, -0.2, 0.0];
        let lr = 0.1;

        let res = sgd(weights, gradients, lr).unwrap();
        assert!(almost_equal(res[0], 0.99));
        assert!(almost_equal(res[1], -0.48));
        assert!(almost_equal(res[2], 2.0));
    }

    #[test]
    fn test_errors() {
        assert!(sgd(&[1.0], &[], 0.1).is_err());
        assert!(sgd(&[], &[], 0.1).is_err());
    }
}
