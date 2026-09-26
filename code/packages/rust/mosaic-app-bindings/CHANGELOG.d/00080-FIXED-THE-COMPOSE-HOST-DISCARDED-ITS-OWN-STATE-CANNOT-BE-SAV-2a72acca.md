### Fixed -- the Compose host discarded its own "state cannot be saved" warning

`effectWarning` was written and never read. It is set in exactly the two places
where the host has lost the ability to persist for the rest of the process --
an effect arrived with an id nothing can answer, or effects were still
outstanding after the drain gave up -- and both messages say so in as many
words. But `withPersistenceWarning` consulted only `persistenceWarning`, so the
message went to a dead field: the user's data quietly stopped being durable
with no indication in the props, the UI, or on stderr.

A port omission rather than a decision -- the SwiftUI host it was derived from
passes `effectWarning ?? persistenceWarning` at every settle site. The effect
warning wins and is sticky, because `persistenceWarning` is cleared by the next
successful write, and announcing that saving recovered while the runtime is
still refusing to snapshot would be worse than saying nothing. It now also
reaches stderr, like every other persistence failure.

Neither condition is reachable through the conformance app, so this is fixed by
inspection against SwiftUI rather than pinned by the acceptance.

