# An emitter that falls through to a sample value binds host data to a constant

`list<list<text>>` — the shape every table slot uses — hit `_ => fallback` in
the host-binding match on Compose, SwiftUI and Flutter. `fallback` is the
*sample* value, so the generated shell emitted:

```kotlin
deckName = mosaicString(hostProps, "deck-name", "Sample DeckName")
deckRows = emptyList()                       // a constant
```

Every neighbouring prop read from the host; this one did not. The app compiles,
launches, and shows an **empty table** whatever the host sends. Nothing errors,
nothing logs, and the artifact looks entirely reasonable in review.

A catch-all arm in an emitter is not a default, it is a silent feature drop.
When adding a slot type, check the emitted artifact **binds** it rather than
merely mentioning it — compare the generated line against a sibling slot of a
type known to work:

```bash
grep -h "deckRows\|browserResultCardIds" <emitted shell>
```

If one reads `hostProps` and the other does not, the type is unhandled. Then
compile the emitted project; matching text is not evidence.
