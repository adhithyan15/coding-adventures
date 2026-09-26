### Fixed -- `restore` returned the update that arrived, not the one it stored

Settling answers effects, and answers move the app, so the raw update's props
are the ones from before that happened and its `effects` list names effects
already discharged. A caller rendering the return value would show state that
`props()` disagrees with. Qt and SwiftUI both return the settled update; this
host returned the raw one, and additionally skipped the persistence warning
that `handleEvent` applies. Both now match.

Not exercised by the acceptance: the conformance app's `restore` mints no
effects, so the divergence is unreachable through that fixture. It is a
consistency fix against the two shipped hosts, verified by compilation only.

