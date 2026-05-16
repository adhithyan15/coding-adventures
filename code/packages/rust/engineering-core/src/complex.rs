//! # Complex-number arithmetic — the IM* family.
//!
//! Excel represents complex numbers as strings ("a+bi" or "a+bj")
//! using either `i` or `j` as the imaginary unit. This module exposes
//! a `Complex` struct for in-Rust use plus parser/formatter helpers
//! at the boundary.
//!
//! Phase 1 ships the most-common 16 IM* functions. The remaining
//! ~10 (IMCOSH, IMSINH, IMTAN, IMCOTH, IMSECH, IMCSCH, etc.) are
//! straightforward additions deferred to Phase 2.

use super::EngineeringError;

/// A complex number stored as separate real and imaginary `f64` parts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Complex {
    /// Real part.
    pub re: f64,
    /// Imaginary part.
    pub im: f64,
}

impl Complex {
    /// Construct from parts.
    pub const fn new(re: f64, im: f64) -> Self {
        Self { re, im }
    }

    /// Modulus / absolute value.
    pub fn abs(self) -> f64 {
        (self.re * self.re + self.im * self.im).sqrt()
    }

    /// Argument (angle) in radians, returned in (-π, π].
    pub fn argument(self) -> f64 {
        self.im.atan2(self.re)
    }

    /// Complex conjugate.
    pub fn conjugate(self) -> Self {
        Complex::new(self.re, -self.im)
    }
}

impl core::ops::Add for Complex {
    type Output = Complex;
    fn add(self, rhs: Complex) -> Complex {
        Complex::new(self.re + rhs.re, self.im + rhs.im)
    }
}

impl core::ops::Sub for Complex {
    type Output = Complex;
    fn sub(self, rhs: Complex) -> Complex {
        Complex::new(self.re - rhs.re, self.im - rhs.im)
    }
}

impl core::ops::Mul for Complex {
    type Output = Complex;
    fn mul(self, rhs: Complex) -> Complex {
        Complex::new(
            self.re * rhs.re - self.im * rhs.im,
            self.re * rhs.im + self.im * rhs.re,
        )
    }
}

impl core::ops::Div for Complex {
    type Output = Complex;
    fn div(self, rhs: Complex) -> Complex {
        let denom = rhs.re * rhs.re + rhs.im * rhs.im;
        Complex::new(
            (self.re * rhs.re + self.im * rhs.im) / denom,
            (self.im * rhs.re - self.re * rhs.im) / denom,
        )
    }
}

// ---------------------------------------------------------------------------
// Parse / format
// ---------------------------------------------------------------------------

/// Parse an Excel-style complex string. Accepts both "i" and "j" as
/// the imaginary suffix.
pub fn parse(s: &str) -> Result<Complex, EngineeringError> {
    let s = s.trim();
    if s.is_empty() {
        return Err(EngineeringError::ParseError {
            function: "complex_parse",
            input: s.to_string(),
        });
    }
    // Pure real?
    if !s.ends_with('i') && !s.ends_with('j') {
        if let Ok(re) = s.parse::<f64>() {
            return Ok(Complex::new(re, 0.0));
        }
        return Err(EngineeringError::ParseError {
            function: "complex_parse",
            input: s.to_string(),
        });
    }
    // Strip the i/j suffix.
    let s_no_suffix = &s[..s.len() - 1];
    if s_no_suffix.is_empty() || s_no_suffix == "+" || s_no_suffix == "-" {
        // "i", "+i", "-i" all parse as ±1i.
        let sign = if s_no_suffix.starts_with('-') {
            -1.0
        } else {
            1.0
        };
        return Ok(Complex::new(0.0, sign));
    }
    // Pure imaginary?
    if let Ok(im) = s_no_suffix.parse::<f64>() {
        return Ok(Complex::new(0.0, im));
    }
    // Find the separator '+' or '-' that's not at position 0 and not
    // preceded by 'e'/'E' (which would be the scientific-notation
    // exponent).
    let bytes = s_no_suffix.as_bytes();
    let mut split: Option<usize> = None;
    for i in (1..bytes.len()).rev() {
        let c = bytes[i];
        if (c == b'+' || c == b'-') && (bytes[i - 1] != b'e' && bytes[i - 1] != b'E') {
            split = Some(i);
            break;
        }
    }
    let split = split.ok_or_else(|| EngineeringError::ParseError {
        function: "complex_parse",
        input: s.to_string(),
    })?;
    let (re_str, im_str) = s_no_suffix.split_at(split);
    let re: f64 = re_str.parse().map_err(|_| EngineeringError::ParseError {
        function: "complex_parse",
        input: s.to_string(),
    })?;
    let im_part = if im_str == "+" || im_str == "-" {
        format!("{im_str}1")
    } else {
        im_str.to_string()
    };
    let im: f64 = im_part.parse().map_err(|_| EngineeringError::ParseError {
        function: "complex_parse",
        input: s.to_string(),
    })?;
    Ok(Complex::new(re, im))
}

