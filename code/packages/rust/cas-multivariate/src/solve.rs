use crate::rational::{gcd_i64, lcm_i64};
use crate::{buchberger, MPoly, Rational};

pub fn rational_roots(coeffs: &[Rational]) -> Vec<Rational> {
    let mut lcm_denom = 1;
    for c in coeffs {
        lcm_denom = lcm_i64(lcm_denom, c.denom).abs();
    }
    let mut int_coeffs: Vec<i64> = coeffs
        .iter()
        .map(|c| c.numer * (lcm_denom / c.denom))
        .collect();
    trim_trailing_zeros(&mut int_coeffs);

    if int_coeffs.len() <= 1 {
        return vec![];
    }

    if int_coeffs[0] == 0 {
        let mut roots = vec![Rational::ZERO];
        let trimmed: Vec<Rational> = int_coeffs[1..]
            .iter()
            .map(|&c| Rational::from_int(c))
            .collect();
        roots.extend(rational_roots(&trimmed));
        dedup(roots)
    } else {
        let p_divs = divisors(int_coeffs[0]);
        let q_divs = divisors(*int_coeffs.last().unwrap());
        let mut roots = Vec::new();
        for p in p_divs {
            for q in &q_divs {
                for sign in [1, -1] {
                    let cand = Rational::new(sign * p, *q);
                    if roots.contains(&cand) {
                        continue;
                    }
                    let val = int_coeffs
                        .iter()
                        .enumerate()
                        .fold(Rational::ZERO, |acc, (k, &c)| {
                            acc + Rational::from_int(c) * cand.pow_usize(k)
                        });
                    if val.is_zero() {
                        roots.push(cand);
                    }
                }
            }
        }
        roots
    }
}

pub fn solve_univariate(coeffs: &[Rational]) -> Option<Vec<Rational>> {
    let mut coeffs = coeffs.to_vec();
    while coeffs.len() > 1 && coeffs.last() == Some(&Rational::ZERO) {
        coeffs.pop();
    }

    let degree = coeffs.len().saturating_sub(1);
    match degree {
        0 => Some(vec![]),
        1 => {
            let b = coeffs[0];
            let a = coeffs[1];
            if a.is_zero() {
                Some(vec![])
            } else {
                Some(vec![-b / a])
            }
        }
        2 => solve_quadratic_rational(&coeffs),
        3 | 4 => {
            let rational = rational_roots(&coeffs);
            if rational.is_empty() {
                return Some(vec![]);
            }
            let mut all_roots = rational.clone();
            let mut remaining = coeffs;
            for root in rational {
                remaining = divide_by_linear(&remaining, root);
                if remaining.is_empty() {
                    break;
                }
            }
            if let Some(more) = solve_univariate(&remaining) {
                all_roots.extend(more);
            }
            Some(dedup(all_roots))
        }
        _ => None,
    }
}

pub fn ideal_solve(polys: &[MPoly]) -> Option<Vec<Vec<Rational>>> {
    ideal_solve_with_order(polys, "lex")
}

pub fn ideal_solve_with_order(polys: &[MPoly], order: &str) -> Option<Vec<Vec<Rational>>> {
    if polys.is_empty() {
        return None;
    }
    let nvars = polys[0].nvars;
    let basis = buchberger(polys, order).ok()?;
    if basis.is_empty() {
        return None;
    }

    let last_var = nvars - 1;
    let univariate_poly = basis.iter().find(|g| g.is_univariate() == Some(last_var))?;
    let roots = solve_univariate(&univariate_poly.to_univariate_coeffs(last_var))?;
    if roots.is_empty() {
        return None;
    }

    let mut solutions = Vec::new();
    for root in roots {
        let reduced_basis: Vec<MPoly> = basis
            .iter()
            .map(|g| g.eval_at(last_var, root))
            .filter(|g| !g.is_zero())
            .collect();

        if nvars == 1 {
            solutions.push(vec![root]);
        } else if nvars == 2 {
            if let Some(poly) = find_linear_in_var(&reduced_basis, 0) {
                if let Some(sol0) = eval_linear_root(poly, 0) {
                    solutions.push(vec![sol0, root]);
                }
            } else if let Some(sub_solutions) = solve_from_basis(&reduced_basis, nvars - 1) {
                for mut sub_sol in sub_solutions {
                    sub_sol.push(root);
                    solutions.push(sub_sol);
                }
            }
        } else if let Some(projected) = project_out_last(&reduced_basis, nvars) {
            if let Some(sub_solutions) = ideal_solve_with_order(&projected, order) {
                for mut sub_sol in sub_solutions {
                    sub_sol.push(root);
                    solutions.push(sub_sol);
                }
            }
        }
    }

    if solutions.is_empty() {
        None
    } else {
        Some(solutions)
    }
}

