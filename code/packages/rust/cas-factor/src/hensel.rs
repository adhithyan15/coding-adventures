//! Bivariate Hensel lifting over ℚ[x, y].
//!
//! Ports the Python `cas_factor.hensel` algorithm to Rust.  Algorithm:
//!
//! 1. Substitute `y = y₀` for a small integer `y₀` (0, ±1, ±2, …).
//!    Require the univariate image `f(x, y₀)` to be squarefree with
//!    full x-degree (a *lucky* substitution).
//! 2. Factor the image over ℚ via [`factor_integer_polynomial`].
//! 3. Lift the factors back to ℚ[x, y] via Hensel's lemma — at each
//!    y-layer solve a univariate diophantine `u·g₀ + v·h₀ = e_k` and
//!    add `v·y^k`, `u·y^k` to the two factors.
//! 4. After `deg_y(f) + 1` iterations the lift is exact; verify
//!    `g·h == f` and return.
//!
//! Multi-factor inputs (univariate image splits into r ≥ 2 pieces) are
//! handled by iterated two-factor lift.
//!
//! Returns `None` on degenerate input (single variable, zero), when no
//! lucky `y₀` exists in the search range, when the image is irreducible,
//! or when final-product verification fails.
//!
//! See `code/packages/python/cas-factor/src/cas_factor/hensel.py` for
//! the complete mathematical exposition.

use std::collections::BTreeMap;

use crate::factor::factor_integer_polynomial;

/// Sparse bivariate polynomial: `(i, j) ↦ coefficient` of `x^i · y^j`.
///
/// Empty map = zero polynomial.  Coefficient `0` is stripped at every
/// normalisation pass.
pub type BiPoly = BTreeMap<(usize, usize), Rat>;

/// Univariate Q[x] polynomial in ascending-degree order.
type UniQPoly = Vec<Rat>;

/// Bound on `|y₀|` we try as a Hensel substitution point.
const MAX_Y0_SEARCH: usize = 8;

// ---------------------------------------------------------------------------
// Rational number type.  We use `i128` so the lift has headroom on
// coefficient cross-multiplications without overflowing.
// ---------------------------------------------------------------------------

/// Exact rational in lowest terms; `denom > 0`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Rat {
    pub numer: i128,
    pub denom: i128,
}

impl Rat {
    pub const ZERO: Rat = Rat { numer: 0, denom: 1 };
    pub const ONE: Rat = Rat { numer: 1, denom: 1 };

    pub fn new(numer: i128, denom: i128) -> Self {
        assert!(denom != 0, "Rat denominator cannot be zero");
        if numer == 0 {
            return Rat { numer: 0, denom: 1 };
        }
        let (mut n, mut d) = (numer, denom);
        if d < 0 {
            n = -n;
            d = -d;
        }
        let g = gcd_i128(n.unsigned_abs(), d.unsigned_abs()) as i128;
        Rat {
            numer: n / g,
            denom: d / g,
        }
    }

    pub fn from_int(n: i128) -> Self {
        Rat { numer: n, denom: 1 }
    }

    pub fn is_zero(&self) -> bool {
        self.numer == 0
    }

    pub fn is_one(&self) -> bool {
        self.numer == 1 && self.denom == 1
    }

    pub fn neg(&self) -> Self {
        Rat {
            numer: -self.numer,
            denom: self.denom,
        }
    }

    pub fn add(&self, other: &Rat) -> Rat {
        Rat::new(
            self.numer * other.denom + other.numer * self.denom,
            self.denom * other.denom,
        )
    }

    pub fn sub(&self, other: &Rat) -> Rat {
        Rat::new(
            self.numer * other.denom - other.numer * self.denom,
            self.denom * other.denom,
        )
    }

    pub fn mul(&self, other: &Rat) -> Rat {
        Rat::new(self.numer * other.numer, self.denom * other.denom)
    }

    pub fn div(&self, other: &Rat) -> Rat {
        assert!(other.numer != 0, "Rat division by zero");
        Rat::new(self.numer * other.denom, self.denom * other.numer)
    }

    pub fn pow(&self, n: usize) -> Rat {
        let mut result = Rat::ONE;
        let mut base = *self;
        let mut exp = n;
        while exp > 0 {
            if exp & 1 == 1 {
                result = result.mul(&base);
            }
            base = base.mul(&base);
            exp >>= 1;
        }
        result
    }
}

