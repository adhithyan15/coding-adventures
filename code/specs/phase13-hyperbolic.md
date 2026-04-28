# Phase 13 — Hyperbolic Functions

## Goal

Add six hyperbolic functions — `sinh`, `cosh`, `tanh` (forward) and `asinh`,
`acosh`, `atanh` (inverse) — to the symbolic VM, including:

- Numeric evaluation via `_elementary` handlers.
- Closed-form integration for `∫ P(x)·sinh(ax+b) dx` and `∫ P(x)·cosh(ax+b) dx`
  via tabular IBP.
- Closed-form integration for `∫ P(x)·asinh(ax+b) dx` and
  `∫ P(x)·acosh(ax+b) dx` via reduction IBP.
- Bare `∫ tanh(ax+b) dx` and `∫ atanh(ax+b) dx` (inline closed forms; poly
  generalizations deferred).
- Differentiation rules for all six.
- MACSYMA compiler mappings for all six.

---

## Mathematical Derivation

### Tabular IBP — sinh and cosh

The antiderivative cycle for `sinh(ax+b)` alternates between `sinh` and `cosh`:

```
∫¹ sinh(ax+b) = (1/a)·cosh(ax+b)
∫² sinh(ax+b) = (1/a²)·sinh(ax+b)
∫³ sinh(ax+b) = (1/a³)·cosh(ax+b)
...
```

Applying tabular IBP with `u = P(x)` (differentiated), `dv = sinh(ax+b) dx`
(integrated repeatedly), and sign alternation `(−1)^k`:

```
∫ P(x)·sinh(ax+b) dx
  = P·(1/a)cosh − P'·(1/a²)sinh + P''·(1/a³)cosh − P'''·(1/a⁴)sinh + ...
```

Collecting by parity of k (0-indexed):

```
C(x) = Σ_{k even} (1/a^(k+1))·P^(k)(x)    — coefficient of cosh
S(x) = Σ_{k odd} (−1/a^(k+1))·P^(k)(x)   — coefficient of sinh

∫ P(x)·sinh(ax+b) dx = C(x)·cosh(ax+b) + S(x)·sinh(ax+b)
```

For `cosh(ax+b)`, the antiderivative cycle is identical (shifted by one), so the
same C and S polynomials appear but with the trig functions swapped:

```
∫ P(x)·cosh(ax+b) dx = C(x)·sinh(ax+b) + S(x)·cosh(ax+b)
```

**Key difference from `trig_poly_integral.py`**: the sign pattern is `(−1)^k`
(period 2) rather than `(−1)^(k//2)` (period 4), because the hyperbolic
derivatives do not introduce a sign reversal after two steps.

### Reduction IBP — asinh

IBP with `u = asinh(ax+b)`, `dv = P(x) dx`:

```
du = a/√((ax+b)²+1) dx,  v = Q(x) = ∫ P(x) dx

∫ P(x)·asinh(ax+b) dx = Q(x)·asinh(ax+b) − a · ∫ Q(x)/√((ax+b)²+1) dx
```

Substitute `t = ax+b`, so `x = (t−b)/a`, `dx = dt/a`, `Q̃(t) = Q((t−b)/a)`:

```
a · ∫ Q(x)/√((ax+b)²+1) dx  =  ∫ Q̃(t)/√(t²+1) dt
```

**Reduction formula** for each monomial:

```
∫ tⁿ/√(t²+1) dt = (1/n)·tⁿ⁻¹·√(t²+1) − (n−1)/n · ∫ tⁿ⁻²/√(t²+1) dt

Base cases:
  n = 0 → asinh(t)
  n = 1 → √(t²+1)
```

By linearity: `∫ Q̃(t)/√(t²+1) dt = A(t)·√(t²+1) + B(t)·asinh(t)`

Back-substituting `t = ax+b`:

```
∫ P(x)·asinh(ax+b) dx = [Q(x) − B(ax+b)]·asinh(ax+b) − A(ax+b)·√((ax+b)²+1)
```

### Reduction IBP — acosh

IBP with `u = acosh(ax+b)`, `dv = P(x) dx`:

```
du = a/√((ax+b)²−1) dx  (same sign as asinh — not negative like acos)
```

Residual: `∫ Q̃(t)/√(t²−1) dt = A(t)·√(t²−1) + B(t)·acosh(t)`

Same reduction formula as asinh but over `√(t²−1)`:

