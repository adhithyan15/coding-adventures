//! Bounded Berlekamp-Zassenhaus-Hensel factoring for monic integer polynomials.
//!
//! This is the Rust parity path for the Python/TypeScript BZH fallback.  It is
//! intentionally bounded to modest degrees and small primes so the pure Rust
//! CAS factorizer stays deterministic and dependency-free.

use crate::polynomial::{normalize, primitive_part, Poly};

const MAX_BZH_DEGREE: usize = 20;
const MAX_BZH_PRIME: usize = 200;

/// Factor a monic integer polynomial using a bounded BZH pipeline.
///
/// Returns `None` when the polynomial is outside this implementation's
/// bounded domain, appears irreducible, or the modular/lifting pipeline cannot
/// find a non-trivial split.
pub fn bzh_factor(poly: &[i64]) -> Option<Vec<Poly>> {
    let f = normalize(poly);
    if f.is_empty() {
        return None;
    }

    let d = f.len() - 1;
    if !(2..=MAX_BZH_DEGREE).contains(&d) {
        return None;
    }
    if f.last().copied() != Some(1) {
        return None;
    }

    let p = small_primes(MAX_BZH_PRIME)
        .into_iter()
        .find(|&prime| is_squarefree_mod_p(&f, prime))?;
    let mod_factors = berlekamp_factor_mod_p(&pmod_i64(&f, p), p);
    if mod_factors.len() < 2 {
        return None;
    }

    let target = 2.0 * zassenhaus_bound(&f) + 1.0;
    if !target.is_finite() || target <= 0.0 {
        return None;
    }

    let lifted = multi_hensel_lift(&f, &mod_factors, p, target.ceil() as i128)?;
    let mut modulus = p as i128;
    while (modulus as f64) <= target {
        modulus *= p as i128;
    }

    let combined = combine_bzh_factors(&f, &lifted, modulus)?;
    if combined.len() < 2 {
        return None;
    }
    if combined.len() == 1 && normalize_positive_leading(&combined[0]) == f {
        return None;
    }
    Some(combined)
}

fn pmod_i64(coeffs: &[i64], p: i64) -> Vec<i64> {
    let mut out: Vec<i64> = coeffs.iter().map(|&coeff| mod_i64(coeff, p)).collect();
    trim_i64(&mut out);
    out
}

fn pdeg(poly: &[i64]) -> i32 {
    poly.len() as i32 - 1
}

fn padd(a: &[i64], b: &[i64], p: i64) -> Vec<i64> {
    let n = a.len().max(b.len());
    let mut result = vec![0; n];
    for (i, &value) in a.iter().enumerate() {
        result[i] = mod_i64(result[i] + value, p);
    }
    for (i, &value) in b.iter().enumerate() {
        result[i] = mod_i64(result[i] + value, p);
    }
    trim_i64(&mut result);
    result
}

fn psub(a: &[i64], b: &[i64], p: i64) -> Vec<i64> {
    let n = a.len().max(b.len());
    let mut result = vec![0; n];
    for (i, &value) in a.iter().enumerate() {
        result[i] = mod_i64(result[i] + value, p);
    }
    for (i, &value) in b.iter().enumerate() {
        result[i] = mod_i64(result[i] - value, p);
    }
    trim_i64(&mut result);
    result
}

fn pmul(a: &[i64], b: &[i64], p: i64) -> Vec<i64> {
    if a.is_empty() || b.is_empty() {
        return vec![];
    }
    let mut result = vec![0; a.len() + b.len() - 1];
    for (i, &av) in a.iter().enumerate() {
        for (j, &bv) in b.iter().enumerate() {
            result[i + j] = mod_i64(result[i + j] + av * bv, p);
        }
    }
    trim_i64(&mut result);
    result
}

fn pscale(poly: &[i64], scalar: i64, p: i64) -> Vec<i64> {
    let mut result: Vec<i64> = poly
        .iter()
        .map(|&coeff| mod_i64(coeff * scalar, p))
        .collect();
    trim_i64(&mut result);
    result
}