/// Format a complex number using the given suffix ("i" or "j").
pub fn format(c: Complex, suffix: char) -> String {
    if c.im == 0.0 {
        return format_f64(c.re);
    }
    if c.re == 0.0 {
        return match c.im {
            1.0 => format!("{suffix}"),
            -1.0 => format!("-{suffix}"),
            other => format!("{}{suffix}", format_f64(other)),
        };
    }
    let im_sign = if c.im >= 0.0 { "+" } else { "-" };
    let im_abs = c.im.abs();
    let im_part = if im_abs == 1.0 {
        String::new()
    } else {
        format_f64(im_abs)
    };
    format!("{}{}{}{}", format_f64(c.re), im_sign, im_part, suffix)
}

fn format_f64(v: f64) -> String {
    // Drop trailing zeros / scientific noise — match Excel's compact
    // output for round numbers.
    if v.fract() == 0.0 && v.abs() < 1e15 {
        format!("{}", v as i64)
    } else {
        let s = format!("{v}");
        s
    }
}

// ---------------------------------------------------------------------------
// Excel IM* functions
// ---------------------------------------------------------------------------

/// Excel `IMABS(inumber)`.
pub fn imabs(c: Complex) -> f64 {
    c.abs()
}

/// Excel `IMABS(inumber)` — string variant.
pub fn imabs_str(s: &str) -> Result<f64, EngineeringError> {
    Ok(imabs(parse(s)?))
}

/// Excel `IMARGUMENT(inumber)`. Angle in radians.
pub fn imargument(c: Complex) -> f64 {
    c.argument()
}

/// Excel `IMREAL(inumber)`.
pub fn imreal(c: Complex) -> f64 {
    c.re
}

/// Excel `IMAGINARY(inumber)`.
pub fn imaginary(c: Complex) -> f64 {
    c.im
}

/// Excel `IMCONJUGATE(inumber)`.
pub fn imconjugate(c: Complex) -> Complex {
    c.conjugate()
}

/// Excel `IMSUM(...)`. Variadic add.
pub fn imsum(values: &[Complex]) -> Complex {
    values.iter().copied().fold(Complex::new(0.0, 0.0), |a, b| a + b)
}

/// Excel `IMSUB(a, b)`.
pub fn imsub(a: Complex, b: Complex) -> Complex {
    a - b
}

/// Excel `IMPRODUCT(...)`. Variadic multiply.
pub fn improduct(values: &[Complex]) -> Complex {
    values.iter().copied().fold(Complex::new(1.0, 0.0), |a, b| a * b)
}

/// Excel `IMDIV(a, b)`.
pub fn imdiv(a: Complex, b: Complex) -> Result<Complex, EngineeringError> {
    if b.re == 0.0 && b.im == 0.0 {
        return Err(EngineeringError::DomainError {
            function: "imdiv",
            what: "division by zero".into(),
        });
    }
    Ok(a / b)
}