fn gcd_i128(a: u128, b: u128) -> u128 {
    let (mut a, mut b) = (a, b);
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a.max(1)
}

fn lcm_i128(a: i128, b: i128) -> i128 {
    if a == 0 || b == 0 {
        return 0;
    }
    let g = gcd_i128(a.unsigned_abs(), b.unsigned_abs()) as i128;
    (a / g) * b
}

fn binomial(n: usize, k: usize) -> i128 {
    if k > n {
        return 0;
    }
    let kk = k.min(n - k);
    let mut num: i128 = 1;
    let mut den: i128 = 1;
    for i in 0..kk {
        num *= (n - i) as i128;
        den *= (i + 1) as i128;
    }
    num / den
}

// ---------------------------------------------------------------------------
// Univariate Q[x] helpers.
// ---------------------------------------------------------------------------

fn u_normalize(p: &[Rat]) -> UniQPoly {
    let mut out: Vec<Rat> = p.to_vec();
    while out.last().is_some_and(|c| c.is_zero()) {
        out.pop();
    }
    out
}

fn u_degree(p: &[Rat]) -> i64 {
    let n = u_normalize(p);
    n.len() as i64 - 1
}

fn u_add(a: &[Rat], b: &[Rat]) -> UniQPoly {
    let n = a.len().max(b.len());
    let mut out = vec![Rat::ZERO; n];
    for (i, c) in a.iter().enumerate() {
        out[i] = out[i].add(c);
    }
    for (i, c) in b.iter().enumerate() {
        out[i] = out[i].add(c);
    }
    u_normalize(&out)
}

fn u_sub(a: &[Rat], b: &[Rat]) -> UniQPoly {
    let n = a.len().max(b.len());
    let mut out = vec![Rat::ZERO; n];
    for (i, c) in a.iter().enumerate() {
        out[i] = out[i].add(c);
    }
    for (i, c) in b.iter().enumerate() {
        out[i] = out[i].sub(c);
    }
    u_normalize(&out)
}

fn u_mul(a: &[Rat], b: &[Rat]) -> UniQPoly {
    if a.is_empty() || b.is_empty() {
        return vec![];
    }
    let mut out = vec![Rat::ZERO; a.len() + b.len() - 1];
    for (i, ca) in a.iter().enumerate() {
        if ca.is_zero() {
            continue;
        }
        for (j, cb) in b.iter().enumerate() {
            out[i + j] = out[i + j].add(&ca.mul(cb));
        }
    }
    u_normalize(&out)
}

fn u_scale(a: &[Rat], s: Rat) -> UniQPoly {
    if s.is_zero() {
        return vec![];
    }
    u_normalize(&a.iter().map(|c| c.mul(&s)).collect::<Vec<_>>())
}

/// Polynomial division `a = q · b + r` in ℚ[x]; returns `(q, r)`.
fn u_divmod(a: &[Rat], b: &[Rat]) -> (UniQPoly, UniQPoly) {
    let na = u_normalize(a);
    let nb = u_normalize(b);
    assert!(!nb.is_empty(), "division by zero polynomial");
    let db = nb.len() - 1;
    let lc_b = *nb.last().unwrap();
    let mut q_rev: Vec<Rat> = vec![];
    let mut rem: Vec<Rat> = na;
    while rem.len() > 0 && rem.len() - 1 >= db {
        let shift = rem.len() - 1 - db;
        let c = rem.last().unwrap().div(&lc_b);
        q_rev.push(c);
        for (k, bk) in nb.iter().enumerate() {
            rem[shift + k] = rem[shift + k].sub(&c.mul(bk));
        }
        while rem.last().is_some_and(|c| c.is_zero()) {
            rem.pop();
        }
    }
    q_rev.reverse();
    (u_normalize(&q_rev), u_normalize(&rem))
}