fn pmod_poly(a_input: &[i64], b: &[i64], p: i64) -> Vec<i64> {
    let mut a = a_input.to_vec();
    let db = pdeg(b);
    assert!(db >= 0, "division by zero polynomial");
    let lead_inv = mod_inverse(*b.last().unwrap(), p);
    while pdeg(&a) >= db {
        let shift = pdeg(&a) - db;
        let factor = mod_i64(*a.last().unwrap() * lead_inv, p);
        for (k, &bv) in b.iter().enumerate() {
            let index = shift as usize + k;
            a[index] = mod_i64(a[index] - factor * bv, p);
        }
        trim_i64(&mut a);
    }
    a
}

fn pdiv_quotient(a_input: &[i64], b: &[i64], p: i64) -> Vec<i64> {
    let mut a = a_input.to_vec();
    let db = pdeg(b);
    assert!(db >= 0, "division by zero polynomial");
    let lead_inv = mod_inverse(*b.last().unwrap(), p);
    let mut quotient = Vec::<i64>::new();
    while pdeg(&a) >= db {
        let shift = pdeg(&a) - db;
        let factor = mod_i64(*a.last().unwrap() * lead_inv, p);
        while quotient.len() <= shift as usize {
            quotient.push(0);
        }
        quotient[shift as usize] = mod_i64(quotient[shift as usize] + factor, p);
        for (k, &bv) in b.iter().enumerate() {
            let index = shift as usize + k;
            a[index] = mod_i64(a[index] - factor * bv, p);
        }
        trim_i64(&mut a);
    }
    trim_i64(&mut quotient);
    quotient
}

fn pgcd(a_input: &[i64], b_input: &[i64], p: i64) -> Vec<i64> {
    let mut a = pmod_i64(a_input, p);
    let mut b = pmod_i64(b_input, p);
    while !b.is_empty() {
        let next = pmod_poly(&a, &b, p);
        a = b;
        b = next;
    }
    if a.last().is_some_and(|&lead| lead != 1) {
        a = pscale(&a, mod_inverse(*a.last().unwrap(), p), p);
    }
    a
}

fn pgcd_extended(a: &[i64], b: &[i64], p: i64) -> (Vec<i64>, Vec<i64>, Vec<i64>) {
    let mut old_r = a.to_vec();
    let mut r = b.to_vec();
    let mut old_s = vec![1];
    let mut s = vec![];
    let mut old_t = vec![];
    let mut t = vec![1];

    while !r.is_empty() {
        let q = pdiv_quotient(&old_r, &r, p);
        let next_r = psub(&old_r, &pmul(&q, &r, p), p);
        old_r = r;
        r = next_r;

        let next_s = psub(&old_s, &pmul(&q, &s, p), p);
        old_s = s;
        s = next_s;

        let next_t = psub(&old_t, &pmul(&q, &t, p), p);
        old_t = t;
        t = next_t;
    }

    if old_r.last().is_some_and(|&lead| lead != 1) {
        let inv = mod_inverse(*old_r.last().unwrap(), p);
        old_r = pscale(&old_r, inv, p);
        old_s = pscale(&old_s, inv, p);
        old_t = pscale(&old_t, inv, p);
    }
    (old_r, old_s, old_t)
}

fn pderiv(poly: &[i64], p: i64) -> Vec<i64> {
    if poly.len() <= 1 {
        return vec![];
    }
    let mut result: Vec<i64> = poly
        .iter()
        .enumerate()
        .skip(1)
        .map(|(i, &coeff)| mod_i64(i as i64 * coeff, p))
        .collect();
    trim_i64(&mut result);
    result
}

fn is_squarefree_mod_p(poly: &[i64], p: i64) -> bool {
    let f = pmod_i64(poly, p);
    if f.is_empty() {
        return false;
    }
    let df = pderiv(&f, p);
    if df.is_empty() {
        return false;
    }
    pdeg(&pgcd(&f, &df, p)) == 0
}

fn poly_powmod(mut exp: i64, mod_poly: &[i64], p: i64) -> Vec<i64> {
    let mut result = vec![1];
    let mut current = pmod_poly(&[0, 1], mod_poly, p);
    while exp > 0 {
        if exp & 1 == 1 {
            result = pmod_poly(&pmul(&result, &current, p), mod_poly, p);
        }
        current = pmod_poly(&pmul(&current, &current, p), mod_poly, p);
        exp /= 2;
    }
    result
}

