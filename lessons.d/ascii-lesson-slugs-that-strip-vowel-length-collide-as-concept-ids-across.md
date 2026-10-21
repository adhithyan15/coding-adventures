---
category: Testing & coverage
---

# ASCII lesson slugs that strip vowel length collide as concept ids across vocabulary tranches

Gujarati A2's four vocabulary tranches were drafted in parallel. Each
drafter slugified its own headwords, and the builder derives the concept id
from the slug (GU-VERB-<SLUG>). Romanization strips vowel length and
retroflex marks, so four verb pairs from different tranches got the same id:
વાળવું/વળવું (valvu), ઉતારવું/ઊતરવું (utarvu), મરવું/મારવું (marvu) and
લાદવું/લડવું (ladvu). `cli validate` reported four duplicate-realization
errors only after generation.

The fix renamed the later lesson's slug in each pair (valvu-turn, uutarvu,
maarvu, ladvu-fight). The generated output was reset to the committed core
and regenerated.

What to do differently: before generating, check concept-id uniqueness
across every tranche spec and the existing track. Slugify all specs
together, upper-case the slugs as the builder does, and fail on any repeat.
For languages with length or retroflex contrasts, prefer doubled vowels (aa,
uu) or a short gloss suffix in slugs from the start.