/// Extended Euclidean algorithm: returns `(g, s, t)` with
/// `s·a + t·b = g`, `g` monic.
fn u_gcd_ext(a: &[Rat], b: &[Rat]) -> (UniQPoly, UniQPoly, UniQPoly) {
    let mut old_r = u_normalize(a);
    let mut r = u_normalize(b);
    let mut old_s: UniQPoly = vec![Rat::ONE];
    let mut s: UniQPoly = vec![];
    let mut old_t: UniQPoly = vec![];
    let mut t: UniQPoly = vec![Rat::ONE];

    while !r.is_empty() {
        let (q, _) = u_divmod(&old_r, &r);
        let new_r = u_sub(&old_r, &u_mul(&q, &r));
        old_r = std::mem::replace(&mut r, new_r);
        let new_s = u_sub(&old_s, &u_mul(&q, &s));
        old_s = std::mem::replace(&mut s, new_s);
        let new_t = u_sub(&old_t, &u_mul(&q, &t));
        old_t = std::mem::replace(&mut t, new_t);
    }

    let mut g = old_r;
    if !g.is_empty() && !g.last().unwrap().is_one() {
        let inv = Rat::ONE.div(g.last().unwrap());
        g = u_scale(&g, inv);
        old_s = u_scale(&old_s, inv);
        old_t = u_scale(&old_t, inv);
    }
    (g, old_s, old_t)
}

/// Solve `u·g₀ + v·h₀ = c` with deg u < deg h₀, deg v < deg g₀.
/// Returns `None` if `gcd(g₀, h₀) != 1`.
fn u_diophantine(g0: &[Rat], h0: &[Rat], c: &[Rat]) -> Option<(UniQPoly, UniQPoly)> {
    let (g, s, t) = u_gcd_ext(g0, h0);
    if u_degree(&g) != 0 {
        return None;
    }
    let inv = Rat::ONE.div(&g[0]);
    let s = u_scale(&s, inv);
    let t = u_scale(&t, inv);
    let sc = u_mul(&s, c);
    let (q, u) = u_divmod(&sc, h0);
    let tc = u_mul(&t, c);
    let v_raw = u_add(&tc, &u_mul(&q, g0));
    let (_, v) = u_divmod(&v_raw, g0);
    Some((u, v))
}

// ---------------------------------------------------------------------------
// Bivariate polynomial helpers.
// ---------------------------------------------------------------------------

fn bi_normalize(p: &BiPoly) -> BiPoly {
    p.iter()
        .filter(|(_, v)| !v.is_zero())
        .map(|(k, v)| (*k, *v))
        .collect()
}

pub fn bi_degree_x(p: &BiPoly) -> i64 {
    p.iter()
        .filter(|(_, v)| !v.is_zero())
        .map(|((i, _), _)| *i as i64)
        .max()
        .unwrap_or(-1)
}

pub fn bi_degree_y(p: &BiPoly) -> i64 {
    p.iter()
        .filter(|(_, v)| !v.is_zero())
        .map(|((_, j), _)| *j as i64)
        .max()
        .unwrap_or(-1)
}

fn bi_sub(a: &BiPoly, b: &BiPoly) -> BiPoly {
    let mut out = a.clone();
    for (k, v) in b {
        let cur = out.get(k).copied().unwrap_or(Rat::ZERO);
        out.insert(*k, cur.sub(v));
    }
    bi_normalize(&out)
}

pub fn bi_mul(a: &BiPoly, b: &BiPoly) -> BiPoly {
    let mut out: BiPoly = BTreeMap::new();
    for ((i1, j1), c1) in a {
        if c1.is_zero() {
            continue;
        }
        for ((i2, j2), c2) in b {
            if c2.is_zero() {
                continue;
            }
            let key = (i1 + i2, j1 + j2);
            let cur = out.get(&key).copied().unwrap_or(Rat::ZERO);
            out.insert(key, cur.add(&c1.mul(c2)));
        }
    }
    bi_normalize(&out)
}

fn bi_equals(a: &BiPoly, b: &BiPoly) -> bool {
    let na = bi_normalize(a);
    let nb = bi_normalize(b);
    na == nb
}

/// Substitute `y = y₀` and return univariate-in-x image.
fn bi_substitute_y(p: &BiPoly, y0: Rat) -> UniQPoly {
    let dx = bi_degree_x(p);
    if dx < 0 {
        return vec![];
    }
    let mut out = vec![Rat::ZERO; (dx + 1) as usize];
    for ((i, j), c) in p {
        out[*i] = out[*i].add(&c.mul(&y0.pow(*j)));
    }
    u_normalize(&out)
}

/// Embed a univariate-in-x polynomial as a bivariate polynomial.
fn bi_uni_x(p: &[Rat]) -> BiPoly {
    let mut out = BTreeMap::new();
    for (i, c) in p.iter().enumerate() {
        if !c.is_zero() {
            out.insert((i, 0), *c);
        }
    }
    out
}

