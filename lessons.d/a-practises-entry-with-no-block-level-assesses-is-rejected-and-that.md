# A `practises` entry with no block-level `assesses` is rejected, and that gate is the point (human-language-data)

Closing spaced-retrieval windows looked like a frontmatter edit: add the atom to
`practises:` → `knowledge: [...]` and the measurement closes. The validator refused:

    ERROR [schema-v2-block-assessment-missing] ES-C05-hasta-luego:
      practised atom 'ES-LEX-ADIOS' is not assessed by any body block

That rule enforces the honesty principle HL09 §7.2 states in prose: **you cannot claim
practice without pointing at where it happens.** The frontmatter-only edit would have
closed the metric while helping no learner — a hollow claim that reads as progress.

The real work is two edits per atom: the frontmatter list **and** the
`assesses=[...]` of the specific `<!-- hl-knowledge: -->` directive on the block whose
prose actually exercises it. Of 58 open windows in Spanish chapters 3–6, only **17**
had prose to point at; the other 41 were genuine absence and were left open. A low hit
rate is the honest result, not a failure of the pass.