```
∫ tⁿ/√(t²−1) dt = (1/n)·tⁿ⁻¹·√(t²−1) − (n−1)/n · ∫ tⁿ⁻²/√(t²−1) dt

Base cases:
  n = 0 → acosh(t)
  n = 1 → √(t²−1)
```

Final result:

```
∫ P(x)·acosh(ax+b) dx = [Q(x) − B(ax+b)]·acosh(ax+b) − A(ax+b)·√((ax+b)²−1)
```

### Bare tanh and atanh (inline)

```
∫ tanh(ax+b) dx = (1/a)·log(cosh(ax+b))

∫ atanh(ax+b) dx = (ax+b)/a · atanh(ax+b) + (1/(2a))·log(1−(ax+b)²)
```

The atanh result is derived by IBP with `u = atanh(ax+b)`, `v = x`, and working
out the constant-shift correction when `b ≠ 0`.

Poly×tanh and poly×atanh are deferred to a future phase.

---

## Worked Examples

### ∫ sinh(x) dx

`P = 1`, `a = 1`, `b = 0`. `_cs_coeffs`: k=0 → C = (1,), S = (). S is empty.

Result: `1·cosh(x)` ✓

### ∫ x·sinh(x) dx

k=0: C += P = (0, 1), k=1: S += −P' = (−1,). Both non-zero.

Result: `x·cosh(x) − sinh(x)` ✓

### ∫ cosh(x) dx

Same C=(1,), S=() as sinh case. Cosh assembly swaps sinh/cosh:

Result: `1·sinh(x)` ✓

### ∫ x·cosh(x) dx

Same C=(0,1), S=(−1,) as the sinh case. Assembly:

Result: `x·sinh(x) − cosh(x)` ✓ (wait — standard result is `x·sinh(x) − cosh(x)`,
let me verify: d/dx[x·sinh(x) − cosh(x)] = sinh(x) + x·cosh(x) − sinh(x) = x·cosh(x) ✓)

### ∫ asinh(x) dx

`P = 1`, `a = 1`, `b = 0`. `Q = x`. `Q̃(t) = t`. `_sqrt_plus_decompose((0,1))`:
- n=1: A = (1,), B = ().

Back-substitute: A_x = (1,) → 1, B_x = ().

Result: `x·asinh(x) − √(x²+1)` ✓

### ∫ x·asinh(x) dx

`Q = x²/2`. `Q̃(t) = t²/2`. `_sqrt_plus_decompose((0, 0, 1/2))`:
- n=2: A_new = (0, 1/4) [positive because leading coefficient is +1/n], B_rec = (1/2,)·(1/2) = (1/4,).
  Wait, let me recalculate: A = (1/2)·(0, 1/2) → wait.

  n=2: A_new = [0, 1/2] (degree-1 monomial 1/2·t^1·√), recursive = (1/2)·∫ 1/√(t²+1) dt = (1/2)·asinh(t).
  So A = (0, 1/2), B = (1/2,)·1/2... 

  Actually: A_new = (1/n)·tⁿ⁻¹ = (1/2)·t so `(0, 1/2)`, then A_rec from n=0 → A_rec=0, B_rec=(1,).
  Total: A = _poly_add((0, Frac(1,2)·(1/2)), scale((n-1)/n, A_rec)) = need coef from Q̃[2]=1/2.
  scale·A_new = (1/2)·(0, 1/2) = (0, 1/4). scale·B_rec = (1/2)·(1/2)·(1,) = (1/4,).
  
  A_total = (0, 1/4), B_total = (1/4,).

Result: `(x²/2 − 1/4)·asinh(x) − x/4·√(x²+1)` ... hmm, let me check:
Standard: ∫ x·asinh(x) dx = (x²/2 + 1/4)·asinh(x) − (x/4)·√(1+x²)

Wait, that's different! Let me re-examine.

∫ P(x)·asinh(ax+b) dx = Q(x)·asinh(ax+b) − ∫ Q̃(t)/√(t²+1) dt back-sub

For P=x, Q=x²/2, Q̃(t)=t²/2.

∫ Q̃(t)/√(t²+1) dt = A(t)·√(t²+1) + B(t)·asinh(t)

Using the reduction formula: ∫ t²/2 / √(t²+1) dt = (1/2)·∫ t²/√(t²+1) dt

∫ t²/√(t²+1) dt (n=2): = (1/2)·t·√(t²+1) - (1/2)·∫ 1/√(t²+1) dt = (1/2)·t·√(t²+1) - (1/2)·asinh(t)

