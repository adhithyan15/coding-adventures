//! Probability distributions.
//!
//! Phase 2A ships the Normal distribution and the shared
//! `ContinuousDistribution` trait that all future continuous
//! distributions will implement. Phase 2B will add Uniform,
//! Exponential, Gamma, Beta, ChiSq, Student's t, F, Cauchy, and
//! LogNormal; the two discrete distributions in the spec (Binomial,
//! Poisson) get a sibling `DiscreteDistribution` trait then.
//!
//! Naming follows R: every distribution has four free functions
//! `d*`, `p*`, `q*`, `r*` plus an explicit struct that implements
//! the trait. Excel's NORM.DIST / NORM.S.DIST / NORM.INV / NORM.S.INV
//! dispatch to the same Rust functions.

use crate::rng::RngState;
use crate::special::erf;

// ---------------------------------------------------------------------------
// Trait
// ---------------------------------------------------------------------------

/// Continuous probability distribution.
///
/// Each implementor provides probability density, cumulative
/// distribution, survival function, quantile (inverse CDF), and a
/// sampler. The default `log_pdf` is `pdf(x).ln()`; implementors
/// override when a more accurate log-space formula exists.
pub trait ContinuousDistribution {
    /// Probability density f(x).
    fn pdf(&self, x: f64) -> f64;
    /// Cumulative distribution F(x) = P(X ≤ x).
    fn cdf(&self, x: f64) -> f64;
    /// Survival function 1 - F(x). Override for tail accuracy.
    fn sf(&self, x: f64) -> f64 {
        1.0 - self.cdf(x)
    }
    /// Log of pdf — override when a log-space formula avoids overflow.
    fn log_pdf(&self, x: f64) -> f64 {
        self.pdf(x).ln()
    }
    /// Inverse CDF / quantile. `p` in `[0, 1]`.
    fn quantile(&self, p: f64) -> f64;
    /// Random sample.
    fn sample(&self, rng: &mut RngState) -> f64;
    /// Mean of the distribution (or `None` if undefined).
    fn mean(&self) -> Option<f64>;
    /// Variance of the distribution (or `None` if undefined).
    fn variance(&self) -> Option<f64>;
}

// ---------------------------------------------------------------------------
// Normal
// ---------------------------------------------------------------------------

/// Normal distribution with mean `mu` and standard deviation `sd`.
#[derive(Debug, Clone, Copy)]
pub struct Normal {
    /// Mean.
    pub mean: f64,
    /// Standard deviation. Must be positive.
    pub sd: f64,
}

impl Normal {
    /// Construct. `sd` must be positive; otherwise returns `None`.
    pub fn new(mean: f64, sd: f64) -> Option<Self> {
        if sd > 0.0 && sd.is_finite() && mean.is_finite() {
            Some(Self { mean, sd })
        } else {
            None
        }
    }
    /// Standard normal (mean = 0, sd = 1).
    pub fn standard() -> Self {
        Self {
            mean: 0.0,
            sd: 1.0,
        }
    }
}

impl ContinuousDistribution for Normal {
    fn pdf(&self, x: f64) -> f64 {
        let z = (x - self.mean) / self.sd;
        let two_pi = 2.0 * core::f64::consts::PI;
        (-(z * z) / 2.0).exp() / (self.sd * two_pi.sqrt())
    }

    fn cdf(&self, x: f64) -> f64 {
        let z = (x - self.mean) / (self.sd * core::f64::consts::SQRT_2);
        0.5 * (1.0 + erf(z))
    }

    fn quantile(&self, p: f64) -> f64 {
        if p < 0.0 || p > 1.0 || p.is_nan() {
            return f64::NAN;
        }
        if p == 0.0 {
            return f64::NEG_INFINITY;
        }
        if p == 1.0 {
            return f64::INFINITY;
        }
        // Beasley-Springer-Moro algorithm — accuracy ~1e-9.
        let z = standard_normal_quantile(p);
        self.mean + self.sd * z
    }

    fn sample(&self, rng: &mut RngState) -> f64 {
        // Box-Muller. Each call returns one of two independent normals;
        // we keep the second cached on the rng state... but to avoid
        // state on RngState, just throw away the second. (Standard
        // sampling literature documents both choices; the throw-away
        // pattern matches R's defaults for non-Marsaglia samplers.)
        let u1 = rng.next_f64().max(f64::MIN_POSITIVE);
        let u2 = rng.next_f64();
        let z = (-2.0 * u1.ln()).sqrt() * (2.0 * core::f64::consts::PI * u2).cos();
        self.mean + self.sd * z
    }

