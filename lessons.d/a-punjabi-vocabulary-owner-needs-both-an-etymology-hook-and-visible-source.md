---
category: Testing & coverage
---

# A Punjabi vocabulary owner needs both an etymology hook and visible source history even when the word stays whole

Punjabi chapter 197's new vocabulary owner had verified sources for its
present-day meaning but deliberately omitted etymology rather than infer an
unsupported derivation. The Punjabi wrapper, focused integration suite, and
all generated-artifact checks passed. The full package suite then rejected one
word lesson without `etymology_hook` and one without learner-visible etymology;
Punjabi pins both counts at zero under HL24 section 9.

Every new Punjabi vocabulary owner must say where the word comes from in two
places: a concise `etymology_hook` and a visible, cited learner block. If the
history is unknown or disputed, say that plainly instead of inventing it. If a
source only supports a matching historical form, describe exactly that narrow
comparison and do not turn it into a productive modern decomposition. Run
`tests/root-ledger.test.ts` before the full suite because language wrappers do
not exercise this corpus-wide pin.

The first correction still failed the visible-etymology pin because “matching
Sanskrit source form” stated a comparison, not an origin. The learner-facing
sentence must answer the actual question with an explicit supported relation,
such as “this learned Punjabi word is from Sanskrit,” while keeping the
productive-decomposition boundary separate.