So ∫ t²/2 / √(t²+1) dt = (1/2)·[(1/2)·t·√(t²+1) - (1/2)·asinh(t)] = (1/4)·t·√(t²+1) - (1/4)·asinh(t)

So A(t) = (0, 1/4), B(t) = (-1/4,) ... wait, B is negative?

Actually: A(t)·√(t²+1) + B(t)·asinh(t) = (1/4)·t·√(t²+1) - (1/4)·asinh(t)

So A = (0, 1/4), B = (-1/4,).

Back-substituting (a=1, b=0, so A_x = A, B_x = B):

Result = [Q(x) - B(ax+b)]·asinh(ax+b) - A(ax+b)·√((ax+b)²+1)
       = [x²/2 - (-1/4)]·asinh(x) - (x/4)·√(x²+1)
       = (x²/2 + 1/4)·asinh(x) - (x/4)·√(x²+1) ✓

Hmm, but in my derivation above I got B = (1/4,) which would give (x²/2 - 1/4)·asinh(x). The sign of B is critical.

Let me re-examine the reduction formula:
∫ tⁿ/√(t²+1) dt = (1/n)·tⁿ⁻¹·√(t²+1) − (n−1)/n · ∫ tⁿ⁻²/√(t²+1) dt

For n=2: A_new = (0, 1/2), then add (1/2)·A_rec where A_rec is for n=0.
n=0: base case → A=(), B=(1,). So A_rec=(), B_rec=(1,).
A_total = (1/2)·A_new + (1/2)·A_rec·scale... wait:

Wait, (n-1)/n = 1/2 for n=2. So:
A_2 = A_new + (n-1)/n · A_rec where A_new = (0, 1/2) and A_rec comes from n=0 → A_rec = ()
A_2 = (0, 1/2) + 0 = (0, 1/2)

But we scale by the coefficient of tⁿ in Q̃, which is 1/2:
A_total = (1/2) · (0, 1/2) = (0, 1/4)

B_2 = (n-1)/n · B_rec = (1/2) · (1,) = (1/2,)
B_total = (1/2) · (1/2,) = (1/4,)

So B(t) = 1/4 (positive!), which means result = [x²/2 - 1/4]·asinh(x) - (x/4)·√(x²+1).

But standard result is (x²/2 + 1/4)·asinh(x) - (x/4)·√(x²+1).

There's a sign discrepancy. Let me recheck the reduction formula.

For n=0: ∫ 1/√(t²+1) dt = asinh(t) → B = (1,), A = ()
For n=1: ∫ t/√(t²+1) dt = √(t²+1) → A = (1,), B = ()
For n=2: ∫ t²/√(t²+1) dt = ?

Integration by parts with u = t, dv = t/√(t²+1) dt:
  v = √(t²+1), du = dt
= t·√(t²+1) - ∫ √(t²+1) dt
= t·√(t²+1) - ∫ (t²+1)/√(t²+1) dt
= t·√(t²+1) - ∫ t²/√(t²+1) dt - ∫ 1/√(t²+1) dt

Let I = ∫ t²/√(t²+1) dt:
I = t·√(t²+1) - I - asinh(t)
2I = t·√(t²+1) - asinh(t)
I = (1/2)·t·√(t²+1) - (1/2)·asinh(t)

So A(t) = (0, 1/2), B(t) = (-1/2,) ... B is NEGATIVE!

So the base case for the reduction formula should give B = -1/2 for n=2, not +1/2. Something's wrong with my reduction formula.

Let me re-derive the reduction formula from scratch:
∫ tⁿ/√(t²+1) dt

Use IBP with u = tⁿ⁻¹, dv = t/√(t²+1) dt:
  v = √(t²+1), du = (n-1)·tⁿ⁻² dt

= tⁿ⁻¹·√(t²+1) - (n-1)·∫ tⁿ⁻²·√(t²+1) dt

But √(t²+1) = (t²+1)/√(t²+1), so:
= tⁿ⁻¹·√(t²+1) - (n-1)·∫ tⁿ⁻²·(t²+1)/√(t²+1) dt
= tⁿ⁻¹·√(t²+1) - (n-1)·∫ tⁿ/√(t²+1) dt - (n-1)·∫ tⁿ⁻²/√(t²+1) dt

Let Iₙ = ∫ tⁿ/√(t²+1) dt:
Iₙ = tⁿ⁻¹·√(t²+1) - (n-1)·Iₙ - (n-1)·Iₙ₋₂
n·Iₙ = tⁿ⁻¹·√(t²+1) - (n-1)·Iₙ₋₂
Iₙ = (1/n)·tⁿ⁻¹·√(t²+1) - (n-1)/n · Iₙ₋₂