/// Extract univariate-in-x coefficient of `y^k`.
fn bi_coeff_at_y_power(p: &BiPoly, k_pow: usize) -> UniQPoly {
    let dx: i64 = p
        .iter()
        .filter(|((_, j), v)| *j == k_pow && !v.is_zero())
        .map(|((i, _), _)| *i as i64)
        .max()
        .unwrap_or(-1);
    if dx < 0 {
        return vec![];
    }
    let mut out = vec![Rat::ZERO; (dx + 1) as usize];
    for ((i, j), c) in p {
        if *j == k_pow {
            out[*i] = out[*i].add(c);
        }
    }
    u_normalize(&out)
}

/// Rewrite `p` as a polynomial in `(y − y₀)` instead of `y`.
fn bi_shift_y(p: &BiPoly, y0: Rat) -> BiPoly {
    if y0.is_zero() {
        return p.clone();
    }
    let mut out: BiPoly = BTreeMap::new();
    for ((i, j), c) in p {
        if c.is_zero() {
            continue;
        }
        for m in 0..=*j {
            let coeff = c
                .mul(&Rat::from_int(binomial(*j, m)))
                .mul(&y0.pow(j - m));
            let key = (*i, m);
            let cur = out.get(&key).copied().unwrap_or(Rat::ZERO);
            out.insert(key, cur.add(&coeff));
        }
    }
    bi_normalize(&out)
}

// ---------------------------------------------------------------------------
// Univariate ℚ-factoring via factor_integer_polynomial.
// ---------------------------------------------------------------------------

fn factor_uni_q(p: &[Rat]) -> Option<Vec<UniQPoly>> {
    let np = u_normalize(p);
    if np.len() < 2 {
        return None;
    }
    // Clear denominators to integer coefficients.
    let mut denom_lcm: i128 = 1;
    for c in &np {
        denom_lcm = lcm_i128(denom_lcm, c.denom);
    }
    let int_p: Vec<i64> = np
        .iter()
        .map(|c| ((c.numer * denom_lcm) / c.denom) as i64)
        .collect();
    let (content, factors) = factor_integer_polynomial(&int_p);
    if factors.is_empty() {
        return None;
    }
    let mut flat: Vec<UniQPoly> = Vec::new();
    for (coeffs, mult) in &factors {
        for _ in 0..*mult {
            flat.push(coeffs.iter().map(|c| Rat::from_int(*c as i128)).collect());
        }
    }
    if flat.len() == 1 {
        let f0 = &flat[0];
        let scale = Rat::new(content as i128, denom_lcm);
        let scaled = u_scale(f0, scale);
        if scaled == np {
            return None;
        }
    }
    if !flat.is_empty() {
        let scale = Rat::new(content as i128, denom_lcm);
        flat[0] = u_scale(&flat[0], scale);
    }
    Some(flat)
}

// ---------------------------------------------------------------------------
// Two-factor bivariate Hensel lift.
// ---------------------------------------------------------------------------

fn two_factor_lift(
    f: &BiPoly,
    g0: &[Rat],
    h0: &[Rat],
    deg_y: i64,
) -> Option<(BiPoly, BiPoly)> {
    let mut g: BiPoly = bi_uni_x(g0);
    let mut h: BiPoly = bi_uni_x(h0);

    for k in 1..=(deg_y as usize) {
        let error = bi_sub(f, &bi_mul(&g, &h));
        if error.is_empty() {
            break;
        }
        let e_k = bi_coeff_at_y_power(&error, k);
        if e_k.is_empty() {
            continue;
        }
        let (u, v) = u_diophantine(g0, h0, &e_k)?;
        for (i, c) in v.iter().enumerate() {
            if c.is_zero() {
                continue;
            }
            let key = (i, k);
            let cur = g.get(&key).copied().unwrap_or(Rat::ZERO);
            g.insert(key, cur.add(c));
        }
        for (i, c) in u.iter().enumerate() {
            if c.is_zero() {
                continue;
            }
            let key = (i, k);
            let cur = h.get(&key).copied().unwrap_or(Rat::ZERO);
            h.insert(key, cur.add(c));
        }
        g = bi_normalize(&g);
        h = bi_normalize(&h);
    }

    if !bi_equals(&bi_mul(&g, &h), f) {
        return None;
    }
    Some((g, h))
}

