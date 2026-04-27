# Phase 4 — Trigonometric Integration

## Status

Phase 4 of the symbolic integration roadmap. Extends the integrator to
handle integrands that involve trigonometric functions beyond what Phases 1
and 3 covered. Three sub-phases, each a clean layer on top of the previous:

- **Phase 4a** — Polynomial × sin or cos via the tabular IBP formula
- **Phase 4b** — Trig squares, trig products (sin·sin, sin·cos, cos·cos)
  via double-angle and product-to-sum identities
- **Phase 4c** — Exp × sin/cos via the complex-exponential double-IBP
  formula

After Phase 4, the only trig integrands still left unevaluated are
those requiring substitution, algebraic extensions, or higher-order
Risch machinery (e.g. `∫ tan(x) dx`, `∫ sin(x)/x dx`).

---

## Scope

### What this phase handles

| Case | Integrand | Method | Sub-phase |
|------|-----------|--------|-----------|
| 4a-s | `p(x)·sin(ax+b)` | tabular IBP | 4a |
| 4a-c | `p(x)·cos(ax+b)` | tabular IBP | 4a |
| 4b-ss | `sin²(ax+b)` | double-angle identity | 4b |
| 4b-cc | `cos²(ax+b)` | double-angle identity | 4b |
| 4b-sc | `sin(ax+b)·cos(ax+b)` | half-angle identity | 4b |
| 4b-sd | `sin(ax+b)·sin(cx+d)` | product-to-sum | 4b |
| 4b-cd | `cos(ax+b)·cos(cx+d)` | product-to-sum | 4b |
| 4b-xd | `sin(ax+b)·cos(cx+d)` with `a≠c` | product-to-sum | 4b |
| 4c-es | `exp(ax+b)·sin(cx+d)` | double IBP | 4c |
| 4c-ec | `exp(ax+b)·cos(cx+d)` | double IBP | 4c |

All arguments are **linear** `a·x + b` with `a, b ∈ Q`, `a ≠ 0` (enforced
by the existing `_try_linear` recogniser from Phase 3). Pure polynomial
operands are recognised via the `to_rational` bridge from Phase 2b.

### What this phase does NOT handle

- `∫ tan(x) dx`, `∫ cot(x) dx`, `∫ sec(x) dx`, `∫ csc(x) dx` — these
  require `log|cos x|` style results; a future Phase 4d could add them.
- `∫ sinⁿ(x) dx` for `n ≥ 3` — reduction formula or beta-function
  approach; deferred.
- `∫ p(x)·sin(x)·cos(x)` after reducing via 4b leaves `p(x)·sin(2x)/2`,
  which Phase 4a then handles — so this one actually falls through for
  free.
- `∫ sin(x)/x dx` — Sine integral Si(x), non-elementary.
- U-substitution detection (e.g. `∫ sin(x²)·2x dx`) — deferred to Phase 5.

---

## Algorithm — Phase 4a: Polynomial × sin/cos (Tabular IBP)

### Derivation

Given `∫ p(x)·sin(ax+b) dx` with `p ∈ Q[x]`, `a ∈ Q \ {0}`, apply IBP
repeatedly with `u = p` and `dv = sin(ax+b) dx`:

    ∫ p·sin = p·(-cos/a) - ∫ p'·(-cos/a) dx
    = -p·cos/a + (1/a)∫ p'·cos dx
    = -p·cos/a + p'·sin/a² - (1/a²)∫ p''·sin dx
    = -p·cos/a + p'·sin/a² + p''·cos/a³ - p'''·sin/a⁴ + ...

After `deg(p) + 1` steps the residual integral is zero (higher derivatives
of a polynomial vanish). The result groups into two polynomial multiples:

    ∫ p(x)·sin(ax+b) dx  =  sin(ax+b)·S(x)  −  cos(ax+b)·C(x)

where:

    C(x) = Σ_{k≥0} (−1)^k · p^(2k)(x) / a^(2k+1)   (sum of even derivatives)
    S(x) = Σ_{k≥0} (−1)^k · p^(2k+1)(x) / a^(2k+2) (sum of odd derivatives)

Both sums are finite because `p^(m) = 0` for `m > deg(p)`.

### Symmetry for cosine

The same tabular computation with `dv = cos(ax+b) dx` gives:

    ∫ p(x)·cos(ax+b) dx  =  sin(ax+b)·C'(x)  +  cos(ax+b)·S'(x)

where:

    C'(x) = Σ_{k≥0} (−1)^k · p^(2k)(x) / a^(2k+1)   = C(x)
    S'(x) = −Σ_{k≥0} (−1)^k · p^(2k+1)(x) / a^(2k+2) = −S(x)

So, using the same pair (C, S):

    ∫ p·sin(ax+b) dx = sin(ax+b)·S(x) − cos(ax+b)·C(x)
    ∫ p·cos(ax+b) dx = sin(ax+b)·C(x) + cos(ax+b)·(−S(x))

This means both integrals share one set of coefficient polynomials; only
the assembly of the IR tree differs.

### Step-by-step coefficient recovery

Given `p` of degree `n`:

1. Build the derivative sequence:
   `derivs = [p, p', p'', ..., p^(n), (0,)]`
   using the `deriv(p)` function from the `polynomial` package.

2. Compute:
   ```
   C = Σ_{k=0,2,4,...} (−1)^(k/2) · derivs[k] / a^(k+1)   (even indices)
   S = Σ_{k=1,3,5,...} (−1)^((k−1)/2) · derivs[k] / a^(k+1) (odd indices)
   ```
   where the scalar divisions are over `Fraction`.

