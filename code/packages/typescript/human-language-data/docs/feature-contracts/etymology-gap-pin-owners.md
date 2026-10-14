# Etymology-gap pin owners

The etymology coverage gate is an exact debt ratchet. Every word lesson must
carry both an `etymology_hook` and learner-visible origin prose; a track's two
pins record the remaining lessons that lack either half. A count may fall, but
it must never rise.

Each track owns one module under
`tests/etymology-gap-pin-owners/<language>.owner.ts`. The module carries the
canonical lowercase language slug, the exact `withoutHook` and
`withoutVisible` values, and the track-specific rationale or source audit that
supports the current boundary. Lowering one track's debt edits only that file.

The shared root-ledger test loads the corpus once, measures coverage once, and
discovers these modules through a bounded glob. The loader compares the owner
set with the tracks in the measured corpus, so a missing or unknown track fails
without a hand-maintained index or total-owner count. It also refuses unsafe or
reserved names, links, non-regular files, case collisions, filename/declaration
mismatches, extra exports or fields, invalid numbers, and empty rationale.

When a pin falls, update the relevant owner in the same change and explain the
evidence in its rationale. When it rises, fix the offending lessons. Never
raise a pin, add a shared registry, or move per-track provenance back into
`root-ledger.test.ts`.
