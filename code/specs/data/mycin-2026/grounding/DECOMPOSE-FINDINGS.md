# Source decomposition — pending citations cleared, fix-up frontier surfaced

The recursion behind "nothing on blind trust" is now **complete for the current corpus**:
every source any grounded fact cites has been fetched, decomposed into byte-provenanced
claims, and committed to the CAS, and every citation is checked against its *independently*
decomposed source.

- **62 source objects · 673 byte-provenanced claims** in `cas/sources/`.
- Citation verification: **36 fully verified · 4 core-verified (composite over-reach) ·
  31 unverified · 0 pending.**

## The check did its job: 12 grounded facts cite a span not in the independent decomposition

A citation is only fully trustworthy if the fact's byte-quote appears in the source as an
*independent* reader decomposed it — not just as it looked on the live page when grounded
(a quote can be cherry-picked or a different sentence than the decomposer captured). Of the
**44 grounded (ACCEPT) facts, 32 verbatim/core-verify; 12 do not** — these are the fix-up
frontier (the FLAG facts that don't verify are expected and already untrusted):

`host_neonate_gnb` · `dose_cefepime` · `dose_meropenem` · `uti_prior_saprophyticus` ·
`uti_prior_pseudomonas` · `uti_prior_gbs` · `uti_finding_urease_proteus` ·
`ci_tmpsmx_pregnancy` · `ci_vancomycin_renal_dose` · `bsi_prior_spneumoniae` ·
`bsi_prior_pseudomonas` · `src_skin_saureus`

These split into two modes (the fix differs):

1. **Different sentence, same fact (corroborated).** The grounding cited one verbatim span;
   the independent decomposition captured a *different* span stating the same fact (e.g.
   `host_neonate_gnb` — Merck's "Key Points" summary vs the body "predominant pathogens" list;
   the same source independently lists E. coli for neonates). The fact IS supported — just not
   by the *same* sentence. Fix: accept at the entailment tier, or re-point the citation at the
   decomposed span.
2. **Genuinely cherry-picked / over-reached.** The decomposed source does not state the fact
   as cited (often a value from a different population/indication than the source's headline
   figure). Fix: re-ground from a source that states it, or demote the verdict to
   `direction_only` (FLAG) — never keep ACCEPT for a citation the independent reading can't
   confirm.

**Next iteration (fix-up pass):** for each of the 12, run a content-corroboration re-check
(does the decomposed source contain a claim about the same organism/value?) → mode 1 re-points
the citation, mode 2 demotes to FLAG + queues re-grounding. The honest invariant to add: a
fact is ACCEPT only if its citation verifies (verbatim or core) against the *decomposed*
source, not merely byte-stable on the live page at grounding time.
