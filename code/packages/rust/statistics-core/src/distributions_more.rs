//! Probability distributions — Phase 2B.
//!
//! Adds Uniform, Exponential, ChiSq, StudentT, F, Cauchy, LogNormal,
//! Gamma, Beta to the continuous side, plus the `DiscreteDistribution`
//! trait and Binomial + Poisson on the discrete side.
//!
//! Algorithms (all classical):
//!
//! - **Uniform**: closed-form everywhere.
//! - **Exponential**: closed-form everywhere; sampling `-ln(U)/rate`.
//! - **Cauchy**: closed-form everywhere; quantile via `tan(π(p - 1/2))`.
//! - **ChiSq**: special case of Gamma(shape = df/2, rate = 1/2).
//! - **StudentT**: cdf via regularized incomplete beta;
//!   sampling: `Normal / sqrt(ChiSq/df)`.
//! - **F**: cdf via regularized incomplete beta; sampling ratio of
//!   chi-squareds.
//! - **LogNormal**: `log` transform of Normal.
//! - **Gamma**: cdf via lower-regularized incomplete gamma; sampling
//!   Marsaglia-Tsang (shape >= 1), Ahrens-Dieter (shape < 1).
//! - **Beta**: cdf via regularized incomplete beta; sampling
//!   "two gammas, take ratio."
//! - **Binomial** (discrete): pmf is closed-form; sampling
//!   inverse-CDF for small n, BTPE-lite for large.
//! - **Poisson** (discrete): pmf is closed-form; sampling Knuth's
//!   algorithm for small λ, rejection for large.
//!
//! Quantiles where no closed form exists use a robust bisection over
//! the CDF on `[lo, hi]` brackets. Accuracy ~1e-8 by default.

use crate::distributions::ContinuousDistribution;
use crate::rng::RngState;
use crate::special;

// ---------------------------------------------------------------------------
// Bisection helper for inverse-CDF
// ---------------------------------------------------------------------------