/// Excel `IMEXP(c)`. `e^(a+bi) = e^a (cos b + i sin b)`.
pub fn imexp(c: Complex) -> Complex {
    let factor = c.re.exp();
    Complex::new(factor * c.im.cos(), factor * c.im.sin())
}

/// Excel `IMLN(c)`. `ln(z) = ln|z| + i*arg(z)`.
pub fn imln(c: Complex) -> Result<Complex, EngineeringError> {
    if c.re == 0.0 && c.im == 0.0 {
        return Err(EngineeringError::DomainError {
            function: "imln",
            what: "ln(0) undefined".into(),
        });
    }
    Ok(Complex::new(c.abs().ln(), c.argument()))
}

/// Excel `IMLOG10(c)`.
pub fn imlog10(c: Complex) -> Result<Complex, EngineeringError> {
    let ln_c = imln(c)?;
    let ln10 = core::f64::consts::LN_10;
    Ok(Complex::new(ln_c.re / ln10, ln_c.im / ln10))
}

/// Excel `IMLOG2(c)`.
pub fn imlog2(c: Complex) -> Result<Complex, EngineeringError> {
    let ln_c = imln(c)?;
    let ln2 = core::f64::consts::LN_2;
    Ok(Complex::new(ln_c.re / ln2, ln_c.im / ln2))
}

/// Excel `IMPOWER(c, n)`. `z^n = exp(n * ln z)`.
pub fn impower(c: Complex, n: f64) -> Result<Complex, EngineeringError> {
    if c.re == 0.0 && c.im == 0.0 {
        if n == 0.0 {
            return Err(EngineeringError::DomainError {
                function: "impower",
                what: "0^0 undefined".into(),
            });
        }
        return Ok(Complex::new(0.0, 0.0));
    }
    let ln_c = imln(c)?;
    Ok(imexp(Complex::new(n * ln_c.re, n * ln_c.im)))
}

/// Excel `IMSQRT(c)`.
pub fn imsqrt(c: Complex) -> Complex {
    let r = c.abs();
    let half = (r + c.re).sqrt() * core::f64::consts::FRAC_1_SQRT_2;
    let half_im = (r - c.re).sqrt() * core::f64::consts::FRAC_1_SQRT_2;
    Complex::new(half, if c.im < 0.0 { -half_im } else { half_im })
}

/// Excel `IMSIN(c)`. `sin(a+bi) = sin a cosh b + i cos a sinh b`.
pub fn imsin(c: Complex) -> Complex {
    Complex::new(c.re.sin() * c.im.cosh(), c.re.cos() * c.im.sinh())
}