OK so the formula IS: Iₙ = (1/n)·tⁿ⁻¹·√(t²+1) - (n-1)/n · Iₙ₋₂

For n=2:
I₂ = (1/2)·t·√(t²+1) - (1/2)·I₀
I₀ = asinh(t)
I₂ = (1/2)·t·√(t²+1) - (1/2)·asinh(t) ✓ (matches what I computed manually)

So A(t) = (0, 1/2), B(t) = (-1/2,).

Now in my formulation, B = (-1/2,) but in my code trace I got B = (1/2,). Let me re-examine:

In `_sqrt_integral_decompose` for ∫ 1−t² (for asin case), n=0 gives B=(1,):
That's ∫ 1/√(1-t²) dt = asin(t), so B=(1,). Correct.

For asin, n=2:
∫ t²/√(1-t²) dt

Use reduction: Iₙ = -tⁿ⁻¹/n·√(1-t²) + (n-1)/n · Iₙ₋₂
(the leading sign is NEGATIVE for asin because d/dt(√(1-t²)) = -t/√(1-t²))

For asinh, n=2:
Iₙ = +tⁿ⁻¹/n·√(t²+1) - (n-1)/n · Iₙ₋₂

The A coefficient is POSITIVE.

So for n=2 in asinh:
A_new = (0, +1/2) (degree-1 monomial with +1/2 coefficient)
B_2 = 0·A_rec from n=0 (A_rec = ()) minus (1/2)·B_rec from n=0 (B_rec = (1,))

Wait, the formula is:
Iₙ = (1/n)·tⁿ⁻¹·√(t²+1) - (n-1)/n · Iₙ₋₂

So:
A_n(t) = (0,…,0, 1/n) [constant 1/n at degree n-1] + (−(n-1)/n) · A_{n-2}(t)
B_n(t) = (−(n-1)/n) · B_{n-2}(t)

For n=2:
A_2 = (0, 1/2) + (−1/2)·A_0 = (0, 1/2) + 0 = (0, 1/2)
B_2 = (−1/2)·B_0 = (−1/2)·(1,) = (−1/2,) ✓

So the sign of the recursive term is NEGATIVE (not positive as I wrote in the plan). The reduction for asinh has:
- A_new: positive leading term `+(1/n)·tⁿ⁻¹` 
- Recursive: subtract `(n-1)/n · I_{n-2}`

This means B can be negative (as we see for n=2).

Now for the code, the asin version uses:
```python
A_new: Poly = (Fraction(0),) * (n - 1) + (Fraction(-1, n),)  # NEGATIVE
A_rec, B_rec = _monomial(n - 2)
coef = Fraction(n - 1, n)
A_total = _poly_add(A_new, _poly_scale(coef, A_rec))  # subtract A_rec ... wait, coef is positive
B_total = _poly_scale(coef, B_rec)  # positive coef
```

Wait, for asin the formula is:
∫ tⁿ/√(1-t²) dt = -tⁿ⁻¹/n·√(1-t²) + (n-1)/n · ∫ tⁿ⁻²/√(1-t²) dt

A_new = -1/n at degree n-1 (NEGATIVE leading term)
Recursive coefficient: +(n-1)/n

For asinh, the formula is:
∫ tⁿ/√(t²+1) dt = +tⁿ⁻¹/n·√(t²+1) - (n-1)/n · ∫ tⁿ⁻²/√(t²+1) dt

A_new = +1/n at degree n-1 (POSITIVE leading term)
Recursive coefficient: -(n-1)/n

So in the asinh code:
```python
A_new: Poly = (Fraction(0),) * (n - 1) + (Fraction(1, n),)  # POSITIVE (different from asin)
A_rec, B_rec = _monomial(n - 2)
coef = Fraction(n - 1, n)
A_total = _poly_add(A_new, _poly_scale(-coef, A_rec))  # SUBTRACT recursive (different from asin)
B_total = _poly_scale(-coef, B_rec)  # SUBTRACT recursive (different from asin)
```

Let me verify for n=2:
A_new = (0, 1/2)
coef = 1/2
A_0 = (), B_0 = (1,)
A_2 = (0, 1/2) + scale(-1/2, ()) = (0, 1/2) ✓
B_2 = scale(-1/2, (1,)) = (-1/2,) ✓

For n=3:
A_new = (0, 0, 1/3)
coef = 2/3
A_1 = (1,), B_1 = ()  [n=1: I₁ = √(t²+1)]

