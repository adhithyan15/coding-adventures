### …and the second bug, which only became visible once the first was fixed

The bad strip had been seeding each lesson's own word set with the stripped tail, which
incidentally stopped it reporting itself. Removing the strip removed that accident, and
**four lessons began being reported for a word sitting in their own headword** —
`مع السلامة` for `السلامة`, `bom dia` for `dia` — which is exactly what this module's
docstring says must not happen.

`ownHeadwordTokens` now does it deliberately, and completely. The accident only ever
covered the tail after the first word, so 45 self-references it had never caught are
gone too; every one was verified to be a token of the reporting lesson's own headword.

**`forwardReferences`: 524 → 443, and none of the 81 was a real finding.** What remains
is a claim about the corpus rather than about the matcher.

**The workaround this forced is removed.** Spanish chapter 41's connective lesson had
been renamed from `así que` to `así` purely to dodge the bug — `así` is three
characters, so `que` registered as first taught there and flagged ten earlier lessons.
The true headword is restored, and the full result set is byte-identical either way.

Four unit tests pin the behaviour. Three of them fail if the length rule comes back;
the fourth asserts the *positive* case — that `el pan` still registers `pan` — and
fails if `taughtWords` returns nothing. Both directions were checked by stubbing.