/// Excel `IMCOS(c)`. `cos(a+bi) = cos a cosh b - i sin a sinh b`.
pub fn imcos(c: Complex) -> Complex {
    Complex::new(c.re.cos() * c.im.cosh(), -c.re.sin() * c.im.sinh())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    fn close_c(a: Complex, b: Complex) -> bool {
        close(a.re, b.re) && close(a.im, b.im)
    }

    #[test]
    fn parse_pure_real() {
        assert_eq!(parse("3.5").unwrap(), Complex::new(3.5, 0.0));
        assert_eq!(parse("-2").unwrap(), Complex::new(-2.0, 0.0));
    }

    #[test]
    fn parse_pure_imaginary() {
        assert_eq!(parse("i").unwrap(), Complex::new(0.0, 1.0));
        assert_eq!(parse("-i").unwrap(), Complex::new(0.0, -1.0));
        assert_eq!(parse("3i").unwrap(), Complex::new(0.0, 3.0));
        assert_eq!(parse("-2.5j").unwrap(), Complex::new(0.0, -2.5));
    }

    #[test]
    fn parse_combined() {
        assert_eq!(parse("3+4i").unwrap(), Complex::new(3.0, 4.0));
        assert_eq!(parse("-1.5-2.5j").unwrap(), Complex::new(-1.5, -2.5));
        // Scientific notation in real part.
        assert_eq!(parse("1.5e2+1i").unwrap(), Complex::new(150.0, 1.0));
    }

    #[test]
    fn parse_malformed_rejected() {
        assert!(parse("").is_err());
        assert!(parse("not a number").is_err());
        assert!(parse("abci").is_err());
    }

    #[test]
    fn format_round_trips_through_parse() {
        for c in [
            Complex::new(0.0, 0.0),
            Complex::new(3.5, -2.0),
            Complex::new(0.0, 1.0),
            Complex::new(-7.0, 0.0),
        ] {
            let s = format(c, 'i');
            let back = parse(&s).unwrap();
            assert_eq!(back, c, "round-trip failed for {c:?} -> '{s}'");
        }
    }

    #[test]
    fn imabs_imargument() {
        let c = Complex::new(3.0, 4.0);
        assert!(close(imabs(c), 5.0));
        assert!(close(imargument(c), 4.0_f64.atan2(3.0)));
    }

    #[test]
    fn imreal_imaginary_imconjugate() {
        let c = Complex::new(2.0, -5.0);
        assert_eq!(imreal(c), 2.0);
        assert_eq!(imaginary(c), -5.0);
        assert_eq!(imconjugate(c), Complex::new(2.0, 5.0));
    }

    #[test]
    fn imsum_imsub_improduct_imdiv() {
        let a = Complex::new(1.0, 2.0);
        let b = Complex::new(3.0, 4.0);
        assert_eq!(imsum(&[a, b]), Complex::new(4.0, 6.0));
        assert_eq!(imsub(a, b), Complex::new(-2.0, -2.0));
        // (1+2i)(3+4i) = -5 + 10i
        assert_eq!(improduct(&[a, b]), Complex::new(-5.0, 10.0));
        // (1+2i)/(3+4i) = 0.44 + 0.08i (verified)
        let q = imdiv(a, b).unwrap();
        assert!(close(q.re, 0.44));
        assert!(close(q.im, 0.08));
    }

    #[test]
    fn imdiv_by_zero_errors() {
        let zero = Complex::new(0.0, 0.0);
        assert!(imdiv(Complex::new(1.0, 1.0), zero).is_err());
    }

    #[test]
    fn imexp_and_imln_inverse_round_trip() {
        let c = Complex::new(1.0, 2.0);
        let back = imexp(imln(c).unwrap());
        assert!(close_c(back, c));
    }

    #[test]
    fn imsqrt_squared_returns_original() {
        let c = Complex::new(3.0, 4.0);
        let r = imsqrt(c);
        let squared = r * r;
        assert!(close_c(squared, c));
    }

    #[test]
    fn impower_consistency() {
        let c = Complex::new(2.0, 0.0);
        // 2^3 = 8.
        let r = impower(c, 3.0).unwrap();
        assert!(close_c(r, Complex::new(8.0, 0.0)));

        // i^2 = -1.
        let i = Complex::new(0.0, 1.0);
        let r = impower(i, 2.0).unwrap();
        assert!(close_c(r, Complex::new(-1.0, 0.0)));
    }

    #[test]
    fn imsin_imcos_identity() {
        // sin² + cos² = 1 even for complex z.
        let c = Complex::new(0.5, 0.3);
        let s = imsin(c);
        let cos = imcos(c);
        let sum = s * s + cos * cos;
        assert!(close_c(sum, Complex::new(1.0, 0.0)));
    }

    #[test]
    fn imlog10_known_value() {
        let r = imlog10(Complex::new(100.0, 0.0)).unwrap();
        assert!(close(r.re, 2.0));
        assert!(close(r.im, 0.0));
    }
}