Wait for n=1: A_1 = (1,), B_1 = ()... but that can't be right because I₁ = √(t²+1) means A(t)=1 (constant), B(t)=0.

Hmm, A_1 = (1,) means A is the constant polynomial 1, so A_1(t)·√(t²+1) = 1·√(t²+1) = √(t²+1). Yes, that's degree 0 = 1 in the polynomial tuple. 

Actually I realize: the degree-0 polynomial (1,) encodes the constant 1. So A_1(t) = 1 → (1,). And the result is 1·√(t²+1) = √(t²+1). ✓

For n=3:
A_3 = (0, 0, 1/3) + scale(-2/3, (1,)) = (0, 0, 1/3) + (-2/3,) = (-2/3, 0, 1/3)
B_3 = scale(-2/3, ()) = ()

So ∫ t³/√(t²+1) dt = (-2/3 + t²/3)·√(t²+1) = (t²/3 - 2/3)·√(t²+1)

Verify: d/dt[(t²/3 - 2/3)·√(t²+1)] = (2t/3)·√(t²+1) + (t²/3 - 2/3)·t/√(t²+1)
= t[(2/3)(t²+1) + (t²/3 - 2/3)] / √(t²+1)
= t[2t²/3 + 2/3 + t²/3 - 2/3] / √(t²+1)
= t[t²] / √(t²+1) = t³/√(t²+1) ✓

Great, the formula is correct.

Now for the back-substitution in the final result:
∫ P(x)·asinh(ax+b) dx = [Q(x) - B(ax+b)]·asinh(ax+b) - A(ax+b)·√((ax+b)²+1)

For ∫ x·asinh(x) dx:
Q = x²/2, Q̃(t) = t²/2
∫ t²/2/√(t²+1) dt: scale Q̃ coefficients by A_2 and B_2:
A_total = (1/2)·A_2 = (1/2)·(0, 1/2) = (0, 1/4) → A_x = (0, 1/4) → x/4
B_total = (1/2)·B_2 = (1/2)·(-1/2,) = (-1/4,) → B_x = (-1/4,) → -1/4

Result = [x²/2 - (-1/4)]·asinh(x) - x/4·√(x²+1)
       = (x²/2 + 1/4)·asinh(x) - x/4·√(x²+1) ✓

So B can be negative, and Q - B will add when B is negative. This is all correct.

Now for acosh:
∫ tⁿ/√(t²-1) dt

Same derivation:
Iₙ = tⁿ⁻¹/n·√(t²-1) - (n-1)/n·Iₙ₋₂
Base: n=0 → acosh(t), n=1 → √(t²-1)

This is IDENTICAL structure to asinh. The formula is:
A_new = (0,...,0, 1/n) [positive]
Recursive coeff: -(n-1)/n

Same code works just by changing the IR head from ASINH to ACOSH and SQRT(arg²+1) to SQRT(arg²-1).

So in my `asinh_poly_integral.py`, I can have one `_monomial_plus` and one `_monomial_minus` that differ only in their base cases and the way the final formula assembles the SQRT inner expression. The reduction formula itself is IDENTICAL.

Actually, since both reduction formulas are the same, I could share a single `_sqrt_decompose` function that takes a variant parameter. But for clarity, I'll just have `_sqrt_plus_decompose` (uses the same recurrence but outputs A and B) and `_sqrt_minus_decompose` (same). They're actually identical functions — I can just use one function and call it for both asinh and acosh.

Let me now write the spec properly with the corrected sign pattern. 

Actually, looking at the spec again, it says:
"**leading sign is positive** (`+(1/n)·tⁿ⁻¹`)"

That part is correct. But the recursive sign should be "-" not "+". Let me check my plan:
"`_sqrt_plus_decompose(Q_tilde) → (A_t, B_t)` — apply `∫ tⁿ/√(t²+1)` reduction; base n=0→asinh coeff, n=1→sqrt coeff; **leading sign is positive** (`+(1/n)·tⁿ⁻¹`)"

This says "leading sign is positive" which refers to A_new having positive coefficient. The recursive term sign is implied by the formula. The plan is probably fine for implementation.

OK, now let me write all the code. I'll start with the spec and then move to the implementation files.

Let me now write the spec:
</thinking>
<function_calls>
<invoke name="Read">
<parameter name="file_path">/Users/adhithya/Documents/coding-adventures/.claude/worktrees/distracted-mayer-3e5208/code/packages/python/symbolic-ir/CHANGELOG.md