3. Assemble:
   - `∫ p·sin = sin_ir·S_ir − cos_ir·C_ir`
   - `∫ p·cos = sin_ir·C_ir + cos_ir·(−S_ir)`

### Worked examples

**Example 1**: `∫ x·sin(x) dx`

- `p = (0, 1)`, `a = 1`, `b = 0`
- `derivs = [(0,1), (1,), ()]`
- `C = derivs[0]/1 = (0, 1)` → `x`
- `S = derivs[1]/1 = (1,)` → `1`
- Result: `sin(x)·1 − cos(x)·x = sin(x) − x·cos(x)`
- Verify: `d/dx = cos(x) − cos(x) + x·sin(x) = x·sin(x)` ✓

**Example 2**: `∫ x²·sin(x) dx`

- `p = (0, 0, 1)`, `a = 1`, `b = 0`
- `derivs = [(0,0,1), (0,2), (2,), ()]`
- `C = derivs[0] − derivs[2]/1 = (0,0,1) − (2,) = (−2, 0, 1)` → `x²−2`
- `S = derivs[1]/1 − 0 = (0, 2)` → `2x`
- Result: `sin(x)·2x − cos(x)·(x²−2) = 2x·sin(x) + (2−x²)·cos(x)`
- Verify: `d/dx = 2sin + 2x·cos + (−2x)(−sin) + (2−x²)·(−sin)... ` wait let me be careful:
  d/dx[2x sin] = 2sin + 2x cos
  d/dx[(2-x²)cos] = -2x·cos + (2-x²)(-sin) = -2x cos - (2-x²)sin
  Sum = 2sin + 2x cos - 2x cos - 2sin + x²sin = x²sin(x) ✓

**Example 3**: `∫ x·sin(2x+1) dx`

- `p = (0, 1)`, `a = 2`, `b = 1`
- `derivs = [(0,1), (1,), ()]`
- `C = (0,1)/2 = (0, 1/2)` → `x/2`
- `S = (1,)/4 = (1/4,)` → `1/4`
- Result: `sin(2x+1)·(1/4) − cos(2x+1)·(x/2)`
- Verify: `d/dx = (1/4)·2·cos(2x+1) − cos(2x+1)/2 + (x/2)·2·sin(2x+1)`
  = `(1/2)cos − (1/2)cos + x·sin(2x+1) = x·sin(2x+1)` ✓

**Example 4**: `∫ x·cos(x) dx` (cos case)

