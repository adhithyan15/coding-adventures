# A curriculum gate can silently constrain what may be authored

Chasing "600 headwords at or below A1", ten fully-verified verb candidates were dropped —
not for duplication, not for etymology, but because **no A1 spine node can host a verb**.
`vocabularyOf` credits a headword to a level through its curriculum segment's spine node,
and the three A1 nodes in use are "mark out a specific known thing", "ask where something
is" and "the numbers one to five". A `canDo` reading "I can say *aprender*, *olvidar* and
*necesitar*, and mark out which specific thing I mean" is a slot being filled, not a
capability.

Nothing reports this. The gate counts headwords and is satisfied; the fact that the only
headwords it can count are nouns and adjectives is invisible to it, and the resulting
curriculum shape is one nobody chose.

**Generalisable check:** when a metric is defined by a join against a taxonomy, the taxonomy
silently bounds what can be built. Ask what the metric *cannot* count before assuming a
shortfall is a content problem.