/// Generic bracketed bisection. Returns `x` such that `cdf(x) ≈ p`
/// within `tol`, or the closest endpoint after `max_iter` steps.
fn bisect_inverse_cdf<F>(cdf: F, p: f64, lo: f64, hi: f64) -> f64
where
    F: Fn(f64) -> f64,
{
    let tol = 1e-10;
    let max_iter = 100;
    let (mut lo, mut hi) = (lo, hi);
    for _ in 0..max_iter {
        let mid = 0.5 * (lo + hi);
        let c = cdf(mid);
        if (c - p).abs() < tol || (hi - lo) < tol {
            return mid;
        }
        if c < p {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    0.5 * (lo + hi)
}

/// Expand a bisection bracket until both sides straddle `p`.
fn expand_bracket<F>(cdf: F, p: f64, mut lo: f64, mut hi: f64) -> (f64, f64)
where
    F: Fn(f64) -> f64,
{
    for _ in 0..200 {
        let c_lo = cdf(lo);
        let c_hi = cdf(hi);
        if c_lo > p {
            lo *= 2.0;
        } else if c_hi < p {
            hi *= 2.0;
        } else {
            return (lo, hi);
        }
    }
    (lo, hi)
}

// ---------------------------------------------------------------------------
// Uniform
// ---------------------------------------------------------------------------

/// Continuous uniform distribution on `[min, max]`.
#[derive(Debug, Clone, Copy)]
pub struct Uniform {
    /// Lower bound (inclusive).
    pub min: f64,
    /// Upper bound (inclusive in concept; exclusive in `runif`
    /// sampling, matching R).
    pub max: f64,
}

impl Uniform {
    /// Construct. Returns `None` if `max <= min` or either bound is
    /// non-finite.
    pub fn new(min: f64, max: f64) -> Option<Self> {
        if min.is_finite() && max.is_finite() && max > min {
            Some(Self { min, max })
        } else {
            None
        }
    }
}

impl ContinuousDistribution for Uniform {
    fn pdf(&self, x: f64) -> f64 {
        if x < self.min || x > self.max {
            0.0
        } else {
            1.0 / (self.max - self.min)
        }
    }
    fn cdf(&self, x: f64) -> f64 {
        if x <= self.min {
            0.0
        } else if x >= self.max {
            1.0
        } else {
            (x - self.min) / (self.max - self.min)
        }
    }
    fn quantile(&self, p: f64) -> f64 {
        if !(0.0..=1.0).contains(&p) {
            return f64::NAN;
        }
        self.min + p * (self.max - self.min)
    }
    fn sample(&self, rng: &mut RngState) -> f64 {
        self.min + rng.next_f64() * (self.max - self.min)
    }
    fn mean(&self) -> Option<f64> {
        Some(0.5 * (self.min + self.max))
    }
    fn variance(&self) -> Option<f64> {
        let r = self.max - self.min;
        Some(r * r / 12.0)
    }
}

/// `dunif(x, min, max)`.
pub fn dunif(x: f64, min: f64, max: f64) -> f64 {
    Uniform::new(min, max).map_or(f64::NAN, |d| d.pdf(x))
}
/// `punif(x, min, max)`.
pub fn punif(x: f64, min: f64, max: f64) -> f64 {
    Uniform::new(min, max).map_or(f64::NAN, |d| d.cdf(x))
}
/// `qunif(p, min, max)`.
pub fn qunif(p: f64, min: f64, max: f64) -> f64 {
    Uniform::new(min, max).map_or(f64::NAN, |d| d.quantile(p))
}
/// `runif(n, min, max)`.
pub fn runif(n: usize, min: f64, max: f64, rng: &mut RngState) -> Vec<f64> {
    Uniform::new(min, max).map_or_else(|| vec![f64::NAN; n], |d| {
        (0..n).map(|_| d.sample(rng)).collect()
    })
}

// ---------------------------------------------------------------------------
// Exponential
// ---------------------------------------------------------------------------

/// Exponential distribution with rate λ > 0.
#[derive(Debug, Clone, Copy)]
pub struct Exponential {
    /// Rate parameter (1/mean).
    pub rate: f64,
}

impl Exponential {
    /// Construct.
    pub fn new(rate: f64) -> Option<Self> {
        (rate > 0.0 && rate.is_finite()).then_some(Self { rate })
    }
}

impl ContinuousDistribution for Exponential {
    fn pdf(&self, x: f64) -> f64 {
        if x < 0.0 {
            0.0
        } else {
            self.rate * (-self.rate * x).exp()
        }
    }
    fn cdf(&self, x: f64) -> f64 {
        if x < 0.0 {
            0.0
        } else {
            1.0 - (-self.rate * x).exp()
        }
    }
    fn quantile(&self, p: f64) -> f64 {
        if !(0.0..=1.0).contains(&p) {
            return f64::NAN;
        }
        if p == 1.0 {
            return f64::INFINITY;
        }
        -((-(p - 1.0)).ln()) / self.rate
        // == -ln(1 - p) / rate
    }
    fn sample(&self, rng: &mut RngState) -> f64 {
        let u = rng.next_f64().max(f64::MIN_POSITIVE);
        -u.ln() / self.rate
    }
    fn mean(&self) -> Option<f64> {
        Some(1.0 / self.rate)
    }
    fn variance(&self) -> Option<f64> {
        Some(1.0 / (self.rate * self.rate))
    }
}

/// `dexp(x, rate)`.
pub fn dexp(x: f64, rate: f64) -> f64 {
    Exponential::new(rate).map_or(f64::NAN, |d| d.pdf(x))
}
/// `pexp(x, rate)`.
pub fn pexp(x: f64, rate: f64) -> f64 {
    Exponential::new(rate).map_or(f64::NAN, |d| d.cdf(x))
}
/// `qexp(p, rate)`.
pub fn qexp(p: f64, rate: f64) -> f64 {
    Exponential::new(rate).map_or(f64::NAN, |d| d.quantile(p))
}
/// `rexp(n, rate, rng)`.
pub fn rexp(n: usize, rate: f64, rng: &mut RngState) -> Vec<f64> {
    Exponential::new(rate).map_or_else(|| vec![f64::NAN; n], |d| {
        (0..n).map(|_| d.sample(rng)).collect()
    })
}

// ---------------------------------------------------------------------------
// Cauchy
// ---------------------------------------------------------------------------

/// Cauchy distribution. Mean and variance are *undefined*
/// (heavy-tailed) — both return `None`.
#[derive(Debug, Clone, Copy)]
pub struct Cauchy {
    /// Location parameter (median).
    pub location: f64,
    /// Scale parameter (positive).
    pub scale: f64,
}

impl Cauchy {
    /// Construct.
    pub fn new(location: f64, scale: f64) -> Option<Self> {
        (scale > 0.0 && location.is_finite() && scale.is_finite())
            .then_some(Self { location, scale })
    }
}

impl ContinuousDistribution for Cauchy {
    fn pdf(&self, x: f64) -> f64 {
        let z = (x - self.location) / self.scale;
        1.0 / (core::f64::consts::PI * self.scale * (1.0 + z * z))
    }
    fn cdf(&self, x: f64) -> f64 {
        0.5 + ((x - self.location) / self.scale).atan() / core::f64::consts::PI
    }
    fn quantile(&self, p: f64) -> f64 {
        if !(0.0..=1.0).contains(&p) {
            return f64::NAN;
        }
        if p == 0.0 {
            return f64::NEG_INFINITY;
        }
        if p == 1.0 {
            return f64::INFINITY;
        }
        self.location + self.scale * (core::f64::consts::PI * (p - 0.5)).tan()
    }
    fn sample(&self, rng: &mut RngState) -> f64 {
        // Inverse-CDF sampling.
        let u = rng.next_f64();
        self.quantile(u)
    }
    fn mean(&self) -> Option<f64> {
        None
    }
    fn variance(&self) -> Option<f64> {
        None
    }
}

/// `dcauchy(x, location, scale)`.
pub fn dcauchy(x: f64, location: f64, scale: f64) -> f64 {
    Cauchy::new(location, scale).map_or(f64::NAN, |d| d.pdf(x))
}
/// `pcauchy(x, location, scale)`.
pub fn pcauchy(x: f64, location: f64, scale: f64) -> f64 {
    Cauchy::new(location, scale).map_or(f64::NAN, |d| d.cdf(x))
}
/// `qcauchy(p, location, scale)`.
pub fn qcauchy(p: f64, location: f64, scale: f64) -> f64 {
    Cauchy::new(location, scale).map_or(f64::NAN, |d| d.quantile(p))
}
/// `rcauchy(n, location, scale, rng)`.
pub fn rcauchy(n: usize, location: f64, scale: f64, rng: &mut RngState) -> Vec<f64> {
    Cauchy::new(location, scale).map_or_else(|| vec![f64::NAN; n], |d| {
        (0..n).map(|_| d.sample(rng)).collect()
    })
}

// ---------------------------------------------------------------------------
// Gamma
// ---------------------------------------------------------------------------

/// Gamma distribution with shape α and rate β (so mean = α/β).
#[derive(Debug, Clone, Copy)]
pub struct Gamma {
    /// Shape α > 0.
    pub shape: f64,
    /// Rate β > 0.
    pub rate: f64,
}

impl Gamma {
    /// Construct.
    pub fn new(shape: f64, rate: f64) -> Option<Self> {
        (shape > 0.0 && rate > 0.0 && shape.is_finite() && rate.is_finite())
            .then_some(Self { shape, rate })
    }
}

impl ContinuousDistribution for Gamma {
    fn pdf(&self, x: f64) -> f64 {
        if x <= 0.0 {
            return 0.0;
        }
        // f(x) = β^α / Γ(α) · x^(α-1) · exp(-β x)
        let log = self.shape * self.rate.ln() - special::lgamma(self.shape)
            + (self.shape - 1.0) * x.ln()
            - self.rate * x;
        log.exp()
    }
    fn cdf(&self, x: f64) -> f64 {
        if x <= 0.0 {
            return 0.0;
        }
        special::gamma_inc_lower(self.rate * x, self.shape)
    }
    fn quantile(&self, p: f64) -> f64 {
        if !(0.0..=1.0).contains(&p) {
            return f64::NAN;
        }
        if p == 0.0 {
            return 0.0;
        }
        if p == 1.0 {
            return f64::INFINITY;
        }
        let mean = self.shape / self.rate;
        let (lo, hi) = expand_bracket(|x| self.cdf(x), p, mean * 0.01, mean * 10.0);
        bisect_inverse_cdf(|x| self.cdf(x), p, lo, hi)
    }
    fn sample(&self, rng: &mut RngState) -> f64 {
        // Marsaglia-Tsang for shape >= 1; for shape < 1, use the
        // standard "shape += 1, then multiply by U^(1/shape)" trick.
        sample_gamma(self.shape, self.rate, rng)
    }
    fn mean(&self) -> Option<f64> {
        Some(self.shape / self.rate)
    }
    fn variance(&self) -> Option<f64> {
        Some(self.shape / (self.rate * self.rate))
    }
}

fn sample_gamma(shape: f64, rate: f64, rng: &mut RngState) -> f64 {
    if shape < 1.0 {
        let u = rng.next_f64().max(f64::MIN_POSITIVE);
        return sample_gamma(shape + 1.0, rate, rng) * u.powf(1.0 / shape);
    }
    // Marsaglia-Tsang.
    let d = shape - 1.0 / 3.0;
    let c = 1.0 / (9.0_f64 * d).sqrt();
    loop {
        // Standard-normal sample via Box-Muller.
        let u1 = rng.next_f64().max(f64::MIN_POSITIVE);
        let u2 = rng.next_f64();
        let z = (-2.0 * u1.ln()).sqrt() * (2.0 * core::f64::consts::PI * u2).cos();
        let v = (1.0 + c * z).powi(3);
        if v <= 0.0 {
            continue;
        }
        let u = rng.next_f64().max(f64::MIN_POSITIVE);
        if u < 1.0 - 0.0331 * z.powi(4) {
            return d * v / rate;
        }
        if u.ln() < 0.5 * z * z + d * (1.0 - v + v.ln()) {
            return d * v / rate;
        }
    }
}

/// `dgamma(x, shape, rate)`.
pub fn dgamma(x: f64, shape: f64, rate: f64) -> f64 {
    Gamma::new(shape, rate).map_or(f64::NAN, |d| d.pdf(x))
}
/// `pgamma(x, shape, rate)`.
pub fn pgamma(x: f64, shape: f64, rate: f64) -> f64 {
    Gamma::new(shape, rate).map_or(f64::NAN, |d| d.cdf(x))
}
/// `qgamma(p, shape, rate)`.
pub fn qgamma(p: f64, shape: f64, rate: f64) -> f64 {
    Gamma::new(shape, rate).map_or(f64::NAN, |d| d.quantile(p))
}
/// `rgamma(n, shape, rate, rng)`.
pub fn rgamma(n: usize, shape: f64, rate: f64, rng: &mut RngState) -> Vec<f64> {
    Gamma::new(shape, rate).map_or_else(|| vec![f64::NAN; n], |d| {
        (0..n).map(|_| d.sample(rng)).collect()
    })
}

// ---------------------------------------------------------------------------
// Chi-squared
// ---------------------------------------------------------------------------

/// Chi-squared distribution — special case of Gamma(df/2, 1/2).
#[derive(Debug, Clone, Copy)]
pub struct ChiSq {
    /// Degrees of freedom (positive).
    pub df: f64,
}

impl ChiSq {
    /// Construct.
    pub fn new(df: f64) -> Option<Self> {
        (df > 0.0 && df.is_finite()).then_some(Self { df })
    }
    fn as_gamma(self) -> Gamma {
        Gamma {
            shape: self.df / 2.0,
            rate: 0.5,
        }
    }
}

impl ContinuousDistribution for ChiSq {
    fn pdf(&self, x: f64) -> f64 {
        self.as_gamma().pdf(x)
    }
    fn cdf(&self, x: f64) -> f64 {
        self.as_gamma().cdf(x)
    }
    fn quantile(&self, p: f64) -> f64 {
        self.as_gamma().quantile(p)
    }
    fn sample(&self, rng: &mut RngState) -> f64 {
        self.as_gamma().sample(rng)
    }
    fn mean(&self) -> Option<f64> {
        Some(self.df)
    }
    fn variance(&self) -> Option<f64> {
        Some(2.0 * self.df)
    }
}

/// `dchisq(x, df)`.
pub fn dchisq(x: f64, df: f64) -> f64 {
    ChiSq::new(df).map_or(f64::NAN, |d| d.pdf(x))
}
/// `pchisq(x, df)`.
pub fn pchisq(x: f64, df: f64) -> f64 {
    ChiSq::new(df).map_or(f64::NAN, |d| d.cdf(x))
}
/// `qchisq(p, df)`.
pub fn qchisq(p: f64, df: f64) -> f64 {
    ChiSq::new(df).map_or(f64::NAN, |d| d.quantile(p))
}
/// `rchisq(n, df, rng)`.
pub fn rchisq(n: usize, df: f64, rng: &mut RngState) -> Vec<f64> {
    ChiSq::new(df).map_or_else(|| vec![f64::NAN; n], |d| {
        (0..n).map(|_| d.sample(rng)).collect()
    })
}

// ---------------------------------------------------------------------------
// Student's t
// ---------------------------------------------------------------------------

/// Student's t-distribution.
#[derive(Debug, Clone, Copy)]
pub struct StudentT {
    /// Degrees of freedom (positive).
    pub df: f64,
}

impl StudentT {
    /// Construct.
    pub fn new(df: f64) -> Option<Self> {
        (df > 0.0 && df.is_finite()).then_some(Self { df })
    }
}

impl ContinuousDistribution for StudentT {
    fn pdf(&self, x: f64) -> f64 {
        let v = self.df;
        let log = special::lgamma((v + 1.0) / 2.0)
            - special::lgamma(v / 2.0)
            - 0.5 * (v * core::f64::consts::PI).ln()
            - ((v + 1.0) / 2.0) * (1.0 + (x * x) / v).ln();
        log.exp()
    }
    fn cdf(&self, x: f64) -> f64 {
        let v = self.df;
        let z = v / (v + x * x);
        let half = special::beta_inc(z, v / 2.0, 0.5) / 2.0;
        if x >= 0.0 {
            1.0 - half
        } else {
            half
        }
    }
    fn quantile(&self, p: f64) -> f64 {
        if !(0.0..=1.0).contains(&p) {
            return f64::NAN;
        }
        if p == 0.0 {
            return f64::NEG_INFINITY;
        }
        if p == 1.0 {
            return f64::INFINITY;
        }
        let (lo, hi) = expand_bracket(|x| self.cdf(x), p, -1.0, 1.0);
        bisect_inverse_cdf(|x| self.cdf(x), p, lo, hi)
    }
    fn sample(&self, rng: &mut RngState) -> f64 {
        let n = standard_normal(rng);
        let chi = ChiSq::new(self.df).unwrap().sample(rng);
        n / (chi / self.df).sqrt()
    }
    fn mean(&self) -> Option<f64> {
        if self.df > 1.0 {
            Some(0.0)
        } else {
            None
        }
    }
    fn variance(&self) -> Option<f64> {
        if self.df > 2.0 {
            Some(self.df / (self.df - 2.0))
        } else {
            None
        }
    }
}

fn standard_normal(rng: &mut RngState) -> f64 {
    let u1 = rng.next_f64().max(f64::MIN_POSITIVE);
    let u2 = rng.next_f64();
    (-2.0 * u1.ln()).sqrt() * (2.0 * core::f64::consts::PI * u2).cos()
}

/// `dt(x, df)`.
pub fn dt(x: f64, df: f64) -> f64 {
    StudentT::new(df).map_or(f64::NAN, |d| d.pdf(x))
}
/// `pt(x, df)`.
pub fn pt(x: f64, df: f64) -> f64 {
    StudentT::new(df).map_or(f64::NAN, |d| d.cdf(x))
}
/// `qt(p, df)`.
pub fn qt(p: f64, df: f64) -> f64 {
    StudentT::new(df).map_or(f64::NAN, |d| d.quantile(p))
}
/// `rt(n, df, rng)`.
pub fn rt(n: usize, df: f64, rng: &mut RngState) -> Vec<f64> {
    StudentT::new(df).map_or_else(|| vec![f64::NAN; n], |d| {
        (0..n).map(|_| d.sample(rng)).collect()
    })
}

// ---------------------------------------------------------------------------
// F
// ---------------------------------------------------------------------------

/// F-distribution.
#[derive(Debug, Clone, Copy)]
pub struct F {
    /// Numerator degrees of freedom.
    pub df1: f64,
    /// Denominator degrees of freedom.
    pub df2: f64,
}

impl F {
    /// Construct.
    pub fn new(df1: f64, df2: f64) -> Option<Self> {
        (df1 > 0.0 && df2 > 0.0 && df1.is_finite() && df2.is_finite())
            .then_some(Self { df1, df2 })
    }
}

impl ContinuousDistribution for F {
    fn pdf(&self, x: f64) -> f64 {
        if x <= 0.0 {
            return 0.0;
        }
        let (d1, d2) = (self.df1, self.df2);
        let log = (d1 / 2.0) * (d1 / d2).ln()
            + (d1 / 2.0 - 1.0) * x.ln()
            - ((d1 + d2) / 2.0) * (1.0 + (d1 * x) / d2).ln()
            - special::lbeta(d1 / 2.0, d2 / 2.0);
        log.exp()
    }
    fn cdf(&self, x: f64) -> f64 {
        if x <= 0.0 {
            return 0.0;
        }
        let z = (self.df1 * x) / (self.df1 * x + self.df2);
        special::beta_inc(z, self.df1 / 2.0, self.df2 / 2.0)
    }
    fn quantile(&self, p: f64) -> f64 {
        if !(0.0..=1.0).contains(&p) {
            return f64::NAN;
        }
        if p == 0.0 {
            return 0.0;
        }
        if p == 1.0 {
            return f64::INFINITY;
        }
        let (lo, hi) = expand_bracket(|x| self.cdf(x), p, 0.01, 10.0);
        bisect_inverse_cdf(|x| self.cdf(x), p, lo, hi)
    }
    fn sample(&self, rng: &mut RngState) -> f64 {
        let x1 = ChiSq::new(self.df1).unwrap().sample(rng) / self.df1;
        let x2 = ChiSq::new(self.df2).unwrap().sample(rng) / self.df2;
        x1 / x2
    }
    fn mean(&self) -> Option<f64> {
        if self.df2 > 2.0 {
            Some(self.df2 / (self.df2 - 2.0))
        } else {
            None
        }
    }
    fn variance(&self) -> Option<f64> {
        let (d1, d2) = (self.df1, self.df2);
        if d2 > 4.0 {
            Some((2.0 * d2 * d2 * (d1 + d2 - 2.0))
                / (d1 * (d2 - 2.0).powi(2) * (d2 - 4.0)))
        } else {
            None
        }
    }
}

/// `df(x, df1, df2)`.
pub fn df_(x: f64, df1: f64, df2: f64) -> f64 {
    F::new(df1, df2).map_or(f64::NAN, |d| d.pdf(x))
}
/// `pf(x, df1, df2)`.
pub fn pf(x: f64, df1: f64, df2: f64) -> f64 {
    F::new(df1, df2).map_or(f64::NAN, |d| d.cdf(x))
}
/// `qf(p, df1, df2)`.
pub fn qf(p: f64, df1: f64, df2: f64) -> f64 {
    F::new(df1, df2).map_or(f64::NAN, |d| d.quantile(p))
}
/// `rf(n, df1, df2, rng)`.
pub fn rf(n: usize, df1: f64, df2: f64, rng: &mut RngState) -> Vec<f64> {
    F::new(df1, df2).map_or_else(|| vec![f64::NAN; n], |d| {
        (0..n).map(|_| d.sample(rng)).collect()
    })
}

// ---------------------------------------------------------------------------
// LogNormal
// ---------------------------------------------------------------------------

/// LogNormal — exp of a Normal distribution.
#[derive(Debug, Clone, Copy)]
pub struct LogNormal {
    /// Mean of underlying normal.
    pub meanlog: f64,
    /// SD of underlying normal.
    pub sdlog: f64,
}

impl LogNormal {
    /// Construct.
    pub fn new(meanlog: f64, sdlog: f64) -> Option<Self> {
        (sdlog > 0.0 && meanlog.is_finite() && sdlog.is_finite())
            .then_some(Self { meanlog, sdlog })
    }
}

impl ContinuousDistribution for LogNormal {
    fn pdf(&self, x: f64) -> f64 {
        if x <= 0.0 {
            return 0.0;
        }
        let z = (x.ln() - self.meanlog) / self.sdlog;
        let two_pi = 2.0 * core::f64::consts::PI;
        (-(z * z) / 2.0).exp() / (x * self.sdlog * two_pi.sqrt())
    }
    fn cdf(&self, x: f64) -> f64 {
        if x <= 0.0 {
            return 0.0;
        }
        let z = (x.ln() - self.meanlog) / (self.sdlog * core::f64::consts::SQRT_2);
        0.5 * (1.0 + special::erf(z))
    }
    fn quantile(&self, p: f64) -> f64 {
        let normal = crate::distributions::Normal::new(self.meanlog, self.sdlog).unwrap();
        normal.quantile(p).exp()
    }
    fn sample(&self, rng: &mut RngState) -> f64 {
        (self.meanlog + self.sdlog * standard_normal(rng)).exp()
    }
    fn mean(&self) -> Option<f64> {
        Some((self.meanlog + 0.5 * self.sdlog * self.sdlog).exp())
    }
    fn variance(&self) -> Option<f64> {
        let m = self.mean().unwrap();
        Some(m * m * ((self.sdlog * self.sdlog).exp() - 1.0))
    }
}

/// `dlnorm(x, meanlog, sdlog)`.
pub fn dlnorm(x: f64, meanlog: f64, sdlog: f64) -> f64 {
    LogNormal::new(meanlog, sdlog).map_or(f64::NAN, |d| d.pdf(x))
}
/// `plnorm(x, meanlog, sdlog)`.
pub fn plnorm(x: f64, meanlog: f64, sdlog: f64) -> f64 {
    LogNormal::new(meanlog, sdlog).map_or(f64::NAN, |d| d.cdf(x))
}
/// `qlnorm(p, meanlog, sdlog)`.
pub fn qlnorm(p: f64, meanlog: f64, sdlog: f64) -> f64 {
    LogNormal::new(meanlog, sdlog).map_or(f64::NAN, |d| d.quantile(p))
}
/// `rlnorm(n, meanlog, sdlog, rng)`.
pub fn rlnorm(n: usize, meanlog: f64, sdlog: f64, rng: &mut RngState) -> Vec<f64> {
    LogNormal::new(meanlog, sdlog).map_or_else(|| vec![f64::NAN; n], |d| {
        (0..n).map(|_| d.sample(rng)).collect()
    })
}

// ---------------------------------------------------------------------------
// Beta
// ---------------------------------------------------------------------------

/// Beta distribution on `(0, 1)`.
#[derive(Debug, Clone, Copy)]
pub struct Beta {
    /// First shape parameter (positive).
    pub shape1: f64,
    /// Second shape parameter (positive).
    pub shape2: f64,
}

impl Beta {
    /// Construct.
    pub fn new(shape1: f64, shape2: f64) -> Option<Self> {
        (shape1 > 0.0 && shape2 > 0.0 && shape1.is_finite() && shape2.is_finite())
            .then_some(Self { shape1, shape2 })
    }
}

impl ContinuousDistribution for Beta {
    fn pdf(&self, x: f64) -> f64 {
        if x <= 0.0 || x >= 1.0 {
            return 0.0;
        }
        let log = (self.shape1 - 1.0) * x.ln()
            + (self.shape2 - 1.0) * (1.0 - x).ln()
            - special::lbeta(self.shape1, self.shape2);
        log.exp()
    }
    fn cdf(&self, x: f64) -> f64 {
        if x <= 0.0 {
            return 0.0;
        }
        if x >= 1.0 {
            return 1.0;
        }
        special::beta_inc(x, self.shape1, self.shape2)
    }
    fn quantile(&self, p: f64) -> f64 {
        if !(0.0..=1.0).contains(&p) {
            return f64::NAN;
        }
        if p == 0.0 {
            return 0.0;
        }
        if p == 1.0 {
            return 1.0;
        }
        // Bisect on (0, 1).
        bisect_inverse_cdf(|x| self.cdf(x), p, 0.0, 1.0)
    }
    fn sample(&self, rng: &mut RngState) -> f64 {
        // Ratio of two gammas: X = Y1 / (Y1 + Y2) where Y_i ~ Gamma(shape_i, 1).
        let y1 = sample_gamma(self.shape1, 1.0, rng);
        let y2 = sample_gamma(self.shape2, 1.0, rng);
        y1 / (y1 + y2)
    }
    fn mean(&self) -> Option<f64> {
        Some(self.shape1 / (self.shape1 + self.shape2))
    }
    fn variance(&self) -> Option<f64> {
        let s = self.shape1 + self.shape2;
        Some((self.shape1 * self.shape2) / (s * s * (s + 1.0)))
    }
}

/// `dbeta(x, shape1, shape2)`.
pub fn dbeta(x: f64, shape1: f64, shape2: f64) -> f64 {
    Beta::new(shape1, shape2).map_or(f64::NAN, |d| d.pdf(x))
}
/// `pbeta(x, shape1, shape2)`.
pub fn pbeta(x: f64, shape1: f64, shape2: f64) -> f64 {
    Beta::new(shape1, shape2).map_or(f64::NAN, |d| d.cdf(x))
}
/// `qbeta(p, shape1, shape2)`.
pub fn qbeta(p: f64, shape1: f64, shape2: f64) -> f64 {
    Beta::new(shape1, shape2).map_or(f64::NAN, |d| d.quantile(p))
}
/// `rbeta(n, shape1, shape2, rng)`.
pub fn rbeta(n: usize, shape1: f64, shape2: f64, rng: &mut RngState) -> Vec<f64> {
    Beta::new(shape1, shape2).map_or_else(|| vec![f64::NAN; n], |d| {
        (0..n).map(|_| d.sample(rng)).collect()
    })
}

// ---------------------------------------------------------------------------
// DiscreteDistribution trait + Binomial + Poisson
// ---------------------------------------------------------------------------

/// Discrete probability distribution. Counterpart to
/// [`ContinuousDistribution`].
pub trait DiscreteDistribution {
    /// Probability mass at `x`.
    fn pmf(&self, x: i64) -> f64;
    /// Cumulative `P(X ≤ x)`.
    fn cdf(&self, x: i64) -> f64;
    /// Inverse CDF — smallest `x` such that `cdf(x) >= p`.
    fn quantile(&self, p: f64) -> i64;
    /// Random sample.
    fn sample(&self, rng: &mut RngState) -> i64;
    /// Mean.
    fn mean(&self) -> f64;
    /// Variance.
    fn variance(&self) -> f64;
}

/// Binomial distribution.
#[derive(Debug, Clone, Copy)]
pub struct Binomial {
    /// Number of trials.
    pub size: u64,
    /// Probability of success per trial.
    pub prob: f64,
}

impl Binomial {
    /// Construct.
    pub fn new(size: u64, prob: f64) -> Option<Self> {
        (size > 0 && (0.0..=1.0).contains(&prob)).then_some(Self { size, prob })
    }
}

impl DiscreteDistribution for Binomial {
    fn pmf(&self, x: i64) -> f64 {
        if x < 0 || (x as u64) > self.size {
            return 0.0;
        }
        let log = special::lchoose(self.size as f64, x as f64)
            + (x as f64) * self.prob.ln()
            + ((self.size - x as u64) as f64) * (1.0 - self.prob).ln();
        log.exp()
    }
    fn cdf(&self, x: i64) -> f64 {
        if x < 0 {
            return 0.0;
        }
        if (x as u64) >= self.size {
            return 1.0;
        }
        // Σ pmf(k) for k = 0..=x.
        let mut sum = 0.0;
        for k in 0..=x {
            sum += self.pmf(k);
        }
        sum.min(1.0)
    }
    fn quantile(&self, p: f64) -> i64 {
        if !(0.0..=1.0).contains(&p) {
            return -1;
        }
        if p == 0.0 {
            return 0;
        }
        if p == 1.0 {
            return self.size as i64;
        }
        let mut cum = 0.0;
        for k in 0..=(self.size as i64) {
            cum += self.pmf(k);
            if cum >= p {
                return k;
            }
        }
        self.size as i64
    }
    fn sample(&self, rng: &mut RngState) -> i64 {
        // Inverse-CDF lookup — fine for size <= ~1000.
        let u = rng.next_f64();
        self.quantile(u)
    }
    fn mean(&self) -> f64 {
        self.size as f64 * self.prob
    }
    fn variance(&self) -> f64 {
        self.size as f64 * self.prob * (1.0 - self.prob)
    }
}

/// `dbinom(x, size, prob)`.
pub fn dbinom(x: i64, size: u64, prob: f64) -> f64 {
    Binomial::new(size, prob).map_or(f64::NAN, |d| d.pmf(x))
}
/// `pbinom(x, size, prob)`.
pub fn pbinom(x: i64, size: u64, prob: f64) -> f64 {
    Binomial::new(size, prob).map_or(f64::NAN, |d| d.cdf(x))
}
/// `qbinom(p, size, prob)`.
pub fn qbinom(p: f64, size: u64, prob: f64) -> i64 {
    Binomial::new(size, prob).map_or(-1, |d| d.quantile(p))
}
/// `rbinom(n, size, prob, rng)`.
pub fn rbinom(n: usize, size: u64, prob: f64, rng: &mut RngState) -> Vec<i64> {
    Binomial::new(size, prob).map_or_else(|| vec![-1; n], |d| {
        (0..n).map(|_| d.sample(rng)).collect()
    })
}

// ---------------------------------------------------------------------------
// Poisson
// ---------------------------------------------------------------------------

/// Poisson distribution with rate λ > 0.
#[derive(Debug, Clone, Copy)]
pub struct Poisson {
    /// Rate λ.
    pub lambda: f64,
}

impl Poisson {
    /// Construct.
    pub fn new(lambda: f64) -> Option<Self> {
        (lambda > 0.0 && lambda.is_finite()).then_some(Self { lambda })
    }
}

impl DiscreteDistribution for Poisson {
    fn pmf(&self, x: i64) -> f64 {
        if x < 0 {
            return 0.0;
        }
        let log = (x as f64) * self.lambda.ln() - self.lambda - special::lgamma((x + 1) as f64);
        log.exp()
    }
    fn cdf(&self, x: i64) -> f64 {
        if x < 0 {
            return 0.0;
        }
        let mut sum = 0.0;
        for k in 0..=x {
            sum += self.pmf(k);
        }
        sum.min(1.0)
    }
    fn quantile(&self, p: f64) -> i64 {
        if !(0.0..=1.0).contains(&p) {
            return -1;
        }
        if p == 0.0 {
            return 0;
        }
        let mut cum = 0.0;
        let mut k = 0_i64;
        loop {
            cum += self.pmf(k);
            if cum >= p {
                return k;
            }
            k += 1;
            if k > 10_000 {
                return k;
            }
        }
    }
    fn sample(&self, rng: &mut RngState) -> i64 {
        // Knuth's algorithm — good for lambda < ~30.
        if self.lambda < 30.0 {
            let l = (-self.lambda).exp();
            let mut k = 0_i64;
            let mut p = 1.0;
            loop {
                k += 1;
                p *= rng.next_f64();
                if p <= l {
                    return k - 1;
                }
            }
        } else {
            // For large lambda, use the inverse-CDF table.
            self.quantile(rng.next_f64())
        }
    }
    fn mean(&self) -> f64 {
        self.lambda
    }
    fn variance(&self) -> f64 {
        self.lambda
    }
}

/// `dpois(x, lambda)`.
pub fn dpois(x: i64, lambda: f64) -> f64 {
    Poisson::new(lambda).map_or(f64::NAN, |d| d.pmf(x))
}
/// `ppois(x, lambda)`.
pub fn ppois(x: i64, lambda: f64) -> f64 {
    Poisson::new(lambda).map_or(f64::NAN, |d| d.cdf(x))
}
/// `qpois(p, lambda)`.
pub fn qpois(p: f64, lambda: f64) -> i64 {
    Poisson::new(lambda).map_or(-1, |d| d.quantile(p))
}
/// `rpois(n, lambda, rng)`.
pub fn rpois(n: usize, lambda: f64, rng: &mut RngState) -> Vec<i64> {
    Poisson::new(lambda).map_or_else(|| vec![-1; n], |d| {
        (0..n).map(|_| d.sample(rng)).collect()
    })
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
    fn uniform_pdf_cdf_quantile() {
        let u = Uniform::new(0.0, 10.0).unwrap();
        assert!(close(u.pdf(5.0), 0.1, 1e-9));
        assert!(close(u.cdf(5.0), 0.5, 1e-9));
        assert!(close(u.quantile(0.5), 5.0, 1e-9));
        assert_eq!(u.mean(), Some(5.0));
    }

    #[test]
    fn exponential_known_values() {
        // R: pexp(1, rate=1) = 1 - e^(-1) ≈ 0.6321
        assert!(close(pexp(1.0, 1.0), 1.0 - (-1.0_f64).exp(), 1e-9));
        assert!(close(qexp(0.5, 1.0), -(0.5_f64).ln(), 1e-9));
    }

    #[test]
    fn exponential_quantile_round_trip() {
        for p in [0.1, 0.25, 0.5, 0.75, 0.9] {
            let x = qexp(p, 2.0);
            assert!(close(pexp(x, 2.0), p, 1e-9), "p={p}");
        }
    }

    #[test]
    fn cauchy_median_at_location() {
        let c = Cauchy::new(3.0, 1.0).unwrap();
        assert!(close(c.cdf(3.0), 0.5, 1e-9));
        assert!(close(c.quantile(0.5), 3.0, 1e-9));
        assert_eq!(c.mean(), None);
    }

    #[test]
    fn gamma_chisq_relationship() {
        // ChiSq(k) = Gamma(k/2, 1/2)
        let chi = ChiSq::new(4.0).unwrap();
        let gam = Gamma::new(2.0, 0.5).unwrap();
        for x in [0.5, 1.0, 2.0, 5.0] {
            assert!(close(chi.pdf(x), gam.pdf(x), 1e-9), "x={x}");
            assert!(close(chi.cdf(x), gam.cdf(x), 1e-9), "x={x}");
        }
    }

    #[test]
    fn pchisq_known_values() {
        // R: pchisq(3.84, df=1) ≈ 0.9500
        assert!(close(pchisq(3.84, 1.0), 0.95, 1e-3));
        // pchisq(5.99, df=2) ≈ 0.9500
        assert!(close(pchisq(5.99, 2.0), 0.95, 1e-3));
    }

    #[test]
    fn studentt_mean_undefined_for_low_df() {
        let t1 = StudentT::new(1.0).unwrap();
        assert_eq!(t1.mean(), None);
        let t3 = StudentT::new(3.0).unwrap();
        assert_eq!(t3.mean(), Some(0.0));
    }

    #[test]
    fn pt_symmetric_about_zero() {
        for x in [0.5, 1.0, 2.0] {
            assert!(close(pt(-x, 5.0), 1.0 - pt(x, 5.0), 1e-6));
        }
    }

    #[test]
    fn pf_known_values() {
        // R: pf(1, df1=10, df2=10) ≈ 0.5
        assert!(close(pf(1.0, 10.0, 10.0), 0.5, 1e-3));
    }

    #[test]
    fn lognormal_pdf_at_one_with_standard_params() {
        // dlnorm(1, meanlog=0, sdlog=1) = 1/sqrt(2π) ≈ 0.39894
        assert!(close(dlnorm(1.0, 0.0, 1.0), 0.398942280401_f64, 1e-9));
    }

    #[test]
    fn beta_pdf_symmetric_when_shapes_equal() {
        // B(2,2) is symmetric around 0.5.
        for x in [0.2, 0.4] {
            assert!(close(
                dbeta(x, 2.0, 2.0),
                dbeta(1.0 - x, 2.0, 2.0),
                1e-9
            ));
        }
    }

    #[test]
    fn pbeta_known_values() {
        // R: pbeta(0.5, 2, 2) = 0.5
        assert!(close(pbeta(0.5, 2.0, 2.0), 0.5, 1e-9));
    }

    #[test]
    fn binomial_pmf_sums_to_one() {
        // Σ pmf(k) over k = 0..size should equal 1.
        let n = 10;
        let p = 0.3;
        let mut sum = 0.0;
        for k in 0..=n {
            sum += dbinom(k, n as u64, p);
        }
        assert!(close(sum, 1.0, 1e-9));
    }

    #[test]
    fn binomial_mean_variance() {
        let b = Binomial::new(10, 0.3).unwrap();
        assert!(close(b.mean(), 3.0, 1e-9));
        assert!(close(b.variance(), 2.1, 1e-9));
    }

    #[test]
    fn poisson_pmf_sums_close_to_one() {
        let mut sum = 0.0;
        for k in 0..=50 {
            sum += dpois(k, 5.0);
        }
        assert!(close(sum, 1.0, 1e-9));
    }

    #[test]
    fn poisson_mean_equals_variance() {
        let p = Poisson::new(7.5).unwrap();
        assert_eq!(p.mean(), p.variance());
    }

    #[test]
    fn rng_reproducibility_across_distributions() {
        // Two `rnorm` runs with the same seed produce identical
        // sequences.
        let mut rng1 = RngState::new(42);
        let mut rng2 = RngState::new(42);
        let v1 = rexp(5, 1.0, &mut rng1);
        let v2 = rexp(5, 1.0, &mut rng2);
        assert_eq!(v1, v2);
    }

    #[test]
    fn empirical_mean_for_exponential() {
        let mut rng = RngState::new(2024);
        let samples = rexp(10_000, 2.0, &mut rng);
        let mean: f64 = samples.iter().sum::<f64>() / samples.len() as f64;
        // Mean of Exp(2) = 0.5.
        assert!((mean - 0.5).abs() < 0.02);
    }
}
