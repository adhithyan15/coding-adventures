---
category: Rust
---

# A generalisation that filters a safety check's inputs to the live ones turns missing evidence into permission

Making `ca-assurance-report` package-agnostic (for gf256, whose proofs are all
`not-live`) changed `audit_context` to build its constant-time / steps-bound
sets from `is_live()` entries only. For a package where *every* proof is
`not-live` that is harmless; for a *mixed* manifest it meant the operations that
passed, plus the benchmarks, discharged CWE-400 and backed CWE-208 while other
operations had no proof at all. Main's version had used every entry and would
have failed `--check`.

Fix: the discharge conditions now require that no obligation of the kind is
unproven (`step_unproven` / `ct_unproven`), `weakest()` and the proof flags look
at all obligations, only proven entries are cited, and `--check` re-derives the
rule so the checker, not just the generator, refuses the forged report.

Do differently: when relaxing a check so a new case can pass (here, an
acknowledged absence of proof), add the relaxation as a *separate branch* for
that case and keep the original predicate over the full input set. Test the
mixed case (some pass, some not) explicitly, and make the checker enforce the
rule independently of the generator. Also make the relaxation an opt-in declared
in a reviewed input (`proofs_not_live=` in `evidence.bindings`), not something a
generated manifest can assert about itself.
