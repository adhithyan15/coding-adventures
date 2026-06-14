# ADJ100 - Defensible HLE reruns: FINDINGS

**Status: pilot complete.** Two 10-item reruns over the frozen ADJ99 HLE set.

The clarified thesis is not "framework must drastically improve accuracy." It is:

> The framework should preserve reasonable accuracy while sharply reducing wrong accepted answers and
> making every accepted answer auditable from input bytes, source bytes, and executed programs.

## Headline

| run | sample | blind correct | framework mechanical correct | strict accepted | strict correct | wrong accepted |
|---|---:|---:|---:|---:|---:|---:|
| run 1 | items 1-10 | 3/10 | 4/10 | 3/10 | 2/3 | 1 |
| run 2 | items 11-20 | 1/10 | 4/10 | 3/10 | 3/3 | 0 |

Run 2 is the cleanest expression of the clarified goal: the framework improved mechanical accuracy over
blind, accepted fewer answers, and had **zero wrong accepted answers**.

Run 1 is the cautionary case: a prompt-only or too-permissive framework can still accept a defensible-
looking but gold-wrong result. The parent verifier must be strict enough to reject weak source chains,
unresolved conventions, and source/program disagreement.

## What changed after clarification

The earlier framing over-emphasized raw accuracy lift. Under that framing, the first full run looked
mixed: framework mechanical accuracy was only modestly above blind and strict acceptance was low.

After clarification, the useful metric became:

- how many answers are accepted;
- how many accepted answers are correct;
- how many wrong answers are accepted;
- whether rejected answers carry repairable reasons.

That makes the framework look more promising, because it converts many low-confidence answers into
explicit rejects instead of polished guesses.

## Run 1 texture

Run 1 used items 1-10. Summary:

- blind baseline: 3/10;
- framework mechanical: 4/10 after equivalence adjustment;
- strict accepted: 3/10;
- strict correct: 2/3;
- wrong accepted: 1.

Important observations:

- The artist item should have been rejected: source association pointed to Chagall, but exact quote
  provenance did not establish attribution; gold was Eduardo Chillida.
- The economics item exposed a benchmark disagreement: the framework derived `H - S` from standard
  profit/loss definitions, while gold was `H-T`. Under the run's acceptance rule this was accepted and
  wrong; this is the wrong-accepted failure to avoid.
- Formal/programmatic items were strongest: IP ACL, graph Cheeger constant, pavement thickness, and
  hard-core cavity energy had executable derivations or close executable checks.
- PDF provenance remained brittle. Raw PDF bytes often did not contain literal quote strings until a
  text-extraction adapter was used.

## Run 2 texture

Run 2 used items 11-20. Summary:

- blind baseline: 1/10;
- framework mechanical: 4/10;
- strict accepted: 3/10;
- strict correct: 3/3;
- wrong accepted: 0.

Accepted:

- Spanish poetry attribution: source bytes matched the Spanish lines and identified Leon Felipe /
  `El nino de Vallecas`.
- Genetics recombination: a small enumerator derived 30 and documented the orientation/haploid-sequence
  assumptions.
- Aerofoil lift ratio: one-vortex/mirror-image program derived `7/5 = 1.4`; model assumptions were
  explicit and close enough to the prompt.

Rejected but informative:

- Coding/printf item: strict C-string provenance rejected the short-object `printf` trick, while gold
  assumes a contest/platform byte layout and asks for the little-endian `s` value.
- Molecule design: RDKit verified the gold-like SMILES satisfies many descriptors but has one carbonyl,
  directly conflicting with the prompt's "avoid carbonyls" bytes.
- Representation theory: executable count produced 9, gold was 8; convention not resolved, so reject.
- GPU number format: NF4 codebook/rounding/saturation were missing from input bytes; reject.
- Minecraft: framework assumed a witch/glowstone route, gold was Snow Block; world/version/mechanics
  were under-specified, so reject.
- Puntland vessel: framework matched gold mechanically, but parent could not fetch source bytes for the
  cited vessel without relying on a dataset mirror or search snippet, so it was downgraded from strict
  acceptance.

## What we learned

1. **The framework is a precision filter, not just an accuracy engine.**
   It should say "unsupported" when bytes, rules, or conventions are missing.

2. **Wrong-accepted is the key failure metric.**
   Run 2's `0` wrong accepted answers is more important than accepting all 10.

3. **Executable derivations are the strongest wins.**
   Enumeration, descriptor checks, bit/float simulations, graph counts, and simple physics models are
   where the framework can turn reasoning into auditable programs.

4. **Prompt/gold conflicts become visible.**
   The molecule item is the clearest case: gold appears to violate an explicit negative constraint.
   The framework should surface that conflict, not hide it.

5. **Convention gaps need first-class representation.**
   NF4 semantics, C platform assumptions, representation-theory inclusion rules, and Minecraft version
   mechanics all changed the answer. If the convention is not grounded, strict acceptance should fail.

6. **Parent enforcement is non-negotiable.**
   Agent-generated "CAS-like" output is not enough. The parent must own source fetch, quote verification,
   program execution, source/program storage, and final acceptance.

## Recommended next step

Turn this pilot into a reusable acceptance harness:

- a CAS writer for input/source/program/output bytes;
- source adapters for HTML, PDF text layers, and rendered pages;
- a program runner with captured stdout/stderr hashes;
- an acceptance schema with `accepted`, `rejected_reason`, `convention_gap`, `source_gap`, and
  `would_flip_if`;
- a scorer that reports both mechanical accuracy and wrong-accepted rate.

The target metric for future runs should be:

```text
wrong_accepted_rate = wrong_strict_accepted / strict_accepted
accepted_correct_rate = correct_strict_accepted / strict_accepted
coverage = strict_accepted / total
mechanical_accuracy = framework_gold_matches / total
```

The desired direction is not maximum coverage at all costs. It is higher coverage while keeping
`wrong_accepted_rate` near zero.