fn solve_quadratic_rational(coeffs: &[Rational]) -> Option<Vec<Rational>> {
    let c = coeffs[0];
    let b = coeffs[1];
    let a = coeffs[2];
    let disc = b * b - Rational::from_int(4) * a * c;
    if disc.is_negative() {
        return Some(vec![]);
    }
    if disc.is_zero() {
        return Some(vec![-b / (Rational::from_int(2) * a)]);
    }
    let Some(sqrt_disc) = rational_square_root(disc) else {
        return Some(vec![]);
    };
    let r1 = (-b + sqrt_disc) / (Rational::from_int(2) * a);
    let r2 = (-b - sqrt_disc) / (Rational::from_int(2) * a);
    if r1 == r2 {
        Some(vec![r1])
    } else {
        Some(vec![r1, r2])
    }
}

fn rational_square_root(value: Rational) -> Option<Rational> {
    let n = integer_square_root(value.numer)?;
    let d = integer_square_root(value.denom)?;
    Some(Rational::new(n, d))
}

fn integer_square_root(value: i64) -> Option<i64> {
    if value < 0 {
        return None;
    }
    let root = (value as f64).sqrt() as i64;
    for candidate in root.saturating_sub(1)..=root + 1 {
        if candidate >= 0 && candidate.saturating_mul(candidate) == value {
            return Some(candidate);
        }
    }
    None
}

fn divisors(n: i64) -> Vec<i64> {
    let n = n.abs();
    if n == 0 {
        return vec![];
    }
    let mut out = Vec::new();
    let mut d = 1;
    while d * d <= n {
        if n % d == 0 {
            out.push(d);
            if d != n / d {
                out.push(n / d);
            }
        }
        d += 1;
    }
    out.sort_unstable();
    out
}

fn divide_by_linear(coeffs: &[Rational], root: Rational) -> Vec<Rational> {
    let mut p = coeffs.to_vec();
    while p.len() > 1 && p.last() == Some(&Rational::ZERO) {
        p.pop();
    }
    if p.len() <= 1 {
        return vec![];
    }

    let degree = p.len() - 1;
    let mut q = vec![Rational::ZERO; degree];
    q[degree - 1] = p[degree];
    for i in (1..degree).rev() {
        q[i - 1] = p[i] + root * q[i];
    }
    trim_trailing_rational_zeros(&mut q);
    q
}

fn find_linear_in_var(basis: &[MPoly], var_idx: usize) -> Option<&MPoly> {
    basis.iter().find(|p| {
        p.is_univariate() == Some(var_idx)
            && p.to_univariate_coeffs(var_idx).len() == 2
            && p.to_univariate_coeffs(var_idx)[1] != Rational::ZERO
    })
}

fn eval_linear_root(p: &MPoly, var_idx: usize) -> Option<Rational> {
    let coeffs = p.to_univariate_coeffs(var_idx);
    if coeffs.len() != 2 || coeffs[1].is_zero() {
        return None;
    }
    Some(-coeffs[0] / coeffs[1])
}

fn solve_from_basis(basis: &[MPoly], nvars: usize) -> Option<Vec<Vec<Rational>>> {
    if nvars == 1 {
        for p in basis {
            if !p.is_zero() && p.is_univariate() == Some(0) {
                let roots = solve_univariate(&p.to_univariate_coeffs(0))?;
                if !roots.is_empty() {
                    return Some(roots.into_iter().map(|r| vec![r]).collect());
                }
            }
        }
    }
    None
}

fn project_out_last(basis: &[MPoly], nvars: usize) -> Option<Vec<MPoly>> {
    let mut result = Vec::new();
    for p in basis {
        if p.is_zero() {
            continue;
        }
        if p.coeffs.keys().any(|m| m[nvars - 1] != 0) {
            return None;
        }
        result.push(MPoly::new(
            p.coeffs.iter().map(|(m, c)| (m[..nvars - 1].to_vec(), *c)),
            nvars - 1,
        ));
    }
    if result.is_empty() {
        None
    } else {
        Some(result)
    }
}

fn trim_trailing_zeros(values: &mut Vec<i64>) {
    while values.last() == Some(&0) {
        values.pop();
    }
}

fn trim_trailing_rational_zeros(values: &mut Vec<Rational>) {
    while values.last() == Some(&Rational::ZERO) {
        values.pop();
    }
}

fn dedup(values: Vec<Rational>) -> Vec<Rational> {
    let mut out = Vec::new();
    for value in values {
        if !out.contains(&value) {
            out.push(value);
        }
    }
    out
}

#[allow(dead_code)]
fn primitive_content(values: &[i64]) -> i64 {
    values.iter().fold(0, |acc, &v| gcd_i64(acc, v.abs()))
}