fn null_space_mod_p(matrix: &[Vec<i64>], n: usize, p: i64) -> Vec<Vec<i64>> {
    let mut a = matrix.to_vec();
    let mut pivot_cols = Vec::new();
    let mut row = 0usize;

    for col in 0..n {
        let pivot = (row..n).find(|&r| a[r][col] != 0);
        let Some(pivot) = pivot else {
            continue;
        };
        a.swap(row, pivot);
        let inv = mod_inverse(a[row][col], p);
        for value in &mut a[row] {
            *value = mod_i64(*value * inv, p);
        }
        for r in 0..n {
            if r == row || a[r][col] == 0 {
                continue;
            }
            let factor = a[r][col];
            for j in 0..n {
                a[r][j] = mod_i64(a[r][j] - factor * a[row][j], p);
            }
        }
        pivot_cols.push(col);
        row += 1;
    }

    let mut basis = Vec::new();
    for free_col in 0..n {
        if pivot_cols.contains(&free_col) {
            continue;
        }
        let mut vector = vec![0; n];
        vector[free_col] = 1;
        for (pivot_row, &pivot_col) in pivot_cols.iter().enumerate() {
            vector[pivot_col] = mod_i64(-a[pivot_row][free_col], p);
        }
        basis.push(vector);
    }

    if basis.is_empty() {
        let mut one = vec![0; n];
        if !one.is_empty() {
            one[0] = 1;
        }
        vec![one]
    } else {
        basis
    }
}

fn berlekamp_factor_mod_p(f: &[i64], p: i64) -> Vec<Vec<i64>> {
    let n = pdeg(f);
    if n <= 0 {
        return if f.is_empty() {
            vec![]
        } else {
            vec![f.to_vec()]
        };
    }
    if n == 1 {
        return vec![f.to_vec()];
    }

    let xp_mod_f = poly_powmod(p, f, p);
    let n = n as usize;
    let mut q_matrix = Vec::with_capacity(n);
    let mut current = vec![1];
    for _ in 0..n {
        let mut row = current.clone();
        row.resize(n, 0);
        q_matrix.push(row);
        current = pmod_poly(&pmul(&current, &xp_mod_f, p), f, p);
    }

    let mut matrix = vec![vec![0; n]; n];
    for i in 0..n {
        for j in 0..n {
            matrix[i][j] = mod_i64(q_matrix[j][i] - if i == j { 1 } else { 0 }, p);
        }
    }

    let basis = null_space_mod_p(&matrix, n, p);
    let target_factor_count = basis.len();
    if target_factor_count == 1 {
        return vec![f.to_vec()];
    }

    let mut factors = vec![f.to_vec()];
    for vector in basis.iter().skip(1) {
        if factors.len() == target_factor_count {
            break;
        }
        let mut next_factors = Vec::new();
        for factor in factors {
            if pdeg(&factor) <= 0 {
                next_factors.push(factor);
                continue;
            }

            let mut split_found = false;
            for s in 0..p {
                let mut shifted = vector.clone();
                shifted[0] = mod_i64(shifted[0] - s, p);
                trim_i64(&mut shifted);
                let h = pgcd(&factor, if shifted.is_empty() { &[0] } else { &shifted }, p);
                if pdeg(&h) > 0 && pdeg(&h) < pdeg(&factor) {
                    next_factors.push(h.clone());
                    next_factors.push(pdiv_quotient(&factor, &h, p));
                    split_found = true;
                    break;
                }
            }

            if !split_found {
                next_factors.push(factor);
            }
        }
        factors = next_factors;
    }

    factors
        .into_iter()
        .filter(|factor| !factor.is_empty())
        .map(|factor| {
            if factor.last().copied() == Some(1) {
                factor
            } else {
                pscale(&factor, mod_inverse(*factor.last().unwrap(), p), p)
            }
        })
        .collect()
}

fn zassenhaus_bound(poly: &[i64]) -> f64 {
    let d = poly.len().saturating_sub(1);
    let sum_squares: f64 = poly
        .iter()
        .map(|&coeff| (coeff as f64) * (coeff as f64))
        .sum();
    2f64.powi(d as i32) * ((d + 1) as f64).sqrt() * sum_squares.sqrt()
}

