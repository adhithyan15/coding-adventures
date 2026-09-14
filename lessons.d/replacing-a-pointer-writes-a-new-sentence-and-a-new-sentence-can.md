# Replacing a pointer writes a new sentence, and a new sentence can be false

Removing "you met this in the Spanish track" means composing something in its place.
Three of my replacements asserted things the originals never had:

- *"the one European blue that is not a Germanic loan at all"* — Spanish's *azul* is
  Arabic too, and the corpus says so two files away.
- *"Latin's own vocabulary family for age and vigour"* — Latin has no such family.
  My *second* attempt, *"narrowed to force alone"*, was also wrong and contradicted
  the same lesson's body three lines down. The body already said it: the two words
  split toward different senses. **Read the rest of the file before rewriting a
  claim in its frontmatter.**
- *"the PIE-root/nox comparison is not independently attested for them"* — Kannada's
  and Telugu's *rātri* is the identical Sanskrit word, so it is attested exactly as
  well. The gap was in the CURRICULUM, not in the scholarship.

Two more turned up in a later review pass — a widening attributed to Malayalam when
the corpus names Malayalam as the counter-example, and a word said to *survive* in
its own ancestor language. All five have the same shape: a statement about **the books** rewritten as a
statement about **the world**. The pointer was the only thing that made the original
true, so deleting it silently widened the claim.

**Rule:** when the fix is a rewrite rather than a deletion, check the new sentence
against the corpus and against fact — not just against grammar. And prefer the
narrower true claim ("a second, separate road to blue") over the tidier absolute
("the one European blue"), because the absolute is the one that will be wrong.

**Corollary on guards:** a guard reading `canonicalLessonSource` is reading JSON, not
prose. Line breaks are the escape `\n`, so `\s+` cannot cross a wrap and `[^.?!\n]`
bounds nothing. Both of my newline-aware protections were inert on the surface I had
just added, and a real defect was sitting in the blind spot. Un-escape before matching
— and when a guard is extended to a new surface, re-prove it on THAT surface.
