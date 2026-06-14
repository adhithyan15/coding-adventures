# adj-setcover — domain-agnostic minimum-cost set-cover (B2b)

The generic core under MYCIN's drug-regimen deriver, lifted out of medicine so any
domain can use it. The point of B2: **combination coverage and defeasance are
domain-general constructs**, not medical ones — so they live here, and medicine is
just one caller.

You give it a `SetCoverSpec`:

| field | meaning |
|---|---|
| `costs` | each element's preference cost (≥ 0) |
| `requirements` | the things that must be covered |
| `covers` | element → requirements it covers **alone** |
| `combinations` | an n-ary **subset** of elements that **jointly** cover a requirement none covers alone |
| `defeated` | observed facts that **void** a specific coverage edge |
| `excluded_by` / `exclusions` | tags that remove elements (an allergy, a policy ban) |

and `solve(spec)` emits an adj-lang integer program, runs `adj-lang-cli`
(`adj-constraint-solver`'s native min-cost set-cover), and returns the cheapest set
covering every requirement — or reports no cover exists.

## Why it's general (two domains, one library)

`examples/drug_regimen.py` and `examples/security_controls.py` call the **same**
`setcover.solve`:

```
$ python3 examples/security_controls.py
1) Nominal: CONTROL PLAN (cost 6): dlp + edr + mfa + monitoring
     + combination mfa + monitoring covers credential_theft        # defense-in-depth = n-ary combo
2) edr has a known bypass (CVE): CONTROL PLAN (cost 7): … + waf     # defeasance re-derives
3) Re-run nominal: … [cache hit]                                    # content-addressed cache

$ python3 examples/drug_regimen.py
1) Adult community: REGIMEN (cost 2): ceftriaxone + vancomycin
     + combination vancomycin + ceftriaxone covers s_pneumoniae_resistant
2) Severe beta-lactam allergy: NO regimen (infeasible) -> escalate  # exclusion → honest abstention
```

A combination conferring a capability from a *subset* (drug synergy / defense-in-
depth) and a defeated edge (drug resistance / a control bypass) are the same
mechanism in both.

## n-ary combinations scale

A k-element combination is an aux `y = AND(elements)` — **k+1 clauses, linear in k**.
With `adj-constraint-solver ≥ 0.10` (B2a) those AND-implication clauses stay on the
scalable SAT path, so combinations scale to a full element set rather than collapsing
to the LIA enumeration. We only encode **declared** combinations — the system never
*discovers* new synergies on the fly (that is a cold-path re-grounding that mints a
new fact set; the warm solve reasons only over the bounded facts it was given).

## Content-addressed caching

`solve(spec, cache_dir=…)` keys the result on `spec.content_hash()` (a stable hash
of the whole spec). A recurring scenario is an instant cache hit; **editing any fact
— a cost, an edge, a defeater, an exclusion — changes the hash and re-derives.** So
the expensive solve runs once per distinct input, and a fact change can never serve
a stale result (the same property the CAS gives the grounded libraries).

## Files

- `setcover.py` — the library (spec → emit → engine → result + cache). Validates
  every name against a token regex before emitting (no adj-lang injection); temp
  file via `mkstemp`; subprocess argv-only.
- `examples/security_controls.py` — non-medical caller (proof of generality).
- `examples/drug_regimen.py` — a compact medical caller (the grounded full formulary
  lives at `../mycin-2026/treatment/antibiotics/`).
- `test_setcover.py` — emit/hash/validation (no model) + solve/cache/defeasance/
  infeasible (skips if the CLI isn't built).
