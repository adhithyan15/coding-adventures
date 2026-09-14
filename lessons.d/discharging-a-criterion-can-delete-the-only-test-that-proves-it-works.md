# Discharging a criterion can DELETE the only test that proves it works

Authoring sixteen verbs cleared `verb-vocabulary` for Spanish — and Spanish was the
only track in the corpus that criterion had ever failed. The suite went green with
`verbVocabularyOf` no longer asserted anywhere: delete the function, delete the
blocker, and every test still passes.

This is the failure mode of pinning a gate to real corpus data. The pin is excellent
evidence right up until somebody fixes the corpus, and then it silently becomes no
evidence at all — with no failing test to announce the transition.

**When a tranche closes a criterion's last real-world failure, it owes the criterion a
synthetic one in the same PR.** The fixture here is a track that satisfies the criterion
it partitions *exactly* and misses the composition floor by one, asserting the blocker
is its sole failure, plus the counterfactual: retag that one lesson and the level is
attained. Authoring can never turn a fixture green.

Same shape as an assertion pinned to a known-bad number that someone later fixes. Ask,
whenever a pin goes from red to green: **what was that pin also proving, and is anything
still proving it?**
