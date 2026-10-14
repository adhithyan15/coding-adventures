## Unreleased — supported Railroad spacing budget

Export MAX_RAILROAD_SPACING=65536 for the three RailroadConfig fields. Parser
and typed-layout admission share the finite inclusive range; accepted settings
are retained exactly. Preserve the producer's default10/8/10 geometry intent.
No new crates.io dependencies; Miri DEFERRED/PENDING, never passed.