    fn mean(&self) -> Option<f64> {
        Some(self.mean)
    }

    fn variance(&self) -> Option<f64> {
        Some(self.sd * self.sd)
    }
}

/// Beasley-Springer-Moro inverse normal CDF (for the standard normal).
fn standard_normal_quantile(p: f64) -> f64 {
    // Lower / upper region coefficients.
    let a = [
        -3.969683028665376e+01,
        2.209460984245205e+02,
        -2.759285104469687e+02,
        1.383577518672690e+02,
        -3.066479806614716e+01,
        2.506628277459239e+00,
    ];
    let b = [
        -5.447609879822406e+01,
        1.615858368580409e+02,
        -1.556989798598866e+02,
        6.680131188771972e+01,
        -1.328068155288572e+01,
    ];
    let c = [
        -7.784894002430293e-03,
        -3.223964580411365e-01,
        -2.400758277161838e+00,
        -2.549732539343734e+00,
        4.374664141464968e+00,
        2.938163982698783e+00,
    ];
    let d = [
        7.784695709041462e-03,
        3.224671290700398e-01,
        2.445134137142996e+00,
        3.754408661907416e+00,
    ];
    let p_low = 0.02425;
    let p_high = 1.0 - p_low;

    if p < p_low {
        let q = (-2.0 * p.ln()).sqrt();
        (((((c[0] * q + c[1]) * q + c[2]) * q + c[3]) * q + c[4]) * q + c[5])
            / ((((d[0] * q + d[1]) * q + d[2]) * q + d[3]) * q + 1.0)
    } else if p <= p_high {
        let q = p - 0.5;
        let r = q * q;
        (((((a[0] * r + a[1]) * r + a[2]) * r + a[3]) * r + a[4]) * r + a[5]) * q
            / (((((b[0] * r + b[1]) * r + b[2]) * r + b[3]) * r + b[4]) * r + 1.0)
    } else {
        let q = (-2.0 * (1.0 - p).ln()).sqrt();
        -((((((c[0] * q + c[1]) * q + c[2]) * q + c[3]) * q + c[4]) * q + c[5])
            / ((((d[0] * q + d[1]) * q + d[2]) * q + d[3]) * q + 1.0))
    }
}

// ---------------------------------------------------------------------------
// R-style dnorm/pnorm/qnorm/rnorm free functions
// ---------------------------------------------------------------------------

/// `dnorm(x, mean=0, sd=1)`.
pub fn dnorm(x: f64, mean: f64, sd: f64) -> f64 {
    match Normal::new(mean, sd) {
        Some(d) => d.pdf(x),
        None => f64::NAN,
    }
}

/// `pnorm(x, mean=0, sd=1)`.
pub fn pnorm(x: f64, mean: f64, sd: f64) -> f64 {
    match Normal::new(mean, sd) {
        Some(d) => d.cdf(x),
        None => f64::NAN,
    }
}

/// `qnorm(p, mean=0, sd=1)`.
pub fn qnorm(p: f64, mean: f64, sd: f64) -> f64 {
    match Normal::new(mean, sd) {
        Some(d) => d.quantile(p),
        None => f64::NAN,
    }
}