fn iz_mul(a: &[i128], b: &[i128]) -> Vec<i128> {
    if a.is_empty() || b.is_empty() {
        return vec![];
    }
    let mut result = vec![0; a.len() + b.len() - 1];
    for (i, &av) in a.iter().enumerate() {
        for (j, &bv) in b.iter().enumerate() {
            result[i + j] += av * bv;
        }
    }
    trim_i128(&mut result);
    result
}

fn iz_sub(a: &[i128], b: &[i128]) -> Vec<i128> {
    let n = a.len().max(b.len());
    let mut result = vec![0; n];
    for (i, &value) in a.iter().enumerate() {
        result[i] += value;
    }
    for (i, &value) in b.iter().enumerate() {
        result[i] -= value;
    }
    trim_i128(&mut result);
    result
}

fn center_mod_i128(coeffs: &[i128], modulus: i128) -> Vec<i128> {
    let half = modulus / 2;
    let mut result: Vec<i128> = coeffs
        .iter()
        .map(|&coeff| {
            let mut r = ((coeff % modulus) + modulus) % modulus;
            if r > half {
                r -= modulus;
            }
            r
        })
        .collect();
    trim_i128(&mut result);
    result
}

fn to_z_centered(poly: &[i64], p: i64) -> Vec<i128> {
    let half = p / 2;
    let mut result: Vec<i128> = poly
        .iter()
        .map(|&coeff| if coeff <= half { coeff } else { coeff - p } as i128)
        .collect();
    trim_i128(&mut result);
    result
}

fn diophantine_mod_p(a: &[i64], b: &[i64], c: &[i64], p: i64) -> (Vec<i64>, Vec<i64>) {
    let (_, s, t) = pgcd_extended(a, b, p);
    let sc = pmul(&s, c, p);
    let u = pmod_poly(&sc, b, p);
    let q = pdiv_quotient(&sc, b, p);
    let v = pmod_poly(&padd(&pmul(&t, c, p), &pmul(&q, a, p), p), a, p);
    (u, v)
}

fn linear_hensel_lift(
    f: &[i64],
    g_init: &[i64],
    h_init: &[i64],
    p: i64,
    target_mod: i128,
) -> Option<(Vec<i128>, Vec<i128>)> {
    let g_mod = pmod_i64(g_init, p);
    let h_mod = pmod_i64(h_init, p);
    if pdeg(&pgcd(&g_mod, &h_mod, p)) != 0 {
        return None;
    }

    let f: Vec<i128> = f.iter().map(|&coeff| coeff as i128).collect();
    let mut g = to_z_centered(&g_mod, p);
    let mut h = to_z_centered(&h_mod, p);
    let mut pk = p as i128;
    let mut modulus = p as i128;

    while modulus < target_mod {
        let diff = iz_sub(&f, &iz_mul(&g, &h));
        if diff.is_empty() {
            break;
        }
        if diff.iter().any(|coeff| coeff % pk != 0) {
            return None;
        }
        let mut error: Vec<i128> = diff.iter().map(|coeff| coeff / pk).collect();
        trim_i128(&mut error);
        let error_mod = pmod_i128_to_i64(&error, p);
        let (u_mod, v_mod) = diophantine_mod_p(&g_mod, &h_mod, &error_mod, p);
        let u = to_z_centered(&u_mod, p);
        let v = to_z_centered(&v_mod, p);

        let mut next_g = g.clone();
        for (i, &value) in v.iter().enumerate() {
            while next_g.len() <= i {
                next_g.push(0);
            }
            next_g[i] += pk * value;
        }
        let mut next_h = h.clone();
        for (i, &value) in u.iter().enumerate() {
            while next_h.len() <= i {
                next_h.push(0);
            }
            next_h[i] += pk * value;
        }

        g = next_g;
        h = next_h;
        trim_i128(&mut g);
        trim_i128(&mut h);
        pk *= p as i128;
        modulus *= p as i128;
    }

    Some((
        center_mod_i128(&g, target_mod),
        center_mod_i128(&h, target_mod),
    ))
}