// ---------------------------------------------------------------------------
// Top-level: try_bivariate_hensel.
// ---------------------------------------------------------------------------

fn y0_candidates() -> Vec<i128> {
    let mut out: Vec<i128> = vec![0];
    let mut i: i128 = 1;
    while out.len() < MAX_Y0_SEARCH {
        out.push(i);
        if out.len() < MAX_Y0_SEARCH {
            out.push(-i);
        }
        i += 1;
    }
    out
}

fn is_lucky(p: &BiPoly, image: &[Rat]) -> bool {
    if u_degree(image) != bi_degree_x(p) {
        return false;
    }
    if u_degree(image) < 1 {
        return false;
    }
    let mut deriv: Vec<Rat> = Vec::new();
    for i in 1..image.len() {
        deriv.push(Rat::from_int(i as i128).mul(&image[i]));
    }
    let dn = u_normalize(&deriv);
    if dn.is_empty() {
        return false;
    }
    let (g, _, _) = u_gcd_ext(image, &dn);
    u_degree(&g) == 0
}

/// Attempt to factor a bivariate polynomial via Hensel lifting.
///
/// Returns a list of irreducible bivariate factors whose product equals
/// `f`, or `None` if no non-trivial factorisation was found.
pub fn try_bivariate_hensel(f_in: &BiPoly) -> Option<Vec<BiPoly>> {
    let f = bi_normalize(f_in);
    if f.is_empty() {
        return None;
    }
    if bi_degree_y(&f) < 1 {
        return None;
    }
    if bi_degree_x(&f) < 1 {
        return None;
    }
    let deg_y = bi_degree_y(&f);

    for y0 in y0_candidates() {
        let y0_frac = Rat::from_int(y0);
        let f_shifted = bi_shift_y(&f, y0_frac);
        let image = bi_substitute_y(&f_shifted, Rat::ZERO);
        if !is_lucky(&f_shifted, &image) {
            continue;
        }
        let uni_factors = match factor_uni_q(&image) {
            Some(v) if v.len() >= 2 => v,
            _ => continue,
        };

        let mut remaining_bi = f_shifted.clone();
        let mut bi_factors: Vec<BiPoly> = Vec::new();
        let mut remaining_uni = uni_factors;
        let mut success = true;

        while remaining_uni.len() >= 2 {
            let g0 = remaining_uni[0].clone();
            let mut h0: UniQPoly = vec![Rat::ONE];
            for q in &remaining_uni[1..] {
                h0 = u_mul(&h0, q);
            }
            match two_factor_lift(&remaining_bi, &g0, &h0, deg_y) {
                Some((g_bi, h_bi)) => {
                    bi_factors.push(g_bi);
                    remaining_bi = h_bi;
                    remaining_uni = remaining_uni[1..].to_vec();
                }
                None => {
                    success = false;
                    break;
                }
            }
        }
        if !success {
            continue;
        }
        bi_factors.push(remaining_bi);

        // Un-shift back to original y-frame.
        let factors = if y0 == 0 {
            bi_factors
        } else {
            let neg_y0 = Rat::from_int(-y0);
            bi_factors
                .into_iter()
                .map(|fac| bi_shift_y(&fac, neg_y0))
                .collect::<Vec<_>>()
        };

        // Verify product reconstructs f.
        let mut prod: BiPoly = BTreeMap::new();
        prod.insert((0, 0), Rat::ONE);
        for fac in &factors {
            prod = bi_mul(&prod, fac);
        }
        if !bi_equals(&prod, &f) {
            continue;
        }

        // Filter trivial constants; absorb into first non-trivial.
        let mut non_trivial: Vec<BiPoly> = Vec::new();
        let mut scalar = Rat::ONE;
        for fac in factors {
            if bi_degree_x(&fac) == 0 && bi_degree_y(&fac) == 0 {
                if let Some(v) = fac.values().next() {
                    scalar = scalar.mul(v);
                }
            } else {
                non_trivial.push(fac);
            }
        }
        if non_trivial.len() < 2 {
            continue;
        }
        if !scalar.is_one() {
            let mut sc: BiPoly = BTreeMap::new();
            sc.insert((0, 0), scalar);
            non_trivial[0] = bi_mul(&non_trivial[0], &sc);
        }
        return Some(non_trivial);
    }
    None
}