/// `rnorm(n, mean=0, sd=1, rng)`. Returns a Vec since the n is the
/// caller's input.
pub fn rnorm(n: usize, mean: f64, sd: f64, rng: &mut RngState) -> Vec<f64> {
    match Normal::new(mean, sd) {
        Some(d) => (0..n).map(|_| d.sample(rng)).collect(),
        None => vec![f64::NAN; n],
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() < tol
    }

    #[test]
    fn dnorm_standard_at_zero() {
        // pdf(0) for standard normal = 1/sqrt(2π) ≈ 0.39894...
        assert!(close(dnorm(0.0, 0.0, 1.0), 0.39894228040143_f64, 1e-9));
    }

    #[test]
    fn dnorm_known_values_match_r() {
        // R: dnorm(c(-2, -1, 0, 1, 2)) ≈ 0.0539910 0.2419707 0.3989423 0.2419707 0.0539910
        let expected = [
            0.0539909665131881, 0.2419707245191434, 0.3989422804014327,
            0.2419707245191434, 0.0539909665131881,
        ];
        for (i, x) in [-2.0, -1.0, 0.0, 1.0, 2.0].iter().enumerate() {
            let actual = dnorm(*x, 0.0, 1.0);
            assert!(
                close(actual, expected[i], 1e-9),
                "x={x}: got {actual}, expected {}",
                expected[i]
            );
        }
    }

    #[test]
    fn pnorm_standard_at_zero() {
        assert!(close(pnorm(0.0, 0.0, 1.0), 0.5, 1e-9));
    }

    #[test]
    fn pnorm_known_values_match_r() {
        // R: pnorm(c(-1.96, -1, 0, 1, 1.96)) ≈ 0.0250 0.1587 0.5 0.8413 0.9750
        assert!(close(pnorm(-1.96, 0.0, 1.0), 0.0249978951482_f64, 1e-7));
        assert!(close(pnorm(-1.0, 0.0, 1.0), 0.1586552539314_f64, 1e-7));
        assert!(close(pnorm(1.0, 0.0, 1.0), 0.8413447460686_f64, 1e-7));
        assert!(close(pnorm(1.96, 0.0, 1.0), 0.9750021048518_f64, 1e-7));
    }

    #[test]
    fn pnorm_symmetric() {
        for x in [0.5, 1.0, 1.5, 2.5] {
            let left = pnorm(-x, 0.0, 1.0);
            let right = 1.0 - pnorm(x, 0.0, 1.0);
            assert!(close(left, right, 1e-9), "asymmetry at x={x}");
        }
    }

    #[test]
    fn qnorm_quantile_round_trip() {
        // qnorm(pnorm(x)) ≈ x for reasonable x.
        for x in [-2.5, -1.0, -0.5, 0.0, 0.5, 1.0, 2.5] {
            let back = qnorm(pnorm(x, 0.0, 1.0), 0.0, 1.0);
            assert!(close(back, x, 1e-6), "round-trip failed at x={x}: got {back}");
        }
    }

    #[test]
    fn qnorm_known_values_match_r() {
        // R: qnorm(0.975) = 1.9599639...
        assert!(close(qnorm(0.975, 0.0, 1.0), 1.95996398454_f64, 1e-6));
        // qnorm(0.025) = -1.9599639
        assert!(close(qnorm(0.025, 0.0, 1.0), -1.95996398454_f64, 1e-6));
        // qnorm(0.5) = 0
        assert!(close(qnorm(0.5, 0.0, 1.0), 0.0, 1e-9));
    }

    #[test]
    fn qnorm_extreme_p() {
        assert!(qnorm(0.0, 0.0, 1.0).is_infinite() && qnorm(0.0, 0.0, 1.0) < 0.0);
        assert!(qnorm(1.0, 0.0, 1.0).is_infinite() && qnorm(1.0, 0.0, 1.0) > 0.0);
        assert!(qnorm(-0.1, 0.0, 1.0).is_nan());
        assert!(qnorm(1.5, 0.0, 1.0).is_nan());
    }

    #[test]
    fn rnorm_returns_correct_length_and_finite() {
        let mut rng = RngState::new(42);
        let samples = rnorm(100, 0.0, 1.0, &mut rng);
        assert_eq!(samples.len(), 100);
        for &s in &samples {
            assert!(s.is_finite(), "non-finite sample: {s}");
        }
    }

    #[test]
    fn rnorm_empirical_mean_and_sd() {
        let mut rng = RngState::new(2024);
        let n = 10_000;
        let samples = rnorm(n, 5.0, 2.0, &mut rng);
        let mean: f64 = samples.iter().sum::<f64>() / n as f64;
        let var: f64 = samples.iter().map(|&x| (x - mean).powi(2)).sum::<f64>() / (n - 1) as f64;
        assert!((mean - 5.0).abs() < 0.1, "mean={mean}");
        assert!((var.sqrt() - 2.0).abs() < 0.1, "sd={}", var.sqrt());
    }

    #[test]
    fn invalid_sd_returns_nan() {
        assert!(dnorm(0.0, 0.0, -1.0).is_nan());
        assert!(pnorm(0.0, 0.0, 0.0).is_nan());
        assert!(qnorm(0.5, 0.0, f64::INFINITY).is_nan());
    }

    #[test]
    fn distribution_mean_and_variance_round_trip() {
        let d = Normal::new(3.0, 2.0).unwrap();
        assert_eq!(d.mean(), Some(3.0));
        assert_eq!(d.variance(), Some(4.0));
    }
}