fn multi_hensel_lift(
    f: &[i64],
    factors_mod_p: &[Vec<i64>],
    p: i64,
    target: i128,
) -> Option<Vec<Vec<i128>>> {
    if factors_mod_p.is_empty() {
        return Some(vec![]);
    }
    if factors_mod_p.len() == 1 {
        return Some(vec![f.iter().map(|&coeff| coeff as i128).collect()]);
    }

    let mut modulus = p as i128;
    while modulus <= target {
        modulus *= p as i128;
    }

    if factors_mod_p.len() == 2 {
        let (g, h) = linear_hensel_lift(f, &factors_mod_p[0], &factors_mod_p[1], p, modulus)?;
        return Some(vec![g, h]);
    }

    let mid = factors_mod_p.len() / 2;
    let left_factors = &factors_mod_p[..mid];
    let right_factors = &factors_mod_p[mid..];
    let mut left_product = left_factors
        .iter()
        .fold(vec![1], |acc, factor| pmul(&acc, factor, p));
    let mut right_product = right_factors
        .iter()
        .fold(vec![1], |acc, factor| pmul(&acc, factor, p));

    if left_product.last().is_some_and(|&lead| lead != 1) {
        left_product = pscale(
            &left_product,
            mod_inverse(*left_product.last().unwrap(), p),
            p,
        );
    }
    if right_product.last().is_some_and(|&lead| lead != 1) {
        right_product = pscale(
            &right_product,
            mod_inverse(*right_product.last().unwrap(), p),
            p,
        );
    }

    let (left_poly, right_poly) = linear_hensel_lift(f, &left_product, &right_product, p, modulus)?;
    let left_i64 = i128_poly_to_i64(&left_poly)?;
    let right_i64 = i128_poly_to_i64(&right_poly)?;
    let left_lifted = multi_hensel_lift(&left_i64, left_factors, p, target)?;
    let right_lifted = multi_hensel_lift(&right_i64, right_factors, p, target)?;
    Some([left_lifted, right_lifted].concat())
}

fn exact_polynomial_divides(poly: &[i64], divisor: &[i64]) -> Option<Poly> {
    let f = normalize(poly);
    let g = normalize(divisor);
    if g.is_empty() || g.len() > f.len() {
        return None;
    }
    if g.len() == 1 {
        let divisor = g[0];
        if divisor == 0 || f.iter().any(|coeff| coeff % divisor != 0) {
            return None;
        }
        return Some(normalize(
            &f.iter().map(|coeff| coeff / divisor).collect::<Vec<_>>(),
        ));
    }

    let mut remainder = f;
    let mut quotient = vec![0; remainder.len() - g.len() + 1];
    while remainder.len() >= g.len() {
        let shift = remainder.len() - g.len();
        let lead = *remainder.last().unwrap();
        let divisor_lead = *g.last().unwrap();
        if lead % divisor_lead != 0 {
            return None;
        }
        let q = lead / divisor_lead;
        quotient[shift] = q;
        for (i, &value) in g.iter().enumerate() {
            remainder[shift + i] -= q * value;
        }
        trim_i64(&mut remainder);
    }
    if remainder.is_empty() {
        Some(normalize(&quotient))
    } else {
        None
    }
}

fn combine_bzh_factors(f: &[i64], lifted: &[Vec<i128>], modulus: i128) -> Option<Vec<Poly>> {
    let mut remaining_f = f.to_vec();
    let mut remaining_lifted = lifted.to_vec();
    let mut factors = Vec::new();

    while remaining_lifted.len() > 1 {
        let mut found = false;
        let max_size = remaining_lifted.len() / 2;
        for size in 1..=max_size {
            for subset in combinations(remaining_lifted.len(), size) {
                let mut product_poly = vec![1i128];
                for &index in &subset {
                    product_poly = iz_mul(&product_poly, &remaining_lifted[index]);
                }
                let centered = center_mod_i128(&product_poly, modulus);
                let primitive = normalize_positive_leading(&primitive_part_i128_to_i64(&centered)?);
                if primitive.is_empty() {
                    continue;
                }
                if let Some(quotient) = exact_polynomial_divides(&remaining_f, &primitive) {
                    factors.push(primitive);
                    remaining_f = normalize_positive_leading(&primitive_part(&quotient));
                    let selected: std::collections::BTreeSet<_> = subset.into_iter().collect();
                    remaining_lifted = remaining_lifted
                        .into_iter()
                        .enumerate()
                        .filter_map(|(index, factor)| {
                            (!selected.contains(&index)).then_some(factor)
                        })
                        .collect();
                    found = true;
                    break;
                }
            }
            if found {
                break;
            }
        }
        if !found {
            break;
        }
    }

    remaining_f = normalize(&remaining_f);
    if remaining_f.len() > 1 && !(remaining_f.len() == 1 && remaining_f[0].abs() == 1) {
        factors.push(normalize_positive_leading(&remaining_f));
    }

    if factors.is_empty() {
        None
    } else {
        Some(factors)
    }
}

