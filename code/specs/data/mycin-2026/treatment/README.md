# Treatment as a solved constraint problem

The diagnostic layer answers *"how probable is bacterial meningitis?"* But the
practice of medicine asks a different question — *"do we treat now, and with what?"*
— and that one is **not** the argmax of the differential. Medicine treats **fast
and cheap**: missing bacterial meningitis is catastrophic, empirical antibiotics
are cheap and low-harm, and there is a hard **door-to-antibiotic deadline**. So
the treatment decision is itself a **constraint problem**, solved by the *same*
`adj-lang` constraint solver (`symbol` / `constrain` / `solve` / `check`) the
diagnostic engine already uses — at **0 answer-time model calls**.

```sh
python3 treatment/treat.py <case_id>        # diagnose, then solve the treatment decision
python3 treatment/treat.py x --p 0.30       # evaluate the policy at any P(bacterial)
python3 treatment/test_treatment.py         # CI guard
```

## The two constraints the solver solves

**(A) Cost break-even** — solve for the probability `p*` at which treating equals
waiting:

```adj
symbol p_star : scalar
observe cost_miss(100)        % harm of MISSING bacterial meningitis (death/disability)
observe cost_treat(1)         % harm/cost of empirical antibiotics if not bacterial
constrain p_star * cost_miss = cost_treat
solve for { p_star }          % -> p_star = 0.01 ; treat iff P(bacterial) >= p_star
```

Because `cost_miss >> cost_treat`, `p*` is tiny — you treat even at a low
probability. The numbers are the **policy**, and like the rulebook they are
auditable, editable inputs, not magic constants.

**(B) Time feasibility** — `check` whether **waiting** for a definitive culture
result can satisfy the door-to-antibiotic deadline:

```adj
symbol wait_strategy : scalar
observe culture_hours(48)     % time to a definitive CSF culture
observe deadline_hours(1)     % door-to-antibiotic target (IDSA ~1 h)
constrain wait_strategy = culture_hours
constrain wait_strategy <= deadline_hours
check                         % -> UNSAT (IIS core [0,1]) : you CANNOT wait
```

The solver **proves** the decision cannot be deferred — it must be made now, on
current evidence.

## The headline: cost+time override the argmax probability

This resolves the honest paradox from the diagnostic layer (the M8
cost-to-correct finding): after calibration a pre-culture case can be *more
probably viral* by base rate, yet:

```
P(bacterial)=0.30  most-probable dx = viral_meningitis
  -> ACTION: TREAT EMPIRICALLY NOW
  ^ the cost+time constraints OVERRIDE the argmax probability:
    viral is more probable, but you treat for bacterial now.
```

You act on **cost and time**, not on the argmax probability — and the
recommendation cites the solved `p*` and the binding time constraint (its IIS
core), so it is auditable end to end, with no model in the decision loop.

## Honest limits
- The cost/time parameters are an illustrative policy (utility units), not a
  validated health-economic model; they are explicit inputs precisely so a
  clinician can audit and edit them.
- Single empirical regimen, single deadline; antibiotic-choice, resistance, and
  renal-dose constraints (all natural `constrain`s) are future work.
- This is a mechanism demonstration on the meningitis differential, not a
  treatment-authorizing tool.