- Same `C = (0,1)` → `x`, `S = (1,)` → `1`
- Result: `sin(x)·x + cos(x)·(−1) = x·sin(x) − cos(x)`
- Verify: `d/dx = sin(x) + x·cos(x) + sin(x) = ...` wait:
  d/dx[x sin] = sin + x cos
  d/dx[-cos] = sin
  Sum = sin + x cos + sin ... that's wrong. Let me recheck.

  Actually: `∫ x cos = sin(x)·C + cos(x)·(−S) = sin(x)·x + cos(x)·(−1) = x sin(x) − cos(x)`
  d/dx[x sin(x) - cos(x)] = sin(x) + x cos(x) + sin(x) = 2sin(x) + x cos(x)
  
  That's wrong. Let me recompute. ∫ x cos(x) dx by hand:
  IBP: u=x, dv=cos dx → v=sin, du=dx
  = x sin(x) - ∫ sin(x) dx = x sin(x) + cos(x)
  
  So the correct answer is `x sin(x) + cos(x)`, not `x sin(x) - cos(x)`.
  
  My formula gives `sin·C + cos·(-S)` = `x sin(x) + cos(x)·(-1)` = `x sin(x) - cos(x)`. That's wrong!
  
  Let me re-derive more carefully.
  
  `∫ p cos(ax+b) dx` via IBP:
  u = p, dv = cos(ax+b) dx → v = sin(ax+b)/a
  = p·sin/a - (1/a)∫ p'·sin dx
  
  Now ∫ p'·sin dx = -p'·cos/a + p''·sin/a² + ... (from the sin formula)
  = sin·S(p') - cos·C(p')
  where C(p') = p'/a + ..., S(p') = p''/a² + ...
  
  So:
  ∫ p cos = p·sin/a - (1/a)[sin·S(p') - cos·C(p')]
  = p·sin/a - sin·S(p')/a + cos·C(p')/a
  
  Hmm. S(p') = Σ_k (-1)^k (p')^(2k+1)/a^(2k+2) = Σ_k (-1)^k p^(2k+2)/a^(2k+2)
  C(p') = Σ_k (-1)^k (p')^(2k)/a^(2k+1) = Σ_k (-1)^k p^(2k+1)/a^(2k+1)
  
  sin coefficient: p/a - S(p')/a = p/a - Σ_k (-1)^k p^(2k+2)/a^(2k+3)
  = p/a - p''/a³ + p^(4)/a^5 - ...
  = Σ_k (-1)^k p^(2k)/a^(2k+1) = C(x)  [where C uses p, not p']
  
  cos coefficient: C(p')/a = Σ_k (-1)^k p^(2k+1)/a^(2k+1) / a = Σ_k (-1)^k p^(2k+1)/a^(2k+2)
  = S(x)  [the S from the sin case]
  
  So: ∫ p cos = sin·C(x) + cos·S(x)
  
  But C(x) = Σ_k (-1)^k p^(2k)/a^(2k+1), S(x) = Σ_k (-1)^k p^(2k+1)/a^(2k+2)
  
  For p=x, a=1: C = x/1 = x, S = 1/1 = 1
  ∫ x cos = sin·x + cos·1 = x sin + cos ✓
  
  So the formulas are:
  ∫ p·sin = sin·S - cos·C
  ∫ p·cos = sin·C + cos·S

  That's what I had! But let me re-verify the cos example:
  For p=x, a=1: C=x, S=1
  ∫ x cos = sin·C + cos·S = x sin + 1·cos = x sin + cos ✓
  
  Earlier I wrote `sin·C + cos·(-S)` which was wrong. The correct formula is `sin·C + cos·S`.
  
  Let me double-check with p=x², a=1:
  C = x² - 2, S = 2x
  ∫ x² cos = sin·(x²-2) + cos·2x = (x²-2)sin + 2x cos
  d/dx = 2x sin + (x²-2)cos + 2cos - 2x sin = (x²-2)cos + 2cos = (x²+2-2)cos... wait:
  d/dx[(x²-2)sin] = 2x sin + (x²-2)cos
  d/dx[2x cos] = 2cos - 2x sin
  Sum = 2x sin + (x²-2)cos + 2cos - 2x sin = (x²-2+2)cos = x² cos ✓
  
  Great! So the formulas are:
  ∫ p·sin(ax+b) = sin(ax+b)·S − cos(ax+b)·C
  ∫ p·cos(ax+b) = sin(ax+b)·C + cos(ax+b)·S
  
  Let me fix example 4:

**Example 4**: `∫ x·cos(x) dx` (cos case)
  C = x, S = 1
  ∫ x cos = sin(x)·x + cos(x)·1 = x sin(x) + cos(x) ✓

OK so the spec I'm writing had an error in example 4. Let me fix it when writing.

Now let me also check the formula more carefully.

For C and S, the indices are:
- C uses derivs at indices 0, 2, 4, ... (even) with signs (+1, -1, +1, ...) i.e. (-1)^k for k=0,1,2,...
- S uses derivs at indices 1, 3, 5, ... (odd) with signs (+1, -1, +1, ...) i.e. (-1)^k for k=0,1,2,...

C(x) = derivs[0]/a - derivs[2]/a³ + derivs[4]/a^5 - ...
S(x) = derivs[1]/a² - derivs[3]/a^4 + derivs[5]/a^6 - ...

Where derivs[k] = p^(k)(x) (k-th derivative of p, as a polynomial).

Division of a polynomial by a scalar: just multiply each coefficient by the scalar fraction.

For deriving C and S in code:
```python
def _cs_coeffs(p: Polynomial, a: Fraction) -> tuple[Polynomial, Polynomial]:
    """Compute C and S polynomials for tabular trig integration."""
    derivs = [p]
    while normalize(derivs[-1]):
        derivs.append(deriv(derivs[-1]))
    
    C = _zero
    S = _zero
    for k, dk in enumerate(derivs):
        if not normalize(dk):
            break
        sign = Fraction((-1)**(k // 2))  # alternates +1, -1 for pairs
        coef = sign / Fraction(a ** (k + 1))
        scaled = tuple(c * coef for c in dk)
        if k % 2 == 0:
            C = add(C, scaled)
        else:
            S = add(S, scaled)
    return normalize(C), normalize(S)
```

Wait, I need to be more careful. For even k (k=0,2,4,...):
- k=0: sign = (-1)^0 = +1, divisor = a^1
- k=2: sign = (-1)^1 = -1, divisor = a^3
- k=4: sign = (-1)^2 = +1, divisor = a^5

So sign = (-1)^(k//2), divisor = a^(k+1). Let me verify:
- k=0: (-1)^0 = +1, a^1 ✓
- k=2: (-1)^1 = -1, a^3 ✓
- k=4: (-1)^2 = +1, a^5 ✓

For odd k (k=1,3,5,...):
- k=1: sign = (-1)^0 = +1, divisor = a^2
- k=3: sign = (-1)^1 = -1, divisor = a^4

So sign = (-1)^((k-1)//2), divisor = a^(k+1). Equivalently:
- k=1: (-1)^0 = +1, a^2 ✓
- k=3: (-1)^1 = -1, a^4 ✓

Both cases: sign = (-1)^(k//2), divisor = a^(k+1).

For even k: k//2 = k/2 (e.g., k=0 → 0, k=2 → 1, k=4 → 2) ✓
For odd k: k//2 = (k-1)/2 (e.g., k=1 → 0, k=3 → 1, k=5 → 2) ✓

Great! So `sign = (-1)^(k//2)` works for both even and odd k.

The algorithm is clean.

Now let me write the actual spec file. I'll fix the example 4 error.

Also, I realize I need to handle the "bare sin/cos of linear arg" case differently. In Phase 3, we handle `sin(ax+b)` (no polynomial factor, or constant polynomial `p=1`). Phase 4a extends to polynomial `p` of any degree. The case `p = (1,)` (constant polynomial = 1) gives:
- C = 1/a (constant)
- S = 0 (no odd derivatives)
- ∫ 1·sin(ax+b) = sin·0 - cos·(1/a) = -cos(ax+b)/a ← Phase 3 result! ✓
- ∫ 1·cos(ax+b) = sin·(1/a) + cos·0 = sin(ax+b)/a ← Phase 3 result! ✓

So Phase 4a strictly generalizes Phase 3 cases 3b and 3c. We should let Phase 4a handle these (and Phase 3's specialized handling can still fire first as a fast path — they produce identical results).

Actually, let me think about this. Currently Phase 3 already handles `sin(ax+b)` and `cos(ax+b)`. Phase 4a handles `p(x)·sin(ax+b)`. When Phase 3 fires for the bare trig case, it's fine. When the MUL branch fires and we check for `_try_sin_product`/`_try_cos_product` in Phase 4a, it handles the polynomial × trig case. The bare trig case would only reach Phase 4a if it was written as `1·sin(ax+b)` which normally would be simplified. So they don't conflict.

The bare sin/cos case (Phase 3b/3c) fires from the elementary function section (non-MUL node), while Phase 4a fires from the MUL branch. So they're in separate code paths. No conflict.

Now let me write the spec.
</thinking>
<parameter name="content"># Phase 4 — Trigonometric Integration

## Status

Phase 4 of the symbolic integration roadmap. Extends the integrator to
handle integrands that involve trigonometric functions beyond what Phases 1
and 3 covered. Three sub-phases, each a clean layer on top of the previous:

- **Phase 4a** — Polynomial × sin or cos via the tabular IBP formula
- **Phase 4b** — Trig squares and trig products (sin·sin, sin·cos, cos·cos)
  via double-angle and product-to-sum identities
- **Phase 4c** — Exp × sin/cos via the complex-exponential double-IBP formula

After Phase 4, the main unevaluated cases that remain are:
`∫ tan(x) dx`, `∫ sin^n(x)·cos^m(x) dx` for `n,m ≥ 3`, `∫ sin(x)/x dx`
(sine integral), and anything requiring algebraic extensions or u-substitution.

---

## Scope

### What this phase handles

| Case | Integrand | Method | Sub-phase |
|------|-----------|--------|-----------|
| 4a-s | `p(x)·sin(ax+b)` | tabular IBP | 4a |
| 4a-c | `p(x)·cos(ax+b)` | tabular IBP | 4a |
| 4b-ss | `sin²(ax+b)` | double-angle identity | 4b |
| 4b-cc | `cos²(ax+b)` | double-angle identity | 4b |
| 4b-sc | `sin(ax+b)·cos(ax+b)` (same freq) | half-angle identity | 4b |
| 4b-sd | `sin(ax+b)·sin(cx+d)` | product-to-sum | 4b |
| 4b-cd | `cos(ax+b)·cos(cx+d)` | product-to-sum | 4b |
| 4b-xd | `sin(ax+b)·cos(cx+d)` with `a ≠ c` | product-to-sum | 4b |
| 4c-es | `exp(ax+b)·sin(cx+d)` | double IBP | 4c |
| 4c-ec | `exp(ax+b)·cos(cx+d)` | double IBP | 4c |

All transcendental arguments are **linear** `a·x + b` with `a, b ∈ Q`,
`a ≠ 0`. Polynomial coefficients `p(x)` are recognised via `to_rational`
(denominator = 1). The existing `_try_linear` recogniser from Phase 3
handles all linear-argument detection.

### What this phase does NOT handle

- `∫ tan(ax+b) dx = −log|cos(ax+b)|/a` — requires careful handling of
  the absolute value; deferred to a future sub-phase.
- `∫ sinⁿ(x) dx` for `n ≥ 3` — reduction formula / Wallis; deferred.
- `∫ p(x)·exp(ax+b)·sin(cx+d) dx` — polynomial × exp × trig product;
  would need a combined Risch DE and double-IBP; deferred.
- U-substitution (e.g. `∫ sin(x²)·2x dx`) — deferred to Phase 5.
- `∫ sin(x)/x dx` — Sine integral Si(x); non-elementary.

---

## Algorithm — Phase 4a: Polynomial × sin/cos (Tabular IBP)

### Derivation

Given `∫ p(x)·sin(ax+b) dx` with `p ∈ Q[x]`, `a ∈ Q \ {0}`, apply IBP
repeatedly with `u = p` (differentiated at each step) and `dv = sin(ax+b) dx`
(integrated at each step):

```
∫ p·sin(ax+b) dx
= p·(−cos/a) + (1/a)·∫ p'·cos dx
= −p cos/a + p' sin/a² − (1/a²)·∫ p''·sin dx
= −p cos/a + p' sin/a² + p'' cos/a³ − p''' sin/a⁴ + …
```

After `deg(p) + 1` steps the residual integral is zero (the polynomial
has been differentiated to zero). Grouping by sin and cos:

```
∫ p(x)·sin(ax+b) dx  =  sin(ax+b)·S(x)  −  cos(ax+b)·C(x)
```

where the two **coefficient polynomials** are:

```
C(x) = Σ_{k=0,2,4,…} (−1)^(k/2)   · p^(2k)(x) / a^(2k+1)   (even derivatives)
S(x) = Σ_{k=0,2,4,…} (−1)^k        · p^(2k+1)(x) / a^(2k+2) (odd derivatives)
```

More uniformly, for index `k` (starting at 0):

```
sign(k) = (−1)^(k // 2)
divisor(k) = a^(k+1)

even indices (k = 0,2,4,…): term contributes (sign(k) / divisor(k)) · p^(k)(x) to C
odd  indices (k = 1,3,5,…): term contributes (sign(k) / divisor(k)) · p^(k)(x) to S
```

The sums terminate when `p^(k) = 0`, i.e. after `deg(p) + 1` steps.

### Symmetry for cosine

The same tabular process with `dv = cos(ax+b) dx` gives:

```
∫ p(x)·cos(ax+b) dx  =  sin(ax+b)·C(x)  +  cos(ax+b)·S(x)
```

with the **identical** polynomials `C` and `S`. The only difference from
the sine case is which polynomial multiplies sin and which multiplies cos:

| Integrand | sin coefficient | cos coefficient |
|-----------|-----------------|-----------------|
| `p·sin(ax+b)` | `S(x)` | `−C(x)` |
| `p·cos(ax+b)` | `C(x)` | `S(x)` |

This symmetry lets both cases share one `_cs_coeffs` helper.

### Step-by-step algorithm

```
Input: p ∈ Q[x], a ∈ Q \ {0}, b ∈ Q, x

1. Build derivative sequence:
      derivs = [p, deriv(p), deriv²(p), …]
   until the next derivative is the zero polynomial.

2. For each index k = 0, 1, …, len(derivs)−1:
      sign = (−1)^(k // 2)
      scale = Fraction(sign) / Fraction(a)^(k+1)
      scaled_poly = each coefficient of derivs[k] multiplied by scale

      if k is even: add scaled_poly to C
      if k is odd:  add scaled_poly to S

3. C = normalize(C),  S = normalize(S)

4a. For sin: return  Add(Mul(sin_ir, S_ir),  Neg(Mul(cos_ir, C_ir)))
    — simplified if S or C is zero / ±1·monomial.

4b. For cos: return  Add(Mul(sin_ir, C_ir),  Mul(cos_ir, S_ir))
    — similarly simplified.
```

Here `sin_ir = IRApply(SIN, (linear_to_ir(a, b, x),))` and analogously
for `cos_ir`. `S_ir` and `C_ir` are produced by `from_polynomial`.

### Worked examples

**Example 1**: `∫ x·sin(x) dx`  —  `p = (0,1)`, `a = 1`, `b = 0`

Derivative sequence: `[(0,1), (1,), ()]`

| k | deriv | sign | scale | target |
|---|-------|------|-------|--------|
| 0 | `x` | +1 | 1/1 | C += x |
| 1 | `1` | +1 | 1/1 | S += 1 |

`C = x`, `S = 1`. Result: `sin(x)·1 − cos(x)·x = sin(x) − x·cos(x)`.

Verify: `d/dx[sin − x cos] = cos − cos + x sin = x sin(x)` ✓

---

**Example 2**: `∫ x²·sin(x) dx`  —  `p = (0,0,1)`, `a = 1`, `b = 0`

Derivative sequence: `[(0,0,1), (0,2), (2,), ()]`

| k | deriv | sign | scale | target |
|---|-------|------|-------|--------|
| 0 | `x²` | +1 | 1 | C += x² |
| 1 | `2x` | +1 | 1 | S += 2x |
| 2 | `2` | −1 | 1 | C += −2 |

`C = x²−2`, `S = 2x`. Result: `sin(x)·2x − cos(x)·(x²−2)`.

Verify:
```
d/dx[2x sin + (2−x²)cos]
  = 2sin + 2x cos − 2x cos + (2−x²)(−sin)
  = 2sin − 2sin + x²sin = x²sin(x) ✓
```

---

**Example 3**: `∫ x·sin(2x+1) dx`  —  `p = (0,1)`, `a = 2`, `b = 1`

| k | deriv | sign | scale = sign/a^(k+1) | target |
|---|-------|------|----------------------|--------|
| 0 | `x` | +1 | 1/2 | C += x/2 |
| 1 | `1` | +1 | 1/4 | S += 1/4 |

`C = x/2`, `S = 1/4`. Result: `sin(2x+1)/4 − cos(2x+1)·x/2`.

Verify: `d/dx = (1/4)·2·cos − cos/2 + (x/2)·2·sin = (1/2)cos−(1/2)cos + x·sin = x·sin(2x+1)` ✓

---

**Example 4**: `∫ x·cos(x) dx`  —  same `C = x`, `S = 1`

Result for cosine: `sin(x)·C + cos(x)·S = x·sin(x) + cos(x)`.

Verify: `d/dx[x sin + cos] = sin + x cos − sin = x cos(x)` ✓

---

**Example 5**: `∫ cos(2x+3) dx`  —  `p = (1,)` (constant), `a = 2`, `b = 3`

| k | deriv | sign | scale | target |
|---|-------|------|-------|--------|
| 0 | `1` | +1 | 1/2 | C += 1/2 |

`C = 1/2`, `S = 0`. Result for cosine: `sin(2x+3)·(1/2) + cos(2x+3)·0 = sin(2x+3)/2`.

This matches the Phase 3 case 3c result (generalises Phase 3). ✓

---

### New module: `symbolic_vm/trig_poly_integral.py`

```
trig_sin_integral(poly, a, b, x_sym) → IRNode
trig_cos_integral(poly, a, b, x_sym) → IRNode
_cs_coeffs(poly, a) → (C: Polynomial, S: Polynomial)
```

`trig_sin_integral` calls `_cs_coeffs`, then emits `sin·S − cos·C`.
`trig_cos_integral` calls `_cs_coeffs`, then emits `sin·C + cos·S`.

Both handle the degenerate case `poly = (0,)` (zero polynomial) by
returning `IRInteger(0)`.

---

## Algorithm — Phase 4b: Trig Products via Identities

### Motivation

When the integrand is a product of two trig functions of linear arguments,
no new algorithms are needed: the standard trigonometric product identities
expand the product into a sum of bare sin/cos of linear arguments, and
Phase 3 (cases 3b/3c) already integrates those.

### Product-to-sum identities

For linear arguments `u = ax+b`, `v = cx+d`:

```
sin(u)·sin(v) = [cos(u−v) − cos(u+v)] / 2
cos(u)·cos(v) = [cos(u−v) + cos(u+v)] / 2
sin(u)·cos(v) = [sin(u+v) + sin(u−v)] / 2
```

The sum and difference arguments are also linear:
- `u + v = (a+c)x + (b+d)` — linear with coefficient `a+c`
- `u − v = (a−c)x + (b−d)` — linear with coefficient `a−c`

Special cases:

| Identity | Condition | Simplification |
|----------|-----------|----------------|
| `sin(u)·sin(u)` | `a=c, b=d` | `[1 − cos(2ax+2b)] / 2` |
| `cos(u)·cos(u)` | `a=c, b=d` | `[1 + cos(2ax+2b)] / 2` |
| `sin(u)·cos(u)` | `a=c, b=d` | `sin(2ax+2b) / 2` |

When `a = c` and `b = d` (same-frequency case), `u − v = 0`, so
`cos(u−v) = 1` and `sin(u−v) = 0`, which simplifies the general
product-to-sum formula to the familiar double-angle forms.

### Algorithm

1. Recognise a `MUL(f₁, f₂)` where both `f₁` and `f₂` are `SIN` or
   `COS` of a linear argument. Let `(h₁, a₁, b₁)` and `(h₂, a₂, b₂)`
   be their head and linear coefficients.

2. Compute `(a_sum, b_sum) = (a₁+a₂, b₁+b₂)` and
   `(a_diff, b_diff) = (a₁−a₂, b₁−b₂)`.

3. Apply the appropriate identity:
   - Both SIN: `(cos(diff) − cos(sum)) / 2`
   - Both COS: `(cos(diff) + cos(sum)) / 2`
   - SIN × COS (or COS × SIN): `(sin(sum) + sin(diff)) / 2`

4. If `a_diff = 0` and `b_diff = 0` (same argument), `cos(0) = 1` and
   `sin(0) = 0`, so substitute the constant directly.

5. Integrate the resulting linear combination with Phase 3 (cases 3b/3c).

This requires no new module — it is wired directly into `_integrate` at
the `MUL` branch before the Phase 3 helpers.

### IR emission for the reduced integral

After applying the identity, the integrand is a linear combination
`α · sin/cos(linear₁) ± β · sin/cos(linear₂)`. The `_integrate` function
already handles `Add` and `Sub` by linearity, and Phase 3 handles
`sin/cos(linear)`, so recursive integration handles the result automatically.

### Worked examples

**Example 1**: `∫ sin²(x) dx`

Identity: `sin²(x) = (1 − cos(2x)) / 2`
Integral: `x/2 − sin(2x)/4`

Verify: `d/dx = 1/2 − cos(2x)/2 = (1 − cos(2x))/2 = sin²(x)` ✓

---

**Example 2**: `∫ cos²(3x+1) dx`

Identity: `cos²(3x+1) = (1 + cos(6x+2)) / 2`
Integral: `x/2 + sin(6x+2)/12`

Verify: `d/dx = 1/2 + cos(6x+2)·6/12 = (1 + cos(6x+2))/2 = cos²(3x+1)` ✓

---

**Example 3**: `∫ sin(x)·cos(x) dx`

Identity: `sin(x)·cos(x) = sin(2x)/2`
Integral: `−cos(2x)/4`

Verify: `d/dx = sin(2x)·2/4 = sin(2x)/2 = sin(x)cos(x)` ✓

---

**Example 4**: `∫ sin(x)·sin(2x) dx`

Identity: `sin(x)·sin(2x) = [cos(x−2x) − cos(x+2x)]/2 = [cos(−x) − cos(3x)]/2`
Since `cos(−x) = cos(x)`: `= [cos(x) − cos(3x)]/2`
Integral: `sin(x)/2 − sin(3x)/6`

Verify: `d/dx = cos(x)/2 − cos(3x)/2 = [cos(x) − cos(3x)]/2 = sin(x)sin(2x)` ✓

---

**Example 5**: `∫ sin(x)·cos(2x) dx`

Identity: `sin(x)cos(2x) = [sin(3x) + sin(−x)]/2 = [sin(3x) − sin(x)]/2`
Integral: `−cos(3x)/6 + cos(x)/2`

Verify: `d/dx = sin(3x)/2 − sin(x)/2 = [sin(3x)−sin(x)]/2 = sin(x)cos(2x)` ✓

---

### Implementation note

Phase 4b fires in the `MUL` branch of `_integrate`, before Phase 3's
`_try_exp_product` and `_try_log_product`. The recogniser checks whether
both factors are `SIN` or `COS` of a linear argument. If so, it emits
the reduced integrand as an `Add` or `Sub` of scaled trig terms and
recurses — those recursive calls will be caught by Phase 3 (bare
sin/cos of linear).

The degenerate case where the sum-argument has `a_sum = 0` (e.g.
`sin(x)·cos(x)` with `a_sum = 2`) is fine — `linear_to_ir` handles
non-zero `a`. The case where `a_diff = 0` and `b_diff = 0` produces
a constant `cos(0) = 1` or `sin(0) = 0`, which the recursive integrator
handles as a constant.

No new module needed; logic lives in `integrate.py`.

---

## Algorithm — Phase 4c: Exp × Trig (Double IBP)

### Derivation

Given `I = ∫ exp(ax+b)·sin(cx+d) dx` with `a, c ∈ Q \ {0}`:

**IBP round 1** — `u = exp(ax+b)`, `dv = sin(cx+d) dx`:

```
v = −cos(cx+d)/c,  du = a·exp(ax+b) dx

I = −exp(ax+b)·cos(cx+d)/c  + (a/c)·∫ exp(ax+b)·cos(cx+d) dx
```

**IBP round 2** — `u = exp(ax+b)`, `dv = cos(cx+d) dx` on the new integral:

```
Let J = ∫ exp(ax+b)·cos(cx+d) dx
v = sin(cx+d)/c,  du = a·exp(ax+b) dx

J = exp(ax+b)·sin(cx+d)/c  − (a/c)·∫ exp(ax+b)·sin(cx+d) dx
  = exp(ax+b)·sin(cx+d)/c  − (a/c)·I
```

Substituting back:

```
I = −exp·cos/c + (a/c)·[exp·sin/c − (a/c)·I]
I = −exp·cos/c + a·exp·sin/c²  − a²·I/c²
I·(1 + a²/c²) = exp(ax+b)·[a·sin(cx+d) − c·cos(cx+d)] / c²
I·(a²+c²)/c²  = exp(ax+b)·[a·sin(cx+d) − c·cos(cx+d)] / c²
I = exp(ax+b)·[a·sin(cx+d) − c·cos(cx+d)] / (a²+c²)
```

Similarly for the cosine variant:

```
∫ exp(ax+b)·cos(cx+d) dx = exp(ax+b)·[a·cos(cx+d) + c·sin(cx+d)] / (a²+c²)
```

### Mnemonic structure

Let `E = exp(ax+b)`, `α = a/(a²+c²)`, `β = c/(a²+c²)`:

```
∫ E·sin(cx+d) dx = E·[ α·sin(cx+d) − β·cos(cx+d) ]
∫ E·cos(cx+d) dx = E·[ α·cos(cx+d) + β·sin(cx+d) ]
```

The denominator `D = a²+c²` is always positive for `a, c ∈ Q \ {0}`.

### Worked examples

**Example 1**: `∫ eˣ·sin(x) dx`  —  `a = c = 1`, `b = d = 0`

`D = 1+1 = 2`, `α = 1/2`, `β = 1/2`

Result: `eˣ·[sin(x)/2 − cos(x)/2] = eˣ·(sin x − cos x)/2`

Verify: `d/dx = eˣ(sin−cos)/2 + eˣ(cos+sin)/2 = eˣ·2sin/2 = eˣ sin(x)` ✓

---

**Example 2**: `∫ e^(2x)·cos(3x) dx`  —  `a = 2`, `c = 3`, `b = d = 0`

`D = 4+9 = 13`, `α = 2/13`, `β = 3/13`

Result: `e^(2x)·[2cos(3x)/13 + 3sin(3x)/13]`

Verify: `d/dx = e^(2x)·(4cos+6sin)/13 + e^(2x)·(−6sin+9cos)/13 ...`
wait let me compute:
d/dx[e^(2x)·(2cos(3x)+3sin(3x))/13]
= 2e^(2x)·(2cos+3sin)/13 + e^(2x)·(−6sin+9cos)/13
= e^(2x)·[(4cos+6sin) + (−6sin+9cos)]/13
= e^(2x)·13cos/13 = e^(2x)cos(3x) ✓

---

**Example 3**: `∫ e^(x+1)·sin(2x+3) dx`  —  `a=1, b=1, c=2, d=3`

`D = 1+4 = 5`, `α = 1/5`, `β = 2/5`

Result: `exp(x+1)·[sin(2x+3)/5 − 2cos(2x+3)/5]`

---

### New module: `symbolic_vm/exp_trig_integral.py`

```
exp_sin_integral(a, b, c, d, x_sym) → IRNode
exp_cos_integral(a, b, c, d, x_sym) → IRNode
```

Both use the closed formula: compute `D = a²+c²`, then emit:

```python
def exp_sin_integral(a, b, c, d, x):
    D = a*a + c*c          # always > 0
    exp_arg = linear_to_ir(a, b, x)
    sin_arg = linear_to_ir(c, d, x)
    E = IRApply(EXP, (exp_arg,))
    # exp · [a·sin − c·cos] / D
    sin_term = IRApply(MUL, (_frac_ir(a / D), IRApply(SIN, (sin_arg,))))
    cos_term = IRApply(MUL, (_frac_ir(c / D), IRApply(COS, (sin_arg,))))
    return IRApply(MUL, (E, IRApply(SUB, (sin_term, cos_term))))

def exp_cos_integral(a, b, c, d, x):
    D = a*a + c*c
    exp_arg = linear_to_ir(a, b, x)
    cos_arg = linear_to_ir(c, d, x)
    E = IRApply(EXP, (exp_arg,))
    # exp · [a·cos + c·sin] / D
    cos_term = IRApply(MUL, (_frac_ir(a / D), IRApply(COS, (cos_arg,))))
    sin_term = IRApply(MUL, (_frac_ir(c / D), IRApply(SIN, (cos_arg,))))
    return IRApply(MUL, (E, IRApply(ADD, (cos_term, sin_term))))
```

---

## Recognition in the IR

### Phase 4a: Polynomial × trig detection

In the `MUL` branch of `_integrate`, after Phase 3's exp/log checks fail,
try `_try_trig_product(f₁, f₂, x)` and `_try_trig_product(f₂, f₁, x)`.

`_try_trig_product(trig_factor, other_factor, x)`:
1. Check `trig_factor.head ∈ {SIN, COS}`.
2. Check `_try_linear(trig_factor.args[0], x)` gives `(a, b)`.
3. Check `to_rational(other_factor, x)` gives `(poly, (1,))` (pure polynomial,
   denominator = 1).
4. If all pass: call `trig_sin_integral` or `trig_cos_integral` as appropriate.

### Phase 4b: Trig × trig detection

In the `MUL` branch, before Phase 4a, try `_try_trig_trig(f₁, f₂, x)`.

`_try_trig_trig(f₁, f₂, x)`:
1. Both `f₁.head` and `f₂.head` must be in `{SIN, COS}`.
2. Both arguments must pass `_try_linear`.
3. Apply the appropriate product-to-sum identity.
4. Return the integral of the reduced expression (recursive `_integrate` call
   on the simplified `Add`/`Sub` node).

### Phase 4c: Exp × trig detection

In the `MUL` branch, before Phase 4a, also try `_try_exp_trig(f₁, f₂, x)`
and `_try_exp_trig(f₂, f₁, x)`.

`_try_exp_trig(exp_factor, trig_factor, x)`:
1. Check `exp_factor.head == EXP` and `trig_factor.head ∈ {SIN, COS}`.
2. Both arguments must pass `_try_linear`.
3. Call `exp_sin_integral` or `exp_cos_integral` as appropriate.

### Priority in the MUL branch

The recogniser order in the `MUL` branch (after the constant-factor and
Phase 3 checks) is:

1. Phase 4b: `_try_trig_trig` — both factors are trig; fire first because
   the product-to-sum expansion reduces to Phase 3 without recursion depth.
2. Phase 4c: `_try_exp_trig` — one factor is `EXP`, the other is trig.
3. Phase 4a: `_try_trig_product` — one factor is trig, the other is
   polynomial (can be degree 0, which matches the bare-trig case but Phase 3
   already caught that in a different branch).
4. Fall through to `None` → unevaluated `Integrate`.

---

## Integration into `_integrate`

The three Phase 4 sub-phases each add to the `MUL` branch in
`integrate.py`:

```python
# Phase 4b: trig × trig
result = _try_trig_trig(a, b, x) or _try_trig_trig(b, a, x)
if result is not None:
    return result

# Phase 4c: exp × trig
result = _try_exp_trig(a, b, x) or _try_exp_trig(b, a, x)
if result is not None:
    return result

# Phase 4a: poly × trig  (both orderings already checked)
result = _try_trig_product(a, b, x) or _try_trig_product(b, a, x)
if result is not None:
    return result
```

These come after Phase 3's `_try_exp_product` and `_try_log_product`.

---

## Dependency changes

No new IR primitives required. All Phase 4 cases use existing IR nodes
(`EXP`, `SIN`, `COS`, `MUL`, `ADD`, `SUB`, `DIV`, `NEG`).

Phase 4a depends on `polynomial.deriv` (already available in
`coding-adventures-polynomial ≥ 0.4.0`, the current minimum).

---

## New modules

| Module | Public API | Sub-phase |
|--------|-----------|-----------|
| `symbolic_vm/trig_poly_integral.py` | `trig_sin_integral(poly, a, b, x_sym)`, `trig_cos_integral(poly, a, b, x_sym)` | 4a |
| `symbolic_vm/exp_trig_integral.py` | `exp_sin_integral(a, b, c, d, x_sym)`, `exp_cos_integral(a, b, c, d, x_sym)` | 4c |

Phase 4b adds no new module; its logic lives in `integrate.py`.

---

## Test strategy

| Test class | Cases |
|---|---|
| `TestTrigPolyIntegral` | `x·sin(x)`, `x²·sin(x)`, `x·sin(2x+1)`, `x·cos(x)`, `x²·cos(x)`, `x·cos(2x+3)` |
| `TestTrigProducts` | `sin²(x)`, `cos²(x)`, `sin²(2x+1)`, `cos²(3x)`, `sin(x)cos(x)`, `sin(x)sin(2x)`, `cos(x)cos(2x)`, `sin(x)cos(2x)` |
| `TestExpTrig` | `eˣsin(x)`, `eˣcos(x)`, `e^(2x)sin(3x)`, `e^(2x)cos(3x)`, `e^(x+1)sin(2x+3)` |
| `TestFallsThrough` | `tan(x)` → unevaluated, `sin(x)/x` → unevaluated |
| `TestEndToEnd` | via full VM for one case per sub-phase |

All correctness tests use the numerical re-differentiation helper
from previous phases: `d/dx[antideriv(xv)] ≈ integrand(xv)` with
combined tolerance `atol + rtol·|expected|`.

Regression: existing Phase 3 tests for `sin(ax+b)`, `cos(ax+b)` must
still pass — Phase 3 handles bare trig in the elementary-function branch
(not the MUL branch), so there is no conflict.