fn combinations(n: usize, size: usize) -> Vec<Vec<usize>> {
    fn rec(
        n: usize,
        size: usize,
        start: usize,
        prefix: &mut Vec<usize>,
        out: &mut Vec<Vec<usize>>,
    ) {
        if prefix.len() == size {
            out.push(prefix.clone());
            return;
        }
        for i in start..=n - (size - prefix.len()) {
            prefix.push(i);
            rec(n, size, i + 1, prefix, out);
            prefix.pop();
        }
    }

    let mut out = Vec::new();
    rec(n, size, 0, &mut Vec::new(), &mut out);
    out
}

fn small_primes(limit: usize) -> Vec<i64> {
    let mut sieve = vec![true; limit + 1];
    sieve[0] = false;
    sieve[1] = false;
    let mut primes = Vec::new();
    for i in 2..=limit {
        if !sieve[i] {
            continue;
        }
        primes.push(i as i64);
        let mut j = i * i;
        while j <= limit {
            sieve[j] = false;
            j += i;
        }
    }
    primes
}

fn mod_i64(value: i64, p: i64) -> i64 {
    ((value % p) + p) % p
}

fn mod_i128(value: i128, p: i64) -> i64 {
    (((value % p as i128) + p as i128) % p as i128) as i64
}

fn mod_inverse(value: i64, p: i64) -> i64 {
    let mut t = 0;
    let mut next_t = 1;
    let mut r = p;
    let mut next_r = mod_i64(value, p);
    while next_r != 0 {
        let q = r / next_r;
        (t, next_t) = (next_t, t - q * next_t);
        (r, next_r) = (next_r, r - q * next_r);
    }
    assert!(r <= 1, "value is not invertible modulo p");
    mod_i64(t, p)
}

fn normalize_positive_leading(poly: &[i64]) -> Poly {
    let normalized = normalize(poly);
    if normalized.last().is_some_and(|&lead| lead < 0) {
        normalized.into_iter().map(|coeff| -coeff).collect()
    } else {
        normalized
    }
}

fn primitive_part_i128_to_i64(poly: &[i128]) -> Option<Poly> {
    i128_poly_to_i64(poly).map(|poly| primitive_part(&poly))
}

fn i128_poly_to_i64(poly: &[i128]) -> Option<Poly> {
    let mut out = Vec::with_capacity(poly.len());
    for &coeff in poly {
        out.push(i64::try_from(coeff).ok()?);
    }
    Some(normalize(&out))
}

fn pmod_i128_to_i64(coeffs: &[i128], p: i64) -> Vec<i64> {
    let mut out: Vec<i64> = coeffs.iter().map(|&coeff| mod_i128(coeff, p)).collect();
    trim_i64(&mut out);
    out
}

fn trim_i64(values: &mut Vec<i64>) {
    while values.last() == Some(&0) {
        values.pop();
    }
}

fn trim_i128(values: &mut Vec<i128>) {
    while values.last() == Some(&0) {
        values.pop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn berlekamp_splits_x_squared_minus_one_mod_three() {
        let factors = berlekamp_factor_mod_p(&pmod_i64(&[-1, 0, 1], 3), 3);
        assert_eq!(factors.len(), 2);
    }

    #[test]
    fn bzh_handles_x5_minus_one_residual() {
        let mut factors = bzh_factor(&[-1, 0, 0, 0, 0, 1]).unwrap();
        factors.sort();
        assert_eq!(factors, vec![vec![-1, 1], vec![1, 1, 1, 1, 1]]);
    }

    #[test]
    fn bzh_confirms_x4_plus_one_irreducible() {
        assert_eq!(bzh_factor(&[1, 0, 0, 0, 1]), None);
    }
}
